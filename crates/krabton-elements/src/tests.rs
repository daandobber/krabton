use crate::{Part, Patch, PerformanceState, ResonatorModel, BLOCK_SIZE, SAMPLE_RATE};

fn render(part: &mut Part, states: &[PerformanceState], blocks: usize) -> (Vec<f32>, Vec<f32>) {
    let (mut main, mut aux) = (Vec::new(), Vec::new());
    for _ in 0..blocks {
        let mut m = [0.0; BLOCK_SIZE];
        let mut a = [0.0; BLOCK_SIZE];
        part.process(states, &mut m, &mut a);
        main.extend_from_slice(&m);
        aux.extend_from_slice(&a);
    }
    (main, aux)
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
}

/// Pitch by normalized autocorrelation with parabolic refinement.
fn pitch_hz(x: &[f32], lo_hz: f32, hi_hz: f32) -> f32 {
    let min_lag = (SAMPLE_RATE / hi_hz) as usize;
    let max_lag = (SAMPLE_RATE / lo_hz) as usize;
    let n = x.len() - max_lag - 2;
    let corr = |lag: usize| {
        let (mut xy, mut xx, mut yy) = (0.0f64, 0.0f64, 0.0f64);
        for i in 0..n {
            let (a, b) = (x[i] as f64, x[i + lag] as f64);
            xy += a * b;
            xx += a * a;
            yy += b * b;
        }
        xy / (xx * yy).sqrt().max(1e-20)
    };
    let scores: Vec<f64> = (min_lag..=max_lag + 1).map(corr).collect();
    let best = (1..scores.len() - 1).max_by(|&a, &b| scores[a].partial_cmp(&scores[b]).unwrap()).unwrap();
    let (y0, y1, y2) = (scores[best - 1], scores[best], scores[best + 1]);
    let shift = 0.5 * (y0 - y2) / (y0 - 2.0 * y1 + y2);
    SAMPLE_RATE / ((min_lag + best) as f64 + shift) as f32
}

fn dry_patch() -> Patch {
    // No reverb, no stereo spread, no dry exciter on the outputs.
    Patch { space: 0.1, ..Patch::default() }
}

fn note(n: f32) -> PerformanceState {
    PerformanceState { gate: true, note: n, modulation: 0.0, strength: 0.5 }
}

#[test]
fn modal_strike_rings_at_the_note() {
    let mut part = Part::new(1);
    part.patch = Patch { resonator_geometry: 0.27, exciter_strike_meta: 0.3, ..dry_patch() };
    let (main, _) = render(&mut part, &[note(57.0)], 400);
    let tail = &main[4000..];
    assert!(rms(tail) > 0.005, "too quiet: {}", rms(tail));
    let f = pitch_hz(tail, 100.0, 800.0);
    assert!((f - 220.0).abs() < 2.0, "pitch {f}");
}

#[test]
fn string_models_ring_at_the_note() {
    for model in [ResonatorModel::String, ResonatorModel::Strings] {
        let mut part = Part::new(1);
        part.set_resonator_model(model);
        part.patch = Patch { resonator_geometry: 0.25, resonator_damping: 0.5, ..dry_patch() };
        let (main, _) = render(&mut part, &[note(45.0)], 400);
        let tail = &main[4000..];
        assert!(rms(tail) > 0.005, "{model:?} too quiet: {}", rms(tail));
        // The chord of the Strings model includes a string an octave below.
        let f = pitch_hz(tail, 50.0, 400.0);
        let expected = if model == ResonatorModel::String { 110.0 } else { 55.0 };
        assert!((f - expected).abs() < expected * 0.02, "{model:?} pitch {f}");
    }
}

#[test]
fn bow_and_blow_sustain_while_the_gate_is_high() {
    for (bow, blow) in [(1.0, 0.0), (0.0, 0.8), (0.0, 1.0)] {
        let mut part = Part::new(1);
        part.patch = Patch { exciter_strike_level: 0.0, exciter_bow_level: bow, exciter_blow_level: blow, exciter_envelope_shape: 0.8, ..dry_patch() };
        let (held, _) = render(&mut part, &[note(60.0)], 4000);
        let late = rms(&held[48000..]);
        assert!(late > 0.003, "bow {bow} blow {blow}: no sustain ({late})");
        let (released, _) = render(&mut part, &[PerformanceState { gate: false, ..note(60.0) }], 6000);
        let end = rms(&released[80000..]);
        assert!(end < late * 0.05, "bow {bow} blow {blow}: still sounding after release ({end} vs {late})");
    }
}

