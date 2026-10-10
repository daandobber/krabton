//! Behaviour tests for all models: sound, pitch, release, controls, sample
//! rates, note range and presets. Run with `--nocapture` to see the level
//! and pitch table printed by `report`.

use crate::*;

const RATES: [f32; 2] = [44100.0, 96000.0];

fn voice(id: &str, sr: f32) -> Voice {
    create_voice(id, sr).unwrap_or_else(|| panic!("unknown model {id}"))
}

/// Renders `seconds` of the raw model output (bypassing the safety guard),
/// failing on non-finite or runaway samples.
fn render(v: &mut Voice, sr: f32, seconds: f32, what: &str) -> Vec<f32> {
    let n = (sr * seconds) as usize;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let y = v.model.tick();
        assert!(y.is_finite() && y.abs() < 64.0, "{what}: sample {i} is {y}");
        out.push(y as f32);
    }
    out
}

fn peak(x: &[f32]) -> f32 {
    x.iter().fold(0.0f32, |m, v| m.max(v.abs()))
}

/// Fundamental frequency by normalized autocorrelation: the shortest lag
/// whose correlation is within 10% of the best one.
fn estimate_pitch(x: &[f32], sr: f32) -> Option<f32> {
    let min_lag = (sr / 2500.0) as usize;
    let max_lag = (sr / 40.0) as usize;
    let n = x.len().checked_sub(max_lag + 2)?;
    let energy = |a: usize| -> f64 { x[a..a + n].iter().map(|&v| v as f64 * v as f64).sum() };
    let e0 = energy(0);
    if e0 <= 0.0 {
        return None;
    }
    let mut r = vec![0.0f64; max_lag + 2];
    for (lag, r_lag) in r.iter_mut().enumerate().take(max_lag + 2).skip(min_lag.max(1) - 1) {
        let s: f64 = (0..n).map(|i| x[i] as f64 * x[i + lag] as f64).sum();
        *r_lag = s / (e0 * energy(lag)).sqrt().max(1e-30);
    }
    let best = (min_lag..=max_lag).map(|l| r[l]).fold(f64::MIN, f64::max);
    if best < 0.3 {
        return None;
    }
    let lag = (min_lag..=max_lag).find(|&l| r[l] >= 0.9 * best && r[l] >= r[l - 1] && r[l] >= r[l + 1])?;
    let (a, b, c) = (r[lag - 1], r[lag], r[lag + 1]);
    let denom = a - 2.0 * b + c;
    let offset = if denom.abs() > 1e-12 { 0.5 * (a - c) / denom } else { 0.0 };
    Some(sr / (lag as f64 + offset.clamp(-0.5, 0.5)) as f32)
}

/// Magnitude of the DFT of `x` at `f` Hz.
fn goertzel(x: &[f32], f: f32, sr: f32) -> f32 {
    let w = 2.0 * std::f64::consts::PI * f as f64 / sr as f64;
    let (mut re, mut im) = (0.0f64, 0.0f64);
    for (n, &v) in x.iter().enumerate() {
        re += v as f64 * (w * n as f64).cos();
        im -= v as f64 * (w * n as f64).sin();
    }
    (re * re + im * im).sqrt() as f32
}

/// The fundamental: by autocorrelation, or for the inharmonic ModalBar as
/// the strongest spectral peak an octave around the note.
fn measure_pitch(id: &str, x: &[f32], sr: f32) -> Option<f32> {
    if id == "modalbar" {
        let candidates = (0..=600).map(|i| 150.0 + i as f32 * 0.25);
        let best = candidates.map(|f| (f, goertzel(x, f, sr))).fold((0.0, 0.0), |b, c| if c.1 > b.1 { c } else { b });
        return (best.1 > 0.0).then_some(best.0);
    }
    estimate_pitch(x, sr)
}

/// The window used to measure pitch, in seconds after note-on.
fn pitch_window(id: &str) -> (f32, f32) {
    match id {
        "modalbar" => (0.02, 0.2),
        "plucked" | "mandolin" | "stifkarp" => (0.05, 0.15),
        _ => (0.4, 0.15),
    }
}

/// Whistle transposes and wobbles, and STK's Recorder locks onto upper
/// pipe modes or edge tones; neither is tested for pitch.
fn is_pitched(id: &str) -> bool {
    id != "whistle" && id != "recorder"
}

fn presets_of(id: &str) -> usize {
    MODELS.iter().find(|m| m.id == id).map(|m| m.presets.len()).unwrap_or(0)
}

