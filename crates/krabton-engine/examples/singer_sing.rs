//! Sings a short tune with lyrics to a WAV file and prints its level: with a
//! UTAU voicebank's folder, or with the Singer's own voice.
//!
//! ```sh
//! cargo run --release -p krabton-engine --example singer_sing -- out.wav [voicebank_dir] ["sa ku ra sa ku ra"]
//! ```

use krabton_engine::midi_fx::Lyric;
use krabton_engine::params::defaults;
use krabton_engine::singer::{kana, SingerState, VoicebankRef, SPECS};

fn main() {
    let mut args = std::env::args().skip(1);
    let out = std::path::PathBuf::from(args.next().unwrap_or_else(|| "singer.wav".into()));
    let bank = args.next().filter(|a| !a.is_empty()).map(|dir| VoicebankRef::load(std::path::Path::new(&dir)).expect("read the voicebank"));
    let lyrics = kana::split_lyrics(&args.next().unwrap_or_else(|| "sa ku ra sa ku ra".into()));
    if let Some(b) = &bank {
        // Its pitch periods are found in the background: wait for them.
        while b.ready() < 1.0 {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        println!("{}: {} samples, {}", b.name, b.entries.len(), if b.vcv { "VCV" } else { "CV" });
    }
    let sr = 48000u32;
    let spb = sr as f64 * 0.5; // 120 BPM
    let tune: [(f64, f64, u8); 6] = [(0.0, 1.0, 69), (1.0, 1.0, 69), (2.0, 2.0, 71), (4.0, 1.0, 69), (5.0, 1.0, 69), (6.0, 2.0, 71)];
    let p = defaults(SPECS);
    let mut s = SingerState::new(sr as f32);
    let total = (spb * 9.0) as usize;
    let (mut l, mut r) = (vec![0.0f32; total], vec![0.0f32; total]);
    // As the engine sends a clip's notes to a Singer: a quarter second early.
    let lead = krabton_engine::singer::LEAD_SECS * sr as f64;
    let mut events: Vec<(usize, bool, u8, Option<String>)> = Vec::new();
    for (k, (start, len, note)) in tune.iter().enumerate() {
        let at = (start * spb - lead).max(0.0) as usize;
        events.push((at, true, *note, lyrics.get(k % lyrics.len().max(1)).cloned()));
        events.push((((start + len) * spb - lead - 0.02 * sr as f64) as usize, false, *note, None));
    }
    events.sort_by_key(|e| (e.0, e.1));
    let mut pos = 0;
    for (at, on, note, lyric) in events {
        if at > pos {
            for (a, b) in l[pos..at].chunks_mut(256).zip(r[pos..at].chunks_mut(256)) {
                s.render(&p, bank.as_ref(), a, b);
            }
            pos = at;
        }
        if on {
            s.set_lyric(Lyric::new(lyric.as_deref().unwrap_or(""), true));
            s.note_on(&p, bank.as_ref(), note, 0.85);
        } else {
            s.note_off(&p, note);
        }
    }
    for (a, b) in l[pos..].chunks_mut(256).zip(r[pos..].chunks_mut(256)) {
        s.render(&p, bank.as_ref(), a, b);
    }
    let peak = l.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    let rms = (l.iter().map(|x| x * x).sum::<f32>() / total as f32).sqrt();
    // The biggest jump from one sample to the next (a click shows as one).
    let jump = l.windows(2).fold(0.0f32, |m, w| m.max((w[1] - w[0]).abs()));
    let interleaved: Vec<f32> = l.iter().zip(&r).flat_map(|(a, b)| [*a, *b]).collect();
    krabton_engine::render::write_wav(&out, sr, &interleaved).expect("write wav");
    println!("peak {:+.1} dBFS  rms {:+.1} dBFS  largest step {jump:.3}  -> {}", 20.0 * peak.max(1e-9).log10(), 20.0 * rms.max(1e-9).log10(), out.display());
}
