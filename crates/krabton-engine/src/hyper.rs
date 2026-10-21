//! Hyper: a Virus emulated by its own firmware, played as one of Krabton's
//! instruments (The Usual Suspects' DSP56300 emulator and virusLib, compiled
//! in: see the krabton-virus crate). The player brings the firmware (a Virus
//! TI's, or an A, B or C operating system); Krabton keeps where it is, never
//! the firmware itself. A device boots from it on a thread of its own, which
//! takes a few seconds, and the track is silent until then. The factory
//! presets come from the firmware; the knobs send the Virus's own controllers
//! (a controller number is the parameter's place on its first page), so a
//! preset shows its own values on them.

use crate::params::{ParamSpec, Unit};
use krabton_virus::{Rom, Synth};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, Weak};

/// The controller each knob sends (the knobs are `SPECS`, in order).
pub const CCS: [u8; 29] = [17, 22, 26, 33, 34, 37, 27, 40, 42, 44, 46, 54, 55, 56, 58, 59, 60, 61, 63, 67, 74, 79, 88, 105, 113, 114, 115, 91, 10];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::int("Shape 1", "Oscillators", 0.0, 127.0, 64.0, Unit::None),
    ParamSpec::int("Shape 2", "Oscillators", 0.0, 127.0, 64.0, Unit::None),
    ParamSpec::int("Detune", "Oscillators", 0.0, 127.0, 0.0, Unit::None),
    ParamSpec::int("Osc Mix", "Oscillators", 0.0, 127.0, 64.0, Unit::None),
    ParamSpec::int("Sub", "Oscillators", 0.0, 127.0, 0.0, Unit::None),
    ParamSpec::int("Noise", "Oscillators", 0.0, 127.0, 0.0, Unit::None),
    ParamSpec::int("FM", "Oscillators", 0.0, 127.0, 0.0, Unit::None),
    ParamSpec::int("Cutoff", "Filter", 0.0, 127.0, 127.0, Unit::None),
    ParamSpec::int("Resonance", "Filter", 0.0, 127.0, 0.0, Unit::None),
    ParamSpec::int("Env Amount", "Filter", 0.0, 127.0, 0.0, Unit::None),
    ParamSpec::int("Key Track", "Filter", 0.0, 127.0, 64.0, Unit::None),
    ParamSpec::int("F Attack", "Filter Env", 0.0, 127.0, 0.0, Unit::None),
    ParamSpec::int("F Decay", "Filter Env", 0.0, 127.0, 64.0, Unit::None),
    ParamSpec::int("F Sustain", "Filter Env", 0.0, 127.0, 64.0, Unit::None),
    ParamSpec::int("F Release", "Filter Env", 0.0, 127.0, 32.0, Unit::None),
    ParamSpec::int("Attack", "Amp Env", 0.0, 127.0, 0.0, Unit::None),
    ParamSpec::int("Decay", "Amp Env", 0.0, 127.0, 64.0, Unit::None),
    ParamSpec::int("Sustain", "Amp Env", 0.0, 127.0, 100.0, Unit::None),
    ParamSpec::int("Release", "Amp Env", 0.0, 127.0, 32.0, Unit::None),
    ParamSpec::int("LFO 1 Rate", "LFO", 0.0, 127.0, 64.0, Unit::None),
    ParamSpec::int("Vibrato", "LFO", 0.0, 127.0, 64.0, Unit::None),
    ParamSpec::int("LFO 2 Rate", "LFO", 0.0, 127.0, 64.0, Unit::None),
    ParamSpec::int("LFO > Cutoff", "LFO", 0.0, 127.0, 64.0, Unit::None),
    ParamSpec::int("Chorus", "Effects", 0.0, 127.0, 0.0, Unit::None),
    ParamSpec::int("Delay", "Effects", 0.0, 127.0, 0.0, Unit::None),
    ParamSpec::int("Delay Time", "Effects", 0.0, 127.0, 64.0, Unit::None),
    ParamSpec::int("Feedback", "Effects", 0.0, 127.0, 32.0, Unit::None),
    ParamSpec::int("Volume", "Output", 0.0, 127.0, 100.0, Unit::None),
    ParamSpec::int("Pan", "Output", 0.0, 127.0, 64.0, Unit::None),
];

// ---------------------------------------------------------------- firmware

/// A factory preset: its name and the values of its first page (where the
/// controllers reach).
#[derive(Clone)]
pub struct FactoryPreset {
    pub name: String,
    pub page_a: [u8; 128],
}

