use crate::resample::BlockAdapter;
use crate::{EggFx, Model, Params, Rings};

const MODELS: [Model; 6] = [
    Model::Modal,
    Model::SympatheticString,
    Model::InharmonicString,
    Model::FmVoice,
    Model::SympatheticStringQuantized,
    Model::StringAndReverb,
];

const EGG_FX: [EggFx; 6] = [EggFx::Formant, EggFx::Chorus, EggFx::Reverb, EggFx::Formant2, EggFx::Ensemble, EggFx::Reverb2];

/// Renders `secs` seconds in host blocks of 64 (the last one shorter).
fn render(r: &mut Rings, p: &Params, input: Option<&[f32]>, secs: f32, sr: f32) -> (Vec<f32>, Vec<f32>) {
    let n = (secs * sr) as usize;
    let mut l = vec![0.0; n];
    let mut rr = vec![0.0; n];
    let mut pos = 0;
    while pos < n {
        let end = (pos + 64).min(n);
        let inp = input.map(|x| &x[pos..end]);
        r.process(p, inp, &mut l[pos..end], &mut rr[pos..end]);
        pos = end;
    }
    (l, rr)
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
}

fn peak(a: &[f32], b: &[f32]) -> f32 {
    a.iter().chain(b).fold(0.0f32, |m, v| m.max(v.abs()))
}

/// Pitch by normalized autocorrelation with parabolic refinement.
fn pitch_hz(x: &[f32], sr: f32, lo: f32, hi: f32) -> f32 {
    let (min_lag, max_lag) = ((sr / hi) as usize, (sr / lo) as usize);
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
    let s: Vec<f64> = (min_lag..=max_lag + 1).map(corr).collect();
    let b = (1..s.len() - 1).max_by(|&a, &b| s[a].partial_cmp(&s[b]).unwrap()).unwrap();
    let shift = 0.5 * (s[b - 1] - s[b + 1]) / (s[b - 1] - 2.0 * s[b] + s[b + 1]);
    sr / ((min_lag + b) as f64 + shift) as f32
}

