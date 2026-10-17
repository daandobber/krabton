//! The orchestra's and the soloists' instruments on this computer (see
//! `orchestra`): opening them when Krabton starts, and installing one: its
//! SFZ files (with those they include) and the recordings they name are
//! downloaded with the system's `curl`, a few at a time (files already there
//! are skipped, so a second try picks up where the first stopped), then
//! built into Krabton's format and the downloads removed.

use krabton_engine::orchestra::{self, Library, INSTRUMENTS};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const WORKERS: usize = 6;

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Downloading { done: usize, total: usize },
    Building { done: usize, total: usize },
    Ready,
    Failed(String),
}

pub type Install = Arc<Mutex<State>>;

/// Documents/Krabton/Library/<its library>/<instrument>.
pub fn dir(instrument: usize) -> PathBuf {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from).unwrap_or_default();
    let def = &INSTRUMENTS[instrument];
    home.join("Documents").join("Krabton").join("Library").join(def.src.folder).join(def.name)
}

/// A library path under a folder.
fn local(dir: &Path, path: &str) -> PathBuf {
    path.split('/').fold(dir.to_path_buf(), |d, part| d.join(part))
}

pub fn installed(instrument: usize) -> bool {
    orchestra::installed(&dir(instrument))
}

fn open(instrument: usize) -> Option<Arc<Library>> {
    match Library::open(&dir(instrument)) {
        Ok(lib) => Some(lib),
        Err(e) => {
            eprintln!("orchestra: {e}");
            None
        }
    }
}

/// Opens every installed instrument for the engines.
pub fn open_all() {
    for i in 0..INSTRUMENTS.len() {
        if installed(i) {
            orchestra::set_library(i, open(i));
        }
    }
}

/// After an install: the instrument opened for the engines.
pub fn open_one(instrument: usize) -> bool {
    let lib = open(instrument);
    let ok = lib.is_some();
    orchestra::set_library(instrument, lib);
    ok
}

/// Starts installing an instrument in the background; the state tells how far it is.
pub fn start(instrument: usize) -> Install {
    let state: Install = Arc::new(Mutex::new(State::Downloading { done: 0, total: 0 }));
    let s = state.clone();
    std::thread::spawn(move || {
        let result = install(instrument, &s);
        if let Ok(mut st) = s.lock() {
            *st = match result {
                Ok(()) => State::Ready,
                Err(e) => State::Failed(e),
            };
        }
    });
    state
}

fn install(instrument: usize, state: &Install) -> Result<(), String> {
    let def = &INSTRUMENTS[instrument];
    let out = dir(instrument);
    let downloads = out.join("download");
    std::fs::create_dir_all(&downloads).map_err(|e| format!("{}: {e}", downloads.display()))?;
    // The SFZ files first (and those they include): they name the recordings.
    let mut files: Vec<String> = Vec::new();
    for a in def.arts {
        let regions = orchestra::read_sfz(a.sfz, a.cc, &mut |p| {
            let to = local(&downloads, p);
            if !to.is_file() {
                if let Some(parent) = to.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                crate::piano_lib::fetch(&def.src.url(p), &to)?;
            }
            orchestra::read_text(&to)
        })?;
        for r in regions {
            if !files.contains(&r.sample) {
                files.push(r.sample);
            }
        }
    }
    let total = files.len();
    let queue = Arc::new(Mutex::new(files));
    let done = Arc::new(AtomicUsize::new(0));
    let failed: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let src = def.src;
    let workers: Vec<_> = (0..WORKERS)
        .map(|_| {
            let (queue, done, failed, downloads) = (queue.clone(), done.clone(), failed.clone(), downloads.clone());
            std::thread::spawn(move || loop {
                let next = queue.lock().ok().and_then(|mut q| q.pop());
                let Some(path) = next else { break };
                let to = local(&downloads, &path);
                if !to.is_file() {
                    if let Some(parent) = to.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if let Err(e) = crate::piano_lib::fetch(&src.url(&path), &to) {
                        if let Ok(mut f) = failed.lock() {
                            *f = Some(e);
                        }
                        break;
                    }
                }
                done.fetch_add(1, Ordering::Relaxed);
            })
        })
        .collect();
    while workers.iter().any(|w| !w.is_finished()) {
        if let Ok(mut st) = state.lock() {
            *st = State::Downloading { done: done.load(Ordering::Relaxed), total };
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    for w in workers {
        let _ = w.join();
    }
    if let Some(e) = failed.lock().ok().and_then(|f| f.clone()) {
        return Err(e);
    }
    let progress = |d: usize, t: usize| {
        if let Ok(mut st) = state.lock() {
            *st = State::Building { done: d, total: t };
        }
    };
    orchestra::build(def, &downloads, &out, &progress)?;
    let _ = std::fs::remove_dir_all(&downloads);
    let _ = std::fs::write(out.join("README.txt"), format!("{}\n", def.src.credit));
    if orchestra::installed(&out) {
        Ok(())
    } else {
        Err("the instrument could not be built; try again".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Installs the instruments in ORCHESTRA_INSTALL (names, by commas), for real.
    #[test]
    #[ignore]
    fn installs_for_real() {
        let Ok(names) = std::env::var("ORCHESTRA_INSTALL") else { return };
        let started: Vec<(usize, Install)> = names
            .split(',')
            .map(|n| {
                let i = orchestra::index_of(n.trim()).unwrap_or_else(|| panic!("{n}?"));
                (i, start(i))
            })
            .collect();
        let t = std::time::Instant::now();
        loop {
            let states: Vec<(usize, State)> = started.iter().map(|(i, s)| (*i, s.lock().unwrap().clone())).collect();
            if states.iter().all(|(_, s)| matches!(s, State::Ready | State::Failed(_))) {
                for (i, s) in states {
                    eprintln!("{}: {s:?} ({:.0} s)", INSTRUMENTS[i].name, t.elapsed().as_secs_f32());
                    assert_eq!(s, State::Ready);
                    assert!(open_one(i));
                }
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}
