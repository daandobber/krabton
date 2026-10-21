//! The Virus emulation of gearmulator, compiled into Krabton: The Usual
//! Suspects' DSP56300 emulator running a Virus's own firmware (a TI's, or an
//! A, B or C operating system), which the player brings: it is never part of
//! Krabton. Licensed GPL-3.0 (see `vendor/`), which makes Krabton's builds
//! GPL-3.0 as a whole.
//!
//! A [`Rom`] is a firmware read (its model and factory presets); a [`Synth`]
//! is a device booted from one (slow: do it off the audio thread) that takes
//! MIDI and gives stereo audio at the host's rate.

use std::ffi::{c_char, c_int, CString};
use std::path::Path;
use std::ptr::NonNull;

#[repr(C)]
struct KvRom {
    _p: [u8; 0],
}

#[repr(C)]
struct KvSynth {
    _p: [u8; 0],
}

extern "C" {
    fn kv_rom_open(path: *const c_char, err: *mut c_char, err_len: usize) -> *mut KvRom;
    fn kv_rom_close(rom: *mut KvRom);
    fn kv_rom_model(rom: *const KvRom) -> c_int;
    fn kv_rom_model_name(rom: *const KvRom, out: *mut c_char, len: usize) -> c_int;
    fn kv_rom_banks(rom: *const KvRom) -> u32;
    fn kv_rom_presets_per_bank(rom: *const KvRom) -> u32;
    fn kv_rom_preset_name(rom: *const KvRom, bank: u32, program: u32, out: *mut c_char, len: usize) -> c_int;
    fn kv_rom_preset_data(rom: *const KvRom, bank: u32, program: u32, out: *mut u8, len: usize) -> c_int;
    fn kv_create(rom: *const KvRom, samplerate: f32, err: *mut c_char, err_len: usize) -> *mut KvSynth;
    fn kv_destroy(s: *mut KvSynth);
    fn kv_midi(s: *mut KvSynth, a: u8, b: u8, c: u8, offset: u32);
    fn kv_sysex(s: *mut KvSynth, data: *const u8, len: usize);
    fn kv_load_preset(s: *mut KvSynth, bank: u32, program: u32) -> c_int;
    fn kv_process(s: *mut KvSynth, l: *mut f32, r: *mut f32, frames: u32, bpm: f32);
    fn kv_latency(s: *const KvSynth) -> u32;
    fn kv_get_state(s: *mut KvSynth, out: *mut u8, cap: usize) -> usize;
    fn kv_set_state(s: *mut KvSynth, data: *const u8, len: usize) -> c_int;
}

