//! Runs a CLAP effect: directly on a test signal, then on the master of
//! the demo project in the full Krabton engine (no GUI, no audio device).
//!
//! ```sh
//! cargo run -p krabton-clap --example fx_probe -- path/to/Effect.clap
//! ```

use krabton_clap::ClapHost;
use krabton_engine::{Command, Engine, PluginInstrument};
use std::sync::Arc;

fn peak(buf: &[f32]) -> f32 {
    buf.iter().fold(0.0f32, |m, v| m.max(v.abs()))
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: fx_probe <file.clap>");
    let mut host = ClapHost::new();
    let (infos, errors) = host.scan(&[path.into()]);
    for e in errors {
        eprintln!("{e}");
    }
    let info = infos.into_iter().find(|i| i.is_effect()).expect("no effect plugin");
    println!("{} by {} ({})", info.name, info.vendor, info.id);
    let sr = 48000.0;

    // A test signal: a loud sine on the left, a quiet one on the right.
    let inst = host.instantiate(&info.path, &info.id, sr as f64, krabton_engine::engine::MAX_BLOCK as u32).expect("instantiate");
    let mut proc = inst.processor();
    let n = 4800;
    let sine = |k: usize, a: f32| a * (k as f32 * 440.0 * std::f32::consts::TAU / sr).sin();
    let mut l: Vec<f32> = (0..n).map(|k| sine(k, 0.8)).collect();
    let mut r: Vec<f32> = (0..n).map(|k| sine(k, 0.2)).collect();
    proc.process_effect(&mut l, &mut r);
    let (pl, pr) = (peak(&l[n / 2..]), peak(&r[n / 2..]));
    println!("test signal in: left 0.800, right 0.200 -> out: left {pl:.3}, right {pr:.3}");
    proc.suspend();
    drop(proc);

    // On the master of the demo project.
    let mut p = krabton_engine::demo::demo_project();
    let dev = p.make_plugin_device(PluginInstrument { instance: 0, format: "clap".into(), plugin_id: info.id.clone(), name: info.name.clone(), vendor: info.vendor.clone(), path: info.path.clone(), state: None });
    let instance = dev.plugin.as_ref().map(|x| x.instance).unwrap_or(0);
    p.master.devices.push(dev);
    let (mut engine, handle) = Engine::new(sr, Arc::new(p));
    handle.send(Command::AttachProcessor { instance, processor: inst.processor() });
    handle.send(Command::LaunchScene(0));
    let worker = std::thread::spawn(move || {
        let mut out = vec![0.0f32; (2.0 * sr) as usize * 2];
        for chunk in out.chunks_mut(512) {
            engine.process(chunk);
        }
        (peak(&out), engine.take_processors(), handle)
    });
    let (master, procs, handle) = worker.join().expect("engine thread");
    drop(procs);
    handle.collect_garbage();
    println!("demo project through it on the master: peak {master:.3}");
    if master <= 1e-4 || !master.is_finite() {
        eprintln!("FAIL: no sound through the engine");
        std::process::exit(2);
    }
    println!("OK");
}
