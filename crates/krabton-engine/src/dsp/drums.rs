//! Drums: a drum rack. Every pad plays one voice: Krabton's own drum
//! synthesis (Classic), a voice after a classic drum machine (808, 909,
//! CR-78, electronic toms; see `machines`), a Mutable Instruments drum model (the 808-style and
//! synthetic kicks, snares and hi-hats of Plaits, MIT licensed, through
//! mi-plaits-dsp), a PhISEM shaker (see `shakers`), or a sample.

use super::acoustic::AcousticVoice;
use super::retro::RetroVoice;
use super::chip_drums::{self, ChipKnobs, ChipVoice};
use super::circuit_kick::{CircuitKick, KickKnobs};
use super::macro_drums::{self, MacroVoice};
use super::peaks::{self, PeaksVoice};
use super::machines::MachineVoice;
use super::shakers::{self, Shaker, ShakerModel};
use super::*;
use crate::project::DrumPad;
use mi_plaits_dsp::drums::analog_bass_drum::AnalogBassDrum;
use mi_plaits_dsp::drums::analog_snare_drum::AnalogSnareDrum;
use mi_plaits_dsp::drums::hihat::{Hihat, NoiseType, VcaType};
use mi_plaits_dsp::drums::synthetic_bass_drum::SyntheticBassDrum;
use mi_plaits_dsp::drums::synthetic_snare_drum::SyntheticSnareDrum;
use mi_plaits_dsp::fx::overdrive::Overdrive;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrumVoice {
    Kick,
    Snare,
    ClosedHat,
    OpenHat,
    Clap,
    Tom,
    Rim,
    Cowbell,
    AnalogKick,
    SynthKick,
    AnalogSnare,
    SynthSnare,
    MetalHat,
    RingHat,
    Maraca,
    Cabasa,
    Sekere,
    Tambourine,
    SleighBells,
    Bamboo,
    Sandpaper,
    Can,
    Crunch,
    Rocks,
    Gravel,
    M808Kick,
    M808Snare,
    M808Tom,
    M808Conga,
    M808Rim,
    M808Clap,
    M808Claves,
    M808Maracas,
    M808Cowbell,
    M808Cymbal,
    M808ClosedHat,
    M808OpenHat,
    M909Kick,
    M909Snare,
    M909Tom,
    M909Rim,
    M909Clap,
    M909ClosedHat,
    M909OpenHat,
    Cr78Kick,
    Cr78Snare,
    Cr78Hat,
    Cr78Cymbal,
    Cr78Bongo,
    Cr78Guiro,
    Cr78Tambourine,
    Cr78Metal,
    ElectroTom,
    M606Kick,
    M606Snare,
    M606LowTom,
    M606HighTom,
    M606ClosedHat,
    M606OpenHat,
    M606Cymbal,
    FmKick,
    FmSnare,
    FmTom,
    FmHat,
    FmBell,
    FmZap,
    FmPerc,
    FmNoise,
    AcKick,
    AcSnare,
    AcBrush,
    AcStick,
    AcTomHigh,
    AcTomMid,
    AcTomFloor,
    AcHatClosed,
    AcHatPedal,
    AcHatOpen,
    AcRide,
    AcRideBell,
    AcCrash,
    AcChina,
    RbKick,
    RbSnare,
    RbHat,
    RbCymbal,
    RbBongoHigh,
    RbBongoLow,
    RbClaves,
    RbGuiro,
    RbMaracas,
    HxKick,
    HxSnare,
    HxTomHigh,
    HxTomMid,
    HxTomLow,
    DgKick,
    DgSnare,
    DgClap,
    DgRim,
    DgHat,
    DgOpenHat,
    DgTomHigh,
    DgTomLow,
    DgCowbell,
    PkKick,
    PkSnare,
    PkHat,
    PkOpenHat,
    PkCymbal,
    PkClap,
    RsKick,
    RsSnare,
    RsHat,
    RsFmKick,
    RsFmSnare,
    RsCircuitKick,
    MkKick,
    MkSnare,
    MkHat,
    MkClap,
    MkTom,
    MkRim,
    MkFm,
    MkNoise,
    MkCowbell,
    MkCymbal,
    MkShaker,
    MkClave,
    ChipConsoleKick,
    ChipConsoleSampleKick,
    ChipConsoleSnare,
    ChipConsoleSampleSnare,
    ChipConsoleHat,
    ChipConsoleOpenHat,
    ChipConsoleTom,
    ChipConsoleCrash,
    ChipHandheldKick,
    ChipHandheldSnare,
    ChipHandheldHat,
    ChipHandheldOpenHat,
    ChipHandheldTom,
    ChipHandheldClap,
    ChipMicroKick,
    ChipMicroSnare,
    ChipMicroTom,
    ChipMicroClap,
    ChipMicroRim,
    ChipMicroHat,
    ChipMicroOpenHat,
    ChipMicroPunchKick,
    ChipMicroPunchSnare,
    ChipMicroPunchClap,
    ChipMicroPunchRim,
    ChipMicroPunchHat,
    ChipMicroPunchOpenHat,
    ChipMicroPunchTom,
    ChipArcadeKick,
    ChipArcadeSnare,
    ChipArcadeTom,
    ChipArcadeHat,
    ChipArcadeOpenHat,
    ChipArcadeCowbell,
    ChipBeeperKick,
    ChipBeeperSnare,
    ChipBeeperHat,
    ChipBeeperTom,
    ChipWoodKick,
    ChipWoodSnare,
    ChipWoodHat,
    ChipWoodTom,
    ChipWoodZap,
    ChipWoodBoom,
}

/// Where a voice comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Classic,
    Machine,
    Mutable,
    Shaker,
    /// Modelled acoustic drums (see `acoustic`).
    Acoustic,
    /// More drum machines (see `retro`).
    Retro,
    /// Drums that ring (see `peaks` and `circuit_kick`).
    Resonant,
    /// Twelve models with macros (see `macro_drums`).
    Macro,
    /// The old chips' drums (see `chip_drums`).
    Chip,
}

impl DrumVoice {
    pub const CLASSIC: [DrumVoice; 8] = [
        DrumVoice::Kick,
        DrumVoice::Snare,
        DrumVoice::ClosedHat,
        DrumVoice::OpenHat,
        DrumVoice::Clap,
        DrumVoice::Tom,
        DrumVoice::Rim,
        DrumVoice::Cowbell,
    ];
    pub const MUTABLE: [DrumVoice; 6] = [DrumVoice::AnalogKick, DrumVoice::SynthKick, DrumVoice::AnalogSnare, DrumVoice::SynthSnare, DrumVoice::MetalHat, DrumVoice::RingHat];
    /// In the order of `shakers::MODELS`.
    pub const SHAKERS: [DrumVoice; 11] = [
        DrumVoice::Maraca,
        DrumVoice::Cabasa,
        DrumVoice::Sekere,
        DrumVoice::Tambourine,
        DrumVoice::SleighBells,
        DrumVoice::Bamboo,
        DrumVoice::Sandpaper,
        DrumVoice::Can,
        DrumVoice::Crunch,
        DrumVoice::Rocks,
        DrumVoice::Gravel,
    ];
    pub const M808: [DrumVoice; 12] = [
        DrumVoice::M808Kick,
        DrumVoice::M808Snare,
        DrumVoice::M808Tom,
        DrumVoice::M808Conga,
        DrumVoice::M808Rim,
        DrumVoice::M808Clap,
        DrumVoice::M808Claves,
        DrumVoice::M808Maracas,
        DrumVoice::M808Cowbell,
        DrumVoice::M808Cymbal,
        DrumVoice::M808ClosedHat,
        DrumVoice::M808OpenHat,
    ];
    pub const M909: [DrumVoice; 7] = [DrumVoice::M909Kick, DrumVoice::M909Snare, DrumVoice::M909Tom, DrumVoice::M909Rim, DrumVoice::M909Clap, DrumVoice::M909ClosedHat, DrumVoice::M909OpenHat];
    pub const CR78: [DrumVoice; 8] = [
        DrumVoice::Cr78Kick,
        DrumVoice::Cr78Snare,
        DrumVoice::Cr78Hat,
        DrumVoice::Cr78Cymbal,
        DrumVoice::Cr78Bongo,
        DrumVoice::Cr78Guiro,
        DrumVoice::Cr78Tambourine,
        DrumVoice::Cr78Metal,
    ];
    pub const ELECTRO: [DrumVoice; 1] = [DrumVoice::ElectroTom];
    pub const M606: [DrumVoice; 7] = [DrumVoice::M606Kick, DrumVoice::M606Snare, DrumVoice::M606LowTom, DrumVoice::M606HighTom, DrumVoice::M606ClosedHat, DrumVoice::M606OpenHat, DrumVoice::M606Cymbal];
    pub const FM: [DrumVoice; 8] = [DrumVoice::FmKick, DrumVoice::FmSnare, DrumVoice::FmTom, DrumVoice::FmHat, DrumVoice::FmBell, DrumVoice::FmZap, DrumVoice::FmPerc, DrumVoice::FmNoise];
    pub const ACOUSTIC: [DrumVoice; 14] = [
        DrumVoice::AcKick,
        DrumVoice::AcSnare,
        DrumVoice::AcBrush,
        DrumVoice::AcStick,
        DrumVoice::AcTomHigh,
        DrumVoice::AcTomMid,
        DrumVoice::AcTomFloor,
        DrumVoice::AcHatClosed,
        DrumVoice::AcHatPedal,
        DrumVoice::AcHatOpen,
        DrumVoice::AcRide,
        DrumVoice::AcRideBell,
        DrumVoice::AcCrash,
        DrumVoice::AcChina,
    ];
    /// Hand percussion that goes with an acoustic kit.
    pub const KIT_PERC: [DrumVoice; 4] = [DrumVoice::Tambourine, DrumVoice::Cowbell, DrumVoice::Clap, DrumVoice::Maraca];
    pub const RETRO: [DrumVoice; 29] = [DrumVoice::RbKick, DrumVoice::RbSnare, DrumVoice::RbHat, DrumVoice::RbCymbal, DrumVoice::RbBongoHigh, DrumVoice::RbBongoLow, DrumVoice::RbClaves, DrumVoice::RbGuiro, DrumVoice::RbMaracas, DrumVoice::HxKick, DrumVoice::HxSnare, DrumVoice::HxTomHigh, DrumVoice::HxTomMid, DrumVoice::HxTomLow, DrumVoice::DgKick, DrumVoice::DgSnare, DrumVoice::DgClap, DrumVoice::DgRim, DrumVoice::DgHat, DrumVoice::DgOpenHat, DrumVoice::DgTomHigh, DrumVoice::DgTomLow, DrumVoice::DgCowbell, DrumVoice::PkKick, DrumVoice::PkSnare, DrumVoice::PkHat, DrumVoice::PkOpenHat, DrumVoice::PkCymbal, DrumVoice::PkClap];
    pub const RESONANT: [DrumVoice; 6] = [DrumVoice::RsKick, DrumVoice::RsSnare, DrumVoice::RsHat, DrumVoice::RsFmKick, DrumVoice::RsFmSnare, DrumVoice::RsCircuitKick];
    pub const MACRO: [DrumVoice; 12] = [DrumVoice::MkKick, DrumVoice::MkSnare, DrumVoice::MkHat, DrumVoice::MkClap, DrumVoice::MkTom, DrumVoice::MkRim, DrumVoice::MkFm, DrumVoice::MkNoise, DrumVoice::MkCowbell, DrumVoice::MkCymbal, DrumVoice::MkShaker, DrumVoice::MkClave];
    pub const CHIP: [DrumVoice; 44] = [DrumVoice::ChipConsoleKick, DrumVoice::ChipConsoleSampleKick, DrumVoice::ChipConsoleSnare, DrumVoice::ChipConsoleSampleSnare, DrumVoice::ChipConsoleHat, DrumVoice::ChipConsoleOpenHat, DrumVoice::ChipConsoleTom, DrumVoice::ChipConsoleCrash, DrumVoice::ChipHandheldKick, DrumVoice::ChipHandheldSnare, DrumVoice::ChipHandheldHat, DrumVoice::ChipHandheldOpenHat, DrumVoice::ChipHandheldTom, DrumVoice::ChipHandheldClap, DrumVoice::ChipMicroKick, DrumVoice::ChipMicroSnare, DrumVoice::ChipMicroTom, DrumVoice::ChipMicroClap, DrumVoice::ChipMicroRim, DrumVoice::ChipMicroHat, DrumVoice::ChipMicroOpenHat, DrumVoice::ChipMicroPunchKick, DrumVoice::ChipMicroPunchSnare, DrumVoice::ChipMicroPunchClap, DrumVoice::ChipMicroPunchRim, DrumVoice::ChipMicroPunchHat, DrumVoice::ChipMicroPunchOpenHat, DrumVoice::ChipMicroPunchTom, DrumVoice::ChipArcadeKick, DrumVoice::ChipArcadeSnare, DrumVoice::ChipArcadeTom, DrumVoice::ChipArcadeHat, DrumVoice::ChipArcadeOpenHat, DrumVoice::ChipArcadeCowbell, DrumVoice::ChipBeeperKick, DrumVoice::ChipBeeperSnare, DrumVoice::ChipBeeperHat, DrumVoice::ChipBeeperTom, DrumVoice::ChipWoodKick, DrumVoice::ChipWoodSnare, DrumVoice::ChipWoodHat, DrumVoice::ChipWoodTom, DrumVoice::ChipWoodZap, DrumVoice::ChipWoodBoom];
    /// The voices as the pad menu groups them.
    pub const GROUPS: [(&'static str, &'static [DrumVoice]); 14] = [
        ("Acoustic", &Self::ACOUSTIC),
        ("Retro", &Self::RETRO),
        ("Classic", &Self::CLASSIC),
        ("808", &Self::M808),
        ("909", &Self::M909),
        ("606", &Self::M606),
        ("FM", &Self::FM),
        ("CR-78", &Self::CR78),
        ("Electronic", &Self::ELECTRO),
        ("Mutable", &Self::MUTABLE),
        ("Shakers", &Self::SHAKERS),
        ("Resonant", &Self::RESONANT),
        ("Macro", &Self::MACRO),
        ("Chip", &Self::CHIP),
    ];

    pub fn family(self) -> Family {
        if Self::M808.contains(&self) || Self::M909.contains(&self) || Self::CR78.contains(&self) || Self::ELECTRO.contains(&self) || Self::M606.contains(&self) || Self::FM.contains(&self) {
            Family::Machine
        } else if Self::MUTABLE.contains(&self) {
            Family::Mutable
        } else if Self::SHAKERS.contains(&self) {
            Family::Shaker
        } else if Self::ACOUSTIC.contains(&self) {
            Family::Acoustic
        } else if Self::RETRO.contains(&self) {
            Family::Retro
        } else if Self::RESONANT.contains(&self) {
            Family::Resonant
        } else if Self::MACRO.contains(&self) {
            Family::Macro
        } else if Self::CHIP.contains(&self) {
            Family::Chip
        } else {
            Family::Classic
        }
    }

