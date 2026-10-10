use crate::{constructor, create, Effect, EffectInfo, EFFECTS};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

/// Counts allocations per thread, so a test can check that `process` makes none.
struct CountingAlloc;

thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
    /// How often the finite-output guard had to reset an effect on this thread.
    pub(crate) static GUARD_TRIPS: Cell<usize> = const { Cell::new(0) };
}

fn guard_trips() -> usize {
    GUARD_TRIPS.with(|c| c.get())
}

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = ALLOCATIONS.try_with(|c| c.set(c.get() + 1));
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let _ = ALLOCATIONS.try_with(|c| c.set(c.get() + 1));
        System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

fn allocations() -> usize {
    ALLOCATIONS.with(|c| c.get())
}

const RATES: [f32; 2] = [44100.0, 96000.0];
const CATEGORIES: [&str; 8] = ["Reverb", "Tape", "Saturation", "Dynamics", "EQ", "Lo-Fi", "Delay", "Other"];

/// Deterministic white noise in -1..1.
struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.0 >> 8) as f32 / (1u32 << 23) as f32 - 1.0
    }
}

/// A 440 Hz sine at half scale plus a little noise, slightly different per channel.
fn test_signal(sample_rate: f32, seconds: f32) -> (Vec<f32>, Vec<f32>) {
    let n = (sample_rate * seconds) as usize;
    let mut noise = Noise(12345);
    let mut l = Vec::with_capacity(n);
    let mut r = Vec::with_capacity(n);
    for i in 0..n {
        let s = (std::f32::consts::TAU * 440.0 * i as f32 / sample_rate).sin() * 0.5;
        l.push(s + noise.next() * 0.05);
        r.push(s * 0.9 + noise.next() * 0.05);
    }
    (l, r)
}

fn defaults(info: &EffectInfo) -> Vec<f32> {
    info.params.iter().map(|p| p.default).collect()
}

/// Runs `l`/`r` through the effect in place, in blocks of varying length.
fn run(effect: &mut dyn Effect, params: &[f32], l: &mut [f32], r: &mut [f32]) {
    const BLOCKS: [usize; 6] = [256, 1, 64, 7, 512, 1000];
    let mut start = 0;
    let mut i = 0;
    while start < l.len() {
        let end = (start + BLOCKS[i % BLOCKS.len()]).min(l.len());
        effect.process(params, &mut l[start..end], &mut r[start..end]);
        start = end;
        i += 1;
    }
}

fn peak(x: &[f32]) -> f32 {
    x.iter().fold(0.0f32, |m, v| m.max(v.abs()))
}

fn all_finite(x: &[f32]) -> bool {
    x.iter().all(|v| v.is_finite())
}

#[test]
fn every_effect_is_listed_and_can_be_created() {
    assert!(EFFECTS.len() >= 14);
    for (i, info) in EFFECTS.iter().enumerate() {
        assert!(EFFECTS[..i].iter().all(|other| other.id != info.id), "duplicate id {}", info.id);
        assert!(CATEGORIES.contains(&info.category), "{}: bad category {}", info.id, info.category);
        assert!(!info.name.is_empty() && !info.description.is_empty(), "{}", info.id);
        assert!(!info.params.is_empty(), "{}", info.id);
        for p in info.params {
            assert!((0.0..=1.0).contains(&p.default), "{}: {} default {}", info.id, p.name, p.default);
            assert!(!p.name.is_empty());
        }
        for &sr in &RATES {
            assert!(create(info.id, sr).is_some(), "{} at {sr}", info.id);
        }
    }
    assert!(create("NoSuchEffect", 44100.0).is_none());
    assert!(create("", 44100.0).is_none());
}

#[test]
fn param_counts_match_what_process_reads() {
    for info in EFFECTS {
        let (_, kernel_params) = constructor(info.id).unwrap_or_else(|| panic!("{} has no constructor", info.id));
        assert_eq!(info.params.len(), kernel_params, "{}", info.id);
        assert!(kernel_params <= crate::MAX_PARAMS, "{}", info.id);
    }
}

