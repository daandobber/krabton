//! Renders a short phrase with every Krab FM factory voice (or a .syx bank)
//! to WAV files and prints their levels.
//!
//! ```sh
//! cargo run --release -p krabton-engine --example fm_voices -- out_dir [bank.syx]
//! ```

use krabton_engine::dsp::dx7::{self, Dx7Voice};
use krabton_engine::dsp::fm::{FmState, SPECS};
use krabton_engine::params::defaults;

fn main() {
    let mut args = std::env::args().skip(1);
    let out_dir = std::path::PathBuf::from(args.next().unwrap_or_else(|| "fm_voices".into()));
    let voices: Vec<Dx7Voice> = match args.next() {
        Some(path) => dx7::parse_sysex(&std::fs::read(&path).expect("read bank")).expect("parse bank"),
        None => dx7::factory_bank(),
    };
    std::fs::create_dir_all(&out_dir).expect("create output dir");
    let sr = 44100u32;
    let p = defaults(SPECS);
    // A small chord progression: (start beat, length, notes).
    let phrase: [(f32, f32, &[u8]); 4] = [(0.0, 1.5, &[48, 60, 64, 67]), (2.0, 1.5, &[45, 57, 60, 64]), (4.0, 0.4, &[72]), (4.5, 0.4, &[76])];
    let spb = sr as f32 * 0.5; // 120 BPM
    for (i, v) in voices.iter().enumerate() {
        let mut s = FmState::new();
        let total = (spb * 8.0) as usize;
        let mut l = vec![0.0f32; total];
        let mut r = vec![0.0f32; total];
        let mut events: Vec<(usize, bool, u8)> = Vec::new();
        for (start, len, notes) in phrase {
            for &n in notes {
                events.push(((start * spb) as usize, true, n));
                events.push((((start + len) * spb) as usize, false, n));
            }
        }
        events.sort();
        let mut pos = 0;
        for (at, on, note) in events {
            if at > pos {
                s.render(&p, &v.0, &mut l[pos..at], &mut r[pos..at], sr as f32);
                pos = at;
            }
            if on {
                s.note_on(&p, &v.0, 0x3f, note, 0.8);
            } else {
                s.note_off(&p, note);
            }
        }
        s.render(&p, &v.0, &mut l[pos..], &mut r[pos..], sr as f32);
        let peak = l.iter().chain(&r).fold(0.0f32, |m, x| m.max(x.abs()));
        // Brightness proxy: RMS of the first difference relative to the RMS.
        let rms = (l.iter().map(|x| x * x).sum::<f32>() / total as f32).sqrt();
        let drms = (l.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum::<f32>() / total as f32).sqrt();
        let bright = if rms > 0.0 { drms / rms } else { 0.0 };
        let interleaved: Vec<f32> = l.iter().zip(&r).flat_map(|(a, b)| [*a, *b]).collect();
        let name = format!("{:02}_{}.wav", i + 1, v.name().replace(' ', "_"));
        krabton_engine::render::write_wav(&out_dir.join(&name), sr, &interleaved).expect("write wav");
        println!("{:<12} alg {:>2}  peak {:+6.1} dBFS  brightness {:.3}  -> {name}", v.name(), v.algorithm() + 1, 20.0 * peak.max(1e-9).log10(), bright);
    }
}