    fn shaker(self) -> Option<&'static ShakerModel> {
        Self::SHAKERS.iter().position(|v| *v == self).map(|i| &shakers::MODELS[i])
    }

    pub fn name(self) -> &'static str {
        if let Some(m) = self.shaker() {
            return m.name;
        }
        match self {
            DrumVoice::Kick => "Kick",
            DrumVoice::Snare => "Snare",
            DrumVoice::ClosedHat => "Closed Hat",
            DrumVoice::OpenHat => "Open Hat",
            DrumVoice::Clap => "Clap",
            DrumVoice::Tom => "Tom",
            DrumVoice::Rim => "Rim",
            DrumVoice::Cowbell => "Cowbell",
            DrumVoice::AnalogKick => "Analog Kick",
            DrumVoice::SynthKick => "Synth Kick",
            DrumVoice::AnalogSnare => "Analog Snare",
            DrumVoice::SynthSnare => "Synth Snare",
            DrumVoice::MetalHat => "Metal Hat",
            DrumVoice::RingHat => "Ring Hat",
            DrumVoice::M808Kick => "808 Kick",
            DrumVoice::M808Snare => "808 Snare",
            DrumVoice::M808Tom => "808 Tom",
            DrumVoice::M808Conga => "808 Conga",
            DrumVoice::M808Rim => "808 Rim",
            DrumVoice::M808Clap => "808 Clap",
            DrumVoice::M808Claves => "808 Claves",
            DrumVoice::M808Maracas => "808 Maracas",
            DrumVoice::M808Cowbell => "808 Cowbell",
            DrumVoice::M808Cymbal => "808 Cymbal",
            DrumVoice::M808ClosedHat => "808 Closed Hat",
            DrumVoice::M808OpenHat => "808 Open Hat",
            DrumVoice::M909Kick => "909 Kick",
            DrumVoice::M909Snare => "909 Snare",
            DrumVoice::M909Tom => "909 Tom",
            DrumVoice::M909Rim => "909 Rim",
            DrumVoice::M909Clap => "909 Clap",
            DrumVoice::M909ClosedHat => "909 Closed Hat",
            DrumVoice::M909OpenHat => "909 Open Hat",
            DrumVoice::Cr78Kick => "CR-78 Kick",
            DrumVoice::Cr78Snare => "CR-78 Snare",
            DrumVoice::Cr78Hat => "CR-78 Hat",
            DrumVoice::Cr78Cymbal => "CR-78 Cymbal",
            DrumVoice::Cr78Bongo => "CR-78 Bongo",
            DrumVoice::Cr78Guiro => "CR-78 Guiro",
            DrumVoice::Cr78Tambourine => "CR-78 Tambourine",
            DrumVoice::Cr78Metal => "CR-78 Metal",
            DrumVoice::ElectroTom => "Electro Tom",
            DrumVoice::M606Kick => "606 Kick",
            DrumVoice::M606Snare => "606 Snare",
            DrumVoice::M606LowTom => "606 Low Tom",
            DrumVoice::M606HighTom => "606 High Tom",
            DrumVoice::M606ClosedHat => "606 Closed Hat",
            DrumVoice::M606OpenHat => "606 Open Hat",
            DrumVoice::M606Cymbal => "606 Cymbal",
            DrumVoice::FmKick => "FM Kick",
            DrumVoice::FmSnare => "FM Snare",
            DrumVoice::FmTom => "FM Tom",
            DrumVoice::FmHat => "FM Hat",
            DrumVoice::FmBell => "FM Bell",
            DrumVoice::FmZap => "FM Zap",
            DrumVoice::FmPerc => "FM Perc",
            DrumVoice::FmNoise => "FM Noise",
            DrumVoice::AcKick => "Kick Drum",
            DrumVoice::AcSnare => "Snare Drum",
            DrumVoice::AcBrush => "Brush Snare",
            DrumVoice::AcStick => "Side Stick",
            DrumVoice::AcTomHigh => "High Tom",
            DrumVoice::AcTomMid => "Mid Tom",
            DrumVoice::AcTomFloor => "Floor Tom",
            DrumVoice::AcHatClosed => "Hi-Hat Closed",
            DrumVoice::AcHatPedal => "Hi-Hat Pedal",
            DrumVoice::AcHatOpen => "Hi-Hat Open",
            DrumVoice::AcRide => "Ride",
            DrumVoice::AcRideBell => "Ride Bell",
            DrumVoice::AcCrash => "Crash",
            DrumVoice::AcChina => "China",
            DrumVoice::RbKick => "Box Kick",
            DrumVoice::RbSnare => "Box Snare",
            DrumVoice::RbHat => "Box Hat",
            DrumVoice::RbCymbal => "Box Cymbal",
            DrumVoice::RbBongoHigh => "Box Bongo High",
            DrumVoice::RbBongoLow => "Box Bongo Low",
            DrumVoice::RbClaves => "Box Claves",
            DrumVoice::RbGuiro => "Box Guiro",
            DrumVoice::RbMaracas => "Box Maracas",
            DrumVoice::HxKick => "Hex Kick",
            DrumVoice::HxSnare => "Hex Snare",
            DrumVoice::HxTomHigh => "Hex Tom High",
            DrumVoice::HxTomMid => "Hex Tom Mid",
            DrumVoice::HxTomLow => "Hex Tom Low",
            DrumVoice::DgKick => "Digital Kick",
            DrumVoice::DgSnare => "Digital Snare",
            DrumVoice::DgClap => "Digital Clap",
            DrumVoice::DgRim => "Digital Rim",
            DrumVoice::DgHat => "Digital Hat",
            DrumVoice::DgOpenHat => "Digital Open Hat",
            DrumVoice::DgTomHigh => "Digital Tom High",
            DrumVoice::DgTomLow => "Digital Tom Low",
            DrumVoice::DgCowbell => "Digital Cowbell",
            DrumVoice::PkKick => "Pocket Kick",
            DrumVoice::PkSnare => "Pocket Snare",
            DrumVoice::PkHat => "Pocket Hat",
            DrumVoice::PkOpenHat => "Pocket Open Hat",
            DrumVoice::PkCymbal => "Pocket Cymbal",
            DrumVoice::PkClap => "Pocket Clap",
            DrumVoice::RsKick => "Resonant Kick",
            DrumVoice::RsSnare => "Resonant Snare",
            DrumVoice::RsHat => "Resonant Hat",
            DrumVoice::RsFmKick => "FM Drum",
            DrumVoice::RsFmSnare => "FM Snare",
            DrumVoice::RsCircuitKick => "Circuit Kick",
            DrumVoice::MkKick => "Macro Kick",
            DrumVoice::MkSnare => "Macro Snare",
            DrumVoice::MkHat => "Macro Hat",
            DrumVoice::MkClap => "Macro Clap",
            DrumVoice::MkTom => "Macro Tom",
            DrumVoice::MkRim => "Macro Rim",
            DrumVoice::MkFm => "Macro FM",
            DrumVoice::MkNoise => "Macro Noise",
            DrumVoice::MkCowbell => "Macro Cowbell",
            DrumVoice::MkCymbal => "Macro Cymbal",
            DrumVoice::MkShaker => "Macro Shaker",
            DrumVoice::MkClave => "Macro Clave",
            DrumVoice::ChipConsoleKick => "Console Kick",
            DrumVoice::ChipConsoleSampleKick => "Console Sample Kick",
            DrumVoice::ChipConsoleSnare => "Console Snare",
            DrumVoice::ChipConsoleSampleSnare => "Console Sample Snare",
            DrumVoice::ChipConsoleHat => "Console Hat",
            DrumVoice::ChipConsoleOpenHat => "Console Open Hat",
            DrumVoice::ChipConsoleTom => "Console Tom",
            DrumVoice::ChipConsoleCrash => "Console Crash",
            DrumVoice::ChipHandheldKick => "Handheld Kick",
            DrumVoice::ChipHandheldSnare => "Handheld Snare",
            DrumVoice::ChipHandheldHat => "Handheld Hat",
            DrumVoice::ChipHandheldOpenHat => "Handheld Open Hat",
            DrumVoice::ChipHandheldTom => "Handheld Tom",
            DrumVoice::ChipHandheldClap => "Handheld Clap",
            DrumVoice::ChipMicroKick => "Micro Kick",
            DrumVoice::ChipMicroSnare => "Micro Snare",
            DrumVoice::ChipMicroTom => "Micro Tom",
            DrumVoice::ChipMicroClap => "Micro Clap",
            DrumVoice::ChipMicroRim => "Micro Rim",
            DrumVoice::ChipMicroHat => "Micro Hat",
            DrumVoice::ChipMicroOpenHat => "Micro Open Hat",
            DrumVoice::ChipMicroPunchKick => "Micro Punch Kick",
            DrumVoice::ChipMicroPunchSnare => "Micro Punch Snare",
            DrumVoice::ChipMicroPunchClap => "Micro Punch Clap",
            DrumVoice::ChipMicroPunchRim => "Micro Punch Rim",
            DrumVoice::ChipMicroPunchHat => "Micro Punch Hat",
            DrumVoice::ChipMicroPunchOpenHat => "Micro Punch Open Hat",
            DrumVoice::ChipMicroPunchTom => "Micro Punch Tom",
            DrumVoice::ChipArcadeKick => "Arcade Kick",
            DrumVoice::ChipArcadeSnare => "Arcade Snare",
            DrumVoice::ChipArcadeTom => "Arcade Tom",
            DrumVoice::ChipArcadeHat => "Arcade Hat",
            DrumVoice::ChipArcadeOpenHat => "Arcade Open Hat",
            DrumVoice::ChipArcadeCowbell => "Arcade Cowbell",
            DrumVoice::ChipBeeperKick => "Beeper Kick",
            DrumVoice::ChipBeeperSnare => "Beeper Snare",
            DrumVoice::ChipBeeperHat => "Beeper Hat",
            DrumVoice::ChipBeeperTom => "Beeper Tom",
            DrumVoice::ChipWoodKick => "Woodgrain Kick",
            DrumVoice::ChipWoodSnare => "Woodgrain Snare",
            DrumVoice::ChipWoodHat => "Woodgrain Hat",
            DrumVoice::ChipWoodTom => "Woodgrain Tom",
            DrumVoice::ChipWoodZap => "Woodgrain Zap",
            DrumVoice::ChipWoodBoom => "Woodgrain Boom",
            _ => "",
        }
    }

    /// General MIDI note for this voice, so MIDI drum controllers just work.
    pub fn gm_note(self) -> u8 {
        match self {
            DrumVoice::Kick | DrumVoice::AnalogKick => 36,
            DrumVoice::SynthKick => 35,
            DrumVoice::Rim => 37,
            DrumVoice::Snare | DrumVoice::AnalogSnare => 38,
            DrumVoice::SynthSnare => 40,
            DrumVoice::Clap => 39,
            DrumVoice::ClosedHat | DrumVoice::MetalHat => 42,
            DrumVoice::Tom => 45,
            DrumVoice::OpenHat | DrumVoice::RingHat => 46,
            DrumVoice::Cowbell => 56,
            DrumVoice::Tambourine => 54,
            DrumVoice::Cabasa => 69,
            DrumVoice::Maraca => 70,
            DrumVoice::Sandpaper => 74,
            DrumVoice::Rocks => 75,
            DrumVoice::Can => 76,
            DrumVoice::Crunch => 77,
            DrumVoice::Gravel => 79,
            DrumVoice::Sekere => 82,
            DrumVoice::SleighBells => 83,
            DrumVoice::Bamboo => 84,
            DrumVoice::M808Kick | DrumVoice::M909Kick | DrumVoice::Cr78Kick => 36,
            DrumVoice::M808Rim | DrumVoice::M909Rim => 37,
            DrumVoice::M808Snare | DrumVoice::M909Snare | DrumVoice::Cr78Snare => 38,
            DrumVoice::M808Clap | DrumVoice::M909Clap => 39,
            DrumVoice::M808ClosedHat | DrumVoice::M909ClosedHat | DrumVoice::Cr78Hat => 42,
            DrumVoice::M808Tom | DrumVoice::M909Tom | DrumVoice::ElectroTom => 45,
            DrumVoice::M808OpenHat | DrumVoice::M909OpenHat => 46,
            DrumVoice::M808Cymbal | DrumVoice::Cr78Cymbal => 49,
            DrumVoice::Cr78Tambourine => 54,
            DrumVoice::M808Cowbell | DrumVoice::Cr78Metal => 56,
            DrumVoice::Cr78Bongo => 60,
            DrumVoice::M808Conga => 63,
            DrumVoice::M808Maracas => 70,
            DrumVoice::Cr78Guiro => 73,
            DrumVoice::M808Claves => 75,
            DrumVoice::M606Kick => 36,
            DrumVoice::M606Snare => 38,
            DrumVoice::M606LowTom => 45,
            DrumVoice::M606HighTom => 50,
            DrumVoice::M606ClosedHat => 42,
            DrumVoice::M606OpenHat => 46,
            DrumVoice::M606Cymbal => 49,
            DrumVoice::FmKick => 36,
            DrumVoice::FmSnare => 38,
            DrumVoice::FmTom => 45,
            DrumVoice::FmHat => 42,
            DrumVoice::FmBell => 56,
            DrumVoice::FmZap => 39,
            DrumVoice::FmPerc => 37,
            DrumVoice::FmNoise => 46,
            DrumVoice::AcKick => 36,
            DrumVoice::AcStick => 37,
            DrumVoice::AcSnare => 38,
            DrumVoice::AcBrush => 40,
            DrumVoice::AcTomFloor => 43,
            DrumVoice::AcTomMid => 47,
            DrumVoice::AcTomHigh => 50,
            DrumVoice::AcHatClosed => 42,
            DrumVoice::AcHatPedal => 44,
            DrumVoice::AcHatOpen => 46,
            DrumVoice::AcCrash => 49,
            DrumVoice::AcRide => 51,
            DrumVoice::AcChina => 52,
            DrumVoice::AcRideBell => 53,
            DrumVoice::RbKick => 36,
            DrumVoice::RbSnare => 38,
            DrumVoice::RbHat => 42,
            DrumVoice::RbCymbal => 49,
            DrumVoice::RbBongoHigh => 60,
            DrumVoice::RbBongoLow => 61,
            DrumVoice::RbClaves => 75,
            DrumVoice::RbGuiro => 73,
            DrumVoice::RbMaracas => 70,
            DrumVoice::HxKick => 36,
            DrumVoice::HxSnare => 38,
            DrumVoice::HxTomHigh => 50,
            DrumVoice::HxTomMid => 47,
            DrumVoice::HxTomLow => 43,
            DrumVoice::DgKick => 36,
            DrumVoice::DgSnare => 38,
            DrumVoice::DgClap => 39,
            DrumVoice::DgRim => 37,
            DrumVoice::DgHat => 42,
            DrumVoice::DgOpenHat => 46,
            DrumVoice::DgTomHigh => 48,
            DrumVoice::DgTomLow => 45,
            DrumVoice::DgCowbell => 56,
            DrumVoice::PkKick => 36,
            DrumVoice::PkSnare => 38,
            DrumVoice::PkHat => 42,
            DrumVoice::PkOpenHat => 46,
            DrumVoice::PkCymbal => 49,
            DrumVoice::PkClap => 39,
            DrumVoice::RsKick => 36,
            DrumVoice::RsSnare => 38,
            DrumVoice::RsHat => 42,
            DrumVoice::RsFmKick => 41,
            DrumVoice::RsFmSnare => 40,
            DrumVoice::RsCircuitKick => 35,
            DrumVoice::MkKick => 36,
            DrumVoice::MkSnare => 38,
            DrumVoice::MkHat => 42,
            DrumVoice::MkClap => 39,
            DrumVoice::MkTom => 45,
            DrumVoice::MkRim => 37,
            DrumVoice::MkFm => 47,
            DrumVoice::MkNoise => 54,
            DrumVoice::MkCowbell => 56,
            DrumVoice::MkCymbal => 49,
            DrumVoice::MkShaker => 70,
            DrumVoice::MkClave => 75,
            DrumVoice::ChipConsoleKick => 36,
            DrumVoice::ChipConsoleSampleKick => 35,
            DrumVoice::ChipConsoleSnare => 38,
            DrumVoice::ChipConsoleSampleSnare => 40,
            DrumVoice::ChipConsoleHat => 42,
            DrumVoice::ChipConsoleOpenHat => 46,
            DrumVoice::ChipConsoleTom => 45,
            DrumVoice::ChipConsoleCrash => 49,
            DrumVoice::ChipHandheldKick => 36,
            DrumVoice::ChipHandheldSnare => 38,
            DrumVoice::ChipHandheldHat => 42,
            DrumVoice::ChipHandheldOpenHat => 46,
            DrumVoice::ChipHandheldTom => 45,
            DrumVoice::ChipHandheldClap => 39,
            DrumVoice::ChipMicroKick => 36,
            DrumVoice::ChipMicroSnare => 38,
            DrumVoice::ChipMicroTom => 45,
            DrumVoice::ChipMicroClap => 39,
            DrumVoice::ChipMicroRim => 37,
            DrumVoice::ChipMicroHat => 42,
            DrumVoice::ChipMicroOpenHat => 46,
            DrumVoice::ChipMicroPunchKick => 36,
            DrumVoice::ChipMicroPunchSnare => 38,
            DrumVoice::ChipMicroPunchClap => 39,
            DrumVoice::ChipMicroPunchRim => 37,
            DrumVoice::ChipMicroPunchHat => 42,
            DrumVoice::ChipMicroPunchOpenHat => 46,
            DrumVoice::ChipMicroPunchTom => 47,
            DrumVoice::ChipArcadeKick => 36,
            DrumVoice::ChipArcadeSnare => 38,
            DrumVoice::ChipArcadeTom => 45,
            DrumVoice::ChipArcadeHat => 42,
            DrumVoice::ChipArcadeOpenHat => 46,
            DrumVoice::ChipArcadeCowbell => 56,
            DrumVoice::ChipBeeperKick => 36,
            DrumVoice::ChipBeeperSnare => 38,
            DrumVoice::ChipBeeperHat => 42,
            DrumVoice::ChipBeeperTom => 45,
            DrumVoice::ChipWoodKick => 36,
            DrumVoice::ChipWoodSnare => 38,
            DrumVoice::ChipWoodHat => 42,
            DrumVoice::ChipWoodTom => 45,
            DrumVoice::ChipWoodZap => 51,
            DrumVoice::ChipWoodBoom => 57,
        }
    }

    /// Names of the pad's two extra knobs for this voice (None: not used).
    pub fn macros(self) -> [Option<&'static str>; 2] {
        match self.family() {
            Family::Classic => [None, None],
            Family::Acoustic => match self {
                DrumVoice::AcKick => [Some("Beater"), None],
                DrumVoice::AcSnare | DrumVoice::AcBrush => [Some("Wires"), None],
                DrumVoice::AcTomHigh | DrumVoice::AcTomMid | DrumVoice::AcTomFloor => [Some("Bend"), None],
                DrumVoice::AcRide => [Some("Bell"), None],
                DrumVoice::AcCrash | DrumVoice::AcChina => [Some("Size"), None],
                _ => [None, None],
            },
            Family::Resonant => match self {
                DrumVoice::RsKick => [Some("Punch"), None],
                DrumVoice::RsSnare => [Some("Snappy"), None],
                DrumVoice::RsHat => [None, None],
                DrumVoice::RsFmKick => [Some("FM"), Some("Noise")],
                DrumVoice::RsFmSnare => [Some("FM"), Some("Noise")],
                DrumVoice::RsCircuitKick => [Some("Tight"), Some("Bounce")],
                _ => [None, None],
            },
            Family::Macro => match self {
                DrumVoice::MkKick => [Some("Sweep"), Some("Noise")],
                DrumVoice::MkSnare => [Some("Sweep"), Some("Noise")],
                DrumVoice::MkHat => [Some("Sweep"), Some("FM")],
                DrumVoice::MkClap => [Some("Sweep"), Some("Noise")],
                DrumVoice::MkTom => [Some("Sweep"), Some("FM")],
                DrumVoice::MkRim => [Some("Sweep"), Some("FM")],
                DrumVoice::MkFm => [Some("Sweep"), Some("FM")],
                DrumVoice::MkNoise => [Some("Sweep"), Some("Noise")],
                DrumVoice::MkCowbell => [Some("Sweep"), Some("FM")],
                DrumVoice::MkCymbal => [Some("Sweep"), Some("FM")],
                DrumVoice::MkShaker => [Some("Sweep"), Some("Noise")],
                DrumVoice::MkClave => [Some("Sweep"), Some("FM")],
                _ => [None, None],
            },
            Family::Chip => [Some("Sweep"), Some("Noise")],
            Family::Retro => match self {
                DrumVoice::HxKick => [Some("Drop"), None],
                DrumVoice::HxTomHigh | DrumVoice::HxTomMid | DrumVoice::HxTomLow => [Some("Sweep"), None],
                DrumVoice::DgSnare => [Some("Gate"), None],
                _ => [None, None],
            },
            Family::Machine => match self {
                DrumVoice::M808Snare | DrumVoice::M909Snare | DrumVoice::Cr78Snare => [Some("Snappy"), None],
                DrumVoice::M909Kick => [Some("Attack"), None],
                DrumVoice::ElectroTom => [Some("Bend"), None],
                DrumVoice::M606Snare => [Some("Snappy"), None],
                v if Self::FM.contains(&v) => [Some("FM"), None],
                _ => [None, None],
            },
            Family::Shaker => [Some("Objects"), None],
            Family::Mutable => match self {
                DrumVoice::AnalogKick => [Some("Punch"), None],
                DrumVoice::SynthKick => [Some("Dirt"), Some("FM")],
                DrumVoice::AnalogSnare | DrumVoice::SynthSnare => [Some("Snappy"), None],
                _ => [Some("Noise"), None],
            },
        }
    }

    /// Pitch of a Mutable model at Tune 0, in Hz.
    fn base_hz(self) -> f32 {
        match self {
            DrumVoice::AnalogKick => 52.0,
            DrumVoice::SynthKick => 50.0,
            DrumVoice::AnalogSnare => 190.0,
            DrumVoice::SynthSnare => 200.0,
            // The metallic oscillators sit around 400 Hz (an 808's).
            _ => 207.0,
        }
    }

    /// Output trim so the Mutable models play at similar levels.
    fn trim(self) -> f32 {
        if self == DrumVoice::RingHat {
            0.75
        } else {
            1.0
        }
    }
}

