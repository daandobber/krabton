//! MIDI out: what the engine sends to the tracks' MIDI To ports (their notes,
//! and the clock, Start and Stop at each track's tempo) goes out on a thread
//! of its own, each message at its time, to every port in use at once. The
//! ports open when a track first plays one and close when none does.

use crossbeam_channel::{bounded, Receiver, RecvTimeoutError, Sender};
use krabton_engine::MidiOut;
use midir::{MidiOutput, MidiOutputConnection};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Each track's port and channel.
pub type Routes = HashMap<u64, (String, u8)>;

pub struct MidiOutputs {
    /// The MIDI out ports there are.
    pub ports: Vec<String>,
    routes: Arc<Mutex<Routes>>,
    sent: Routes,
    swap: Sender<(Receiver<MidiOut>, Instant)>,
    /// A port that would not open, to show once.
    pub error: Arc<Mutex<Option<String>>>,
}

impl MidiOutputs {
    /// Starts the thread on the engine's MIDI out.
    pub fn start(rx: Receiver<MidiOut>, epoch: Instant) -> Self {
        let routes = Arc::new(Mutex::new(Routes::new()));
        let error = Arc::new(Mutex::new(None));
        let (swap, swap_rx) = bounded(4);
        let (r, e) = (routes.clone(), error.clone());
        let _ = std::thread::Builder::new().name("krabton-midi-out".into()).spawn(move || run(rx, epoch, swap_rx, r, e));
        let mut m = Self { ports: Vec::new(), routes, sent: Routes::new(), swap, error };
        m.refresh();
        m
    }

    pub fn refresh(&mut self) {
        self.ports = MidiOutput::new("krabton-scan-out").map(|out| out.ports().iter().filter_map(|p| out.port_name(p).ok()).collect()).unwrap_or_default();
    }

    /// A new engine: its MIDI out from now on.
    pub fn attach(&self, rx: Receiver<MidiOut>, epoch: Instant) {
        let _ = self.swap.try_send((rx, epoch));
    }

    /// The tracks' ports, when they changed.
    pub fn set_routes(&mut self, routes: Routes) {
        if routes != self.sent {
            if let Ok(mut r) = self.routes.lock() {
                *r = routes.clone();
            }
            self.sent = routes;
        }
    }
}

/// A message waiting for its time: (when, in what order it came, track, bytes, length).
type Due = Reverse<(u64, u64, u64, [u8; 3], u8)>;

fn run(mut rx: Receiver<MidiOut>, mut epoch: Instant, swap: Receiver<(Receiver<MidiOut>, Instant)>, routes: Arc<Mutex<Routes>>, error: Arc<Mutex<Option<String>>>) {
    let mut due: BinaryHeap<Due> = BinaryHeap::new();
    let mut seq = 0u64;
    let mut ports = Ports { open: HashMap::new(), failed: HashMap::new(), error };
    let mut last = Routes::new();
    loop {
        if let Ok((r, e)) = swap.try_recv() {
            (rx, epoch) = (r, e);
            due.clear();
        }
        // A track moved to another port or channel (or off): its old one hears all notes off.
        let now_routes = routes.lock().map(|r| r.clone()).unwrap_or_default();
        if now_routes != last {
            for (track, (port, ch)) in &last {
                if now_routes.get(track) != Some(&(port.clone(), *ch)) {
                    ports.send(port, &[0xB0 | ch, 123, 0]);
                }
            }
            ports.open.retain(|name, _| now_routes.values().any(|(p, _)| p == name));
            last = now_routes;
        }
        // What is due goes out.
        let now = epoch.elapsed().as_nanos() as u64;
        while let Some(Reverse((at, _, track, msg, len))) = due.peek().copied() {
            if at > now + 300_000 {
                break;
            }
            due.pop();
            if let Some((port, _)) = last.get(&track) {
                ports.send(port, &msg[..len as usize]);
            }
        }
        let wait = due.peek().map(|Reverse(d)| d.0.saturating_sub(now)).unwrap_or(5_000_000).min(5_000_000);
        match rx.recv_timeout(Duration::from_nanos(wait.max(200_000))) {
            Ok(m) => {
                for m in std::iter::once(m).chain(rx.try_iter()) {
                    if last.contains_key(&m.track) {
                        seq += 1;
                        due.push(Reverse((m.at, seq, m.track, m.msg, m.len)));
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => std::thread::sleep(Duration::from_millis(5)),
        }
    }
}

struct Ports {
    open: HashMap<String, MidiOutputConnection>,
    /// Ports that would not open, and when: tried again after a while.
    failed: HashMap<String, Instant>,
    error: Arc<Mutex<Option<String>>>,
}

impl Ports {
    fn send(&mut self, name: &str, bytes: &[u8]) {
        if !self.open.contains_key(name) {
            if self.failed.get(name).is_some_and(|t| t.elapsed() < Duration::from_secs(3)) {
                return;
            }
            match connect(name) {
                Ok(c) => {
                    self.failed.remove(name);
                    self.open.insert(name.to_string(), c);
                }
                Err(e) => {
                    if !self.failed.contains_key(name) {
                        if let Ok(mut err) = self.error.lock() {
                            *err = Some(format!("MIDI out '{name}': {e}"));
                        }
                    }
                    self.failed.insert(name.to_string(), Instant::now());
                    return;
                }
            }
        }
        if let Some(c) = self.open.get_mut(name) {
            if c.send(bytes).is_err() {
                // Unplugged: opened again next time.
                self.open.remove(name);
            }
        }
    }
}

fn connect(name: &str) -> Result<MidiOutputConnection, String> {
    let out = MidiOutput::new("krabton").map_err(|e| e.to_string())?;
    let port = out.ports().into_iter().find(|p| out.port_name(p).is_ok_and(|n| n == name)).ok_or_else(|| "not found".to_string())?;
    out.connect(&port, "krabton-out").map_err(|e| e.to_string())
}
