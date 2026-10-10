use crate::*;

const MODES: [PlaybackMode; 4] =
    [PlaybackMode::Granular, PlaybackMode::Stretch, PlaybackMode::LoopingDelay, PlaybackMode::Spectral];

struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.0 >> 8) as f32 / (1u32 << 23) as f32 - 1.0
    }
}

/// Runs `input` through `clouds` in blocks of varying length.
fn run(clouds: &mut Clouds, p: &Params, input: &[(f32, f32)]) -> (Vec<f32>, Vec<f32>) {
    let mut l: Vec<f32> = input.iter().map(|x| x.0).collect();
    let mut r: Vec<f32> = input.iter().map(|x| x.1).collect();
    let sizes = [64usize, 97, 1, 256, 31, 128];
    let mut start = 0;
    let mut k = 0;
    while start < l.len() {
        let end = (start + sizes[k % sizes.len()]).min(l.len());
        clouds.process(p, &mut l[start..end], &mut r[start..end]);
        start = end;
        k += 1;
    }
    (l, r)
}

fn noise_and_sine(sr: f32, secs: f32, seed: u32) -> Vec<(f32, f32)> {
    let mut noise = Noise(seed);
    (0..(secs * sr) as usize)
        .map(|i| {
            let s = (2.0 * std::f32::consts::PI * 330.0 * i as f32 / sr).sin();
            (0.4 * s + 0.2 * noise.next(), 0.4 * s + 0.2 * noise.next())
        })
        .collect()
}

fn sine(sr: f32, secs: f32, hz: f32, amp: f32) -> Vec<(f32, f32)> {
    (0..(secs * sr) as usize)
        .map(|i| {
            let s = amp * (2.0 * std::f32::consts::PI * hz * i as f32 / sr).sin();
            (s, s)
        })
        .collect()
}

fn silence(sr: f32, secs: f32) -> Vec<(f32, f32)> {
    vec![(0.0, 0.0); (secs * sr) as usize]
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
}

fn peak(x: &[f32]) -> f32 {
    x.iter().fold(0.0f32, |m, v| m.max(v.abs()))
}

fn finite(x: &[f32]) -> bool {
    x.iter().all(|v| v.is_finite())
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

/// Settings under which each mode plays its buffer back plainly. POSITION
/// is low enough to stay within the test signal with the 4 to 7 s buffers of
/// the 8-bit qualities.
fn wet(mode: PlaybackMode) -> Params {
    Params {
        mode,
        dry_wet: 1.0,
        position: 0.15,
        density: if mode == PlaybackMode::Granular { 0.8 } else { 0.5 },
        ..Params::default()
    }
}

#[test]
fn clouds_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<Clouds>();
}

#[test]
fn every_mode_and_quality_sounds() {
    for sr in [48000.0f32, 44100.0] {
        for quality in 0..4u8 {
            for mode in MODES {
                let p = Params { quality, ..wet(mode) };
                let mut c = Clouds::new(sr);
                let (l, r) = run(&mut c, &p, &noise_and_sine(sr, 2.0, 7));
                let tail = (sr as usize)..l.len();
                let level = rms(&l[tail.clone()]).max(rms(&r[tail]));
                assert!(finite(&l) && finite(&r), "{sr} {quality} {mode:?}: not finite");
                assert!(level > 0.02, "{sr} {quality} {mode:?}: silent ({level})");
                let pk = peak(&l).max(peak(&r));
                assert!(pk < 4.0, "{sr} {quality} {mode:?}: peak {pk}");
                assert_eq!(c.underruns(), 0);
            }
        }
    }
}

/// With dry/wet at 0 the output is the input, delayed by `latency()`.
#[test]
fn dry_path_is_the_input() {
    for sr in [48000.0f32, 44100.0] {
        for mode in MODES {
            let mut c = Clouds::new(sr);
            let p = Params { mode, dry_wet: 0.0, ..Params::default() };
            let input: Vec<(f32, f32)> = (0..(sr as usize))
                .map(|i| {
                    let t = i as f32 / sr;
                    let a = 0.3 * (2.0 * std::f32::consts::PI * 440.0 * t).sin();
                    let b = 0.2 * (2.0 * std::f32::consts::PI * 2750.0 * t).sin();
                    (a + b, a - b)
                })
                .collect();
            let (l, r) = run(&mut c, &p, &input);
            // The exact delay is fractional; find it to 1/20 sample, with
            // linear interpolation of the input.
            let err = |lag: f32| {
                let n = lag.floor() as usize;
                let f = lag - n as f32;
                let delayed = |i: usize, ch: usize| {
                    let (a, b) = (input[i - n], input[i - n - 1]);
                    let (a, b) = if ch == 0 { (a.0, b.0) } else { (a.1, b.1) };
                    a + (b - a) * f
                };
                let (mut e, mut s) = (0.0f64, 0.0f64);
                for i in 4000..input.len() - 200 {
                    for (ch, out) in [&l, &r].into_iter().enumerate() {
                        let x = delayed(i, ch) as f64;
                        e += (out[i] as f64 - x).powi(2);
                        s += x * x;
                    }
                }
                (e / s).sqrt()
            };
            let lat = c.latency() as f32;
            let best = (0..=200)
                .map(|k| lat - 5.0 + k as f32 * 0.05)
                .min_by(|&a, &b| err(a).partial_cmp(&err(b)).unwrap())
                .unwrap();
            let exact = 32.0 + 67.0 * sr / 32000.0;
            assert!((best - exact).abs() < 0.2, "{sr} {mode:?}: delay {best}, expected {exact}");
            assert!((lat - exact).abs() <= 0.5, "{sr}: latency() {lat}, exact {exact}");
            let e = err(best);
            assert!(e < 0.03, "{sr} {mode:?}: relative error {e}");
        }
    }
}

