//! Renders a short phrase with every Krab Macro preset to WAV files and
//! prints their levels.
//!
//! ```sh
//! cargo run --release -p krabton-engine --example macro_presets -- out_dir
//! ```

use krabton_engine::dsp::macro_osc::{self, MacroState, VOICES};

fn main() {
    let out_dir = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "macro_presets".into()));
    std::fs::create_dir_all(&out_dir).expect("create output dir");
    let sr = 48000u32;
    let spb = sr as f32 * 0.5; // 120 BPM
    let chords: [(f32, f32, &[u8]); 4] = [(0.0, 1.5, &[48, 60, 64, 67]), (2.0, 1.5, &[45, 57, 60, 64]), (4.0, 0.4, &[72]), (4.5, 0.4, &[76])];
    let line: [(f32, f32, &[u8]); 6] = [(0.0, 0.4, &[48]), (0.5, 0.6, &[51]), (1.0, 0.4, &[55]), (1.5, 0.9, &[53]), (2.5, 0.4, &[48]), (3.0, 1.0, &[46])];
    for (i, name) in macro_osc::preset_names().iter().enumerate() {
        let p = macro_osc::preset(name);
        let phrase: &[(f32, f32, &[u8])] = if p[VOICES] <= 2.0 { &line } else { &chords };
        let mut s = MacroState::new(sr as f32);
        let total = (spb * 7.0) as usize;
        let mut l = vec![0.0f32; total];
        let mut r = vec![0.0f32; total];
        let mut events: Vec<(usize, bool, u8)> = Vec::new();
        for (start, len, notes) in phrase {
            for &n in *notes {
                events.push(((start * spb) as usize, true, n));
                events.push((((start + len) * spb) as usize, false, n));
            }
        }
        events.sort_by_key(|e| (e.0, e.1));
        let mut pos = 0;
        for (at, on, note) in events {
            if at > pos {
                s.render(&p, &mut l[pos..at], &mut r[pos..at]);
                pos = at;
            }
            if on {
                s.note_on(&p, note, 0.85);
            } else {
                s.note_off(&p, note);
            }
        }
        s.render(&p, &mut l[pos..], &mut r[pos..]);
        let peak = l.iter().chain(&r).fold(0.0f32, |m, x| m.max(x.abs()));
        let rms = (l.iter().chain(&r).map(|x| x * x).sum::<f32>() / (2 * total) as f32).sqrt();
        let interleaved: Vec<f32> = l.iter().zip(&r).flat_map(|(a, b)| [*a, *b]).collect();
        let file = format!("{:02}_{}.wav", i + 1, name.replace(' ', "_"));
        krabton_engine::render::write_wav(&out_dir.join(&file), sr, &interleaved).expect("write wav");
        println!("{:<15} peak {:+6.1} dBFS  rms {:+6.1} dBFS  -> {file}", name, 20.0 * peak.max(1e-9).log10(), 20.0 * rms.max(1e-9).log10());
    }
}