impl FactoryPreset {
    /// The knobs as this preset has them.
    pub fn knobs(&self) -> Vec<f32> {
        CCS.iter().map(|&cc| self.page_a[cc as usize] as f32).collect()
    }
}

pub struct FirmwareData {
    pub name: String,
    pub path: Option<PathBuf>,
    pub rom: Option<Rom>,
    /// The model it is for ("TI2", "C", ...).
    pub model: String,
    /// The factory presets, bank by bank.
    pub banks: Vec<Vec<FactoryPreset>>,
}

/// A firmware read, shared; equal when it is the same one; saved as its
/// name and path.
#[derive(Clone)]
pub struct FirmwareRef(pub Arc<FirmwareData>);

impl std::ops::Deref for FirmwareRef {
    type Target = FirmwareData;
    fn deref(&self) -> &FirmwareData {
        &self.0
    }
}

impl PartialEq for FirmwareRef {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl fmt::Debug for FirmwareRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Firmware({}, {})", self.name, self.model)
    }
}

static OPEN: Mutex<Vec<(PathBuf, Weak<FirmwareData>)>> = Mutex::new(Vec::new());

impl FirmwareRef {
    /// Reads a firmware file (or shares the one already open), with its presets.
    pub fn load(path: &Path) -> Result<Self, String> {
        if let Some(open) = OPEN.lock().ok().and_then(|o| o.iter().find(|(p, _)| p == path).and_then(|(_, w)| w.upgrade())) {
            return Ok(FirmwareRef(open));
        }
        let rom = Rom::open(path)?;
        let mut banks = Vec::new();
        for b in 0..rom.banks().min(64) {
            let mut bank = Vec::new();
            for p in 0..rom.presets_per_bank().min(128) {
                let (Some(name), Some(data)) = (rom.preset_name(b, p), rom.preset_data(b, p)) else { continue };
                let mut page_a = [0u8; 128];
                let n = data.len().min(128);
                page_a[..n].copy_from_slice(&data[..n]);
                bank.push(FactoryPreset { name, page_a });
            }
            banks.push(bank);
        }
        let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Firmware".into());
        let model = rom.model_name();
        let r = FirmwareRef(Arc::new(FirmwareData { name, path: Some(path.to_path_buf()), rom: Some(rom), model, banks }));
        if let Ok(mut open) = OPEN.lock() {
            open.retain(|(_, w)| w.strong_count() > 0);
            open.push((path.to_path_buf(), Arc::downgrade(&r.0)));
        }
        Ok(r)
    }

    /// A firmware that could not be read: silent, its path kept.
    pub fn missing(name: String, path: Option<PathBuf>) -> Self {
        FirmwareRef(Arc::new(FirmwareData { name, path, rom: None, model: String::new(), banks: Vec::new() }))
    }

    pub fn is_missing(&self) -> bool {
        self.rom.is_none()
    }

    pub fn preset(&self, bank: u16, program: u16) -> Option<&FactoryPreset> {
        self.banks.get(bank as usize).and_then(|b| b.get(program as usize))
    }

    /// The bank's name, as the Virus calls them (A, B, ...).
    pub fn bank_name(bank: u16) -> String {
        if bank < 26 {
            format!("Bank {}", (b'A' + bank as u8) as char)
        } else {
            format!("Bank {}", bank + 1)
        }
    }
}

#[derive(Serialize, Deserialize)]
struct FirmwareOnDisk {
    name: String,
    path: Option<PathBuf>,
}

impl Serialize for FirmwareRef {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        FirmwareOnDisk { name: self.name.clone(), path: self.path.clone() }.serialize(s)
    }
}

impl<'de> Deserialize<'de> for FirmwareRef {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let disk = FirmwareOnDisk::deserialize(d)?;
        let Some(path) = disk.path else {
            return Ok(FirmwareRef::missing(disk.name, None));
        };
        Ok(match FirmwareRef::load(&path) {
            Ok(r) => r,
            Err(e) => {
                crate::sample::note_missing(format!("{}: {e}", disk.name));
                FirmwareRef::missing(disk.name, Some(path))
            }
        })
    }
}

// ---------------------------------------------------------------- the device

/// How far a device is.
#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Off,
    Booting,
    Ready,
    Failed(String),
}

const OFF: u8 = 0;
const BOOTING: u8 = 1;
const READY: u8 = 2;
const FAILED: u8 = 3;

pub struct HyperShared {
    /// The booted device; the audio thread only ever tries the lock.
    pub synth: Mutex<Option<Synth>>,
    status: AtomicU8,
    /// Counts the devices booted, so the engine sees a new one.
    generation: AtomicU32,
    error: Mutex<String>,
    /// What the device was booted from: the firmware (by identity) and the rate.
    booted: Mutex<Option<(usize, u32)>>,
}

