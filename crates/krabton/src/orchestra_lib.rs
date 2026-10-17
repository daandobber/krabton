//! The orchestra's instruments on this computer (see `orchestra`): opening
//! them when Krabton starts, and installing one: its SFZ files and the
//! recordings they name are downloaded with the system's `curl`, a few at a
//! time (files already there are skipped, so a second try picks up where
//! the first stopped), then built into Krabton's format and the downloads
//! removed.

use krabton_engine::orchestra::{self, Library, INSTRUMENTS};
use std::path::PathBuf;
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

/// Documents/Krabton/Library/VS Chamber Orchestra 2/<instrument>.
pub fn dir(instrument: usize) -> PathBuf {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from).unwrap_or_default();
    home.join("Documents").join("Krabton").join("Library").join(orchestra::FOLDER).join(INSTRUMENTS[instrument].name)
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
    // The SFZ files first: they name the recordings.
    let mut files: Vec<String> = Vec::new();
    for a in def.arts {
        let to = downloads.join(a.sfz);
        if !to.is_file() {
            crate::piano_lib::fetch(&orchestra::url(a.sfz), &to)?;
        }
        let text = std::fs::read_to_string(&to).map_err(|e| e.to_string())?;
        for r in orchestra::parse_sfz(&text) {
            if !files.contains(&r.sample) {
                files.push(r.sample);
            }
        }
    }
    let total = files.len();
    let queue = Arc::new(Mutex::new(files));
    let done = Arc::new(AtomicUsize::new(0));
    let failed: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let workers: Vec<_> = (0..WORKERS)
        .map(|_| {
            let (queue, done, failed, downloads) = (queue.clone(), done.clone(), failed.clone(), downloads.clone());
            std::thread::spawn(move || loop {
                let next = queue.lock().ok().and_then(|mut q| q.pop());
                let Some(path) = next else { break };
                let to = path.split('/').fold(downloads.clone(), |d, part| d.join(part));
                if !to.is_file() {
                    if let Some(parent) = to.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if let Err(e) = crate::piano_lib::fetch(&orchestra::url(&path), &to) {
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
    let _ = std::fs::write(out.join("README.txt"), format!("{}\n", orchestra::CREDIT));
    if orchestra::installed(&out) {
        Ok(())
    } else {
        Err("the instrument could not be built; try again".into())
    }
}
