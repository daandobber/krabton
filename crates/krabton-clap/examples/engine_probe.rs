//! Runs a CLAP instrument through the full Krabton engine (no GUI, no audio
//! device): a MIDI track with the plugin, live notes and a launched clip.
//!
//! ```sh
//! cargo run -p krabton-clap --example engine_probe -- path/to/Plugin.clap
//! ```

use krabton_clap::ClapHost;
use krabton_engine::{Clip, Command, Engine, Instrument, Note, PluginInstrument, Project};
use std::sync::Arc;

fn peak(buf: &[f32]) -> f32 {
    buf.iter().fold(0.0f32, |m, v| m.max(v.abs()))
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: engine_probe <file.clap>");
    let mut host = ClapHost::new();
    let (infos, _) = host.scan(&[path.into()]);
    let info = infos.into_iter().find(|i| i.is_instrument()).expect("no instrument plugin");

    let mut p = Project::blank();
    let instance = p.alloc_id();
    let t = p.add_midi_track(Some(Instrument::Plugin(PluginInstrument {
        instance,
        format: "clap".into(),
        plugin_id: info.id.clone(),
        name: info.name.clone(),
        vendor: info.vendor.clone(),
        path: info.path.clone(),
        state: None,
    })));
    let cid = p.alloc_id();
    let mut clip = Clip::new_midi(cid, "test", [255, 255, 255], 4.0);
    if let Some(n) = clip.notes_mut() {
        n.extend([Note::new(57, 0.0, 1.0, 0.9), Note::new(60, 1.0, 1.0, 0.9), Note::new(64, 2.0, 2.0, 0.9)]);
    }
    p.tracks[t].slots[0] = Some(clip);
    let track_id = p.tracks[t].id;

    // Offline render (what Export Audio does), with its own instance.
    let export_inst = host.instantiate(&info.path, &info.id, 44100.0, krabton_engine::engine::MAX_BLOCK as u32).expect("instantiate for export");
    let render_project = p.clone();
    let processors = vec![(instance, export_inst.processor())];
    let render = std::thread::spawn(move || {
        krabton_engine::render::render_with_plugins(&render_project, krabton_engine::render::RenderSource::Scene { scene: 0, beats: 8.0 }, 44100, 1.0, None, processors)
    });
    let (audio, leftovers) = render.join().expect("render thread");
    drop(leftovers);
    drop(export_inst);
    let rendered = peak(&audio.expect("render"));
    println!("offline render peak {rendered:.4}");

    let sr = 48000.0;
    let inst = host.instantiate(&info.path, &info.id, sr as f64, krabton_engine::engine::MAX_BLOCK as u32).expect("instantiate");
    let (mut engine, handle) = Engine::new(sr, Arc::new(p));
    handle.send(Command::AttachProcessor { instance, processor: inst.processor() });

    // Run the engine on its own "audio" thread, like cpal would.
    let worker = std::thread::spawn(move || {
        let mut run = |secs: f32| {
            let mut out = vec![0.0f32; (secs * sr) as usize * 2];
            for chunk in out.chunks_mut(512) {
                engine.process(chunk);
            }
            out
        };
        let silent = peak(&run(0.3));
        handle.send(Command::NoteOn { track: track_id, pitch: 60, velocity: 0.9 });
        let live = peak(&run(1.0));
        handle.send(Command::NoteOff { track: track_id, pitch: 60 });
        run(1.0);
        handle.send(Command::LaunchScene(0));
        let clip = peak(&run(2.0));
        let procs = engine.take_processors();
        (silent, live, clip, procs, handle)
    });
    let (silent, live, clip, procs, handle) = worker.join().expect("engine thread");
    drop(procs);
    handle.collect_garbage();
    println!("{}: before notes {:.4}, live note {:.4}, launched clip {:.4}", info.name, silent, live, clip);
    if live < 1e-3 || clip < 1e-3 || rendered < 1e-3 {
        eprintln!("FAIL: no sound through the engine");
        std::process::exit(2);
    }
    println!("OK");
}