/// The pad's Decay (0.1..4, 1 = as designed) as a Mutable model's decay
/// (0..1).
fn model_decay(voice: DrumVoice, decay: f32) -> f32 {
    let base = match voice {
        DrumVoice::AnalogKick | DrumVoice::SynthKick => 0.55,
        _ => 0.45,
    };
    (base + 0.22 * decay.max(0.01).log2()).clamp(0.0, 1.0)
}

/// A Macro model's pitch at Tune 0 (Hz) and its length at Decay 1 (s).
fn macro_base(model: macro_drums::Model) -> (f32, f32) {
    use macro_drums::Model::*;
    match model {
        Kick => (50.0, 0.45),
        Snare => (200.0, 0.25),
        Hat => (261.6, 0.08),
        Clap => (261.6, 0.3),
        Tom => (110.0, 0.4),
        Rim => (300.0, 0.06),
        Fm => (70.0, 0.4),
        Noise => (261.6, 0.3),
        Cowbell => (560.0, 0.35),
        Cymbal => (261.6, 1.2),
        Shaker => (261.6, 0.15),
        Clave => (400.0, 0.1),
    }
}

/// A drum instrument: which voices its pads can play, and its kits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DrumEngine {
    /// Every voice, and samples.
    #[default]
    Rack,
    /// Modelled acoustic kits.
    Acoustic,
    /// Recorded kits (see `real_drums`).
    RealDrums,
    /// Recorded drum machines and electronic kits (see `real_drums`).
    SampledMachines,
    /// More machines (see `retro`).
    Retro,
    M808,
    M909,
    Cr78,
    Mutable,
    Shakers,
    M606,
    FmDrums,
    /// Drums that ring: Peaks' drums and a circuit kick.
    Resonant,
    /// Twelve models with macros.
    MacroDrums,
    /// The old chips' drums.
    Chip,
}

static G606: [(&str, &[DrumVoice]); 1] = [("606", &DrumVoice::M606)];
static GFM: [(&str, &[DrumVoice]); 1] = [("FM", &DrumVoice::FM)];
static G808: [(&str, &[DrumVoice]); 1] = [("808", &DrumVoice::M808)];
static G909: [(&str, &[DrumVoice]); 1] = [("909", &DrumVoice::M909)];
static GCR78: [(&str, &[DrumVoice]); 1] = [("CR-78", &DrumVoice::CR78)];
static GMUTABLE: [(&str, &[DrumVoice]); 1] = [("Mutable", &DrumVoice::MUTABLE)];
static GSHAKERS: [(&str, &[DrumVoice]); 1] = [("Shakers", &DrumVoice::SHAKERS)];
static GACOUSTIC: [(&str, &[DrumVoice]); 2] = [("Acoustic", &DrumVoice::ACOUSTIC), ("Percussion", &DrumVoice::KIT_PERC)];
static REAL_PERC: [DrumVoice; 8] = [DrumVoice::Tambourine, DrumVoice::Cowbell, DrumVoice::Clap, DrumVoice::Maraca, DrumVoice::Cr78Bongo, DrumVoice::M808Conga, DrumVoice::M808Claves, DrumVoice::M808Tom];
static GMACHINES: [(&str, &[DrumVoice]); 3] = [("808", &DrumVoice::M808), ("Acoustic", &DrumVoice::ACOUSTIC), ("Percussion", &REAL_PERC)];
static GRETRO: [(&str, &[DrumVoice]); 1] = [("Retro", &DrumVoice::RETRO)];
static GRESONANT: [(&str, &[DrumVoice]); 1] = [("Resonant", &DrumVoice::RESONANT)];
static GMACRO: [(&str, &[DrumVoice]); 1] = [("Macro", &DrumVoice::MACRO)];
static GCHIP: [(&str, &[DrumVoice]); 1] = [("Chip", &DrumVoice::CHIP)];
static GREAL: [(&str, &[DrumVoice]); 2] = [("Acoustic", &DrumVoice::ACOUSTIC), ("Percussion", &REAL_PERC)];

impl DrumEngine {
    /// The engines to choose from. `Shakers` is only kept for old projects:
    /// its kits are the Drum Rack's now, and Shakers is an instrument.
    pub const ALL: [DrumEngine; 14] = [DrumEngine::RealDrums, DrumEngine::SampledMachines, DrumEngine::Acoustic, DrumEngine::Retro, DrumEngine::Chip, DrumEngine::Resonant, DrumEngine::MacroDrums, DrumEngine::M808, DrumEngine::M909, DrumEngine::M606, DrumEngine::Cr78, DrumEngine::Mutable, DrumEngine::FmDrums, DrumEngine::Rack];

    pub fn name(self) -> &'static str {
        match self {
            DrumEngine::Rack => "Drum Rack",
            DrumEngine::M808 => "808",
            DrumEngine::M909 => "909",
            DrumEngine::Cr78 => "CR-78",
            DrumEngine::Mutable => "Mutable Drums",
            DrumEngine::Shakers => "Shakers",
            DrumEngine::M606 => "606",
            DrumEngine::FmDrums => "FM Drums",
            DrumEngine::Acoustic => "Acoustic Drums",
            DrumEngine::RealDrums => "Real Drums",
            DrumEngine::Retro => "Retro Machines",
            DrumEngine::Resonant => "Resonant Drums",
            DrumEngine::MacroDrums => "Macro Drums",
            DrumEngine::Chip => "Chip Drums",
            DrumEngine::SampledMachines => "Sampled Machines",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            DrumEngine::Rack => "A rack of pads: every drum voice of Krabton, and samples (drop them on the pads, or make a kit from a folder)",
            DrumEngine::M808 => "The 808, rebuilt from its circuits: bridged-T kick, toms and congas, snare, clap, six-square hats, cymbal and cowbell",
            DrumEngine::M909 => "The 909, rebuilt from its circuits: swept kick with its attack, snare with snappy, toms, rim, clap and hats",
            DrumEngine::Cr78 => "The CR-78, rebuilt from its circuits: soft kick and snare, gated-noise hats and cymbal, bongo, guiro, tambourine and metal",
            DrumEngine::Mutable => "The drum models of Mutable Instruments Plaits: analog and synthetic kicks and snares, metallic and ring-modulated hats",
            DrumEngine::Shakers => "Perry Cook's PhISEM shakers: maraca, cabasa, tambourine, sleigh bells, bamboo, cans, rocks and more",
            DrumEngine::M606 => "The 606, rebuilt from its circuits: tight kick, snare, two toms, six-square hats and cymbal; the acid house drummer",
            DrumEngine::FmDrums => "Two-operator FM drums: kick, snare, tom, hat, bell, zap, perc and noise, with an FM knob on every pad",
            DrumEngine::SampledMachines => "Drum machines and electronic kits, recorded: the 707 and the 808 (free sample sets from the nineties), an electro kit, a club kit and a glitch kit (Sonic Pi's samples); downloaded on first use",
            DrumEngine::Resonant => "Drums that ring: the bass drum, snare, hi-hat and FM drum of Peaks by Emilie Gillet (ported, MIT), resonant filters struck by pulses, in fixed point as on the module; and a circuit kick after ChowKick by Jatin Chowdhury (BSD): the 808's pulse shaper as a wave digital filter with its diode, into a resonator that drops its pitch as it saturates (Tight, Bounce)",
            DrumEngine::MacroDrums => "Twelve compact drum models (kick, snare, hat, clap, tom, rim, FM, noise, cowbell, cymbal, shaker, clave), all played with the same macros: Decay, Tone, Sweep and Noise or FM. A port of Bogie by Pierre-Emmanuel Surga (MIT)",
            DrumEngine::Chip => "Drums as the old consoles, handhelds and home computers made them, from their chips' own parts: the console's frame-stepped triangle kicks, metallic noise and crunchy 1-bit delta sample drums; the handheld's 4-bit wave and 15/7-bit noise; SID-style 808 and 909 kits (after sidacid); arcade FM with PSG noise; a 1-bit beeper; the woodgrain console's polynomial counters",
            DrumEngine::Retro => "More drum machines: a seventies rhythm box (bongos, claves, guiro), the hexagonal electronic kit with its sweeping toms, the crunchy 8-bit digital eighties, a pocket rhythm machine",
            DrumEngine::RealDrums => "Recorded drums: acoustic kits from rusty rock, classic rock, metal and a big room to garage, seventies, a jazz club, brushes and dry; hand and body percussion and tabla. With velocity layers and round robins (free samples, each kit downloaded on first use; until then Krabton's own drums stand in)",
            DrumEngine::Acoustic => "A real drum kit, modelled: kick, snare with its wires, side stick, three toms, hi-hat (closed, pedal, open), ride and its bell, crashes; every hit a little different, harder hits brighter",
        }
    }

    /// The voices its pads can play, grouped as in the pad menu.
    pub fn groups(self) -> &'static [(&'static str, &'static [DrumVoice])] {
        match self {
            DrumEngine::Rack => &DrumVoice::GROUPS,
            DrumEngine::M808 => &G808,
            DrumEngine::M909 => &G909,
            DrumEngine::Cr78 => &GCR78,
            DrumEngine::Mutable => &GMUTABLE,
            DrumEngine::Shakers => &GSHAKERS,
            DrumEngine::M606 => &G606,
            DrumEngine::FmDrums => &GFM,
            DrumEngine::Acoustic => &GACOUSTIC,
            DrumEngine::RealDrums => &GREAL,
            DrumEngine::Retro => &GRETRO,
            DrumEngine::Resonant => &GRESONANT,
            DrumEngine::MacroDrums => &GMACRO,
            DrumEngine::Chip => &GCHIP,
            DrumEngine::SampledMachines => &GMACHINES,
        }
    }

    pub fn kits(self) -> &'static [&'static str] {
        match self {
            DrumEngine::M808 => &["808 Classic", "808 Boom", "808 Electro", "808 Latin"],
            DrumEngine::M909 => &["909 Classic", "909 House", "909 Techno", "909 Toms"],
            DrumEngine::Cr78 => &["CR-78 Classic", "CR-78 Disco", "CR-78 Bossa"],
            DrumEngine::Mutable => &["Mutable Analog", "Mutable Synthetic", "Mutable Industrial"],
            DrumEngine::Shakers => &[],
            DrumEngine::M606 => &["606 Classic", "606 Acid House"],
            DrumEngine::FmDrums => &["FM Kit", "FM Industrial", "FM Glitch"],
            DrumEngine::Acoustic => &["Studio Kit", "Rock Kit", "Jazz Kit", "Funk Kit", "Vintage Kit", "Brush Kit"],
            DrumEngine::RealDrums => &["Real Kit", "Rusty Kit", "Classic Rock Kit", "Naked Kit", "Metal Kit", "Room Kit", "Garage Kit", "70s Kit", "Jazz Club Kit", "Lounge Kit", "Dry Kit", "Real Percussion", "Body Percussion", "Tabla"],
            DrumEngine::SampledMachines => &["707 Samples", "808 Samples", "Electro Kit", "Club Kit", "Glitch Kit"],
            DrumEngine::Retro => &["Rhythm Box", "Hexagon", "Digital 80s", "Pocket Rhythm"],
            DrumEngine::Resonant => &["Resonant Kit", "Circuit Kit", "FM Perc Kit"],
            DrumEngine::MacroDrums => &["Macro Kit", "Macro Electro", "Macro Industrial"],
            DrumEngine::Chip => &["Console Kit", "Handheld Kit", "Micro Kit", "Micro Punch Kit", "Arcade Kit", "Beeper Kit", "Woodgrain Kit"],
            DrumEngine::Rack => &["Classic 808", "Classic 909", "Classic Lo-Fi", "Hybrid", "Electro", "606 Electro", "Shaker Kit", "Hand Percussion", "Found Sounds"],
        }
    }
}