fn text(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

/// Whether a file looks like a Virus TI's firmware: Access's container
/// ("FORM" first), 6 to 9 MB, whatever it is called.
pub fn looks_like_ti_firmware(path: &Path) -> bool {
    use std::io::Read;
    let Ok(meta) = std::fs::metadata(path) else { return false };
    if !(6 * 1024 * 1024..=9 * 1024 * 1024).contains(&meta.len()) {
        return false;
    }
    let mut magic = [0u8; 4];
    std::fs::File::open(path).and_then(|mut f| f.read_exact(&mut magic)).is_ok() && &magic == b"FORM"
}

/// The Virus TI firmware in a folder and its folders (a few levels down):
/// where Access's installer was unpacked or installed, say.
pub fn find_ti_firmware(dir: &Path) -> Vec<std::path::PathBuf> {
    fn walk(dir: &Path, depth: u32, seen: &mut u32, out: &mut Vec<std::path::PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        let mut entries: Vec<std::path::PathBuf> = rd.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            *seen += 1;
            if *seen > 50_000 {
                return;
            }
            if p.is_dir() {
                if depth < 8 {
                    walk(&p, depth + 1, seen, out);
                }
            } else if looks_like_ti_firmware(&p) {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, 0, &mut 0, &mut out);
    out
}

/// A Virus model, as its firmware says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    A,
    B,
    C,
    Snow,
    Ti,
    Ti2,
}

/// A firmware, read: its model and its factory presets.
pub struct Rom(NonNull<KvRom>);

// Read only once made.
unsafe impl Send for Rom {}
unsafe impl Sync for Rom {}

impl Rom {
    /// Reads a firmware file (a Virus TI's, or an A, B or C operating system).
    pub fn open(path: &Path) -> Result<Rom, String> {
        let c = CString::new(path.to_string_lossy().as_bytes()).map_err(|_| "a path with a zero byte in it".to_string())?;
        let mut err = [0u8; 512];
        let p = unsafe { kv_rom_open(c.as_ptr(), err.as_mut_ptr() as *mut c_char, err.len()) };
        NonNull::new(p).map(Rom).ok_or_else(|| text(&err))
    }

    pub fn model(&self) -> Option<Model> {
        Some(match unsafe { kv_rom_model(self.0.as_ptr()) } {
            0 => Model::A,
            1 => Model::B,
            2 => Model::C,
            3 => Model::Snow,
            4 => Model::Ti,
            5 => Model::Ti2,
            _ => return None,
        })
    }

    pub fn model_name(&self) -> String {
        let mut buf = [0u8; 64];
        unsafe { kv_rom_model_name(self.0.as_ptr(), buf.as_mut_ptr() as *mut c_char, buf.len()) };
        text(&buf)
    }

    /// How many banks of factory presets, and how many in each.
    pub fn banks(&self) -> u32 {
        unsafe { kv_rom_banks(self.0.as_ptr()) }
    }

    pub fn presets_per_bank(&self) -> u32 {
        unsafe { kv_rom_presets_per_bank(self.0.as_ptr()) }
    }

    /// A factory preset's name, if there is one.
    pub fn preset_name(&self, bank: u32, program: u32) -> Option<String> {
        let mut buf = [0u8; 64];
        let n = unsafe { kv_rom_preset_name(self.0.as_ptr(), bank, program, buf.as_mut_ptr() as *mut c_char, buf.len()) };
        (n >= 0).then(|| text(&buf).trim_end().to_string())
    }

    /// A factory preset's data (its first 128 bytes are the parameters the controllers reach).
    pub fn preset_data(&self, bank: u32, program: u32) -> Option<Vec<u8>> {
        let mut buf = vec![0u8; 512];
        let n = unsafe { kv_rom_preset_data(self.0.as_ptr(), bank, program, buf.as_mut_ptr(), buf.len()) };
        (n >= 0).then(|| {
            buf.truncate(n as usize);
            buf
        })
    }
}

impl Drop for Rom {
    fn drop(&mut self) {
        unsafe { kv_rom_close(self.0.as_ptr()) }
    }
}

/// A Virus booted from its firmware, sounding at the host's rate.
pub struct Synth(NonNull<KvSynth>);

// Used by one thread at a time (the engine's), made on another.
unsafe impl Send for Synth {}

impl Synth {
    /// Boots a device from `rom` (this takes a while: not on the audio thread).
    pub fn new(rom: &Rom, samplerate: f32) -> Result<Synth, String> {
        let mut err = [0u8; 512];
        let p = unsafe { kv_create(rom.0.as_ptr(), samplerate, err.as_mut_ptr() as *mut c_char, err.len()) };
        NonNull::new(p).map(Synth).ok_or_else(|| text(&err))
    }

    /// A MIDI message, `offset` frames into the next block.
    pub fn midi(&mut self, status: u8, d1: u8, d2: u8, offset: u32) {
        unsafe { kv_midi(self.0.as_ptr(), status, d1, d2, offset) }
    }

    pub fn sysex(&mut self, data: &[u8]) {
        unsafe { kv_sysex(self.0.as_ptr(), data.as_ptr(), data.len()) }
    }

    /// A factory preset into the part that plays.
    pub fn load_preset(&mut self, bank: u32, program: u32) -> bool {
        unsafe { kv_load_preset(self.0.as_ptr(), bank, program) != 0 }
    }

    /// The next block: stereo, as long as `l` (and `r`).
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32], bpm: f32) {
        let n = l.len().min(r.len());
        unsafe { kv_process(self.0.as_ptr(), l.as_mut_ptr(), r.as_mut_ptr(), n as u32, bpm) }
    }

    /// Frames from a note to its sound.
    pub fn latency(&self) -> u32 {
        unsafe { kv_latency(self.0.as_ptr()) }
    }

    /// Its patches and settings, as the Virus's own sysex.
    pub fn state(&mut self) -> Vec<u8> {
        let n = unsafe { kv_get_state(self.0.as_ptr(), std::ptr::null_mut(), 0) };
        let mut out = vec![0u8; n];
        let m = unsafe { kv_get_state(self.0.as_ptr(), out.as_mut_ptr(), out.len()) };
        out.truncate(m.min(n));
        out
    }

    pub fn set_state(&mut self, state: &[u8]) -> bool {
        unsafe { kv_set_state(self.0.as_ptr(), state.as_ptr(), state.len()) != 0 }
    }
}

