use super::*;
use std::f32::consts::TAU;

const SR: f32 = 48000.0;

fn sine(n: usize, hz: f32, amp: f32) -> (Vec<f32>, Vec<f32>) {
    let l: Vec<f32> = (0..n).map(|i| (TAU * hz * i as f32 / SR).sin() * amp).collect();
    (l.clone(), l)
}

fn run(b: &mut Beads, p: &Params, l: &mut [f32], r: &mut [f32]) {
    for (a, c) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
        b.process(p, a, c);
    }
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
}

fn mag(x: &[f32], hz: f32) -> f32 {
    let w = TAU as f64 * hz as f64 / SR as f64;
    let n = x.len() as f64;
    let (mut re, mut im) = (0.0f64, 0.0f64);
    for (i, &s) in x.iter().enumerate() {
        let win = 0.5 - 0.5 * (TAU as f64 * i as f64 / n).cos();
        re += s as f64 * win * (w * i as f64).cos();
        im += s as f64 * win * (w * i as f64).sin();
    }
    ((re * re + im * im).sqrt() / n) as f32
}

#[test]
fn dry_comes_through_at_unity_after_the_latency() {
    for quality in [Media::Digital, Media::VintageDigital, Media::CleanTape] {
        let mut b = Beads::new(SR);
        let p = Params { dry_wet: 0.0, reverb: 0.0, quality, ..Params::default() };
        let (mut l, mut r) = sine(48000, 440.0, 0.3);
        run(&mut b, &p, &mut l, &mut r);
        let level = rms(&l[24000..]);
        assert!((level / (0.3 / 2f32.sqrt()) - 1.0).abs() < 0.15, "{quality:?}: {level}");
    }
}

#[test]
fn every_quality_makes_grains_and_stays_bounded() {
    for quality in [Media::DirtyTape, Media::CleanTape, Media::VintageDigital, Media::Digital] {
        for mono in [false, true] {
            let mut b = Beads::new(SR);
            let p = Params { dry_wet: 1.0, quality, mono, density: 0.75, ..Params::default() };
            let (mut l, mut r) = sine(48000 * 3, 330.0, 0.5);
            run(&mut b, &p, &mut l, &mut r);
            assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() <= 1.5), "{quality:?}");
            let level = rms(&l[48000..]);
            assert!(level > 0.02, "{quality:?} mono {mono}: {level}");
        }
    }
}

#[test]
fn pitch_shifts_the_grains() {
    let mut b = Beads::new(SR);
    let p = Params { dry_wet: 1.0, reverb: 0.0, pitch: 12.0, size: 0.6, density: 0.3, ..Params::default() };
    let (mut l, mut r) = sine(48000 * 3, 300.0, 0.5);
    run(&mut b, &p, &mut l, &mut r);
    let tail = &l[48000 * 2..];
    assert!(mag(tail, 600.0) > 3.0 * mag(tail, 300.0), "{} vs {}", mag(tail, 600.0), mag(tail, 300.0));
}

#[test]
fn no_grains_without_the_button_at_noon() {
    let mut b = Beads::new(SR);
    let p = Params { dry_wet: 1.0, reverb: 0.0, density: 0.5, seed: Seed::Button(false), ..Params::default() };
    let (mut l, mut r) = sine(48000 * 2, 300.0, 0.5);
    run(&mut b, &p, &mut l, &mut r);
    assert!(rms(&l[48000..]) < 1e-3, "{}", rms(&l[48000..]));
    // Holding the button: grains.
    let p = Params { seed: Seed::Button(true), density: 0.3, ..p };
    let (mut l, mut r) = sine(48000 * 2, 300.0, 0.5);
    run(&mut b, &p, &mut l, &mut r);
    assert!(rms(&l[24000..]) > 0.02);
}

#[test]
fn size_fully_up_is_a_delay() {
    let mut b = Beads::new(SR);
    // A click every second, a delay of a quarter of the longest time.
    let p = Params { dry_wet: 1.0, reverb: 0.0, size: 1.0, density: 0.35, time: 0.0, ..Params::default() };
    let n = 48000 * 4;
    let (mut l, mut r): (Vec<f32>, Vec<f32>) = ((0..n).map(|i| if i % 48000 < 48 { 0.8 } else { 0.0 }).collect(), (0..n).map(|i| if i % 48000 < 48 { 0.8 } else { 0.0 }).collect());
    run(&mut b, &p, &mut l, &mut r);
    assert!(b.status().delay_mode);
    // Echoes between the clicks.
    let between = &l[48000 * 3 + 2000..48000 * 3 + 46000];
    assert!(between.iter().any(|v| v.abs() > 0.05), "no echo");
}

#[test]
fn the_wavetables_play_at_the_pitch() {
    let mut b = Beads::new(SR);
    let p = Params { source: Source::Wavetables, dry_wet: 0.0, reverb: 0.0, pitch: 0.0, ..Params::default() };
    let (mut l, mut r) = (vec![0.0f32; 48000], vec![0.0f32; 48000]);
    run(&mut b, &p, &mut l, &mut r);
    let tail = &l[12000..];
    assert!(rms(tail) > 0.02, "{}", rms(tail));
    // C3, 130.81 Hz.
    assert!(mag(tail, 130.81) > 3.0 * mag(tail, 150.0), "{} {}", mag(tail, 130.81), mag(tail, 150.0));
}

#[test]
fn a_clock_seeds_grains_in_time() {
    let mut b = Beads::new(SR);
    // 120 bpm, a pulse each beat, every pulse a grain (density far left
    // divides by 1).
    let mut p = Params { dry_wet: 1.0, reverb: 0.0, density: 0.0, size: 0.2, ..Params::default() };
    let bps = 2.0 / SR as f64;
    let (mut l, mut r) = sine(48000 * 3, 300.0, 0.5);
    for (k, (a, c)) in l.chunks_mut(480).zip(r.chunks_mut(480)).enumerate() {
        p.clock = Some(Clock { beat: k as f64 * 480.0 * bps, beats_per_sample: bps, division: 1.0 });
        b.process(&p, a, c);
    }
    assert!(rms(&l[48000..]) > 0.01, "{}", rms(&l[48000..]));
}
