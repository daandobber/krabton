//! Loads a CLAP plugin headless, plays a few notes and reports the output.
//!
//! ```sh
//! cargo run -p krabton-clap --example clap_probe -- path/to/Plugin.clap
//! ```

use krabton_clap::ClapHost;
use krabton_engine::plugin::{PluginEvent, PluginEventKind};

fn main() {
    let path = std::env::args().nth(1).expect("usage: clap_probe <file.clap>");
    let mut host = ClapHost::new();
    let (infos, errors) = host.scan(&[path.into()]);
    for e in &errors {
        eprintln!("error: {e}");
    }
    for info in &infos {
        println!("found {} ({}) by {} v{} features={:?}", info.name, info.id, info.vendor, info.version, info.features);
    }
    let Some(info) = infos.iter().find(|i| i.is_instrument()).or(infos.first()) else {
        eprintln!("no plugins");
        std::process::exit(1);
    };

    let sr = 48000.0;
    let inst = host.instantiate(&info.path, &info.id, sr, 256).expect("instantiate");
    println!("instantiated {}; accepts notes: {}; editor: {}", inst.name, inst.accepts_notes(), inst.has_editor());

    // Play a C major chord for one second on a separate "audio" thread.
    let mut proc = inst.processor();
    let handle = std::thread::spawn(move || {
        let mut peak = 0.0f32;
        let mut sum = 0.0f64;
        let mut count = 0usize;
        let block = 256;
        for b in 0..(sr as usize * 2 / block) {
            let mut l = vec![0.0f32; block];
            let mut r = vec![0.0f32; block];
            let events: Vec<PluginEvent> = match b {
                0 => [60u8, 64, 67].iter().map(|&p| PluginEvent { offset: 0, pitch: p, kind: PluginEventKind::NoteOn { velocity: 0.9 } }).collect(),
                187 => [60u8, 64, 67].iter().map(|&p| PluginEvent { offset: 10, pitch: p, kind: PluginEventKind::NoteOff }).collect(),
                _ => Vec::new(),
            };
            proc.process(&events, &mut l, &mut r);
            for (a, b) in l.iter().zip(&r) {
                peak = peak.max(a.abs()).max(b.abs());
                sum += (*a as f64).powi(2);
                count += 1;
            }
        }
        proc.suspend();
        (proc, peak, (sum / count as f64).sqrt())
    });
    let (proc, peak, rms) = handle.join().expect("audio thread");
    drop(proc);
    println!("output: peak {:.3} ({:+.1} dBFS), rms {:+.1} dBFS", peak, 20.0 * (peak.max(1e-9) as f64).log10(), 20.0 * rms.max(1e-12).log10());

    match inst.save_state() {
        Some(state) => {
            println!("state: {} bytes", state.len());
            println!("state reload: {}", if inst.load_state(&state) { "ok" } else { "FAILED" });
        }
        None => println!("state: not supported"),
    }
    if peak < 1e-4 {
        eprintln!("plugin produced silence");
        std::process::exit(2);
    }
}