#[test]
fn models_are_listed_and_created() {
    assert_eq!(MODELS.len(), 16);
    for m in MODELS {
        assert!(["Winds", "Strings", "Mallets", "Voice"].contains(&m.family), "{}", m.id);
        assert!(create(m.id, 44100.0).is_some(), "{}", m.id);
        for c in m.controls {
            assert!((0.0..=128.0).contains(&c.default), "{} control {}", m.id, c.number);
        }
    }
    assert!(create("rhodey", 44100.0).is_none());
    assert!(create("", 44100.0).is_none());
}

#[test]
fn silent_until_note_on() {
    for m in MODELS {
        for sr in RATES {
            let mut v = voice(m.id, sr);
            let out = render(&mut v, sr, 0.3, m.id);
            assert!(peak(&out) < 1e-6, "{} at {sr}: {}", m.id, peak(&out));
        }
    }
}

#[test]
fn notes_sound_with_the_right_pitch() {
    for m in MODELS {
        for sr in RATES {
            let mut v = voice(m.id, sr);
            v.note_on(220.0, 0.8);
            let out = render(&mut v, sr, 1.0, m.id);
            let p = peak(&out);
            assert!(p > 0.01, "{} at {sr} is too quiet: {p}", m.id);
            assert!(p < 4.0, "{} at {sr} is too loud: {p}", m.id);
            if is_pitched(m.id) {
                let (start, len) = pitch_window(m.id);
                let a = (start * sr) as usize;
                let b = ((start + len) * sr) as usize;
                let f = measure_pitch(m.id, &out[a..b], sr);
                let f = f.unwrap_or_else(|| panic!("{} at {sr}: no pitch", m.id));
                assert!((f / 220.0 - 1.0).abs() < 0.03, "{} at {sr}: pitch {f} Hz instead of 220", m.id);
            }
        }
    }
}

#[test]
fn every_preset_sounds() {
    for m in MODELS.iter().filter(|m| !m.presets.is_empty()) {
        for sr in RATES {
            for p in 0..m.presets.len() {
                let mut v = voice(m.id, sr);
                v.set_preset(p);
                v.note_on(220.0, 0.8);
                let what = format!("{} preset {} at {sr}", m.id, m.presets[p]);
                let out = render(&mut v, sr, 0.5, &what);
                let pk = peak(&out);
                assert!(pk > 0.005 && pk < 4.0, "{what}: peak {pk}");
            }
        }
    }
}

/// Peak of the last half second of a four-second release, relative to the
/// note's peak.
fn release_tail(id: &str, preset: Option<usize>, off_amplitude: f32, sr: f32) -> (f32, f32) {
    let mut v = voice(id, sr);
    if let Some(p) = preset {
        v.set_preset(p);
    }
    v.note_on(220.0, 0.8);
    let note = render(&mut v, sr, 1.0, id);
    v.note_off(off_amplitude);
    let tail = render(&mut v, sr, 4.0, id);
    (peak(&note), peak(&tail[(3.5 * sr) as usize..]))
}

#[test]
fn notes_stop_after_note_off() {
    for m in MODELS {
        for sr in RATES {
            for off in [0.5, 0.0, 1.0] {
                let presets: Vec<Option<usize>> = match presets_of(m.id) {
                    0 => vec![None],
                    n if m.id == "voicform" => (0..n).step_by(5).map(Some).collect(),
                    n => (0..n).map(Some).collect(),
                };
                for preset in presets {
                    let (note, tail) = release_tail(m.id, preset, off, sr);
                    assert!(
                        tail < 1e-3 && tail < 0.01 * note.max(1e-3),
                        "{} {preset:?} at {sr}, note-off {off}: tail {tail} after peak {note}",
                        m.id
                    );
                }
            }
        }
    }
}

#[test]
fn controls_at_extremes_stay_finite() {
    for m in MODELS {
        for sr in RATES {
            for c in m.controls {
                for value in [0.0, 128.0] {
                    let what = format!("{} control {} ({}) = {value} at {sr}", m.id, c.number, c.name);
                    // Set before the note, and again while it sounds.
                    let mut v = voice(m.id, sr);
                    v.control_change(c.number, value);
                    v.note_on(220.0, 0.8);
                    v.control_change(c.number, value);
                    let out = render(&mut v, sr, 0.75, &what);
                    assert!(peak(&out) < 16.0, "{what}: peak {}", peak(&out));
                    v.note_off(0.5);
                    render(&mut v, sr, 0.5, &what);
                }
            }
        }
    }
}