impl Drop for Synth {
    fn drop(&mut self) {
        unsafe { kv_destroy(self.0.as_ptr()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_that_is_no_firmware_is_refused() {
        let dir = std::env::temp_dir().join(format!("krabton-virus-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("nothing.bin");
        std::fs::write(&f, vec![0u8; 4096]).unwrap();
        let err = Rom::open(&f).err().expect("refused");
        assert!(!err.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_broken_firmware_container_is_refused_not_crashed_on() {
        let dir = std::env::temp_dir().join(format!("krabton-virus-form-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // "FORM" and nothing sensible after it, of a TI firmware's size, under any name.
        let mut zeros = b"FORM".to_vec();
        zeros.resize(7 * 1024 * 1024, 0);
        let f = dir.join("firmware_bin");
        std::fs::write(&f, &zeros).unwrap();
        assert!(looks_like_ti_firmware(&f));
        assert_eq!(find_ti_firmware(&dir), vec![f.clone()]);
        assert!(Rom::open(&f).is_err());
        // A file table, then a chunk that claims more than there is.
        let mut lying = b"FORM".to_vec();
        lying.extend_from_slice(&[0, 0, 0, 0]);
        lying.extend_from_slice(b"TABL");
        lying.extend_from_slice(&[0, 0, 0, 2, 1, 97]);
        lying.extend_from_slice(b"BIG ");
        lying.extend_from_slice(&[0x7f, 0xff, 0xff, 0xff]);
        lying.resize(7 * 1024 * 1024, 7);
        std::fs::write(&f, &lying).unwrap();
        assert!(Rom::open(&f).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// With a firmware (KRABTON_VIRUS_ROM=path): it boots, plays a note of
    /// its first preset and sounds.
    #[test]
    fn with_a_firmware_it_sounds() {
        let Some(path) = std::env::var_os("KRABTON_VIRUS_ROM") else { return };
        let rom = Rom::open(Path::new(&path)).expect("a firmware");
        eprintln!("{} with {} banks of {}: {:?}", rom.model_name(), rom.banks(), rom.presets_per_bank(), rom.preset_name(0, 0));
        let mut s = Synth::new(&rom, 48_000.0).expect("boots");
        s.load_preset(0, 0);
        let (mut l, mut r) = (vec![0.0f32; 256], vec![0.0f32; 256]);
        for _ in 0..200 {
            s.process(&mut l, &mut r, 120.0);
        }
        s.midi(0x90, 60, 100, 0);
        let mut peak = 0.0f32;
        for _ in 0..400 {
            s.process(&mut l, &mut r, 120.0);
            peak = l.iter().fold(peak, |m, x| m.max(x.abs()));
        }
        assert!(peak > 0.001, "it sounds: {peak}");
    }
}