#[test]
fn defaults_give_finite_audible_output() {
    for info in EFFECTS {
        for &sr in &RATES {
            let mut fx = create(info.id, sr).unwrap();
            let (mut l, mut r) = test_signal(sr, 1.0);
            run(fx.as_mut(), &defaults(info), &mut l, &mut r);
            assert!(all_finite(&l) && all_finite(&r), "{} at {sr}: not finite", info.id);
            let pk = peak(&l).max(peak(&r));
            assert!(pk > 0.01, "{} at {sr}: silent (peak {pk})", info.id);
            assert!(pk < 4.0, "{} at {sr}: too loud (peak {pk})", info.id);
        }
    }
    assert_eq!(guard_trips(), 0, "the finite-output guard fired");
}

#[test]
fn extreme_params_stay_finite() {
    for info in EFFECTS {
        for &sr in &RATES {
            for value in [0.0f32, 1.0] {
                let params = vec![value; info.params.len()];
                let mut fx = create(info.id, sr).unwrap();
                let (mut l, mut r) = test_signal(sr, 2.0);
                run(fx.as_mut(), &params, &mut l, &mut r);
                assert!(all_finite(&l) && all_finite(&r), "{} at {sr}, all params {value}", info.id);
                let t0 = guard_trips();
                // and a +18 dB signal on top
                let (mut l, mut r) = test_signal(sr, 0.5);
                l.iter_mut().chain(r.iter_mut()).for_each(|x| *x *= 8.0);
                run(fx.as_mut(), &params, &mut l, &mut r);
                assert!(all_finite(&l) && all_finite(&r), "{} at {sr}, all params {value}, loud", info.id);
                assert_eq!(t0, 0, "{} at {sr}, all params {value}: the finite-output guard fired", info.id);
                assert_eq!(guard_trips(), 0, "{} at {sr}, all params {value}: the finite-output guard fired on +18 dB input", info.id);
            }
        }
    }
}

#[test]
fn random_param_changes_stay_finite() {
    for info in EFFECTS {
        let sr = 48000.0;
        let mut fx = create(info.id, sr).unwrap();
        let mut noise = Noise(777);
        let (mut l, mut r) = test_signal(sr, 3.0);
        for (bl, br) in l.chunks_mut(128).zip(r.chunks_mut(128)) {
            let params: Vec<f32> = info.params.iter().map(|_| noise.next() * 0.5 + 0.5).collect();
            fx.process(&params, bl, br);
        }
        assert!(all_finite(&l) && all_finite(&r), "{}", info.id);
        assert_eq!(guard_trips(), 0, "{}: the finite-output guard fired", info.id);
    }
}

#[test]
fn tails_decay_to_near_silence() {
    for info in EFFECTS {
        for &sr in &RATES {
            let mut fx = create(info.id, sr).unwrap();
            let params = defaults(info);
            let (mut l, mut r) = test_signal(sr, 1.0);
            run(fx.as_mut(), &params, &mut l, &mut r);
            // 60 s of silence: Galactic at full size has the longest tail
            let block = sr as usize;
            let mut l = vec![0.0f32; block];
            let mut r = vec![0.0f32; block];
            for _ in 0..60 {
                l.fill(0.0);
                r.fill(0.0);
                run(fx.as_mut(), &params, &mut l, &mut r);
            }
            assert!(all_finite(&l) && all_finite(&r), "{} at {sr}", info.id);
            let pk = peak(&l).max(peak(&r));
            assert!(pk < 1e-4, "{} at {sr}: tail still at {pk}", info.id);
        }
    }
}

#[test]
fn silence_in_gives_near_silence_out() {
    for info in EFFECTS {
        let mut fx = create(info.id, 44100.0).unwrap();
        let mut l = vec![0.0f32; 44100];
        let mut r = vec![0.0f32; 44100];
        run(fx.as_mut(), &defaults(info), &mut l, &mut r);
        let pk = peak(&l).max(peak(&r));
        assert!(pk < 1e-5, "{}: {pk}", info.id);
    }
}

