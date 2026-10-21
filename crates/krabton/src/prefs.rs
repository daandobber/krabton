//! Settings that outlive a project, kept as JSON in the user's config folder
//! (`%APPDATA%\Krabton`, `~/Library/Application Support/Krabton` or
//! `~/.config/krabton`). `KRABTON_PREFS` points elsewhere; test windows keep
//! theirs in memory, so they never touch the real ones.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Sizes the user dragged. 0 means "never set": the built-in size.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Layout {
    pub browser_w: f32,
    pub detail_h: f32,
    /// Width of the session's track columns.
    pub track_w: f32,
    pub mixer_h: f32,
    pub arr_head_w: f32,
    pub lane_h: f32,
    /// Width of the clip view's left column.
    pub clip_side_w: f32,
    /// Display widths of devices, by device name.
    pub displays: BTreeMap<String, f32>,
    /// The Rack: the height of a row of devices, and tracks as columns.
    pub rack_h: f32,
    pub rack_columns: bool,
    /// The detail panel (clip editor, devices) folded away: in Session and
    /// Arrangement, and (shown) in the Rack, where it starts folded.
    pub detail_hidden: bool,
    pub rack_detail: bool,
}

impl Layout {
    /// `v`, or `default` while it was never set.
    pub fn or(v: f32, default: f32) -> f32 {
        if v > 0.0 {
            v
        } else {
            default
        }
    }

    pub fn display(&self, name: &str, default: f32) -> f32 {
        self.displays.get(name).copied().unwrap_or(default)
    }
}

/// A browser item in one or more collections (Favorites and the colors).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Tagged {
    /// The item's browser key (see `ui::browser`).
    pub key: String,
    pub label: String,
    /// One bit per collection.
    pub collections: u8,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    /// How far the monome grid is turned (0, 90, 180, 270 degrees).
    pub grid_rotation: u32,
    /// Folders added to the browser's Places.
    pub places: Vec<PathBuf>,
    pub tagged: Vec<Tagged>,
    /// Extra folders with CLAP plugins.
    pub plugin_folders: Vec<PathBuf>,
    /// DX7 banks loaded into the FM browser.
    pub fm_banks: Vec<PathBuf>,
    /// SoundFont banks opened, for the SF2 Player's folder in the browser.
    pub sf2_banks: Vec<PathBuf>,
    /// UTAU voicebank folders opened, for the Singer's folder in the browser.
    pub singer_banks: Vec<PathBuf>,
    /// Virus firmware files opened, for Hyper (they stay where they are).
    pub hyper_firmware: Vec<PathBuf>,
    /// The General MIDI bank MIDI files play with (none: Krabton's instruments).
    pub gm_bank: Option<PathBuf>,
    /// Hear notes while placing and moving them in the clip editor.
    pub note_preview: bool,
    /// Size of everything (1 = normal; 0 = never set).
    pub ui_zoom: f32,
    pub layout: Layout,
    /// The audio output device by name (None: the system default).
    pub audio_output: Option<String>,
    /// The audio input device by name (None: the system default).
    pub audio_input: Option<String>,
    /// Start in Big Picture (for controllers and touch screens).
    pub big_picture: bool,
    /// Devices in the flat look instead of each machine's own.
    pub plain_devices: bool,
}

impl Prefs {
    fn path() -> Option<PathBuf> {
        if let Some(p) = std::env::var_os("KRABTON_PREFS") {
            return Some(PathBuf::from(p));
        }
        if cfg!(test) || std::env::var_os("KRABTON_TEST_WINDOW").is_some() {
            return None;
        }
        let env = |k: &str| std::env::var_os(k).map(PathBuf::from);
        let dir = if cfg!(windows) {
            env("APPDATA").map(|d| d.join("Krabton"))
        } else if cfg!(target_os = "macos") {
            env("HOME").map(|h| h.join("Library/Application Support/Krabton"))
        } else {
            env("XDG_CONFIG_HOME").or_else(|| env("HOME").map(|h| h.join(".config"))).map(|d| d.join("krabton"))
        };
        dir.map(|d| d.join("prefs.json"))
    }

    /// Where crashes are written down: beside the settings.
    pub fn crash_log() -> Option<PathBuf> {
        Self::path().map(|p| p.with_file_name("crash.log"))
    }

    pub fn load() -> Self {
        Self::path().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = Self::path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let tmp = path.with_extension("json.tmp");
            if std::fs::write(&tmp, json).is_ok() {
                let _ = std::fs::rename(&tmp, &path);
            }
        }
    }

    /// The collections (bits) an item is in.
    pub fn collections(&self, key: &str) -> u8 {
        self.tagged.iter().find(|t| t.key == key).map(|t| t.collections).unwrap_or(0)
    }

    /// Puts an item in or takes it out of collection `c`.
    pub fn toggle_collection(&mut self, key: &str, label: &str, c: usize) {
        let bit = 1u8 << c;
        match self.tagged.iter().position(|t| t.key == key) {
            Some(i) => {
                self.tagged[i].collections ^= bit;
                if self.tagged[i].collections == 0 {
                    self.tagged.remove(i);
                }
            }
            None => self.tagged.push(Tagged { key: key.into(), label: label.into(), collections: bit }),
        }
        self.save();
    }

    pub fn clear_collections(&mut self, key: &str) {
        self.tagged.retain(|t| t.key != key);
        self.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collections_toggle_and_survive_a_round_trip() {
        let mut p = Prefs::default();
        p.toggle_collection("inst/Analog/Moog Bass", "Moog Bass", 0);
        p.toggle_collection("inst/Analog/Moog Bass", "Moog Bass", 3);
        assert_eq!(p.collections("inst/Analog/Moog Bass"), 0b1001);
        p.toggle_collection("inst/Analog/Moog Bass", "Moog Bass", 0);
        assert_eq!(p.collections("inst/Analog/Moog Bass"), 0b1000);
        p.grid_rotation = 180;
        let back: Prefs = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(back.grid_rotation, 180);
        assert_eq!(back.tagged, p.tagged);
        // Unknown and missing fields are fine.
        let old: Prefs = serde_json::from_str(r#"{"grid_rotation": 90, "something": 1}"#).unwrap();
        assert_eq!(old.grid_rotation, 90);
        p.toggle_collection("inst/Analog/Moog Bass", "Moog Bass", 3);
        assert!(p.tagged.is_empty());
    }
}