/// Level of the `hz` component (Goertzel, normalized to a sine's
/// amplitude).
fn level_at(x: &[f32], sr: f32, hz: f32) -> f32 {
    let w = 2.0 * std::f64::consts::PI * hz as f64 / sr as f64;
    let c = 2.0 * w.cos();
    let (mut s1, mut s2) = (0.0f64, 0.0f64);
    for &v in x {
        let s0 = v as f64 + c * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    let power = s1 * s1 + s2 * s2 - c * s1 * s2;
    (2.0 * power.sqrt() / x.len() as f64) as f32
}

fn midi_hz(note: f32) -> f32 {
    440.0 * 2f32.powf((note - 69.0) / 12.0)
}

/// Renders until silent (or `max_secs`); returns the seconds it took.
fn time_to_silence(r: &mut Rings, p: &Params, sr: f32, max_secs: f32) -> Option<f32> {
    let mut t = 0.0;
    while t < max_secs {
        let (l, rr) = render(r, p, None, 0.25, sr);
        assert!(l.iter().chain(&rr).all(|v| v.is_finite()));
        t += 0.25;
        if r.is_silent() {
            return Some(t);
        }
    }
    None
}

fn check_note_sounds_and_decays(p: &Params, sr: f32, what: &str) {
    let mut r = Rings::new(sr);
    assert!(r.is_silent(), "{what}: not silent before any note");
    r.note_on(57.0, 0.8);
    assert!(!r.is_silent());
    let (l, rr) = render(&mut r, p, None, 1.0, sr);
    assert!(l.iter().chain(&rr).all(|v| v.is_finite()), "{what}: not finite");
    let level = rms(&l).max(rms(&rr));
    let pk = peak(&l, &rr);
    assert!(level > 2e-3, "{what}: silent ({level})");
    assert!(pk < 4.0, "{what}: peak {pk}");
    assert!(!r.is_silent(), "{what}: silent while ringing");
    let t = time_to_silence(&mut r, p, sr, 90.0);
    assert!(t.is_some(), "{what}: never decays");
    // Silent means silent.
    let (l, rr) = render(&mut r, p, None, 0.1, sr);
    assert!(peak(&l, &rr) < 1e-3, "{what}: is_silent but {}", peak(&l, &rr));
}

#[test]
fn every_model_sounds_and_decays() {
    for model in MODELS {
        for polyphony in [1, 2, 4] {
            let p = Params { model, polyphony, ..Params::default() };
            check_note_sounds_and_decays(&p, 48000.0, &format!("{model:?} x{polyphony}"));
        }
    }
}

#[test]
fn easter_egg_sounds_and_decays_with_every_fx() {
    for egg_fx in EGG_FX {
        for polyphony in [1, 4] {
            let p = Params { easter_egg: true, egg_fx, polyphony, ..Params::default() };
            check_note_sounds_and_decays(&p, 48000.0, &format!("egg {egg_fx:?} x{polyphony}"));
        }
    }
}

#[test]
fn pitch_tracks_the_note() {
    for sr in [48000.0f32, 44100.0] {
        for (model, structure) in [(Model::Modal, 0.27), (Model::InharmonicString, 0.25)] {
            let p = Params { model, structure, damping: 0.7, ..Params::default() };
            let mut f = [0.0; 2];
            for (k, note) in [57.0, 69.0].into_iter().enumerate() {
                let mut r = Rings::new(sr);
                r.note_on(note, 0.8);
                let (l, rr) = render(&mut r, &p, None, 0.6, sr);
                assert!(peak(&l, &rr) < 4.0);
                // The odd output: odd modes, or the string itself.
                f[k] = pitch_hz(&l[(0.1 * sr) as usize..], sr, 150.0, 1000.0);
                let expected = midi_hz(note);
                assert!((f[k] - expected).abs() < expected * 0.01, "{model:?} @ {sr}: note {note} at {} Hz", f[k]);
            }
            let ratio = f[1] / f[0];
            assert!((ratio - 2.0).abs() < 0.02, "{model:?} @ {sr}: A4/A3 = {ratio}");
        }
    }
}

#[test]
fn polyphony_four_lets_four_notes_ring() {
    let notes = [60.0, 64.0, 67.0, 71.0];
    for model in [Model::InharmonicString, Model::Modal] {
        let structure = if model == Model::Modal { 0.27 } else { 0.25 };
        let levels = |polyphony: u8| {
            let p = Params { model, polyphony, structure, damping: 0.7, ..Params::default() };
            let mut r = Rings::new(48000.0);
            for n in notes {
                r.note_on(n, 0.8);
            }
            let (l, rr) = render(&mut r, &p, None, 0.6, 48000.0);
            // Levels add up across the outputs (summing the signals could
            // cancel partials: the outputs are odd/even sets).
            notes.map(|n| level_at(&l[4800..], 48000.0, midi_hz(n)) + level_at(&rr[4800..], 48000.0, midi_hz(n)))
        };
        let poly = levels(4);
        let loudest = poly.iter().fold(0.0f32, |m, &v| m.max(v));
        for (n, l) in notes.iter().zip(poly) {
            assert!(l > loudest * 0.2, "{model:?}: note {n} does not ring with 4 voices ({poly:?})");
        }
        // With one voice, each note retunes the same resonator: the first
        // note is gone.
        let mono = levels(1);
        assert!(mono[0] < mono[3] * 0.1, "{model:?}: one voice rings several notes ({mono:?})");
    }
}

#[test]
fn notes_in_one_block_keep_their_own_pitch() {
    // Two notes far apart, struck in the same host block, on two voices:
    // voice 0 goes to the left output, voice 1 to the right one.
    let p = Params { model: Model::InharmonicString, polyphony: 2, structure: 0.25, damping: 0.7, ..Params::default() };
    let mut r = Rings::new(48000.0);
    r.note_on(57.0, 0.8);
    r.note_on(69.0, 0.8);
    let (l, rr) = render(&mut r, &p, None, 0.6, 48000.0);
    let fl = pitch_hz(&l[4800..], 48000.0, 150.0, 1000.0);
    let fr = pitch_hz(&rr[4800..], 48000.0, 150.0, 1000.0);
    let mut f = [fl, fr];
    f.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert!((f[0] - 220.0).abs() < 3.0 && (f[1] - 440.0).abs() < 5.0, "{fl} / {fr}");
}

#[test]
fn external_input_excites_the_resonator() {
    for sr in [48000.0f32, 44100.0] {
        for model in MODELS {
            let p = Params { model, ..Params::default() };
            let mut r = Rings::new(sr);
            // Nothing in, nothing out.
            let zeros = vec![0.0f32; (0.2 * sr) as usize];
            let (l, rr) = render(&mut r, &p, Some(&zeros), 0.2, sr);
            assert!(peak(&l, &rr) < 1e-6, "{model:?}: sound without input");
            // A 10 ms noise burst.
            let n = (1.0 * sr) as usize;
            let mut input = vec![0.0f32; n];
            let mut seed = 1u32;
            for x in input[..(0.01 * sr) as usize].iter_mut() {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                *x = (seed as f32 / 4294967296.0 - 0.5) * 0.5;
            }
            let (l, rr) = render(&mut r, &p, Some(&input), 1.0, sr);
            assert!(l.iter().chain(&rr).all(|v| v.is_finite()));
            let ring = rms(&l[(0.05 * sr) as usize..(0.3 * sr) as usize]).max(rms(&rr[(0.05 * sr) as usize..(0.3 * sr) as usize]));
            assert!(ring > 1e-3, "{model:?} @ {sr}: the burst does not ring ({ring})");
            assert!(peak(&l, &rr) < 4.0);
            let silence = vec![0.0f32; (0.25 * sr) as usize];
            let mut t = 0.0;
            while !r.is_silent() && t < 60.0 {
                render(&mut r, &p, Some(&silence), 0.25, sr);
                t += 0.25;
            }
            assert!(r.is_silent(), "{model:?} @ {sr}: the burst never decays");
        }
    }
}

#[test]
fn external_input_resonates_at_the_note() {
    // As an effect: notes set the pitch, the input excites.
    let sr = 44100.0;
    let p = Params { model: Model::InharmonicString, structure: 0.25, damping: 0.8, ..Params::default() };
    let mut r = Rings::new(sr);
    r.note_on(57.0, 0.8);
    let n = (0.8 * sr) as usize;
    let mut input = vec![0.0f32; n];
    let mut seed = 7u32;
    for x in input.iter_mut() {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        *x = (seed as f32 / 4294967296.0 - 0.5) * 0.2;
    }
    let (l, rr) = render(&mut r, &p, Some(&input), 0.8, sr);
    assert!(rms(&l) > 1e-3 && peak(&l, &rr) < 4.0);
    let f = pitch_hz(&l[(0.2 * sr) as usize..], sr, 150.0, 1000.0);
    assert!((f - 220.0).abs() < 3.0, "{f}");
}

#[test]
fn onsets_in_the_input_rotate_the_voices() {
    // No notes: hits in the input strum, each on the next voice (left and
    // right outputs alternate with two voices).
    let sr = 48000.0;
    let p = Params { model: Model::Modal, polyphony: 2, ..Params::default() };
    let mut r = Rings::new(sr);
    let n = sr as usize;
    let mut input = vec![0.0f32; n];
    for hit in [0.1f32, 0.4] {
        let start = (hit * sr) as usize;
        for (k, x) in input[start..start + 200].iter_mut().enumerate() {
            *x = 0.5 * (k as f32 * 0.3).sin() * (1.0 - k as f32 / 200.0);
        }
    }
    let (l, rr) = render(&mut r, &p, Some(&input), 1.0, sr);
    let seg = |x: &[f32], a: f32, b: f32| rms(&x[(a * sr) as usize..(b * sr) as usize]);
    // After the first hit one side rings; after the second the other one
    // starts too.
    let (l1, r1) = (seg(&l, 0.15, 0.35), seg(&rr, 0.15, 0.35));
    let (l2, r2) = (seg(&l, 0.45, 0.65), seg(&rr, 0.45, 0.65));
    let first_left = l1 > r1;
    let (quiet_before, after) = if first_left { (r1, r2) } else { (l1, l2) };
    assert!(after > quiet_before * 4.0, "voices do not rotate: {l1} {r1} / {l2} {r2}");
}

#[test]
fn host_rates_44k1_and_48k_both_work() {
    for sr in [44100.0f32, 48000.0, 96000.0, 22050.0] {
        let p = Params { model: Model::Modal, structure: 0.27, ..Params::default() };
        let mut r = Rings::new(sr);
        r.note_on(57.0, 0.8);
        let (l, rr) = render(&mut r, &p, None, 0.6, sr);
        assert!(rms(&l) > 1e-3 && peak(&l, &rr) < 4.0, "{sr}: silent or too loud");
        let f = pitch_hz(&l[(0.1 * sr) as usize..], sr, 150.0, 1000.0);
        assert!((f - 220.0).abs() < 2.0, "{sr}: {f} Hz");
    }
}

#[test]
fn resampler_is_transparent() {
    // An identity block process: what goes in comes out, delayed.
    for sr in [44100.0f32, 48000.0, 96000.0] {
        let mut a = BlockAdapter::new(sr);
        let n = (0.5 * sr) as usize;
        let hz = 1000.0;
        let input: Vec<f32> = (0..n).map(|i| (2.0 * std::f32::consts::PI * hz * i as f32 / sr).sin() * 0.5).collect();
        let mut l = vec![0.0; n];
        let mut r = vec![0.0; n];
        for start in (0..n).step_by(100) {
            let end = (start + 100).min(n);
            a.process(Some(&input[start..end]), &mut l[start..end], &mut r[start..end], |i, o1, o2| {
                o1.copy_from_slice(i);
                o2.copy_from_slice(i);
            });
        }
        let latency = a.latency();
        let tail = &l[n / 2..];
        let amp = level_at(tail, sr, hz);
        assert!((amp - 0.5).abs() < 0.005, "{sr}: gain {amp}");
        // Matches the expected delay.
        let err: f32 = (n / 2..n)
            .map(|i| {
                let t = i as f32 - latency;
                let want = (2.0 * std::f32::consts::PI * hz * t / sr).sin() * 0.5;
                (l[i] - want).abs()
            })
            .fold(0.0, f32::max);
        assert!(err < 0.01, "{sr}: latency {latency} off by {err}");
    }
}

#[test]
fn extreme_parameters_stay_finite() {
    let values = [0.0, 1.0, -3.0, 7.0, f32::NAN, f32::INFINITY];
    let mut k = 0usize;
    for easter_egg in [false, true] {
        for (mi, model) in MODELS.into_iter().enumerate() {
            let mut r = Rings::new(44100.0);
            for corner in 0..24 {
                k += 1;
                let v = |i: usize| values[(corner * 7 + i * 3 + mi + k) % values.len()];
                let p = Params {
                    model,
                    polyphony: [0, 1, 2, 3, 4, 200][(corner + mi) % 6],
                    easter_egg,
                    egg_fx: EGG_FX[(corner + mi) % 6],
                    structure: v(0),
                    brightness: v(1),
                    damping: v(2),
                    position: v(3),
                    chord: if corner % 3 == 0 { None } else { Some(v(4)) },
                };
                r.note_on([-1000.0, 0.0, 127.0, 1000.0, f32::NAN, 60.5][corner % 6], v(5));
                r.note_on(127.0, 1.0);
                let input: Vec<f32> = (0..2205).map(|i| if i % 50 == 0 { v(6) * 10.0 } else { (i as f32 * 0.37).sin() }).collect();
                let inp = if corner % 2 == 0 { Some(&input[..]) } else { None };
                let (l, rr) = render(&mut r, &p, inp, 0.05, 44100.0);
                assert!(l.iter().chain(&rr).all(|x| x.is_finite()), "{model:?} egg {easter_egg} corner {corner}: not finite");
                assert!(peak(&l, &rr) < 4.0, "{model:?} egg {easter_egg} corner {corner}: peak {}", peak(&l, &rr));
            }
        }
    }
    // Odd host rates and mismatched buffers.
    for sr in [0.0, -1.0, f32::NAN, 1e9, 12345.6] {
        let mut r = Rings::new(sr);
        r.note_on(60.0, 1.0);
        let mut l = vec![0.0; 100];
        let mut rr = vec![0.0; 50];
        r.process(&Params::default(), Some(&[0.1; 10]), &mut l, &mut rr);
        assert!(l.iter().chain(&rr).all(|x| x.is_finite()));
    }
}

#[test]
fn held_drone_and_infinite_sustain_stay_bounded() {
    // Damping fully up: the strings sustain forever and the egg drones.
    for (egg, model) in [(false, Model::InharmonicString), (false, Model::SympatheticString), (false, Model::FmVoice), (true, Model::Modal)] {
        let p = Params { model, easter_egg: egg, damping: 1.0, brightness: 1.0, polyphony: 4, ..Params::default() };
        let mut r = Rings::new(48000.0);
        let mut max = 0.0f32;
        for i in 0..40 {
            r.note_on(40.0 + (i % 7) as f32 * 5.0, 1.0);
            let (l, rr) = render(&mut r, &p, None, 0.1, 48000.0);
            assert!(l.iter().chain(&rr).all(|x| x.is_finite()));
            max = max.max(peak(&l, &rr));
        }
        assert!(max < 4.0, "{model:?} egg {egg}: {max}");
        assert!(!r.is_silent());
    }
}

#[test]
fn rings_is_send() {
    fn send<T: Send>() {}
    send::<Rings>();
}

/// Flushes denormals to zero on this thread, as Krabton's engine does on
/// its audio thread (x86-64 only).
#[allow(deprecated)]
fn set_ftz(on: bool) {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        use std::arch::x86_64::{_mm_getcsr, _mm_setcsr};
        let c = _mm_getcsr();
        _mm_setcsr(if on { c | 0x8040 } else { c & !0x8040 });
    }
    #[cfg(not(target_arch = "x86_64"))]
    let _ = on;
}