#[test]
fn bad_input_and_odd_calls_do_not_break() {
    for info in EFFECTS {
        let mut fx = create(info.id, 44100.0).unwrap();
        // empty blocks, mismatched lengths, missing/extra/NaN params
        fx.process(&[], &mut [], &mut []);
        let mut l = vec![0.25f32; 100];
        let mut r = vec![0.25f32; 37];
        fx.process(&[], &mut l, &mut r);
        let mut too_many = defaults(info);
        too_many.extend([0.3, 0.7, 2.0, -1.0]);
        fx.process(&too_many, &mut l, &mut r);
        let weird = vec![f32::NAN, f32::INFINITY, -5.0, 9.0, f32::NEG_INFINITY, 0.5, 0.5, 0.5];
        fx.process(&weird[..info.params.len().min(weird.len())], &mut l, &mut r);
        assert!(all_finite(&l) && all_finite(&r), "{}", info.id);

        // non-finite and huge samples
        let mut l = vec![f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1e30, -1e30, f32::MAX, 0.5, -0.5];
        let mut r = vec![1e20, f32::NAN, 0.0, f32::MIN, 3.0, -3.0, 1e-40, -1e-40];
        for _ in 0..50 {
            let (mut bl, mut br) = (l.clone(), r.clone());
            fx.process(&defaults(info), &mut bl, &mut br);
            assert!(all_finite(&bl) && all_finite(&br), "{}: {bl:?} {br:?}", info.id);
        }
        // and it still works afterwards
        let (mut sl, mut sr) = test_signal(44100.0, 0.5);
        run(fx.as_mut(), &defaults(info), &mut sl, &mut sr);
        assert!(all_finite(&sl) && all_finite(&sr), "{}", info.id);
        l.clear();
        r.clear();
    }
}

#[test]
fn odd_sample_rates_work() {
    for info in EFFECTS {
        for sr in [22050.0f32, 32000.0, 48000.0, 88200.0, 176400.0, 192000.0, 384000.0, 8000.0, 1e7, 0.0, f32::NAN] {
            let mut fx = create(info.id, sr).unwrap();
            let shown = if sr.is_finite() { sr.clamp(22050.0, 192000.0) } else { 44100.0 };
            let (mut l, mut r) = test_signal(shown, 0.5);
            run(fx.as_mut(), &vec![1.0; info.params.len()], &mut l, &mut r);
            run(fx.as_mut(), &defaults(info), &mut l, &mut r);
            assert!(all_finite(&l) && all_finite(&r), "{} at {sr}", info.id);
            assert_eq!(guard_trips(), 0, "{} at {sr}: the finite-output guard fired", info.id);
        }
    }
}

#[test]
fn process_does_not_allocate() {
    for info in EFFECTS {
        let mut fx = create(info.id, 44100.0).unwrap();
        let (mut l, mut r) = test_signal(44100.0, 0.25);
        let params = defaults(info);
        let before = allocations();
        run(fx.as_mut(), &params, &mut l, &mut r);
        fx.process(&[], &mut l, &mut r);
        let ones = [1.0f32; 8];
        fx.process(&ones[..info.params.len()], &mut l, &mut r);
        // the non-finite path resets state, which must not allocate either
        l[0] = f32::NAN;
        r[3] = f32::INFINITY;
        fx.process(&params, &mut l, &mut r);
        assert_eq!(allocations(), before, "{} allocated in process", info.id);
    }
}

#[test]
fn effects_actually_change_the_sound() {
    // with some settings that should audibly do something, the output differs from the input
    for info in EFFECTS {
        let params: Vec<f32> = info.params.iter().map(|p| if p.default < 0.5 { 0.8 } else { 0.3 }).collect();
        let mut fx = create(info.id, 44100.0).unwrap();
        // long enough for TapeDelay2's 1.7 s echo at Time 0.3
        let (l0, r0) = test_signal(44100.0, 2.5);
        let (mut l, mut r) = (l0.clone(), r0.clone());
        run(fx.as_mut(), &params, &mut l, &mut r);
        let diff = l.iter().zip(&l0).chain(r.iter().zip(&r0)).fold(0.0f32, |m, (a, b)| m.max((a - b).abs()));
        assert!(diff > 1e-3, "{} with {params:?} is a no-op", info.id);
    }
}

