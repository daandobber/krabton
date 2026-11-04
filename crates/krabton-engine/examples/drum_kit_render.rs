//! Renders every pad of a drum kit, one hit each at full velocity, to WAV files.
//!
//! ```sh
//! cargo run --release -p krabton-engine --example drum_kit_render -- out_dir "909 Classic"
//! ```

use krabton_engine::dsp::drums::{kit, DrumState};

fn main() {
    let out_dir = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "drum_kit".into()));
    let name = std::env::args().nth(2).unwrap_or_else(|| "909 Classic".into());
    std::fs::create_dir_all(&out_dir).expect("create output dir");
    let sr = 44100u32;
    let pads = kit(&name);
    for pad in &pads {
        let mut s = DrumState::new();
        s.note_on(&pads, pad.note, 1.0, sr as f32);
        let n = sr as usize * 2;
        let (mut l, mut r) = (vec![0.0f32; n], vec![0.0f32; n]);
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            s.render(&pads, a, b, sr as f32);
        }
        let interleaved: Vec<f32> = l.iter().zip(&r).flat_map(|(a, b)| [*a, *b]).collect();
        let file = format!("{}.wav", pad.name.replace(' ', "_"));
        krabton_engine::render::write_wav(&out_dir.join(&file), sr, &interleaved).expect("write wav");
    }
}