#[test]
fn voices_play_together_and_go_idle() {
    let mut part = Part::new(3);
    part.patch = dry_patch();
    let chord = [note(60.0), note(64.0), note(67.0)];
    let (main, _) = render(&mut part, &chord, 200);
    assert!(rms(&main) > 0.01);
    assert!((0..3).all(|i| part.voice_active(i)));
    let off = [PerformanceState { gate: false, ..note(60.0) }; 3];
    render(&mut part, &off, 20000);
    assert!((0..3).all(|i| !part.voice_active(i)), "voices should stop rendering once silent");
    // A quiet voice starts again on the next gate.
    let (again, _) = render(&mut part, &[note(72.0)], 100);
    assert!(rms(&again) > 0.005);
}

#[test]
fn extreme_settings_stay_finite() {
    for model in [ResonatorModel::Modal, ResonatorModel::String, ResonatorModel::Strings] {
        for corner in 0..64u32 {
            let v = |bit: u32| if corner & (1 << bit) != 0 { 0.9995 } else { 0.0 };
            let mut part = Part::new(1);
            part.set_resonator_model(model);
            part.patch = Patch {
                exciter_bow_level: v(0),
                exciter_blow_level: v(1),
                exciter_strike_level: 1.0,
                exciter_strike_meta: v(2),
                resonator_geometry: v(3),
                resonator_brightness: v(4),
                resonator_damping: v(5),
                exciter_envelope_shape: 0.5,
                space: 1.0,
                ..Patch::default()
            };
            for n in [0.0, 24.0, 96.0, 127.0] {
                let (main, aux) = render(&mut part, &[PerformanceState { strength: 1.0, ..note(n) }], 60);
                assert!(main.iter().chain(aux.iter()).all(|x| x.is_finite() && x.abs() < 4.0), "{model:?} corner {corner} note {n}");
            }
        }
    }
}

#[test]
fn reverb_freeze_holds_the_tail() {
    let mut part = Part::new(1);
    part.patch = Patch { space: 1.0, ..Patch::default() };
    render(&mut part, &[note(60.0)], 100);
    part.patch.space = 2.0;
    let off = [PerformanceState { gate: false, ..note(60.0) }];
    let (frozen, _) = render(&mut part, &off, 8000);
    assert!(rms(&frozen[100000..]) > 0.002, "frozen tail died: {}", rms(&frozen[100000..]));
    part.patch.space = 1.0;
    let (thawed, _) = render(&mut part, &off, 20000);
    assert!(rms(&thawed[300000..]) < 0.0005);
}

#[test]
#[ignore]
fn report() {
    let mut part = Part::new(1);
    part.patch = Patch { resonator_geometry: 0.27, exciter_strike_meta: 0.3, ..dry_patch() };
    let (main, _) = render(&mut part, &[note(57.0)], 400);
    println!("modal 220: rms {} pitch {}", rms(&main[4000..]), pitch_hz(&main[4000..], 100.0, 800.0));
    for model in [ResonatorModel::Modal, ResonatorModel::String, ResonatorModel::Strings] {
        let mut part = Part::new(8);
        part.set_resonator_model(model);
        part.patch = Patch { exciter_bow_level: 0.5, exciter_blow_level: 0.5, ..Patch::default() };
        let states: Vec<_> = (0..8).map(|i| note(48.0 + 3.0 * i as f32)).collect();
        let t = std::time::Instant::now();
        let (main, aux) = render(&mut part, &states, 2000);
        let el = t.elapsed().as_secs_f32();
        let peak = main.iter().chain(aux.iter()).fold(0.0f32, |m, x| m.max(x.abs()));
        println!("{model:?}: 8 voices, 1 s audio in {:.1} ms ({:.1}% of one core), rms {:.3} peak {:.3}", el * 1000.0, el * 100.0, rms(&main), peak);
    }
    for (name, p) in [("default", Patch::default()), ("bow", Patch { exciter_bow_level: 1.0, exciter_strike_level: 0.0, ..Patch::default() }), ("blow", Patch { exciter_blow_level: 1.0, exciter_strike_level: 0.0, ..Patch::default() })] {
        let mut part = Part::new(1);
        part.patch = p;
        let (main, aux) = render(&mut part, &[note(60.0)], 2000);
        let peak = main.iter().chain(aux.iter()).fold(0.0f32, |m, x| m.max(x.abs()));
        println!("{name}: rms {:.3} peak {:.3}", rms(&main), peak);
    }
}