/// Kit names from before the engines existed, and from 0.11.1, which
/// named them differently for a while.
pub fn current_name(name: &str) -> &str {
    match name {
        "808" => "808 Classic",
        "909" => "909 Classic",
        "CR-78" => "CR-78 Classic",
        "Shakers" => "Shaker Kit",
        "808 Krab" => "Classic 808",
        "909 Krab" => "Classic 909",
        "Lo-Fi Krab" => "Classic Lo-Fi",
        "Boom Classic" => "808 Classic",
        "Boom Deep" => "808 Boom",
        "Boom Electro" => "808 Electro",
        "Boom Latin" => "808 Latin",
        "Punch Classic" => "909 Classic",
        "Punch House" => "909 House",
        "Punch Techno" => "909 Techno",
        "Punch Toms" => "909 Toms",
        "Velvet Classic" => "CR-78 Classic",
        "Velvet Disco" => "CR-78 Disco",
        "Velvet Bossa" => "CR-78 Bossa",
        "Model Analog" => "Mutable Analog",
        "Model Synthetic" => "Mutable Synthetic",
        "Model Industrial" => "Mutable Industrial",
        "Tiny Classic" => "606 Classic",
        "Tiny Acid House" => "606 Acid House",
        "Digital Machine" => "707 Samples",
        "Analog Machine" => "808 Samples",
        "Classic Boom" => "Classic 808",
        "Classic Punch" => "Classic 909",
        "Tiny Electro" => "606 Electro",
        n => n,
    }
}

/// The engine a kit belongs to.
pub fn engine_of(kit: &str) -> DrumEngine {
    let kit = current_name(kit);
    DrumEngine::ALL.into_iter().find(|e| e.kits().contains(&kit)).unwrap_or_default()
}

/// Every kit of every engine.
pub fn kit_names() -> Vec<&'static str> {
    DrumEngine::ALL.iter().flat_map(|e| e.kits().iter().copied()).collect()
}

/// An acoustic kit of sixteen pads, on the General MIDI notes.
fn acoustic_kit(name: &str) -> Vec<DrumPad> {
    use DrumVoice::*;
    let brush = name == "Brush Kit";
    let mut pads = vec![
        pad("Kick", AcKick, 36, |p| {
            p.a = 0.5;
            p.level_db = 0.0;
        }),
        pad("Side Stick", AcStick, 37, |p| p.level_db = -7.0),
        pad("Snare", if brush { AcBrush } else { AcSnare }, 38, |p| p.level_db = -1.0),
        pad("Clap", Clap, 39, |p| p.level_db = -8.0),
        pad("Hi-Hat", AcHatClosed, 42, |p| {
            p.choke = 1;
            p.level_db = -8.0;
        }),
        pad("Pedal Hat", AcHatPedal, 44, |p| {
            p.choke = 1;
            p.level_db = -9.0;
        }),
        pad("Open Hat", AcHatOpen, 46, |p| {
            p.choke = 1;
            p.level_db = -9.0;
        }),
        pad("Floor Tom", AcTomFloor, 43, |p| p.level_db = -3.0),
        pad("Mid Tom", AcTomMid, 47, |p| p.level_db = -3.0),
        pad("High Tom", AcTomHigh, 50, |p| p.level_db = -3.0),
        pad("Crash", AcCrash, 49, |p| p.level_db = -7.0),
        pad("Ride", AcRide, 51, |p| p.level_db = -7.0),
        pad("China", AcChina, 52, |p| p.level_db = -9.0),
        pad("Ride Bell", AcRideBell, 53, |p| p.level_db = -8.0),
        pad("Tambourine", Tambourine, 54, |p| p.level_db = -10.0),
        pad("Cowbell", Cowbell, 56, |p| p.level_db = -13.0),
    ];
    let each = |pads: &mut Vec<DrumPad>, v: DrumVoice, f: &dyn Fn(&mut DrumPad)| {
        for p in pads.iter_mut().filter(|p| p.voice == v) {
            f(p);
        }
    };
    let toms = [AcTomHigh, AcTomMid, AcTomFloor];
    match name {
        "Rock Kit" => {
            each(&mut pads, AcKick, &|p| {
                p.tune = -1.0;
                p.decay = 1.2;
                p.a = 0.8;
            });
            each(&mut pads, AcSnare, &|p| {
                p.tune = -1.5;
                p.decay = 1.3;
                p.tone = 0.6;
                p.a = 0.6;
            });
            for t in toms {
                each(&mut pads, t, &|p| {
                    p.tune = -2.0;
                    p.decay = 1.3;
                });
            }
            each(&mut pads, AcCrash, &|p| {
                p.a = 0.85;
                p.tone = 0.6;
            });
        }
        "Jazz Kit" | "Brush Kit" => {
            each(&mut pads, AcKick, &|p| {
                p.tune = 7.0;
                p.decay = 1.4;
                p.a = 0.15;
                p.level_db = -4.0;
            });
            each(&mut pads, AcSnare, &|p| {
                p.tune = 3.0;
                p.decay = 0.8;
                p.a = 0.3;
            });
            for t in toms {
                each(&mut pads, t, &|p| {
                    p.tune = 5.0;
                    p.decay = 1.2;
                    p.a = 0.2;
                });
            }
            each(&mut pads, AcHatClosed, &|p| p.tone = 0.35);
            each(&mut pads, AcHatOpen, &|p| p.tone = 0.35);
            each(&mut pads, AcRide, &|p| {
                p.level_db = -5.0;
                p.decay = 1.3;
                p.tone = 0.4;
            });
            each(&mut pads, AcCrash, &|p| p.a = 0.3);
        }
        "Funk Kit" => {
            each(&mut pads, AcKick, &|p| {
                p.decay = 0.55;
                p.tone = 0.6;
                p.a = 0.7;
            });
            each(&mut pads, AcSnare, &|p| {
                p.tune = 4.0;
                p.tone = 0.75;
                p.decay = 0.8;
            });
            each(&mut pads, AcHatClosed, &|p| p.tone = 0.65);
            for t in toms {
                each(&mut pads, t, &|p| p.decay = 0.8);
            }
        }
        "Vintage Kit" => {
            for p in pads.iter_mut().filter(|p| p.voice.family() == Family::Acoustic) {
                p.tone = 0.3;
            }
            each(&mut pads, AcKick, &|p| {
                p.decay = 0.8;
                p.a = 0.3;
            });
            for t in toms {
                each(&mut pads, t, &|p| p.decay = 0.7);
            }
            each(&mut pads, AcSnare, &|p| {
                p.decay = 0.9;
                p.a = 0.8;
            });
        }
        _ => {}
    }
    pads
}

/// A pad playing `voice` on `note`, named `name`, set up by `f`.
fn pad(name: &str, voice: DrumVoice, note: u8, f: impl FnOnce(&mut DrumPad)) -> DrumPad {
    let mut p = DrumPad::new(voice);
    p.name = name.to_string();
    p.note = note;
    f(&mut p);
    p
}