impl Drop for HyperShared {
    fn drop(&mut self) {
        // A device stops its DSP threads as it goes, which may wait: never on the audio thread.
        if let Some(s) = self.synth.get_mut().ok().and_then(|s| s.take()) {
            std::thread::spawn(move || drop(s));
        }
    }
}

/// One instrument's device, shared by its copies in the project (undo, the
/// engine's); a duplicated track gets a new one (`fresh`).
#[derive(Clone)]
pub struct HyperRef(pub Arc<HyperShared>);

impl Default for HyperRef {
    fn default() -> Self {
        HyperRef(Arc::new(HyperShared { synth: Mutex::new(None), status: AtomicU8::new(OFF), generation: AtomicU32::new(0), error: Mutex::new(String::new()), booted: Mutex::new(None) }))
    }
}

impl PartialEq for HyperRef {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl fmt::Debug for HyperRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hyper({:?})", self.status())
    }
}

impl HyperRef {
    pub fn fresh() -> Self {
        Self::default()
    }

    pub fn status(&self) -> Status {
        match self.0.status.load(Ordering::Acquire) {
            BOOTING => Status::Booting,
            READY => Status::Ready,
            FAILED => Status::Failed(self.0.error.lock().map(|e| e.clone()).unwrap_or_default()),
            _ => Status::Off,
        }
    }

    pub fn generation(&self) -> u32 {
        self.0.generation.load(Ordering::Acquire)
    }

    /// Boots a device from `fw` at `rate` on a thread of its own, unless one
    /// is (being) booted from it already.
    pub fn boot(&self, fw: &FirmwareRef, rate: f32) {
        let key = (Arc::as_ptr(&fw.0) as usize, rate.round() as u32);
        {
            let Ok(mut b) = self.0.booted.lock() else { return };
            if *b == Some(key) {
                return;
            }
            *b = Some(key);
        }
        self.0.status.store(BOOTING, Ordering::Release);
        let shared = self.0.clone();
        let fw = fw.clone();
        std::thread::Builder::new()
            .name("Hyper boot".into())
            .spawn(move || {
                let result = match fw.rom.as_ref() {
                    Some(rom) => Synth::new(rom, rate),
                    None => Err(format!("{}: the firmware is missing", fw.name)),
                };
                match result {
                    Ok(s) => {
                        let old = shared.synth.lock().ok().and_then(|mut g| g.replace(s));
                        drop(old);
                        shared.generation.fetch_add(1, Ordering::AcqRel);
                        shared.status.store(READY, Ordering::Release);
                    }
                    Err(e) => {
                        if let Ok(mut err) = shared.error.lock() {
                            *err = e;
                        }
                        shared.status.store(FAILED, Ordering::Release);
                    }
                }
            })
            .ok();
    }

    /// No firmware any more: the device goes (on a thread of its own).
    pub fn stop(&self) {
        if let Ok(mut b) = self.0.booted.lock() {
            *b = None;
        }
        let old = self.0.synth.lock().ok().and_then(|mut g| g.take());
        self.0.status.store(OFF, Ordering::Release);
        if let Some(s) = old {
            std::thread::spawn(move || drop(s));
        }
    }

    /// Its state (the Virus's own sysex), when booted.
    pub fn state(&self) -> Option<Vec<u8>> {
        self.0.synth.lock().ok().and_then(|mut g| g.as_mut().map(|s| s.state()))
    }
}

// ---------------------------------------------------------------- audio thread

/// The device played on the audio thread: the instrument's knobs and preset
/// sent to it as they change, notes in, its sound out.
pub struct HyperState {
    shared: Arc<HyperShared>,
    generation: u32,
    /// The knob values sent (-1: not yet), and the preset loaded.
    sent: [i16; CCS.len()],
    loaded: Option<(u16, u16)>,
    pub pitch_bend: f32,
    bend_sent: i32,
    l: Vec<f32>,
    r: Vec<f32>,
    level: f32,
}

impl HyperState {
    pub fn new(engine: &HyperRef) -> Self {
        Self { shared: engine.0.clone(), generation: u32::MAX, sent: [-1; CCS.len()], loaded: None, pitch_bend: 0.0, bend_sent: 8192, l: vec![0.0; 2048], r: vec![0.0; 2048], level: 0.0 }
    }

    pub fn is(&self, engine: &HyperRef) -> bool {
        Arc::ptr_eq(&self.shared, &engine.0)
    }

