//! The grand piano's samples on this computer: where they live, opening
//! them when Krabton starts, and installing them. Installing downloads the
//! Salamander Grand Piano's FLAC files (the SFZ edition on GitHub) with the
//! system's `curl`, then decodes them into Krabton's format and removes the
//! downloads.

use krabton_engine::piano::library::{self, PianoLibrary};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const LISTING: &str = "https://api.github.com/repos/sfzinstruments/SalamanderGrandPiano/contents/Samples";
const LICENSE_URL: &str = "https://raw.githubusercontent.com/sfzinstruments/SalamanderGrandPiano/master/LICENSE";
/// Parallel downloads.
const WORKERS: usize = 6;

#[derive(Clone, Debug, PartialEq)]
pub enum InstallState {
    Listing,
    Downloading { done: usize, total: usize, bytes: u64, total_bytes: u64 },
    Building { done: usize, total: usize },
    Ready,
    Failed(String),
}

pub type Install = Arc<Mutex<InstallState>>;

/// Where the library is installed: Documents/Krabton/Library.
pub fn library_dir() -> PathBuf {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from).unwrap_or_default();
    home.join("Documents").join("Krabton").join("Library").join(library::FOLDER)
}

/// Opens the installed library, if there is one.
pub fn open_installed() -> Option<Arc<PianoLibrary>> {
    let dir = library_dir();
    if !library::installed(&dir) {
        return None;
    }
    match PianoLibrary::open(&dir) {
        Ok(lib) => Some(lib),
        Err(e) => {
            eprintln!("grand piano: {e}");
            None
        }
    }
}

pub(crate) fn curl() -> std::process::Command {
    let mut cmd = std::process::Command::new("curl");
    cmd.stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // No console window popping up.
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

pub(crate) fn fetch(url: &str, to: &Path) -> Result<(), String> {
    let part = to.with_extension("part");
    let status = curl()
        .args(["-sSfL", "--retry", "3", "--retry-delay", "2", "-o"])
        .arg(&part)
        .arg(url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| format!("curl could not run ({e}); it comes with Windows 10 and later, macOS and most Linux systems"))?;
    if !status.success() {
        let _ = std::fs::remove_file(&part);
        return Err(format!("downloading {url} failed"));
    }
    std::fs::rename(&part, to).map_err(|e| e.to_string())
}

/// Starts installing in the background; the returned state tells how far it is.
pub fn start_install() -> Install {
    let state: Install = Arc::new(Mutex::new(InstallState::Listing));
    let s = state.clone();
    std::thread::spawn(move || {
        let result = install(&s);
        if let Ok(mut st) = s.lock() {
            *st = match result {
                Ok(()) => InstallState::Ready,
                Err(e) => InstallState::Failed(e),
            };
        }
    });
    state
}

fn install(state: &Install) -> Result<(), String> {
    let dir = library_dir();
    let downloads = dir.join("download");
    std::fs::create_dir_all(&downloads).map_err(|e| format!("{}: {e}", downloads.display()))?;
    // The list of files, with their sizes.
    let out = curl().args(["-sSfL", "-H", "Accept: application/vnd.github+json", LISTING]).output().map_err(|e| format!("curl could not run: {e}"))?;
    if !out.status.success() {
        return Err("could not get the list of piano samples from GitHub".into());
    }
    let listing: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).map_err(|e| format!("the list of samples: {e}"))?;
    let files: Vec<(String, u64, String)> = listing
        .iter()
        .filter_map(|f| Some((f.get("name")?.as_str()?.to_string(), f.get("size")?.as_u64()?, f.get("download_url")?.as_str()?.to_string())))
        .filter(|f| f.0.ends_with(".flac"))
        .collect();
    if files.len() < 500 {
        return Err(format!("the list has only {} samples", files.len()));
    }
    let total = files.len();
    let total_bytes: u64 = files.iter().map(|f| f.1).sum();
    let progress = Arc::new(Mutex::new((0usize, 0u64)));
    let queue = Arc::new(Mutex::new(files));
    let failed: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let set_state = |done: usize, bytes: u64| {
        if let Ok(mut st) = state.lock() {
            *st = InstallState::Downloading { done, total, bytes, total_bytes };
        }
    };
    set_state(0, 0);
    let workers: Vec<_> = (0..WORKERS)
        .map(|_| {
            let (queue, progress, failed, downloads) = (queue.clone(), progress.clone(), failed.clone(), downloads.clone());
            std::thread::spawn(move || loop {
                if failed.lock().map(|f| f.is_some()).unwrap_or(true) {
                    return;
                }
                let Some((name, size, url)) = queue.lock().ok().and_then(|mut q| q.pop()) else { return };
                let path = downloads.join(&name);
                // Already here from an earlier try.
                let have = std::fs::metadata(&path).map(|m| m.len() == size).unwrap_or(false);
                if !have {
                    if let Err(e) = fetch(&url, &path) {
                        if let Ok(mut f) = failed.lock() {
                            *f = Some(e);
                        }
                        return;
                    }
                }
                if let Ok(mut p) = progress.lock() {
                    p.0 += 1;
                    p.1 += size;
                }
            })
        })
        .collect();
    loop {
        let finished = workers.iter().all(|w| w.is_finished());
        if let Ok(p) = progress.lock() {
            set_state(p.0, p.1);
        }
        if finished {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    if let Some(e) = failed.lock().ok().and_then(|f| f.clone()) {
        return Err(e);
    }
    let _ = fetch(LICENSE_URL, &dir.join("LICENSE.txt"));
    let _ = std::fs::write(
        dir.join("README.txt"),
        format!("{}\nhttps://archive.org/details/SalamanderGrandPianoV3\nSFZ edition by kinwie: https://github.com/sfzinstruments/SalamanderGrandPiano\nConverted to 16-bit for Krabton. License: Creative Commons Attribution 3.0 (LICENSE.txt).\n", library::CREDIT),
    );
    library::build(&downloads, &dir, &|done, total| {
        if let Ok(mut st) = state.lock() {
            *st = InstallState::Building { done, total };
        }
    })?;
    let _ = std::fs::remove_dir_all(&downloads);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Installs the real library (downloads 750 MB): `cargo test -p krabton
    /// install_the_real_piano -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn install_the_real_piano() {
        let state = start_install();
        let mut last = String::new();
        loop {
            let s = state.lock().unwrap().clone();
            let now = format!("{s:?}");
            if now != last && !matches!(s, InstallState::Downloading { done, .. } if done % 40 != 0) {
                eprintln!("{now}");
                last = now;
            }
            match s {
                InstallState::Ready => break,
                InstallState::Failed(e) => panic!("{e}"),
                _ => std::thread::sleep(std::time::Duration::from_millis(500)),
            }
        }
        let lib = open_installed().expect("opens");
        assert!(lib.notes.iter().flatten().all(|&s| lib.exists(s)));
    }
}