pub fn kit(name: &str) -> Vec<DrumPad> {
    use DrumVoice::*;
    let closed = |p: &mut DrumPad| {
        p.decay = 0.35;
        p.choke = 1;
    };
    let open = |p: &mut DrumPad| {
        p.decay = 2.5;
        p.choke = 1;
    };
    let hat = |p: &mut DrumPad| p.choke = 1;
    match current_name(name) {
        // ---------------------------------------------------------- Acoustic
        n @ ("Studio Kit" | "Rock Kit" | "Jazz Kit" | "Funk Kit" | "Vintage Kit" | "Brush Kit") => acoustic_kit(n),
        n if super::real_drums::pack_of(n).is_some() => super::real_drums::kit(n),
        // ---------------------------------------------------------- Resonant
        "Resonant Kit" => vec![
            pad("Kick", RsKick, 36, |p| p.level_db = 0.0),
            pad("Circuit Kick", RsCircuitKick, 35, |p| p.level_db = 0.0),
            pad("Snare", RsSnare, 38, |_| {}),
            pad("Snare 2", RsSnare, 40, |p| {
                p.tune = 3.0;
                p.a = 0.85;
            }),
            pad("Hat", RsHat, 42, |p| {
                p.choke = 1;
                p.decay = 0.5;
                p.level_db = -6.0;
            }),
            pad("Open Hat", RsHat, 46, |p| {
                p.choke = 1;
                p.decay = 3.0;
                p.level_db = -6.0;
            }),
            pad("FM Tom", RsFmKick, 41, |p| p.tune = 9.0),
            pad("FM Tom 2", RsFmKick, 43, |p| p.tune = 14.0),
            pad("FM Snare", RsFmSnare, 39, |_| {}),
            pad("FM Perc", RsFmSnare, 37, |p| {
                p.tune = 12.0;
                p.decay = 0.4;
                p.b = 0.2;
            }),
        ],
        "Circuit Kit" => vec![
            pad("Kick", RsCircuitKick, 36, |p| p.level_db = 0.0),
            pad("Tight Kick", RsCircuitKick, 35, |p| {
                p.a = 0.9;
                p.decay = 0.6;
                p.level_db = 0.0;
            }),
            pad("Boom", RsCircuitKick, 41, |p| {
                p.decay = 2.5;
                p.tune = -3.0;
                p.a = 0.3;
                p.level_db = 0.0;
            }),
            pad("Bounce", RsCircuitKick, 43, |p| {
                p.b = 0.7;
                p.level_db = 0.0;
            }),
            pad("Bright Kick", RsCircuitKick, 45, |p| {
                p.tone = 0.85;
                p.tune = 5.0;
            }),
            pad("Snare", RsSnare, 38, |_| {}),
            pad("Hat", RsHat, 42, |p| {
                p.choke = 1;
                p.decay = 0.5;
                p.level_db = -6.0;
            }),
            pad("Open Hat", RsHat, 46, |p| {
                p.choke = 1;
                p.decay = 3.0;
                p.level_db = -6.0;
            }),
            pad("FM Snare", RsFmSnare, 39, |_| {}),
        ],
        "FM Perc Kit" => vec![
            pad("FM Kick", RsFmKick, 36, |p| {
                p.a = 0.6;
                p.level_db = 0.0;
            }),
            pad("Dirty Kick", RsFmKick, 35, |p| {
                p.b = 0.05;
                p.level_db = 0.0;
            }),
            pad("Low Tom", RsFmKick, 41, |p| p.tune = 7.0),
            pad("Mid Tom", RsFmKick, 45, |p| p.tune = 12.0),
            pad("High Tom", RsFmKick, 48, |p| p.tune = 17.0),
            pad("Conga", RsFmSnare, 63, |p| {
                p.tune = 2.0;
                p.a = 0.2;
                p.b = 0.5;
            }),
            pad("Snare", RsFmSnare, 38, |p| p.b = 0.85),
            pad("Click", RsFmSnare, 37, |p| {
                p.tune = 19.0;
                p.decay = 0.3;
                p.a = 0.9;
            }),
            pad("Noise Hat", RsFmSnare, 42, |p| {
                p.tune = 24.0;
                p.b = 1.0;
                p.decay = 0.3;
                p.choke = 1;
            }),
        ],
        // ---------------------------------------------------------- Macro
        "Macro Kit" => vec![
            pad("Kick", MkKick, 36, |p| p.level_db = 0.0),
            pad("Rim", MkRim, 37, |_| {}),
            pad("Snare", MkSnare, 38, |_| {}),
            pad("Clap", MkClap, 39, |_| {}),
            pad("Hat", MkHat, 42, |p| p.choke = 1),
            pad("Open Hat", MkHat, 46, |p| {
                p.choke = 1;
                p.decay = 4.0;
            }),
            pad("Tom", MkTom, 45, |_| {}),
            pad("FM", MkFm, 47, |_| {}),
            pad("Cymbal", MkCymbal, 49, |_| {}),
            pad("Noise", MkNoise, 54, |_| {}),
            pad("Cowbell", MkCowbell, 56, |_| {}),
            pad("Shaker", MkShaker, 70, |_| {}),
            pad("Clave", MkClave, 75, |_| {}),
        ],
        "Macro Electro" => vec![
            pad("Kick", MkKick, 36, |p| {
                p.a = 0.75;
                p.tone = 0.3;
                p.decay = 1.4;
                p.level_db = 0.0;
            }),
            pad("Snare", MkSnare, 38, |p| {
                p.b = 0.75;
                p.tone = 0.7;
            }),
            pad("Clap", MkClap, 39, |p| p.a = 0.8),
            pad("Hat", MkHat, 42, |p| {
                p.choke = 1;
                p.tone = 0.8;
            }),
            pad("Open Hat", MkHat, 46, |p| {
                p.choke = 1;
                p.decay = 5.0;
                p.tone = 0.8;
            }),
            pad("Zap", MkFm, 47, |p| {
                p.a = 1.0;
                p.b = 0.8;
            }),
            pad("Cowbell", MkCowbell, 56, |_| {}),
            pad("Clave", MkClave, 75, |p| p.tune = 5.0),
            pad("Sweep", MkNoise, 54, |p| {
                p.a = 1.0;
                p.decay = 3.0;
            }),
        ],
        "Macro Industrial" => vec![
            pad("Kick", MkKick, 36, |p| {
                p.b = 0.9;
                p.tune = -3.0;
                p.decay = 1.8;
                p.level_db = 0.0;
            }),
            pad("Metal Snare", MkSnare, 38, |p| {
                p.b = 0.95;
                p.tone = 0.9;
            }),
            pad("Clank", MkCowbell, 39, |p| {
                p.b = 1.0;
                p.tune = -7.0;
            }),
            pad("Hat", MkHat, 42, |p| {
                p.choke = 1;
                p.b = 1.0;
            }),
            pad("Open Metal", MkCymbal, 46, |p| {
                p.choke = 1;
                p.b = 1.0;
            }),
            pad("Tom", MkTom, 45, |p| {
                p.b = 0.9;
                p.tune = -5.0;
            }),
            pad("Grind", MkNoise, 49, |p| {
                p.a = 0.2;
                p.b = 1.0;
                p.decay = 2.5;
            }),
            pad("FM Hit", MkFm, 47, |p| {
                p.b = 1.0;
                p.tone = 0.1;
            }),
        ],
        // ---------------------------------------------------------- Chip
        "Console Kit" => vec![
            pad("Kick", ChipConsoleKick, 36, |p| p.level_db = 0.0),
            pad("Sample Kick", ChipConsoleSampleKick, 35, |p| p.level_db = 0.0),
            pad("Snare", ChipConsoleSnare, 38, |_| {}),
            pad("Sample Snare", ChipConsoleSampleSnare, 40, |_| {}),
            pad("Hat", ChipConsoleHat, 42, |p| p.choke = 1),
            pad("Open Hat", ChipConsoleOpenHat, 46, |p| p.choke = 1),
            pad("Tom", ChipConsoleTom, 45, |_| {}),
            pad("High Tom", ChipConsoleTom, 48, |p| p.tune = 5.0),
            pad("Crash", ChipConsoleCrash, 49, |_| {}),
        ],
        "Handheld Kit" => vec![
            pad("Kick", ChipHandheldKick, 36, |p| p.level_db = 0.0),
            pad("Snare", ChipHandheldSnare, 38, |_| {}),
            pad("Clap", ChipHandheldClap, 39, |_| {}),
            pad("Hat", ChipHandheldHat, 42, |p| p.choke = 1),
            pad("Open Hat", ChipHandheldOpenHat, 46, |p| p.choke = 1),
            pad("Tom", ChipHandheldTom, 45, |_| {}),
            pad("High Tom", ChipHandheldTom, 48, |p| p.tune = 5.0),
            pad("Crash", ChipHandheldOpenHat, 49, |p| p.decay = 3.0),
        ],
        "Micro Kit" => vec![
            pad("Kick", ChipMicroKick, 36, |p| p.level_db = 0.0),
            pad("Rim", ChipMicroRim, 37, |_| {}),
            pad("Snare", ChipMicroSnare, 38, |_| {}),
            pad("Clap", ChipMicroClap, 39, |_| {}),
            pad("Hat", ChipMicroHat, 42, |p| p.choke = 1),
            pad("Open Hat", ChipMicroOpenHat, 46, |p| p.choke = 1),
            pad("Low Tom", ChipMicroTom, 41, |_| {}),
            pad("High Tom", ChipMicroTom, 48, |p| p.tune = 4.0),
        ],
        "Micro Punch Kit" => vec![
            pad("Kick", ChipMicroPunchKick, 36, |p| p.level_db = 0.0),
            pad("Rim", ChipMicroPunchRim, 37, |_| {}),
            pad("Snare", ChipMicroPunchSnare, 38, |_| {}),
            pad("Clap", ChipMicroPunchClap, 39, |_| {}),
            pad("Hat", ChipMicroPunchHat, 42, |p| p.choke = 1),
            pad("Open Hat", ChipMicroPunchOpenHat, 46, |p| p.choke = 1),
            pad("Low Tom", ChipMicroPunchTom, 41, |_| {}),
            pad("High Tom", ChipMicroPunchTom, 48, |p| p.tune = 4.0),
        ],
        "Arcade Kit" => vec![
            pad("Kick", ChipArcadeKick, 36, |p| p.level_db = 0.0),
            pad("Snare", ChipArcadeSnare, 38, |_| {}),
            pad("Hat", ChipArcadeHat, 42, |p| p.choke = 1),
            pad("Open Hat", ChipArcadeOpenHat, 46, |p| p.choke = 1),
            pad("Tom", ChipArcadeTom, 45, |_| {}),
            pad("High Tom", ChipArcadeTom, 48, |p| p.tune = 5.0),
            pad("Cowbell", ChipArcadeCowbell, 56, |_| {}),
            pad("Crash", ChipArcadeOpenHat, 49, |p| p.decay = 3.0),
        ],
        "Beeper Kit" => vec![
            pad("Kick", ChipBeeperKick, 36, |p| p.level_db = 0.0),
            pad("Snare", ChipBeeperSnare, 38, |_| {}),
            pad("Hat", ChipBeeperHat, 42, |_| {}),
            pad("Tom", ChipBeeperTom, 45, |_| {}),
            pad("High Tom", ChipBeeperTom, 48, |p| p.tune = 5.0),
            pad("Low Tom", ChipBeeperTom, 41, |p| p.tune = -4.0),
            pad("Open Hat", ChipBeeperHat, 46, |p| p.decay = 5.0),
            pad("Zap", ChipBeeperKick, 51, |p| {
                p.tune = 12.0;
                p.a = 1.0;
            }),
        ],
        "Woodgrain Kit" => vec![
            pad("Kick", ChipWoodKick, 36, |p| p.level_db = 0.0),
            pad("Snare", ChipWoodSnare, 38, |_| {}),
            pad("Hat", ChipWoodHat, 42, |_| {}),
            pad("Tom", ChipWoodTom, 45, |_| {}),
            pad("Zap", ChipWoodZap, 51, |_| {}),
            pad("Boom", ChipWoodBoom, 49, |_| {}),
            pad("Open Hat", ChipWoodHat, 46, |p| p.decay = 6.0),
            pad("High Tom", ChipWoodTom, 48, |p| p.tune = 5.0),
        ],
        // ---------------------------------------------------------- Retro
        "Rhythm Box" => vec![
            pad("Bass Drum", RbKick, 36, |p| p.level_db = 0.0),
            pad("Snare", RbSnare, 38, |_| {}),
            pad("Hat", RbHat, 42, |p| p.level_db = -6.0),
            pad("Cymbal", RbCymbal, 49, |p| p.level_db = -8.0),
            pad("High Bongo", RbBongoHigh, 60, |_| {}),
            pad("Low Bongo", RbBongoLow, 61, |_| {}),
            pad("Maracas", RbMaracas, 70, |p| p.level_db = -6.0),
            pad("Guiro", RbGuiro, 73, |p| p.level_db = -6.0),
            pad("Claves", RbClaves, 75, |p| p.level_db = -8.0),
        ],
        "Hexagon" => vec![
            pad("Kick", HxKick, 36, |p| p.level_db = 0.0),
            pad("Snare", HxSnare, 38, |_| {}),
            pad("Clap", DgClap, 39, |p| p.level_db = -6.0),
            pad("Closed Hat", DgHat, 42, |p| {
                p.choke = 1;
                p.level_db = -8.0;
            }),
            pad("Tom 3", HxTomLow, 43, |_| {}),
            pad("Open Hat", DgOpenHat, 46, |p| {
                p.choke = 1;
                p.level_db = -8.0;
            }),
            pad("Tom 2", HxTomMid, 47, |_| {}),
            pad("Tom 1", HxTomHigh, 50, |_| {}),
        ],
        "Digital 80s" => vec![
            pad("Kick", DgKick, 36, |p| p.level_db = 0.0),
            pad("Rim", DgRim, 37, |p| p.level_db = -8.0),
            pad("Snare", DgSnare, 38, |_| {}),
            pad("Clap", DgClap, 39, |p| p.level_db = -4.0),
            pad("Closed Hat", DgHat, 42, |p| {
                p.choke = 1;
                p.level_db = -7.0;
            }),
            pad("Low Tom", DgTomLow, 45, |_| {}),
            pad("Open Hat", DgOpenHat, 46, |p| {
                p.choke = 1;
                p.level_db = -7.0;
            }),
            pad("High Tom", DgTomHigh, 48, |_| {}),
            pad("Cowbell", DgCowbell, 56, |p| p.level_db = -9.0),
        ],
        "Pocket Rhythm" => vec![
            pad("Low Kick", PkKick, 35, |p| {
                p.tune = -4.0;
                p.level_db = 0.0;
            }),
            pad("Kick", PkKick, 36, |p| p.level_db = 0.0),
            pad("Snare", PkSnare, 38, |_| {}),
            pad("Clap", PkClap, 39, |p| p.level_db = -4.0),
            pad("High Snare", PkSnare, 40, |p| p.tune = 3.0),
            pad("Closed Hat", PkHat, 42, |p| {
                p.choke = 1;
                p.level_db = -7.0;
            }),
            pad("Open Hat", PkOpenHat, 46, |p| {
                p.choke = 1;
                p.level_db = -7.0;
            }),
            pad("Cymbal", PkCymbal, 49, |p| p.level_db = -9.0),
        ],
        // ---------------------------------------------------------- 808
        "808 Classic" => vec![
            pad("Kick", M808Kick, 36, |_| {}),
            pad("Rim", M808Rim, 37, |_| {}),
            pad("Snare", M808Snare, 38, |_| {}),
            pad("Clap", M808Clap, 39, |_| {}),
            pad("Closed Hat", M808ClosedHat, 42, hat),
            pad("Tom", M808Tom, 45, |_| {}),
            pad("Open Hat", M808OpenHat, 46, hat),
            pad("Cowbell", M808Cowbell, 56, |_| {}),
        ],
        "808 Boom" => vec![
            pad("Kick", M808Kick, 36, |p| {
                p.decay = 3.0;
                p.tune = -2.0;
                p.tone = 0.3;
            }),
            pad("Rim", M808Rim, 37, |_| {}),
            pad("Snare", M808Snare, 38, |p| {
                p.a = 0.7;
                p.tone = 0.4;
            }),
            pad("Clap", M808Clap, 39, |_| {}),
            pad("Closed Hat", M808ClosedHat, 42, hat),
            pad("Low Tom", M808Tom, 45, |p| p.tune = -5.0),
            pad("Open Hat", M808OpenHat, 46, hat),
            pad("Cymbal", M808Cymbal, 49, |_| {}),
        ],
        "808 Electro" => vec![
            pad("Kick", M808Kick, 36, |p| {
                p.decay = 0.5;
                p.tone = 0.8;
            }),
            pad("Snare", M808Snare, 38, |p| {
                p.tone = 0.7;
                p.a = 0.9;
                p.decay = 0.7;
            }),
            pad("Clap", M808Clap, 39, |p| p.decay = 0.7),
            pad("Closed Hat", M808ClosedHat, 42, |p| {
                p.choke = 1;
                p.decay = 0.6;
                p.tone = 0.7;
            }),
            pad("Open Hat", M808OpenHat, 46, |p| {
                p.choke = 1;
                p.decay = 0.8;
            }),
            pad("Cowbell", M808Cowbell, 56, |_| {}),
            pad("Conga", M808Conga, 63, |_| {}),
            pad("Claves", M808Claves, 75, |_| {}),
        ],
        "808 Latin" => vec![
            pad("Kick", M808Kick, 36, |_| {}),
            pad("Rim", M808Rim, 37, |_| {}),
            pad("Tom", M808Tom, 45, |p| p.tune = 3.0),
            pad("Cowbell", M808Cowbell, 56, |_| {}),
            pad("High Conga", M808Conga, 63, |_| {}),
            pad("Low Conga", M808Conga, 64, |p| p.tune = -5.0),
            pad("Maracas", M808Maracas, 70, |_| {}),
            pad("Claves", M808Claves, 75, |_| {}),
        ],
        // ---------------------------------------------------------- 909
        // ---------------------------------------------------------- 606
        "606 Classic" => vec![
            pad("Kick", M606Kick, 36, |_| {}),
            pad("Snare", M606Snare, 38, |_| {}),
            pad("Closed Hat", M606ClosedHat, 42, hat),
            pad("Low Tom", M606LowTom, 45, |_| {}),
            pad("Open Hat", M606OpenHat, 46, hat),
            pad("Cymbal", M606Cymbal, 49, |_| {}),
            pad("High Tom", M606HighTom, 50, |_| {}),
            pad("Accent Hat", M606ClosedHat, 44, |p| {
                p.choke = 1;
                p.tone = 0.8;
                p.decay = 1.5;
            }),
        ],
        "606 Acid House" => vec![
            pad("Kick", M606Kick, 36, |p| {
                p.decay = 1.8;
                p.tune = -2.0;
                p.tone = 0.7;
            }),
            pad("Snare", M606Snare, 38, |p| p.a = 0.8),
            pad("Rim Snare", M606Snare, 39, |p| {
                p.tune = 5.0;
                p.a = 1.0;
                p.decay = 0.5;
            }),
            pad("Closed Hat", M606ClosedHat, 42, closed),
            pad("Low Tom", M606LowTom, 45, |p| p.tune = -3.0),
            pad("Open Hat", M606OpenHat, 46, open),
            pad("Cymbal", M606Cymbal, 49, |p| p.decay = 0.6),
            pad("High Tom", M606HighTom, 50, |_| {}),
        ],
        "606 Electro" => vec![
            pad("Kick", M606Kick, 36, |p| {
                p.decay = 0.6;
                p.tone = 0.9;
            }),
            pad("Snare", M606Snare, 38, |p| {
                p.tune = 3.0;
                p.decay = 0.6;
            }),
            pad("Zap", FmZap, 39, |_| {}),
            pad("Closed Hat", M606ClosedHat, 42, |p| {
                p.choke = 1;
                p.decay = 0.5;
            }),
            pad("Low Tom", ElectroTom, 45, |p| p.a = 0.7),
            pad("Open Hat", M606OpenHat, 46, hat),
            pad("Cowbell", M808Cowbell, 56, |_| {}),
            pad("High Tom", ElectroTom, 50, |p| {
                p.a = 0.7;
                p.tune = 7.0;
            }),
        ],
        // ---------------------------------------------------------- FM
        "FM Kit" => vec![
            pad("Kick", FmKick, 36, |_| {}),
            pad("Perc", FmPerc, 37, |_| {}),
            pad("Snare", FmSnare, 38, |_| {}),
            pad("Zap", FmZap, 39, |_| {}),
            pad("Closed Hat", FmHat, 42, hat),
            pad("Tom", FmTom, 45, |_| {}),
            pad("Open Hat", FmHat, 46, |p| {
                p.choke = 1;
                p.decay = 4.0;
            }),
            pad("Bell", FmBell, 56, |_| {}),
        ],
        "FM Industrial" => vec![
            pad("Kick", FmKick, 36, |p| {
                p.a = 0.9;
                p.decay = 1.4;
            }),
            pad("Metal", FmPerc, 37, |p| {
                p.a = 0.9;
                p.tune = -5.0;
            }),
            pad("Snare", FmSnare, 38, |p| {
                p.a = 0.9;
                p.tone = 0.8;
            }),
            pad("Noise", FmNoise, 39, |_| {}),
            pad("Closed Hat", FmHat, 42, |p| {
                p.choke = 1;
                p.a = 0.9;
            }),
            pad("Tom", FmTom, 45, |p| p.a = 0.8),
            pad("Open Noise", FmNoise, 46, |p| {
                p.choke = 1;
                p.decay = 3.0;
            }),
            pad("Anvil", FmBell, 56, |p| {
                p.a = 0.9;
                p.tune = -7.0;
            }),
        ],
        "FM Glitch" => vec![
            pad("Kick", FmKick, 36, |p| {
                p.decay = 0.4;
                p.a = 0.2;
            }),
            pad("Blip", FmPerc, 37, |p| {
                p.tune = 12.0;
                p.decay = 0.3;
            }),
            pad("Snare", FmSnare, 38, |p| p.decay = 0.4),
            pad("Zap", FmZap, 39, |p| p.decay = 0.5),
            pad("Tick", FmHat, 42, |p| {
                p.choke = 1;
                p.decay = 0.3;
            }),
            pad("Laser", FmZap, 45, |p| {
                p.tune = -12.0;
                p.decay = 1.5;
            }),
            pad("Crush", FmNoise, 46, |p| {
                p.choke = 1;
                p.decay = 0.6;
            }),
            pad("Ping", FmBell, 56, |p| {
                p.tune = 12.0;
                p.decay = 0.5;
            }),
        ],
        "909 Classic" => vec![
            pad("Kick", M909Kick, 36, |_| {}),
            pad("Rim", M909Rim, 37, |_| {}),
            pad("Snare", M909Snare, 38, |_| {}),
            pad("Clap", M909Clap, 39, |_| {}),
            pad("Closed Hat", M909ClosedHat, 42, hat),
            pad("Low Tom", M909Tom, 45, |_| {}),
            pad("Open Hat", M909OpenHat, 46, hat),
            pad("High Tom", M909Tom, 50, |p| p.tune = 7.0),
        ],
        "909 House" => vec![
            pad("Kick", M909Kick, 36, |p| {
                p.a = 0.4;
                p.decay = 1.2;
            }),
            pad("Rim", M909Rim, 37, |_| {}),
            pad("Snare", M909Snare, 38, |p| p.a = 0.5),
            pad("Clap", M909Clap, 39, |p| p.decay = 1.3),
            pad("Closed Hat", M909ClosedHat, 42, hat),
            pad("Low Tom", M909Tom, 45, |_| {}),
            pad("Open Hat", M909OpenHat, 46, |p| {
                p.choke = 1;
                p.decay = 0.8;
            }),
            pad("High Tom", M909Tom, 50, |p| p.tune = 7.0),
        ],
        "909 Techno" => vec![
            pad("Kick", M909Kick, 36, |p| {
                p.tune = -2.0;
                p.decay = 1.6;
                p.a = 0.85;
                p.tone = 0.8;
            }),
            pad("Rim", M909Rim, 37, |p| p.tone = 0.8),
            pad("Snare", M909Snare, 38, |p| {
                p.a = 0.7;
                p.tone = 0.6;
            }),
            pad("Clap", M909Clap, 39, |_| {}),
            pad("Closed Hat", M909ClosedHat, 42, |p| {
                p.choke = 1;
                p.decay = 0.6;
                p.tone = 0.7;
            }),
            pad("Low Tom", M909Tom, 45, |p| {
                p.tune = -3.0;
                p.decay = 1.4;
            }),
            pad("Open Hat", M909OpenHat, 46, |p| {
                p.choke = 1;
                p.decay = 1.4;
            }),
            pad("Mid Tom", M909Tom, 47, |p| p.tune = 2.0),
        ],
        "909 Toms" => vec![
            pad("Kick", M909Kick, 36, |_| {}),
            pad("Snare", M909Snare, 38, |_| {}),
            pad("Floor Tom", M909Tom, 41, |p| p.tune = -7.0),
            pad("Closed Hat", M909ClosedHat, 42, hat),
            pad("Low Tom", M909Tom, 45, |_| {}),
            pad("Open Hat", M909OpenHat, 46, hat),
            pad("Mid Tom", M909Tom, 48, |p| p.tune = 5.0),
            pad("High Tom", M909Tom, 50, |p| p.tune = 9.0),
        ],
        // ---------------------------------------------------------- CR-78
        "CR-78 Classic" => vec![
            pad("Kick", Cr78Kick, 36, |_| {}),
            pad("Snare", Cr78Snare, 38, |_| {}),
            pad("Hat", Cr78Hat, 42, hat),
            pad("Open Hat", Cr78Hat, 46, |p| {
                p.decay = 4.0;
                p.choke = 1;
            }),
            pad("Cymbal", Cr78Cymbal, 49, |_| {}),
            pad("Tambourine", Cr78Tambourine, 54, |_| {}),
            pad("Metal", Cr78Metal, 56, |_| {}),
            pad("Guiro", Cr78Guiro, 73, |_| {}),
        ],
        "CR-78 Disco" => vec![
            pad("Kick", Cr78Kick, 36, |p| {
                p.decay = 1.5;
                p.tone = 0.3;
            }),
            pad("Snare", Cr78Snare, 38, |p| p.a = 0.7),
            pad("Hat", Cr78Hat, 42, |p| {
                p.choke = 1;
                p.decay = 0.8;
            }),
            pad("Open Hat", Cr78Hat, 46, |p| {
                p.decay = 4.0;
                p.choke = 1;
            }),
            pad("Cymbal", Cr78Cymbal, 49, |_| {}),
            pad("Tambourine", Cr78Tambourine, 54, |_| {}),
            pad("Metal", Cr78Metal, 56, |_| {}),
            pad("Bongo", Cr78Bongo, 60, |_| {}),
        ],
        "CR-78 Bossa" => vec![
            pad("Kick", Cr78Kick, 36, |p| p.decay = 0.8),
            pad("Rim", Cr78Snare, 38, |p| {
                p.tune = 7.0;
                p.decay = 0.4;
                p.a = 0.2;
            }),
            pad("Hat", Cr78Hat, 42, hat),
            pad("Tambourine", Cr78Tambourine, 54, |_| {}),
            pad("Metal", Cr78Metal, 56, |_| {}),
            pad("High Bongo", Cr78Bongo, 60, |_| {}),
            pad("Low Bongo", Cr78Bongo, 61, |p| p.tune = -5.0),
            pad("Guiro", Cr78Guiro, 73, |_| {}),
        ],
        // ---------------------------------------------------------- Mutable
        "Mutable Analog" => vec![
            pad("Kick", AnalogKick, 36, |p| {
                p.tone = 0.45;
                p.a = 0.35;
            }),
            pad("Rim", AnalogSnare, 37, |p| {
                p.tune = 12.0;
                p.decay = 0.25;
                p.a = 0.1;
                p.level_db = -6.0;
            }),
            pad("Snare", AnalogSnare, 38, |p| p.a = 0.6),
            pad("Clap", SynthSnare, 39, |p| {
                p.tune = 5.0;
                p.decay = 0.5;
                p.a = 1.0;
                p.tone = 0.2;
            }),
            pad("Closed Hat", MetalHat, 42, |p| {
                closed(p);
                p.tone = 0.6;
                p.a = 0.25;
            }),
            pad("Tom", AnalogKick, 45, |p| {
                p.tune = 12.0;
                p.decay = 0.6;
                p.a = 0.1;
                p.level_db = -3.0;
            }),
            pad("Open Hat", MetalHat, 46, |p| {
                open(p);
                p.tone = 0.6;
                p.a = 0.25;
            }),
            pad("Bell", MetalHat, 56, |p| {
                p.tune = 12.0;
                p.tone = 0.9;
                p.decay = 1.5;
                p.a = 0.0;
            }),
        ],
        "Mutable Synthetic" => vec![
            pad("Kick", SynthKick, 36, |p| {
                p.a = 0.3;
                p.b = 0.5;
            }),
            pad("Rim", SynthSnare, 37, |p| {
                p.tune = 12.0;
                p.decay = 0.25;
                p.a = 0.2;
                p.level_db = -8.0;
            }),
            pad("Snare", SynthSnare, 38, |p| {
                p.tone = 0.4;
                p.a = 0.6;
            }),
            pad("Clap", SynthSnare, 39, |p| {
                p.tune = 3.0;
                p.decay = 0.45;
                p.a = 1.0;
                p.tone = 0.1;
            }),
            pad("Closed Hat", RingHat, 42, |p| {
                closed(p);
                p.a = 0.4;
            }),
            pad("Tom", SynthKick, 45, |p| {
                p.tune = 10.0;
                p.decay = 0.6;
                p.b = 0.3;
                p.level_db = -3.0;
            }),
            pad("Open Hat", RingHat, 46, |p| {
                open(p);
                p.a = 0.4;
            }),
            pad("Bell", RingHat, 56, |p| {
                p.tune = 12.0;
                p.decay = 1.4;
                p.a = 0.0;
            }),
        ],
        "Mutable Industrial" => vec![
            pad("Kick", SynthKick, 36, |p| {
                p.a = 0.95;
                p.b = 0.8;
                p.tone = 0.6;
            }),
            pad("Snap", AnalogSnare, 37, |p| {
                p.tune = 7.0;
                p.decay = 0.4;
                p.a = 1.0;
            }),
            pad("Snare", SynthSnare, 38, |p| {
                p.tone = 0.9;
                p.a = 0.95;
            }),
            pad("Clap", AnalogSnare, 39, |p| {
                p.a = 1.0;
                p.tone = 0.8;
                p.decay = 0.6;
            }),
            pad("Closed Hat", RingHat, 42, |p| {
                closed(p);
                p.a = 0.95;
                p.tone = 0.8;
            }),
            pad("Tom", AnalogKick, 45, |p| {
                p.tune = 7.0;
                p.a = 1.0;
                p.decay = 0.7;
            }),
            pad("Open Hat", RingHat, 46, |p| {
                open(p);
                p.a = 0.95;
            }),
            pad("Metal", MetalHat, 56, |p| {
                p.tune = 12.0;
                p.decay = 1.2;
                p.tone = 0.9;
            }),
        ],
        // ---------------------------------------------------------- Shakers
        "Shaker Kit" => [Tambourine, Cabasa, Maraca, Can, Gravel, Sekere, SleighBells, Bamboo].iter().map(|&v| pad(v.name(), v, v.gm_note(), |_| {})).collect(),
        "Hand Percussion" => vec![
            pad("Shaker", Maraca, 44, |p| {
                p.a = 0.9;
                p.decay = 2.0;
            }),
            pad("Tambourine", Tambourine, 54, |_| {}),
            pad("Cabasa", Cabasa, 69, |_| {}),
            pad("Maraca", Maraca, 70, |_| {}),
            pad("Soft Maraca", Maraca, 71, |p| p.a = 0.2),
            pad("Sekere", Sekere, 82, |_| {}),
            pad("Sleigh Bells", SleighBells, 83, |_| {}),
            pad("Bamboo", Bamboo, 84, |_| {}),
        ],
        "Found Sounds" => vec![
            pad("Sandpaper", Sandpaper, 74, |_| {}),
            pad("Rocks", Rocks, 75, |_| {}),
            pad("Can", Can, 76, |_| {}),
            pad("Low Can", Can, 77, |p| p.tune = -12.0),
            pad("Crunch", Crunch, 78, |_| {}),
            pad("Gravel", Gravel, 79, |_| {}),
            pad("Snap", Crunch, 80, |p| p.tune = 7.0),
            pad("Bamboo", Bamboo, 84, |_| {}),
        ],
        // ---------------------------------------------------------- Rack
        "Hybrid" => vec![
            pad("Kick", AnalogKick, 36, |p| p.a = 0.6),
            pad("Rim", Rim, 37, |_| {}),
            pad("Snare", SynthSnare, 38, |p| p.a = 0.7),
            pad("Clap", Clap, 39, |_| {}),
            pad("Closed Hat", MetalHat, 42, closed),
            pad("Shaker", Maraca, 44, |_| {}),
            pad("Open Hat", RingHat, 46, open),
            pad("Tambourine", Tambourine, 54, |_| {}),
        ],
        "Electro" => vec![
            pad("Kick", M909Kick, 36, |p| p.a = 0.7),
            pad("Snare", M909Snare, 38, |_| {}),
            pad("Clap", M808Clap, 39, |_| {}),
            pad("Closed Hat", M808ClosedHat, 42, hat),
            pad("Low Tom", ElectroTom, 45, |p| p.tune = -5.0),
            pad("Open Hat", M808OpenHat, 46, hat),
            pad("Mid Tom", ElectroTom, 47, |_| {}),
            pad("High Tom", ElectroTom, 50, |p| p.tune = 5.0),
        ],
        other => classic_kit(other),
    }
}