#[test]
fn freeze_holds_the_buffer() {
    let sr = 48000.0;
    let mut c = Clouds::new(sr);
    let p = wet(PlaybackMode::Granular);
    run(&mut c, &p, &noise_and_sine(sr, 1.5, 3));
    let frozen = Params { freeze: true, ..p };
    let (l, r) = run(&mut c, &frozen, &silence(sr, 3.0));
    let tail = &l[(2.0 * sr) as usize..];
    assert!(rms(tail) > 0.05 && rms(&r[(2.0 * sr) as usize..]) > 0.05, "frozen buffer is silent ({})", rms(tail));

    // Releasing freeze lets the silence in.
    let (l, _) = run(&mut c, &p, &silence(sr, 3.0));
    assert!(rms(&l[(2.5 * sr) as usize..]) < 1e-3);

    // The looping delay loops when frozen.
    let mut c = Clouds::new(sr);
    let p = wet(PlaybackMode::LoopingDelay);
    run(&mut c, &p, &noise_and_sine(sr, 1.5, 3));
    let (l, _) = run(&mut c, &Params { freeze: true, ..p }, &silence(sr, 3.0));
    assert!(rms(&l[(2.0 * sr) as usize..]) > 0.05);
}

#[test]
fn stretch_shifts_up_an_octave() {
    for (sr, quality) in [(48000.0f32, 0u8), (44100.0, 0), (48000.0, 1), (48000.0, 2)] {
        let mut c = Clouds::new(sr);
        let p = Params { pitch: 12.0, quality, density: 0.0, ..wet(PlaybackMode::Stretch) };
        let (l, _) = run(&mut c, &p, &sine(sr, 2.0, 220.0, 0.5));
        let tail = &l[sr as usize..];
        assert!(rms(tail) > 0.05, "{sr} q{quality}: silent");
        let f = pitch_hz(tail, sr, 300.0, 1000.0);
        assert!((f - 440.0).abs() < 440.0 * 0.02, "{sr} q{quality}: {f} Hz");
        let zc = zero_crossing_hz(tail, sr);
        assert!((zc - 440.0).abs() < 440.0 * 0.03, "{sr} q{quality}: {zc} Hz by zero crossings");
    }
    // Down a fifth too.
    let sr = 48000.0;
    let mut c = Clouds::new(sr);
    let p = Params { pitch: -7.0, density: 0.0, ..wet(PlaybackMode::Stretch) };
    let (l, _) = run(&mut c, &p, &sine(sr, 2.0, 440.0, 0.5));
    let f = pitch_hz(&l[sr as usize..], sr, 200.0, 600.0);
    let expected = 440.0 * 2f32.powf(-7.0 / 12.0);
    assert!((f - expected).abs() < expected * 0.02, "{f} Hz");
}

#[test]
fn pitch_in_granular_and_looping_modes() {
    let sr = 48000.0;
    for mode in [PlaybackMode::Granular, PlaybackMode::LoopingDelay] {
        let mut c = Clouds::new(sr);
        // POSITION 0 for the delay: while its time glides to a new value
        // (as slowly as on the module), the pitch bends. SIZE 1 for its
        // pitch shifter: the shorter its window, the more the crossfades
        // between its two taps pull the frequency (as on the module).
        let looping = mode == PlaybackMode::LoopingDelay;
        let p = Params {
            pitch: 12.0,
            size: if looping { 1.0 } else { 0.6 },
            texture: 0.5,
            position: if mode == PlaybackMode::Granular { 0.15 } else { 0.0 },
            density: if mode == PlaybackMode::Granular { 0.9 } else { 0.0 },
            ..wet(mode)
        };
        let (l, _) = run(&mut c, &p, &sine(sr, 2.5, 220.0, 0.5));
        let f = pitch_hz(&l[(1.5 * sr) as usize..], sr, 300.0, 1000.0);
        assert!((f - 440.0).abs() < 440.0 * 0.03, "{mode:?}: {f} Hz");
    }
}