#[test]
fn note_range_does_not_blow_up() {
    for m in MODELS {
        for sr in RATES {
            for f in [50.0, 80.0, 110.0, 330.0, 660.0, 1000.0, 1500.0, 2000.0] {
                let what = format!("{} at {f} Hz, {sr}", m.id);
                let mut v = voice(m.id, sr);
                v.note_on(f, 0.8);
                let out = render(&mut v, sr, 0.5, &what);
                assert!(peak(&out) < 4.0, "{what}: peak {}", peak(&out));
                // Legato to another note on the same voice.
                v.note_on(f * 1.5, 1.0);
                render(&mut v, sr, 0.25, &what);
            }
        }
    }
}

#[test]
fn hostile_input_does_not_panic() {
    for m in MODELS {
        for sr in [0.0, f32::NAN, 8000.0, 192000.0] {
            let mut v = create(m.id, sr).unwrap();
            v.note_on(f32::NAN, 0.5);
            v.note_on(f32::INFINITY, f32::NAN);
            v.note_on(-100.0, -3.0);
            v.note_on(1e9, 10.0);
            v.note_off(f32::NAN);
            v.note_off(-1.0);
            for n in [0u8, 1, 2, 4, 8, 11, 16, 26, 29, 64, 65, 100, 101, 128, 255] {
                for value in [f32::NAN, -5.0, 0.0, 64.0, 128.0, 1e9] {
                    v.control_change(n, value);
                }
            }
            v.set_preset(usize::MAX);
            v.set_preset(3);
            v.note_on(220.0, 1.0);
            for _ in 0..4000 {
                assert!(v.tick().is_finite(), "{}", m.id);
            }
        }
    }
}

#[test]
fn report() {
    println!("{:<10} {:>8} {:>8} {:>8} {:>9} {:>9}", "model", "peak", "peak96", "pitch", "tail 0.5", "tail 0.0");
    for m in MODELS {
        let sr = 44100.0;
        let mut v = voice(m.id, sr);
        v.note_on(220.0, 0.8);
        let out = render(&mut v, sr, 1.0, m.id);
        let (start, len) = pitch_window(m.id);
        let f = measure_pitch(m.id, &out[(start * sr) as usize..((start + len) * sr) as usize], sr).unwrap_or(0.0);
        let preset = if presets_of(m.id) > 0 { Some(0) } else { None };
        let (_, t5) = release_tail(m.id, preset, 0.5, sr);
        let (_, t0) = release_tail(m.id, preset, 0.0, sr);
        let mut v96 = voice(m.id, 96000.0);
        v96.note_on(220.0, 0.8);
        let p96 = peak(&render(&mut v96, 96000.0, 1.0, m.id));
        println!("{:<10} {:>8.3} {:>8.3} {:>8.1} {:>9.2e} {:>9.2e}", m.id, peak(&out), p96, f, t5, t0);
        for p in 0..presets_of(m.id) {
            let mut v = voice(m.id, sr);
            v.set_preset(p);
            v.note_on(220.0, 0.8);
            let out = render(&mut v, sr, 1.0, m.id);
            println!("    {:<18} {:>8.3}", m.presets[p], peak(&out));
        }
    }
}

/// Bowed bars build up over a second or more (as in STK); STK's uniform
/// bar does not speak when bowed.
#[test]
fn bandedwg_bows_and_stops() {
    for sr in RATES {
        for preset in 1..4 {
            let mut v = voice("bandedwg", sr);
            v.set_preset(preset);
            v.control_change(2, 64.0);
            v.note_on(220.0, 0.8);
            let what = format!("bowed bandedwg preset {preset} at {sr}");
            let out = render(&mut v, sr, 5.0, &what);
            let p = peak(&out);
            assert!(p > 0.05 && p < 4.0, "{what}: peak {p}");
            let f = estimate_pitch(&out[(4.0 * sr) as usize..(4.2 * sr) as usize], sr).unwrap_or(0.0);
            // The bowed Tibetan bowl sings on its 5.7 x mode, as in STK.
            let expected = if preset == 3 { 220.0 * 5.704452 } else { 220.0 };
            assert!((f / expected - 1.0).abs() < 0.03, "{what}: pitch {f}");
            v.note_off(0.0);
            let tail = render(&mut v, sr, 6.0, &what);
            let t = peak(&tail[(5.5 * sr) as usize..]);
            println!("{what}: peak {p:.3} pitch {f:.1} tail {t:.2e}");
            assert!(t < 1e-3, "{what}: tail {t}");
        }
    }
}