/// Krabton's own drums (Classic 808, Classic 909, Classic Lo-Fi).
fn classic_kit(name: &str) -> Vec<DrumPad> {
    let order = [
        DrumVoice::Kick,
        DrumVoice::Rim,
        DrumVoice::Snare,
        DrumVoice::Clap,
        DrumVoice::ClosedHat,
        DrumVoice::Tom,
        DrumVoice::OpenHat,
        DrumVoice::Cowbell,
    ];
    let mut pads: Vec<DrumPad> = order.iter().map(|&v| DrumPad::new(v)).collect();
    for pad in pads.iter_mut() {
        if matches!(pad.voice, DrumVoice::ClosedHat | DrumVoice::OpenHat) {
            pad.choke = 1;
        }
        match (name, pad.voice) {
            ("Classic 808", DrumVoice::Kick) => {
                pad.decay = 1.8;
                pad.tone = 0.15;
            }
            ("Classic 808", DrumVoice::Snare) => pad.tone = 0.35,
            ("Classic 808", DrumVoice::ClosedHat | DrumVoice::OpenHat) => pad.tone = 0.4,
            ("Classic 909", DrumVoice::Kick) => {
                pad.decay = 0.8;
                pad.tune = 2.0;
                pad.tone = 0.65;
            }
            ("Classic 909", DrumVoice::Snare) => {
                pad.tone = 0.75;
                pad.tune = 1.0;
            }
            ("Classic 909", DrumVoice::ClosedHat | DrumVoice::OpenHat) => pad.tone = 0.8,
            ("Classic Lo-Fi", _) => {
                pad.tune -= 3.0;
                pad.tone = (pad.tone - 0.25).max(0.0);
                pad.decay *= 0.8;
            }
            _ => {}
        }
    }
    pads
}

const MAX_PADS: usize = 16;
const METAL: [f32; 6] = [205.3, 304.4, 369.6, 522.7, 540.0, 800.0];

/// A Mutable Instruments drum model, made when a pad first plays it.
enum Model {
    None,
    AnalogKick(AnalogBassDrum, Overdrive),
    SynthKick(SyntheticBassDrum),
    AnalogSnare(AnalogSnareDrum),
    SynthSnare(SyntheticSnareDrum),
    Hat(Hihat),
}

impl Model {
    fn for_voice(voice: DrumVoice, sr: f32) -> Self {
        match voice {
            DrumVoice::AnalogKick => {
                let mut d = AnalogBassDrum::new();
                d.init(sr);
                let mut o = Overdrive::new();
                o.init();
                Model::AnalogKick(d, o)
            }
            DrumVoice::SynthKick => {
                let mut d = SyntheticBassDrum::new();
                d.init(sr);
                Model::SynthKick(d)
            }
            DrumVoice::AnalogSnare => {
                let mut d = AnalogSnareDrum::new();
                d.init(sr);
                Model::AnalogSnare(d)
            }
            DrumVoice::SynthSnare => {
                let mut d = SyntheticSnareDrum::new();
                d.init(sr);
                Model::SynthSnare(d)
            }
            DrumVoice::MetalHat | DrumVoice::RingHat => {
                let mut d = Hihat::new();
                d.init(sr);
                Model::Hat(d)
            }
            _ => Model::None,
        }
    }

    fn plays(&self, voice: DrumVoice) -> bool {
        matches!(
            (self, voice),
            (Model::AnalogKick(..), DrumVoice::AnalogKick)
                | (Model::SynthKick(_), DrumVoice::SynthKick)
                | (Model::AnalogSnare(_), DrumVoice::AnalogSnare)
                | (Model::SynthSnare(_), DrumVoice::SynthSnare)
                | (Model::Hat(_), DrumVoice::MetalHat | DrumVoice::RingHat)
        )
    }
}

/// Longest block a Mutable model renders at once.
const MODEL_BLOCK: usize = 256;

struct PadVoice {
    active: bool,
    t: u32,
    vel: f32,
    phase: [f32; 6],
    amp: f32,
    amp_coef: f32,
    env2: f32,
    env2_coef: f32,
    env3: f32,
    env3_coef: f32,
    f1: Svf,
    f2: Svf,
    rng: Rng,
    choke: f32,
    choking: bool,
    sample_pos: f64,
    /// The recorded layer and round robin playing (pads with layers).
    layer: usize,
    rr: usize,
    /// The hit before, dying away under a new one: its layer, round
    /// robin, position and gain.
    tail: Option<(usize, usize, f64, f32)>,
    model: Model,
    model_sr: f32,
    /// Tell the model it was hit, on its next block.
    model_hit: bool,
    /// Samples the model has been silent.
    quiet: u32,
    shaker: Shaker,
    machine: MachineVoice,
    acoustic: Box<AcousticVoice>,
    retro: RetroVoice,
    peaks: PeaksVoice,
    circuit: CircuitKick,
    macro_voice: MacroVoice,
    chip: ChipVoice,
}

impl PadVoice {
    fn new(seed: u32) -> Self {
        Self {
            active: false,
            t: 0,
            vel: 0.0,
            phase: [0.0; 6],
            amp: 0.0,
            amp_coef: 0.0,
            env2: 0.0,
            env2_coef: 0.0,
            env3: 0.0,
            env3_coef: 0.0,
            f1: Svf::new(),
            f2: Svf::new(),
            rng: Rng::new(seed),
            choke: 1.0,
            choking: false,
            sample_pos: 0.0,
            layer: 0,
            rr: 0,
            tail: None,
            model: Model::None,
            model_sr: 0.0,
            model_hit: false,
            quiet: 0,
            shaker: Shaker::default(),
            machine: MachineVoice::default(),
            acoustic: Box::default(),
            retro: RetroVoice::default(),
            peaks: PeaksVoice::default(),
            circuit: CircuitKick::default(),
            macro_voice: MacroVoice::default(),
            chip: ChipVoice::default(),
        }
    }