#[test]
fn silence_decays() {
    let sr = 48000.0;
    for quality in [0u8, 3] {
        for mode in MODES {
            let mut c = Clouds::new(sr);
            let p = Params { quality, reverb: 0.5, ..wet(mode) };
            run(&mut c, &p, &noise_and_sine(sr, 1.0, 11));
            let secs = if quality == 3 { 12.0 } else { 5.0 };
            let (l, r) = run(&mut c, &p, &silence(sr, secs));
            let tail = (l.len() - sr as usize / 2)..l.len();
            let level = rms(&l[tail.clone()]).max(rms(&r[tail]));
            assert!(level < 1e-3, "q{quality} {mode:?}: {level}");
        }
    }
}

#[test]
fn trigger_seeds_grains_and_acts_on_edges() {
    let sr = 48000.0;
    let mut c = Clouds::new(sr);
    // DENSITY at noon: no grains on their own.
    let p = Params { density: 0.5, ..wet(PlaybackMode::Granular) };
    run(&mut c, &p, &noise_and_sine(sr, 1.2, 5));
    let input = noise_and_sine(sr, 0.5, 6);
    let (quiet, _) = run(&mut c, &p, &input);
    assert!(rms(&quiet) < 1e-3, "grains without trigger ({})", rms(&quiet));

    // A gate held high gives one grain.
    let held = Params { trigger: true, ..p };
    let (l, _) = run(&mut c, &held, &input);
    let first = rms(&l[..(0.25 * sr) as usize]);
    let later = rms(&l[(0.3 * sr) as usize..]);
    assert!(first > 0.01, "no grain ({first})");
    assert!(later < 1e-3, "the held gate keeps triggering ({later})");
}

#[test]
fn extreme_parameters_stay_finite() {
    let values = [0.0f32, 1.0, -1.0, 2.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.999_999, 1e-30];
    let pitches = [-48.0f32, 48.0, -1000.0, 1000.0, f32::NAN, 0.0, 47.9];
    let mut noise = Noise(99);
    for sr in [48000.0f32, 44100.0, 22050.0, 96000.0, f32::NAN, 0.0] {
        let mut c = Clouds::new(sr);
        let mut k = 0usize;
        for round in 0..96 {
            let kk = k;
            let v = |j: usize| values[(kk * 7 + j * 3) % values.len()];
            k += 1;
            let p = Params {
                mode: MODES[round % 4],
                quality: (round / 4 % 6) as u8,
                position: v(0),
                size: v(1),
                pitch: pitches[round % pitches.len()],
                density: v(2),
                texture: v(3),
                dry_wet: v(4),
                stereo_spread: v(5),
                feedback: if round % 3 == 0 { 1.0 } else { v(6) },
                reverb: if round % 2 == 0 { 1.0 } else { v(7) },
                freeze: round % 5 < 2,
                trigger: round % 3 == 1,
            };
            let mut l: Vec<f32> = (0..2400).map(|_| 3.0 * noise.next()).collect();
            let mut r: Vec<f32> = (0..2399).map(|i| if i % 500 == 0 { f32::NAN } else { noise.next() }).collect();
            c.process(&p, &mut l, &mut r);
            assert!(finite(&l[..2399]) && finite(&r), "round {round} at {sr}: not finite");
            assert!(peak(&l).max(peak(&r)) < 4.0, "round {round} at {sr}: too loud");
        }
        // Empty and mismatched buffers.
        c.process(&Params::default(), &mut [], &mut []);
        c.process(&Params::default(), &mut [0.0; 3], &mut []);
    }
}

#[test]
fn long_runs_do_not_drift() {
    for sr in [44100.0f32, 48000.0, 96000.0, 22050.0, 32000.0, 88200.0] {
        let mut c = Clouds::new(sr);
        let p = Params { dry_wet: 0.0, ..Params::default() };
        let mut l = vec![0.0f32; 1000];
        let mut r = vec![0.0f32; 1000];
        let mut min_fill = usize::MAX;
        let mut max_fill = 0;
        for _ in 0..(60.0 * sr / 1000.0) as usize {
            c.process(&p, &mut l, &mut r);
            min_fill = min_fill.min(c.fifo_len());
            max_fill = max_fill.max(c.fifo_len());
        }
        assert_eq!(c.underruns(), 0, "{sr}");
        assert!(max_fill <= 2 * MAX_BLOCK_SIZE + 4, "{sr}: queue grows to {max_fill}");
        assert!(min_fill < MAX_BLOCK_SIZE, "{sr}");
    }
}

#[test]
fn mode_and_quality_changes_are_clean() {
    let sr = 48000.0;
    let mut c = Clouds::new(sr);
    let input = noise_and_sine(sr, 0.25, 21);
    for step in 0..24 {
        let p = Params { mode: MODES[step % 4], quality: (step / 4 % 4) as u8, ..wet(MODES[step % 4]) };
        let (l, r) = run(&mut c, &p, &input);
        assert!(finite(&l) && finite(&r) && peak(&l).max(peak(&r)) < 4.0, "step {step}");
    }
}

fn zero_crossing_hz(x: &[f32], sr: f32) -> f32 {
    let mut n = 0;
    for i in 1..x.len() {
        if x[i - 1] < 0.0 && x[i] >= 0.0 {
            n += 1;
        }
    }
    n as f32 * sr / x.len() as f32
}
