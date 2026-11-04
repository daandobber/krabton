//! Renders the galloping octave line with every Munich preset to WAV files
//! and prints their levels, and the old Moog Bass preset of Analog beside them.
//!
//! ```sh
//! cargo run --release -p krabton-engine --example munich_presets -- out_dir
//! ```

use krabton_engine::dsp::analog::{self, AnalogState};
use krabton_engine::dsp::munich::{self, MunichState};

const SR: u32 = 48000;
const BPM: f32 = 127.0;

/// Eighths of two Cs, a G and a B flat; (sample, on, note).
fn events() -> Vec<(usize, bool, u8)> {
    let eighth = SR as f32 * 30.0 / BPM;
    let mut out = Vec::new();
    for k in 0..24 {
        let note = 48 + [0u8, 12, 7, 10][k % 4];
        out.push(((k as f32 * eighth) as usize, true, note));
        out.push((((k as f32 + 0.44) * eighth) as usize, false, note));
    }
    out
}

/// How strong a frequency is.
fn amount(x: &[f32], f: f32) -> f32 {
    let (mut re, mut im) = (0.0f32, 0.0f32);
    for (i, v) in x.iter().enumerate() {
        let w = 2.0 * std::f32::consts::PI * f * i as f32 / SR as f32;
        re += v * w.cos();
        im += v * w.sin();
    }
    2.0 * (re * re + im * im).sqrt() / x.len().max(1) as f32
}

fn report(out_dir: &std::path::Path, name: &str, file: String, l: &[f32], r: &[f32]) {
    let peak = l.iter().chain(r).fold(0.0f32, |m, x| m.max(x.abs()));
    let rms = (l.iter().chain(r).map(|x| x * x).sum::<f32>() / (l.len() + r.len()).max(1) as f32).sqrt();
    let interleaved: Vec<f32> = l.iter().zip(r).flat_map(|(a, b)| [*a, *b]).collect();
    krabton_engine::render::write_wav(&out_dir.join(&file), SR, &interleaved).expect("write wav");
    let db = |v: f32| 20.0 * v.max(1e-9).log10();
    // The note's own low end and its upper harmonics: how round and how bright.
    let low = amount(l, 65.4).max(amount(l, 130.8));
    let high = amount(l, 654.0) + amount(l, 1308.0);
    println!("{name:<20} peak {:+6.1}  rms {:+6.1}  low {:+6.1}  high {:+6.1} dB -> {file}", db(peak), db(rms), db(low), db(high));
}

fn main() {
    let out_dir = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "munich_presets".into()));
    std::fs::create_dir_all(&out_dir).expect("create output dir");
    let total = (SR as f32 * 30.0 / BPM * 26.0) as usize;
    for (i, name) in munich::preset_names().iter().enumerate() {
        let p = munich::preset(name);
        let mut s = MunichState::new(SR as f32);
        s.bpm = BPM;
        let (mut l, mut r) = (vec![0.0f32; total], vec![0.0f32; total]);
        let mut pos = 0;
        for (at, on, note) in events() {
            s.render(&p, &mut l[pos..at], &mut r[pos..at]);
            pos = at;
            if on {
                s.note_on(&p, note, 0.9);
            } else {
                s.note_off(&p, note);
            }
        }
        s.render(&p, &mut l[pos..], &mut r[pos..]);
        report(&out_dir, name, format!("{:02}_{}.wav", i + 1, name.replace(' ', "_")), &l, &r);
    }
    let p = analog::preset("Moog Bass");
    let mut s = AnalogState::new(SR as f32);
    let (mut l, mut r) = (vec![0.0f32; total], vec![0.0f32; total]);
    let mut pos = 0;
    for (at, on, note) in events() {
        s.render(&p, &mut l[pos..at], &mut r[pos..at]);
        pos = at;
        if on {
            s.note_on(&p, note - 12, 0.9);
        } else {
            s.note_off(&p, note - 12);
        }
    }
    s.render(&p, &mut l[pos..], &mut r[pos..]);
    report(&out_dir, "(Analog Moog Bass)", "analog_moog_bass.wav".into(), &l, &r);
}