#[test]
#[ignore]
fn report() {
    for ftz in [false, true] {
        set_ftz(ftz);
        let mut r = Rings::new(44100.0);
        r.note_on(57.0, 0.8);
        let t = std::time::Instant::now();
        render(&mut r, &Params::default(), None, 2.0, 44100.0);
        println!("Modal x1, denormals flushed: {ftz}: 1 s in {:.1} ms", t.elapsed().as_secs_f32() * 500.0);
    }
    for sr in [44100.0f32, 48000.0] {
        let a = BlockAdapter::new(sr);
        println!("{sr} Hz: audio latency {:.1} samples ({:.2} ms)", a.latency(), a.latency() / sr * 1000.0);
        // Note to sound: first sample above -60 dB.
        let mut r = Rings::new(sr);
        r.note_on(57.0, 0.8);
        let (l, rr) = render(&mut r, &Params { model: Model::InharmonicString, ..Params::default() }, None, 0.05, sr);
        let first = l.iter().zip(&rr).position(|(a, b)| a.abs().max(b.abs()) > 1e-3);
        println!("  note to output: {first:?} samples");
    }
    let sr = 44100.0;
    for model in MODELS {
        for polyphony in [1u8, 4] {
            let p = Params { model, polyphony, ..Params::default() };
            let mut r = Rings::new(sr);
            r.note_on(57.0, 0.8);
            let t = std::time::Instant::now();
            let (l, rr) = render(&mut r, &p, None, 2.0, sr);
            let el = t.elapsed().as_secs_f32();
            println!(
                "{model:?} x{polyphony}: peak {:.3} rms {:.3}, 1 s in {:.1} ms",
                peak(&l, &rr),
                rms(&l).max(rms(&rr)),
                el * 500.0
            );
        }
    }
    for egg_fx in EGG_FX {
        let p = Params { easter_egg: true, egg_fx, ..Params::default() };
        let mut r = Rings::new(sr);
        r.note_on(57.0, 0.8);
        let (l, rr) = render(&mut r, &p, None, 2.0, sr);
        println!("egg {egg_fx:?}: peak {:.3} rms {:.3}", peak(&l, &rr), rms(&l).max(rms(&rr)));
    }
    for model in MODELS {
        let p = Params { model, ..Params::default() };
        let mut r = Rings::new(sr);
        r.note_on(57.0, 0.8);
        render(&mut r, &p, None, 0.5, sr);
        let t = time_to_silence(&mut r, &p, sr, 120.0);
        println!("{model:?}: silent after {t:?} s more");
    }
    set_ftz(false);
}
