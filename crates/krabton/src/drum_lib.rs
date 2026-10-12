//! Real Drums' recordings on this computer: downloading them (CC0 samples
//! from the Versilian Community Sample Library on GitHub) with the system's
//! `curl`, a few at a time. Files already there are skipped, so a second
//! try picks up where the first stopped.

use krabton_engine::dsp::real_drums;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const WORKERS: usize = 6;

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Downloading { done: usize, total: usize },
    Ready,
    Failed(String),
}

pub type Install = Arc<Mutex<State>>;

/// Starts downloading in the background; the state tells how far it is.
pub fn start() -> Install {
    let state: Install = Arc::new(Mutex::new(State::Downloading { done: 0, total: real_drums::files().len() }));
    let s = state.clone();
    std::thread::spawn(move || {
        let result = install(&s);
        if let Ok(mut st) = s.lock() {
            *st = match result {
                Ok(()) => State::Ready,
                Err(e) => State::Failed(e),
            };
        }
    });
    state
}

fn install(state: &Install) -> Result<(), String> {
    let dir = real_drums::dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let files = real_drums::files();
    let total = files.len();
    let queue = Arc::new(Mutex::new(files));
    let done = Arc::new(AtomicUsize::new(0));
    let failed: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let workers: Vec<_> = (0..WORKERS)
        .map(|_| {
            let (queue, done, failed, dir) = (queue.clone(), done.clone(), failed.clone(), dir.clone());
            std::thread::spawn(move || loop {
                let next = queue.lock().ok().and_then(|mut q| q.pop());
                let Some((url, name)) = next else { break };
                let to = dir.join(name);
                if !to.is_file() {
                    if let Err(e) = crate::piano_lib::fetch(&url, &to) {
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
    let _ = std::fs::write(dir.join("README.txt"), format!("{}\n", real_drums::CREDIT));
    if real_drums::installed() {
        Ok(())
    } else {
        Err("some recordings did not arrive; try again".into())
    }
}