    fn trigger(&mut self, pad: &DrumPad, vel: f32, sr: f32) {
        // A recording hit again: the last hit dies away under the new one
        // instead of stopping dead (a cymbal keeps ringing).
        self.tail = None;
        if self.active && !self.choking && (!pad.layers.is_empty() || pad.sample.is_some()) {
            let vel_gain = pad.vel_gain(self.vel);
            self.tail = Some((self.layer, self.rr, self.sample_pos, vel_gain * self.amp * self.choke));
        }
        self.active = true;
        self.t = 0;
        self.vel = vel;
        self.choke = 1.0;
        self.choking = false;
        self.sample_pos = 0.0;
        self.amp = 1.0;
        self.env2 = 1.0;
        self.env3 = 1.0;
        self.f1.reset();
        self.f2.reset();
        for (i, p) in self.phase.iter_mut().enumerate() {
            *p = if i % 2 == 0 { 0.0 } else { 0.37 * i as f32 % 1.0 };
        }
        // Velocity moves the tone and the decay as much as the pad says.
        let (tone, decay) = pad.vel_tone_decay(vel);
        let d = decay.max(0.05);
        let tr = 2f32.powf(pad.tune / 12.0);
        self.env3_coef = decay_coef(0.004, sr);
        self.amp_coef = 1.0;
        self.env2_coef = 0.0;
        match pad.voice.family() {
            Family::Mutable => {
                if !self.model.plays(pad.voice) || self.model_sr != sr {
                    self.model = Model::for_voice(pad.voice, sr);
                    self.model_sr = sr;
                }
                self.model_hit = true;
                self.quiet = 0;
            }
            Family::Shaker => {
                if let Some(m) = pad.voice.shaker() {
                    self.shaker.trigger(m, 1.0, tr, d, tone, pad.a, sr);
                }
            }
            Family::Machine => self.machine.trigger(pad.voice, vel, tr, d, tone, pad.a, sr),
            Family::Acoustic => self.acoustic.trigger(pad.voice, vel, tr, d, tone, pad.a, pad.b, sr),
            Family::Retro => self.retro.trigger(pad.voice, vel, tr, d, tone, pad.a, sr),
            Family::Resonant => {
                let (a, b) = (pad.a.clamp(0.0, 1.0), pad.b.clamp(0.0, 1.0));
                if pad.voice == DrumVoice::RsCircuitKick {
                    let k = KickKnobs { freq: 55.0 * tr, ring: (0.5 + 0.2 * d.log2()).clamp(0.0, 1.0), tone: 300.0 * (7000.0f32 / 300.0).powf(tone), tight: a, bounce: b, ..KickKnobs::default() };
                    self.circuit.trigger(k, vel, sr);
                } else {
                    // The module's knobs: 16 bits, its frequency in its own units.
                    let semis = pad.tune;
                    let fm_freq = |note: f32| (((note + semis - 24.0) / 72.0 * 65536.0).clamp(0.0, 65535.0)) as i32;
                    let near = (semis * 32768.0 / 7.0).clamp(-32768.0, 32767.0) as i32;
                    let (kind, frequency) = match pad.voice {
                        DrumVoice::RsKick => (peaks::Kind::Kick, near),
                        DrumVoice::RsSnare => (peaks::Kind::Snare, near),
                        DrumVoice::RsHat => (peaks::Kind::Hat, near),
                        DrumVoice::RsFmKick => (peaks::Kind::FmKick, fm_freq(36.0)),
                        _ => (peaks::Kind::FmSnare, fm_freq(60.0)),
                    };
                    let u16k = |x: f32| (x.clamp(0.0, 1.0) * 65535.0) as u32;
                    let k = peaks::Knobs { frequency, tone: u16k(tone), decay: (32768.0 + 16384.0 * d.log2()).clamp(0.0, 65535.0) as u32, a: u16k(a), b: u16k(b) };
                    self.peaks.trigger(kind, k, vel, sr);
                }
            }
            Family::Macro => {
                let model = match pad.voice {
                    DrumVoice::MkKick => macro_drums::Model::Kick,
                    DrumVoice::MkSnare => macro_drums::Model::Snare,
                    DrumVoice::MkHat => macro_drums::Model::Hat,
                    DrumVoice::MkClap => macro_drums::Model::Clap,
                    DrumVoice::MkTom => macro_drums::Model::Tom,
                    DrumVoice::MkRim => macro_drums::Model::Rim,
                    DrumVoice::MkFm => macro_drums::Model::Fm,
                    DrumVoice::MkNoise => macro_drums::Model::Noise,
                    DrumVoice::MkCowbell => macro_drums::Model::Cowbell,
                    DrumVoice::MkCymbal => macro_drums::Model::Cymbal,
                    DrumVoice::MkShaker => macro_drums::Model::Shaker,
                    DrumVoice::MkClave => macro_drums::Model::Clave,
                    _ => macro_drums::Model::Kick,
                };
                let (base_hz, secs) = macro_base(model);
                let (a, b) = (pad.a.clamp(0.0, 1.0), pad.b.clamp(0.0, 1.0));
                let fm_knob = matches!(pad.voice.macros()[1], Some("FM"));
                let m = macro_drums::Macros { decay: macro_drums::decay_for(model, secs * d), tone, sweep: a, noise: if fm_knob { 0.5 } else { b }, fm: if fm_knob { b } else { 0.5 }, drive: 0.0 };
                // (velocity sets its loudness in the rack)
                self.macro_voice.trigger(model, base_hz * tr, m, 1.0, sr);
            }
            Family::Chip => {
                let kind = match pad.voice {
                    DrumVoice::ChipConsoleKick => chip_drums::Kind::ConsoleKick,
                    DrumVoice::ChipConsoleSampleKick => chip_drums::Kind::ConsoleSampleKick,
                    DrumVoice::ChipConsoleSnare => chip_drums::Kind::ConsoleSnare,
                    DrumVoice::ChipConsoleSampleSnare => chip_drums::Kind::ConsoleSampleSnare,
                    DrumVoice::ChipConsoleHat => chip_drums::Kind::ConsoleHat,
                    DrumVoice::ChipConsoleOpenHat => chip_drums::Kind::ConsoleOpenHat,
                    DrumVoice::ChipConsoleTom => chip_drums::Kind::ConsoleTom,
                    DrumVoice::ChipConsoleCrash => chip_drums::Kind::ConsoleCrash,
                    DrumVoice::ChipHandheldKick => chip_drums::Kind::HandheldKick,
                    DrumVoice::ChipHandheldSnare => chip_drums::Kind::HandheldSnare,
                    DrumVoice::ChipHandheldHat => chip_drums::Kind::HandheldHat,
                    DrumVoice::ChipHandheldOpenHat => chip_drums::Kind::HandheldOpenHat,
                    DrumVoice::ChipHandheldTom => chip_drums::Kind::HandheldTom,
                    DrumVoice::ChipHandheldClap => chip_drums::Kind::HandheldClap,
                    DrumVoice::ChipMicroKick => chip_drums::Kind::MicroKick,
                    DrumVoice::ChipMicroSnare => chip_drums::Kind::MicroSnare,
                    DrumVoice::ChipMicroTom => chip_drums::Kind::MicroTom,
                    DrumVoice::ChipMicroClap => chip_drums::Kind::MicroClap,
                    DrumVoice::ChipMicroRim => chip_drums::Kind::MicroRim,
                    DrumVoice::ChipMicroHat => chip_drums::Kind::MicroHat,
                    DrumVoice::ChipMicroOpenHat => chip_drums::Kind::MicroOpenHat,
                    DrumVoice::ChipMicroPunchKick => chip_drums::Kind::MicroPunchKick,
                    DrumVoice::ChipMicroPunchSnare => chip_drums::Kind::MicroPunchSnare,
                    DrumVoice::ChipMicroPunchClap => chip_drums::Kind::MicroPunchClap,
                    DrumVoice::ChipMicroPunchRim => chip_drums::Kind::MicroPunchRim,
                    DrumVoice::ChipMicroPunchHat => chip_drums::Kind::MicroPunchHat,
                    DrumVoice::ChipMicroPunchOpenHat => chip_drums::Kind::MicroPunchOpenHat,
                    DrumVoice::ChipMicroPunchTom => chip_drums::Kind::MicroPunchTom,
                    DrumVoice::ChipArcadeKick => chip_drums::Kind::ArcadeKick,
                    DrumVoice::ChipArcadeSnare => chip_drums::Kind::ArcadeSnare,
                    DrumVoice::ChipArcadeTom => chip_drums::Kind::ArcadeTom,
                    DrumVoice::ChipArcadeHat => chip_drums::Kind::ArcadeHat,
                    DrumVoice::ChipArcadeOpenHat => chip_drums::Kind::ArcadeOpenHat,
                    DrumVoice::ChipArcadeCowbell => chip_drums::Kind::ArcadeCowbell,
                    DrumVoice::ChipBeeperKick => chip_drums::Kind::BeeperKick,
                    DrumVoice::ChipBeeperSnare => chip_drums::Kind::BeeperSnare,
                    DrumVoice::ChipBeeperHat => chip_drums::Kind::BeeperHat,
                    DrumVoice::ChipBeeperTom => chip_drums::Kind::BeeperTom,
                    DrumVoice::ChipWoodKick => chip_drums::Kind::WoodKick,
                    DrumVoice::ChipWoodSnare => chip_drums::Kind::WoodSnare,
                    DrumVoice::ChipWoodHat => chip_drums::Kind::WoodHat,
                    DrumVoice::ChipWoodTom => chip_drums::Kind::WoodTom,
                    DrumVoice::ChipWoodZap => chip_drums::Kind::WoodZap,
                    DrumVoice::ChipWoodBoom => chip_drums::Kind::WoodBoom,
                    _ => chip_drums::Kind::ConsoleKick,
                };
                let k = ChipKnobs { tune: tr, decay: d, tone, sweep: pad.a, noise: pad.b };
                self.chip.trigger(kind, k, 1.0, sr);
            }
            Family::Classic => {}
        }
        match pad.voice {
            DrumVoice::Kick => {
                self.amp_coef = decay_coef(0.5 * d, sr);
                self.env2_coef = decay_coef(0.07, sr);
                self.env3_coef = decay_coef(0.005, sr);
            }
            DrumVoice::Snare => {
                self.amp_coef = decay_coef(0.28 * d, sr);
                self.env2_coef = decay_coef(0.14 * d, sr);
                self.f1.set(1200.0 + tone * 3000.0, 0.0, sr);
            }
            DrumVoice::ClosedHat | DrumVoice::OpenHat => {
                let len = if pad.voice == DrumVoice::ClosedHat { 0.09 } else { 0.7 };
                self.amp_coef = decay_coef(len * d, sr);
                self.env2_coef = 0.0;
                self.f1.set_q((8000.0 + tone * 4000.0) * tr.sqrt(), 1.2, sr);
                self.f2.set(6500.0, 0.0, sr);
            }
            DrumVoice::Clap => {
                self.amp_coef = decay_coef(0.4 * d, sr);
                self.env2_coef = 0.0;
                self.f1.set_q(1100.0 * tr * (0.8 + tone * 0.6), 2.0, sr);
            }
            DrumVoice::Tom => {
                self.amp_coef = decay_coef(0.55 * d, sr);
                self.env2_coef = decay_coef(0.18, sr);
            }
            DrumVoice::Rim => {
                self.amp_coef = decay_coef(0.06 * d, sr);
                self.env2_coef = decay_coef(0.02, sr);
                self.f1.set_q(3200.0 * tr, 3.0, sr);
            }
            DrumVoice::Cowbell => {
                self.amp_coef = decay_coef(0.45 * d, sr);
                self.env2_coef = decay_coef(0.04, sr);
                self.f1.set_q(2640.0 * tr, 2.5, sr);
            }
            _ => {}
        }
        if !pad.layers.is_empty() {
            // The layer for this velocity (a little overlap at the borders,
            // as hands vary), and the next of its round robins.
            let n = pad.layers.len();
            let x = (self.vel * n as f32 + 0.3 * self.rng.bipolar()).clamp(0.0, n as f32 - 0.001);
            self.layer = x as usize;
            self.rr = (self.rr + 1) % pad.layers[self.layer].len().max(1);
        }
        if let Some(s) = played_sample(pad, self.layer, self.rr) {
            // Samples fade out over their length times the decay setting.
            let secs = s.duration() as f32 * d.min(1.0);
            self.amp_coef = if d >= 1.0 { 1.0 } else { decay_coef(secs.max(0.01), sr) };
        }
    }

    /// Renders a block of a Mutable model into `out` (`t1`, `t2`: scratch
    /// of the same length).
    fn render_model(&mut self, pad: &DrumPad, tr: f32, sr: f32, out: &mut [f32], t1: &mut [f32], t2: &mut [f32]) {
        let hit = std::mem::take(&mut self.model_hit);
        let voice = pad.voice;
        let f0 = (voice.base_hz() * tr / sr).clamp(1e-5, 0.4);
        let tone = pad.tone.clamp(0.0, 1.0);
        let decay = model_decay(voice, pad.decay);
        let (a, b) = (pad.a.clamp(0.0, 1.0), pad.b.clamp(0.0, 1.0));
        let accent = self.vel.clamp(0.0, 1.0);
        // The knobs map as on Plaits' drum engines.
        match &mut self.model {
            Model::AnalogKick(d, drive) => {
                let attack_fm = (a * 4.0).min(1.0);
                let self_fm = (a * 4.0 - 1.0).clamp(0.0, 1.0);
                let amount = (a * 2.0 - 1.0).max(0.0) * (1.0 - 16.0 * f0).max(0.0);
                d.render(false, hit, accent, f0, tone, decay, attack_fm, self_fm, out);
                drive.process(0.5 + 0.5 * amount, out);
            }
            Model::SynthKick(d) => d.render(false, hit, accent, f0, tone, decay, a, (b * 2.0).min(1.0), (b * 2.0 - 1.0).max(0.0), out),
            Model::AnalogSnare(d) => d.render(false, hit, accent, f0, tone, decay, a, out),
            Model::SynthSnare(d) => d.render(false, hit, accent, f0, tone, decay, a, out),
            Model::Hat(d) => {
                let ring = voice == DrumVoice::RingHat;
                let (noise, vca) = if ring { (NoiseType::RingMod, VcaType::Linear) } else { (NoiseType::Square, VcaType::Swing) };
                d.render(false, hit, accent, f0, tone, decay, a, t1, t2, out, noise, vca, !ring, ring);
            }
            Model::None => out.fill(0.0),
        }
        let trim = voice.trim();
        let mut peak = 0.0f32;
        for v in out.iter_mut() {
            *v *= trim;
            peak = peak.max(v.abs());
        }
        // Done once it has been quiet for a while.
        let n = out.len() as u32;
        self.t = self.t.saturating_add(n);
        if self.t as f32 > 0.02 * sr && peak < 1e-4 {
            self.quiet += n;
        } else {
            self.quiet = 0;
        }
        if self.quiet as f32 > 0.05 * sr || !out.iter().all(|v| v.is_finite()) {
            self.active = false;
        }
    }

    #[inline]
    fn tick(&mut self, pad: &DrumPad, tr: f32, sr: f32) -> f32 {
        match pad.voice.family() {
            Family::Resonant => {
                let circuit = pad.voice == DrumVoice::RsCircuitKick;
                // (the circuit kick's click is sharp and its ring soft: a soft clip evens them)
                let y = if circuit { (self.circuit.tick() * 3.0).tanh() * 0.9 } else { self.peaks.tick() };
                if !(if circuit { self.circuit.is_active() } else { self.peaks.is_active() }) {
                    self.active = false;
                }
                return y;
            }
            Family::Macro => {
                let y = self.macro_voice.tick() * 0.7;
                if !self.macro_voice.is_active() {
                    self.active = false;
                }
                return y;
            }
            Family::Chip => {
                let y = self.chip.tick() * 1.4;
                if !self.chip.is_active() {
                    self.active = false;
                }
                return y;
            }
            _ => {}
        }
        if pad.voice.family() == Family::Retro {
            let y = self.retro.tick(pad.voice, &mut self.rng);
            if !self.retro.is_active() {
                self.active = false;
            }
            return y;
        }
        if pad.voice.family() == Family::Acoustic {
            let y = self.acoustic.tick(tr);
            if !self.acoustic.is_active() {
                self.active = false;
            }
            return y;
        }
        if pad.voice.family() == Family::Machine {
            let y = self.machine.tick(pad.voice, tr, &mut self.rng);
            if !self.machine.is_active() {
                self.active = false;
            }
            return y;
        }
        if pad.voice.family() == Family::Shaker {
            let (sl, sr2) = self.shaker.tick(&mut self.rng);
            let y = 0.5 * (sl + sr2);
            self.t = self.t.saturating_add(1);
            if self.shaker.is_done() {
                self.active = false;
            }
            return y;
        }
        let tone = pad.tone;
        let out = match pad.voice {
            DrumVoice::Kick => {
                let f0 = 44.0 * tr;
                let f1 = (150.0 + tone * 140.0) * tr;
                let f = f0 + (f1 - f0) * self.env2;
                self.phase[0] = wrap_phase(self.phase[0] + f / sr);
                let body = (self.phase[0] * TAU).sin() * self.amp;
                let click = self.rng.bipolar() * self.env3 * (0.1 + tone * 0.5);
                fast_tanh((body + click) * 1.6) * 0.9
            }
            DrumVoice::Snare => {
                self.phase[0] = wrap_phase(self.phase[0] + 185.0 * tr / sr);
                self.phase[1] = wrap_phase(self.phase[1] + 330.0 * tr / sr);
                let body = ((self.phase[0] * TAU).sin() * 0.6 + (self.phase[1] * TAU).sin() * 0.4) * self.env2;
                let n = self.rng.bipolar();
                let nb = self.f1.tick(n);
                let noise = (nb.high * 0.7 + nb.band * 0.5) * self.amp;
                body * (0.8 - tone * 0.4) + noise * (0.45 + tone * 0.55)
            }
            DrumVoice::ClosedHat | DrumVoice::OpenHat => {
                let mut sum = 0.0;
                for (k, f) in METAL.iter().enumerate() {
                    self.phase[k] = wrap_phase(self.phase[k] + f * 1.4 * tr / sr);
                    sum += if self.phase[k] < 0.5 { 1.0 } else { -1.0 };
                }
                let x = sum * (1.0 - tone * 0.5) / 6.0 + self.rng.bipolar() * (0.25 + tone * 0.5);
                let y = self.f1.process(x, FilterMode::Band);
                let y = self.f2.process(y, FilterMode::High);
                y * self.amp * 1.8
            }
            DrumVoice::Clap => {
                let ts = self.t as f32 / sr;
                let burst = if ts < 0.032 {
                    let tb = ts % 0.0105;
                    (-tb / 0.0022).exp()
                } else {
                    0.0
                };
                let env = burst.max(self.amp * 0.55);
                let n = self.rng.bipolar();
                self.f1.process(n, FilterMode::Band) * env * 2.2
            }
            DrumVoice::Tom => {
                let f = (95.0 + 75.0 * self.env2) * tr * (1.0 + tone * 0.6);
                self.phase[0] = wrap_phase(self.phase[0] + f / sr);
                let body = (self.phase[0] * TAU).sin() * self.amp;
                body + self.rng.bipolar() * self.env3 * 0.2
            }
            DrumVoice::Rim => {
                self.phase[0] = wrap_phase(self.phase[0] + 1700.0 * tr / sr);
                let ping = (self.phase[0] * TAU).sin() * self.env2;
                let n = self.f1.process(self.rng.bipolar(), FilterMode::Band);
                (ping * 0.6 + n * (0.6 + tone)) * self.amp
            }
            DrumVoice::Cowbell => {
                self.phase[0] = wrap_phase(self.phase[0] + 540.0 * tr / sr);
                self.phase[1] = wrap_phase(self.phase[1] + 800.0 * tr / sr);
                let a = if self.phase[0] < 0.5 { 1.0 } else { -1.0 };
                let b = if self.phase[1] < 0.5 { 1.0 } else { -1.0 };
                let y = self.f1.process((a + b) * 0.5, FilterMode::Band);
                y * (self.amp * 0.6 + self.env2 * 0.5) * 1.5
            }
            _ => 0.0,
        };
        self.amp *= self.amp_coef;
        self.env2 *= self.env2_coef;
        self.env3 *= self.env3_coef;
        self.t = self.t.saturating_add(1);
        if self.amp < 1e-4 && self.env2 < 1e-4 {
            self.active = false;
        }
        out
    }
}

