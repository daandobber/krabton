//! Renders the demo set (all scenes laid out as a song) to a WAV file,
//! without a GUI or audio device.
//!
//! ```sh
//! cargo run --release -p krabton-engine --example render_demo -- demo.wav
//! ```

use krabton_engine::render::{self, RenderSource};

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "krabton-demo.wav".into());
    let mut project = krabton_engine::demo::demo_project();
    project.scenes_to_arrangement();
    let end = project.arrangement_end();
    let sr = 44100;
    let started = std::time::Instant::now();
    let audio = render::render(&project, RenderSource::Arrangement { start: 0.0, end }, sr, 4.0, None).expect("render");
    let secs = audio.len() as f64 / 2.0 / sr as f64;
    let peak = audio.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    let rms = (audio.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / audio.len().max(1) as f64).sqrt();
    let clipped = audio.iter().filter(|v| v.abs() >= 0.999).count();
    render::write_wav(std::path::Path::new(&path), sr, &audio).expect("write wav");
    println!(
        "wrote {path}: {secs:.1} s of audio in {:.2} s, peak {:.2} dBFS, rms {:.1} dBFS, {clipped} clipped samples",
        started.elapsed().as_secs_f64(),
        20.0 * (peak as f64).log10(),
        20.0 * rms.log10()
    );

    // Per-track peaks (each track soloed), handy for gain staging.
    for i in 0..project.tracks.len() {
        let mut solo = project.clone();
        solo.tracks[i].mixer.solo = true;
        let a = render::render(&solo, RenderSource::Arrangement { start: 0.0, end }, sr, 4.0, None).expect("render");
        let pk = a.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        println!("  {:<8} peak {:+.1} dBFS", solo.tracks[i].name, 20.0 * (pk as f64).log10());
    }
}
