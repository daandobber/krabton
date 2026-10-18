//! SoundFont banks for the SF2 Player to download: free General MIDI sets
//! under the MIT license, fetched with the system's `curl` into
//! Documents/Krabton/Library/SoundFonts, each with its license beside it.

use krabton_engine::dsp::sf2::Sf2Ref;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// A bank Krabton can fetch.
pub struct Offer {
    pub name: &'static str,
    pub file: &'static str,
    pub url: &'static str,
    /// Its license and notices, saved beside it.
    pub license_url: &'static str,
    /// Its size, to show and to follow the download.
    pub bytes: u64,
    pub about: &'static str,
}

pub const OFFERS: [Offer; 2] = [
    Offer {
        name: "MuseScore General",
        file: "MuseScore_General.sf2",
        url: "https://ftp.osuosl.org/pub/musescore/soundfont/MuseScore_General/MuseScore_General.sf2",
        license_url: "https://ftp.osuosl.org/pub/musescore/soundfont/MuseScore_General/MuseScore_General_License.md",
        bytes: 215_614_036,
        about: "A General MIDI set: Fluid R3 in mono, improved by S. Christian Collins for MuseScore (MIT)",
    },
    Offer {
        name: "Fluid R3 GM",
        file: "FluidR3_GM.sf2",
        url: "https://github.com/pianobooster/fluid-soundfont/releases/download/v3.1/FluidR3_GM.sf2",
        license_url: "https://raw.githubusercontent.com/pianobooster/fluid-soundfont/main/COPYING",
        bytes: 148_398_306,
        about: "The classic General MIDI set by Frank Wen, in stereo (MIT)",
    },
];

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Downloading { bytes: u64 },
    /// Here and read.
    Ready(Sf2Ref),
    Failed(String),
}

pub type Install = Arc<Mutex<State>>;

/// Where the banks go: Documents/Krabton/Library/SoundFonts.
pub fn dir() -> PathBuf {
    crate::piano_lib::library_dir().parent().map(Path::to_path_buf).unwrap_or_default().join("SoundFonts")
}

/// The offer's file, when it is here.
pub fn installed(offer: &Offer) -> Option<PathBuf> {
    let path = dir().join(offer.file);
    path.is_file().then_some(path)
}

/// Every offered bank that is here.
pub fn all_installed() -> Vec<PathBuf> {
    OFFERS.iter().filter_map(installed).collect()
}

/// Starts fetching an offer in the background; the state tells how far it is.
pub fn start(i: usize) -> Install {
    let state: Install = Arc::new(Mutex::new(State::Downloading { bytes: 0 }));
    let s = state.clone();
    std::thread::spawn(move || {
        let result = fetch(&OFFERS[i], &s);
        if let Ok(mut st) = s.lock() {
            *st = match result {
                Ok(bank) => State::Ready(bank),
                Err(e) => State::Failed(e),
            };
        }
    });
    state
}

fn fetch(offer: &Offer, state: &Install) -> Result<Sf2Ref, String> {
    let dir = dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let to = dir.join(offer.file);
    let part = to.with_extension("part");
    let mut child = crate::piano_lib::curl()
        .args(["-sSfL", "--retry", "3", "--retry-delay", "2", "-o"])
        .arg(&part)
        .arg(offer.url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("curl could not run ({e}); it comes with Windows 10 and later, macOS and most Linux systems"))?;
    // How far it is: the size of what has arrived.
    let status = loop {
        if let Some(st) = child.try_wait().map_err(|e| e.to_string())? {
            break st;
        }
        let bytes = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
        if let Ok(mut s) = state.lock() {
            *s = State::Downloading { bytes };
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    };
    if !status.success() {
        let _ = std::fs::remove_file(&part);
        return Err(format!("downloading {} failed", offer.name));
    }
    std::fs::rename(&part, &to).map_err(|e| e.to_string())?;
    // Read here, so it is ready to play when it arrives (and is a bank at all).
    let bank = Sf2Ref::load(&to).map_err(|e| {
        let _ = std::fs::remove_file(&to);
        format!("{} arrived but is not a bank: {e}", offer.name)
    })?;
    let license = dir.join(format!("{}-LICENSE.txt", offer.file.trim_end_matches(".sf2")));
    if crate::piano_lib::fetch(offer.license_url, &license).is_err() {
        let _ = std::fs::write(&license, format!("{}\n{}\nLicense: MIT. See {}\n", offer.name, offer.about, offer.license_url));
    }
    Ok(bank)
}