/// The recording a pad plays: its layer's round robin, or its sample.
fn played_sample(pad: &DrumPad, layer: usize, rr: usize) -> Option<&crate::sample::SampleRef> {
    match pad.layers.get(layer) {
        Some(l) if !l.is_empty() => l.get(rr % l.len()),
        _ => pad.sample.as_ref(),
    }
}

pub struct DrumState {
    voices: [PadVoice; MAX_PADS],
}

impl Default for DrumState {
    fn default() -> Self {
        Self::new()
    }
}

/// The choke group of a pad (0: none). Hats of projects from before choke
/// groups existed still cut each other.
fn choke_group(pad: &DrumPad) -> u8 {
    if pad.choke > 0 {
        pad.choke
    } else if matches!(pad.voice, DrumVoice::ClosedHat | DrumVoice::OpenHat) {
        1
    } else {
        0
    }
}

impl DrumState {
    pub fn new() -> Self {
        let voices = std::array::from_fn(|i| PadVoice::new(0xC0FF_EE00 ^ (i as u32 * 104_729 + 7)));
        Self { voices }
    }

    pub fn note_on(&mut self, pads: &[DrumPad], note: u8, vel: f32, sr: f32) {
        for (i, pad) in pads.iter().enumerate().take(MAX_PADS) {
            if pad.note != note {
                continue;
            }
            // A hit cuts off the other pads of its choke group.
            let group = choke_group(pad);
            if group > 0 {
                for (j, other) in pads.iter().enumerate().take(MAX_PADS) {
                    if j != i && choke_group(other) == group && self.voices[j].active {
                        self.voices[j].choking = true;
                    }
                }
            }
            self.voices[i].trigger(pad, vel, sr);
        }
    }

    pub fn all_off(&mut self) {
        for v in self.voices.iter_mut() {
            if v.active {
                v.choking = true;
            }
        }
    }

    pub fn render(&mut self, pads: &[DrumPad], l: &mut [f32], r: &mut [f32], sr: f32) {
        let n = l.len().min(r.len());
        let choke_coef = decay_coef(0.015, sr);
        for (i, pad) in pads.iter().enumerate().take(MAX_PADS) {
            let v = &mut self.voices[i];
            if !v.active {
                continue;
            }
            // Recorded layers carry most of the dynamics themselves.
            let vel_gain = pad.vel_gain(v.vel);
            let level = pad.level_db + if pad.layers.is_empty() { 0.0 } else { pad.trim_db };
            let gain = db_to_gain(level) * vel_gain * 0.32;
            let (pl, pr) = pan_gains(pad.pan);
            let tr = 2f32.powf(pad.tune / 12.0);
            if let Some((tl, trr, mut tpos, mut tg)) = v.tail {
                if let Some(ts) = played_sample(pad, tl, trr) {
                    let trate = tr as f64 * ts.sample_rate as f64 / sr as f64;
                    let fade = decay_coef(if ts.duration() > 1.5 { 0.35 } else { 0.08 }, sr);
                    let base = db_to_gain(level) * 0.32;
                    for i in 0..n {
                        if tpos >= ts.frames() as f64 || tg < 1e-4 {
                            tg = 0.0;
                            break;
                        }
                        let (a, b) = ts.read(tpos);
                        l[i] += a * tg * base * pl;
                        r[i] += b * tg * base * pr;
                        tpos += trate;
                        tg *= fade;
                        if v.choking {
                            tg *= choke_coef;
                        }
                    }
                } else {
                    tg = 0.0;
                }
                v.tail = (tg > 1e-4).then_some((tl, trr, tpos, tg));
            }
            if let Some(s) = played_sample(pad, v.layer, v.rr) {
                let rate = tr as f64 * s.sample_rate as f64 / sr as f64;
                let frames = s.frames() as f64;
                for i in 0..n {
                    if v.sample_pos >= frames {
                        v.active = false;
                        break;
                    }
                    let (sl, sr2) = s.read(v.sample_pos);
                    let g = gain * v.amp * v.choke;
                    l[i] += sl * g * pl;
                    r[i] += sr2 * g * pr;
                    v.sample_pos += rate;
                    v.amp *= v.amp_coef;
                    if v.choking {
                        v.choke *= choke_coef;
                    }
                    if v.amp < 1e-4 || v.choke < 1e-4 {
                        v.active = false;
                        break;
                    }
                }
                continue;
            }
            if pad.voice.family() == Family::Mutable {
                let (mut buf, mut t1, mut t2) = ([0.0f32; MODEL_BLOCK], [0.0f32; MODEL_BLOCK], [0.0f32; MODEL_BLOCK]);
                let mut off = 0;
                while off < n && v.active {
                    let m = (n - off).min(MODEL_BLOCK);
                    v.render_model(pad, tr, sr, &mut buf[..m], &mut t1[..m], &mut t2[..m]);
                    for k in 0..m {
                        let y = buf[k] * gain * v.choke;
                        l[off + k] += y * pl;
                        r[off + k] += y * pr;
                        if v.choking {
                            v.choke *= choke_coef;
                        }
                    }
                    if v.choke < 1e-4 {
                        v.active = false;
                    }
                    off += m;
                }
                continue;
            }
            for i in 0..n {
                let y = v.tick(pad, tr, sr) * gain * v.choke;
                l[i] += y * pl;
                r[i] += y * pr;
                if v.choking {
                    v.choke *= choke_coef;
                    if v.choke < 1e-4 {
                        v.active = false;
                    }
                }
                if !v.active {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Plays `pad` once and returns the left channel for `secs` seconds.
    fn hit(pad: &DrumPad, secs: f32) -> (Vec<f32>, bool) {
        let sr = 48000.0;
        let pads = vec![pad.clone()];
        let mut d = DrumState::new();
        d.note_on(&pads, pad.note, 1.0, sr);
        let mut out = Vec::new();
        for _ in 0..(secs * sr / 256.0) as usize {
            let (mut l, mut r) = ([0.0f32; 256], [0.0f32; 256]);
            d.render(&pads, &mut l, &mut r, sr);
            out.extend_from_slice(&l);
        }
        (out, d.voices[0].active)
    }

    #[test]
    fn every_voice_sounds_and_ends() {
        for (_, voices) in DrumVoice::GROUPS {
            for &v in voices {
                let pad = DrumPad::new(v);
                let (out, still) = hit(&pad, 8.0);
                let peak = out.iter().fold(0.0f32, |m, x| m.max(x.abs()));
                let rms = (out[..4800].iter().map(|x| x * x).sum::<f32>() / 4800.0).sqrt();
                eprintln!("{:<13} peak {peak:.3} rms {rms:.4}", v.name());
                assert!(out.iter().all(|x| x.is_finite()), "{}", v.name());
                if std::env::var_os("DRUM_CAL").is_some() {
                    continue;
                }
                assert!(peak > 0.05 && peak < 1.5, "{}: peak {peak}", v.name());
                assert!(!still, "{} still plays after 8 s", v.name());
            }
        }
    }

    #[test]
    fn every_kit_loads_and_plays() {
        for name in kit_names() {
            let pads = kit(name);
            assert!((8..=MAX_PADS).contains(&pads.len()), "{name}");
            let mut d = DrumState::new();
            for pad in &pads {
                d.note_on(&pads, pad.note, 0.9, 48000.0);
            }
            let (mut l, mut r) = (vec![0.0; 4800], vec![0.0; 4800]);
            d.render(&pads, &mut l, &mut r, 48000.0);
            let peak = l.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(peak > 0.05 && peak < 4.0, "{name}: {peak}");
        }
    }

    #[test]
    fn a_recording_hit_again_rings_on_under_the_new_hit() {
        use crate::sample::{SampleData, SampleRef};
        // A long, steady "cymbal".
        let take = SampleRef::new(SampleData::from_buffers("c", None, 48000, vec![0.5; 96000], vec![0.5; 96000]));
        let mut pad = DrumPad::new(DrumVoice::AcCrash);
        pad.level_db = 0.0;
        pad.layers = vec![vec![take]];
        let pads = vec![pad];
        let mut d = DrumState::new();
        d.note_on(&pads, pads[0].note, 1.0, 48000.0);
        let (mut l, mut r) = (vec![0.0; 4800], vec![0.0; 4800]);
        d.render(&pads, &mut l, &mut r, 48000.0);
        let alone = l[100];
        d.note_on(&pads, pads[0].note, 1.0, 48000.0);
        let (mut l, mut r) = (vec![0.0; 64], vec![0.0; 64]);
        d.render(&pads, &mut l, &mut r, 48000.0);
        // Right after the second hit both sound: more than one hit alone.
        assert!(l[1] > alone * 1.5, "{} vs {alone}", l[1]);
    }

    #[test]
    fn recorded_layers_follow_the_velocity_and_take_turns() {
        use crate::sample::{SampleData, SampleRef};
        // Two layers (a soft and a loud level) with two round robins each,
        // told apart by their level: 0.1, 0.2 (soft) and 0.8, 0.9 (loud).
        let take = |v: f32| SampleRef::new(SampleData::from_buffers("t", None, 48000, vec![v; 4800], vec![v; 4800]));
        let mut pad = DrumPad::new(DrumVoice::AcSnare);
        pad.level_db = 0.0;
        pad.layers = vec![vec![take(0.1), take(0.2)], vec![take(0.8), take(0.9)]];
        let pads = vec![pad];
        let mut d = DrumState::new();
        let mut levels = Vec::new();
        for vel in [0.1, 0.1, 1.0, 1.0] {
            d.note_on(&pads, pads[0].note, vel, 48000.0);
            let (mut l, mut r) = (vec![0.0; 64], vec![0.0; 64]);
            d.render(&pads, &mut l, &mut r, 48000.0);
            // Undo the pad's gain to see which take played.
            let gain = db_to_gain(0.0) * vel.powf(0.5) * 0.32 * pan_gains(0.0).0;
            levels.push(l[10] / gain);
        }
        let near = |a: f32, b: f32| (a - b).abs() < 0.02;
        assert!(near(levels[0], 0.1) || near(levels[0], 0.2), "{levels:?}");
        assert!(!near(levels[0], levels[1]), "round robins take turns: {levels:?}");
        assert!(levels[2] > 0.7 && levels[3] > 0.7 && !near(levels[2], levels[3]), "{levels:?}");
    }

    #[test]
    fn every_kit_belongs_to_one_engine_and_uses_its_voices() {
        for e in DrumEngine::ALL {
            let allowed: Vec<DrumVoice> = e.groups().iter().flat_map(|g| g.1.iter().copied()).collect();
            for name in e.kits() {
                assert_eq!(engine_of(name), e, "{name}");
                for p in kit(name) {
                    assert!(allowed.contains(&p.voice), "{name}: {} plays {:?}", p.name, p.voice);
                }
            }
        }
        // Old names still load.
        assert_eq!(engine_of("909 Krab"), DrumEngine::Rack);
        assert_eq!(kit("909 Krab"), kit("Classic 909"));
        assert_eq!(engine_of("808"), DrumEngine::M808);
    }

    #[test]
    fn a_choke_group_cuts_its_other_pads() {
        let pads = kit("Mutable Analog");
        let open = pads.iter().position(|p| p.name == "Open Hat").unwrap();
        let closed = pads.iter().position(|p| p.name == "Closed Hat").unwrap();
        let mut d = DrumState::new();
        let sr = 48000.0;
        d.note_on(&pads, pads[open].note, 1.0, sr);
        let (mut l, mut r) = (vec![0.0; 2400], vec![0.0; 2400]);
        d.render(&pads, &mut l, &mut r, sr);
        assert!(d.voices[open].active);
        d.note_on(&pads, pads[closed].note, 1.0, sr);
        for _ in 0..4 {
            d.render(&pads, &mut l, &mut r, sr);
        }
        assert!(!d.voices[open].active, "the open hat was choked");
    }

    #[test]
    fn velocity_mod_lengthens_and_brightens_a_crash() {
        // A crash, hit softly and hard: with Vel > Decay up the hard hit
        // rings longer; with Vel > Level at 0 both are as loud.
        let mut pad = DrumPad::new(DrumVoice::AcCrash);
        pad.vel_decay = 1.0;
        let (_, soft) = pad.vel_tone_decay(0.3);
        let (_, hard) = pad.vel_tone_decay(1.0);
        assert!(hard > 2.0 * soft, "{soft} {hard}");
        pad.vel_tone = 1.0;
        assert!(pad.vel_tone_decay(1.0).0 > pad.vel_tone_decay(0.3).0);
        pad.vel_level = 0.0;
        assert!((pad.vel_gain(0.2) - pad.vel_gain(1.0)).abs() < 1e-6);
        // Heard: the hard hit is still ringing when the soft one is gone.
        let tail = |vel: f32| {
            let pads = vec![pad.clone()];
            let mut d = DrumState::new();
            d.note_on(&pads, pads[0].note, vel, 48000.0);
            let mut l = vec![0.0f32; 48000 * 3];
            let mut r = vec![0.0f32; 48000 * 3];
            for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
                d.render(&pads, cl, cr, 48000.0);
            }
            l[48000 * 2..].iter().map(|v| v * v).sum::<f32>()
        };
        assert!(tail(1.0) > 4.0 * tail(0.3), "{} {}", tail(1.0), tail(0.3));
    }

    #[test]
    fn velocity_and_tune_change_a_mutable_kick() {
        let mut pad = DrumPad::new(DrumVoice::AnalogKick);
        let (loud, _) = hit(&pad, 0.5);
        pad.tune = 12.0;
        let (high, _) = hit(&pad, 0.5);
        let crossings = |x: &[f32]| x.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
        assert!(crossings(&high[2400..12000]) > crossings(&loud[2400..12000]) * 3 / 2, "{} vs {}", crossings(&high[2400..12000]), crossings(&loud[2400..12000]));
    }

    #[test]
    fn every_pad_makes_sound_and_ends() {
        let pads = kit("Classic 909");
        let mut d = DrumState::new();
        for pad in &pads {
            d.note_on(&pads, pad.note, 1.0, 48000.0);
        }
        let mut l = vec![0.0; 48000];
        let mut r = vec![0.0; 48000];
        d.render(&pads, &mut l, &mut r, 48000.0);
        let peak = l.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak > 0.1 && peak < 4.0, "peak {peak}");
        for _ in 0..10 {
            d.render(&pads, &mut l, &mut r, 48000.0);
        }
        assert!(d.voices.iter().all(|v| !v.active));
    }
}