    /// The preset and the knobs as the instrument has them, sent where they changed.
    fn sync(&mut self, s: &mut Synth, p: &[f32], preset: (u16, u16)) {
        let generation = self.shared.generation.load(Ordering::Acquire);
        if generation != self.generation {
            // A device just booted: it knows nothing yet.
            self.generation = generation;
            self.loaded = None;
            self.sent = [-1; CCS.len()];
            self.bend_sent = 8192;
        }
        if self.loaded != Some(preset) {
            s.load_preset(preset.0 as u32, preset.1 as u32);
            self.loaded = Some(preset);
            // The knobs went to the preset's values with it (the editor set them); what differs is sent below.
            self.sent = [-1; CCS.len()];
        }
        for (k, &cc) in CCS.iter().enumerate() {
            let v = p.get(k).map(|v| v.round().clamp(0.0, 127.0) as i16).unwrap_or(0);
            if v != self.sent[k] {
                s.midi(0xB0, cc, v as u8, 0);
                self.sent[k] = v;
            }
        }
        // The bend, its range taken as the Virus's default two semitones.
        let bend = (8192.0 + self.pitch_bend / 2.0 * 8191.0).round().clamp(0.0, 16383.0) as i32;
        if bend != self.bend_sent {
            s.midi(0xE0, (bend & 0x7F) as u8, (bend >> 7) as u8, 0);
            self.bend_sent = bend;
        }
    }

    pub fn note_on(&mut self, p: &[f32], preset: (u16, u16), note: u8, vel: f32) {
        let shared = self.shared.clone();
        let Ok(mut g) = shared.synth.try_lock() else { return };
        if let Some(s) = g.as_mut() {
            self.sync(s, p, preset);
            s.midi(0x90, note & 0x7F, (vel.clamp(0.0, 1.0) * 127.0).round().max(1.0) as u8, 0);
        }
    }

    pub fn note_off(&mut self, note: u8) {
        let shared = self.shared.clone();
        let Ok(mut g) = shared.synth.try_lock() else { return };
        if let Some(s) = g.as_mut() {
            s.midi(0x80, note & 0x7F, 64, 0);
        }
    }

    pub fn all_off(&mut self) {
        let shared = self.shared.clone();
        let Ok(mut g) = shared.synth.try_lock() else { return };
        if let Some(s) = g.as_mut() {
            s.midi(0xB0, 123, 0, 0);
        }
    }

    pub fn render(&mut self, p: &[f32], preset: (u16, u16), l: &mut [f32], r: &mut [f32]) {
        let shared = self.shared.clone();
        let Ok(mut g) = shared.synth.try_lock() else { return };
        let Some(s) = g.as_mut() else { return };
        self.sync(s, p, preset);
        let n = l.len().min(r.len());
        let mut done = 0;
        let mut peak = 0.0f32;
        while done < n {
            let m = (n - done).min(self.l.len());
            s.process(&mut self.l[..m], &mut self.r[..m], 120.0);
            for i in 0..m {
                l[done + i] += self.l[i];
                r[done + i] += self.r[i];
                peak = peak.max(self.l[i].abs()).max(self.r[i].abs());
            }
            done += m;
        }
        self.level = self.level * 0.9 + peak * 0.1;
    }

    /// The screen: [0] the device's status (0 off, 1 booting, 2 ready, 3 failed), [1] how loud.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = self.shared.status.load(Ordering::Acquire) as f32;
        out[1] = self.level;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_knob_has_its_controller() {
        assert_eq!(SPECS.len(), CCS.len());
        let mut seen = std::collections::HashSet::new();
        assert!(CCS.iter().all(|c| *c < 128 && seen.insert(*c)), "each controller once");
    }

    #[test]
    fn without_a_device_it_is_silent_and_waits() {
        let engine = HyperRef::fresh();
        assert_eq!(engine.status(), Status::Off);
        let mut st = HyperState::new(&engine);
        let p = crate::params::defaults(SPECS);
        st.note_on(&p, (0, 0), 60, 0.8);
        let (mut l, mut r) = (vec![0.0; 512], vec![0.0; 512]);
        st.render(&p, (0, 0), &mut l, &mut r);
        assert!(l.iter().all(|v| *v == 0.0));
        // A missing firmware boots into a failure, not a crash.
        engine.boot(&FirmwareRef::missing("None".into(), None), 48_000.0);
        for _ in 0..200 {
            if engine.status() != Status::Booting {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(matches!(engine.status(), Status::Failed(_)), "{:?}", engine.status());
    }
}
