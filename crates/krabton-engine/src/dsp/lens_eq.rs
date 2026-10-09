//! Lens EQ: a parametric EQ in the spirit of the big interactive EQs. Twelve
//! bands (bell, shelves, tilt, cuts up to 96 dB/oct, notch, band pass), each
//! on stereo, left, right, mid or side, each optionally dynamic: the band's
//! gain moves by up to its range as the level in its frequency region goes
//! over its threshold. The filters follow their analog shapes up to Nyquist
//! (see `biquad`), and the UI draws the curve from the same designs.

use super::biquad::{self, Cascade, Coefs, State};
use crate::params::{ParamSpec, Unit};
use crate::scope::DeviceTaps;

pub const BANDS: usize = 12;
pub const PER_BAND: usize = 9;

pub const SHAPE: usize = 0;
pub const FREQ: usize = 1;
pub const GAIN: usize = 2;
pub const Q: usize = 3;
pub const SLOPE: usize = 4;
pub const STEREO: usize = 5;
pub const ACTIVE: usize = 6;
pub const DYN_RANGE: usize = 7;
pub const DYN_THRESH: usize = 8;

pub const OUTPUT: usize = BANDS * PER_BAND;
pub const GAIN_SCALE: usize = OUTPUT + 1;
pub const ANALYZER: usize = OUTPUT + 2;
pub const DISPLAY: usize = OUTPUT + 3;

/// Parameter index of field `k` of band `b`.
pub const fn idx(b: usize, k: usize) -> usize {
    b * PER_BAND + k
}

pub const SHAPES: &[&str] = &["Off", "Bell", "Low Shelf", "High Shelf", "Low Cut", "High Cut", "Notch", "Band Pass", "Tilt Shelf"];
pub const SLOPES: &[&str] = &["6 dB/oct", "12 dB/oct", "18 dB/oct", "24 dB/oct", "30 dB/oct", "36 dB/oct", "48 dB/oct", "72 dB/oct", "96 dB/oct"];
const SLOPE_ORDER: [usize; 9] = [1, 2, 3, 4, 5, 6, 8, 12, 16];
pub const STEREO_MODES: &[&str] = &["Stereo", "Left", "Right", "Mid", "Side"];
pub const ANALYZER_MODES: &[&str] = &["Off", "Pre", "Post", "Both"];
pub const DISPLAY_RANGES: &[&str] = &["3 dB", "6 dB", "12 dB", "30 dB"];
pub const DISPLAY_DB: [f32; 4] = [3.0, 6.0, 12.0, 30.0];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shape {
    Off,
    Bell,
    LowShelf,
    HighShelf,
    LowCut,
    HighCut,
    Notch,
    BandPass,
    Tilt,
}

impl Shape {
    pub fn from_param(v: f32) -> Self {
        match v.round() as i32 {
            1 => Shape::Bell,
            2 => Shape::LowShelf,
            3 => Shape::HighShelf,
            4 => Shape::LowCut,
            5 => Shape::HighCut,
            6 => Shape::Notch,
            7 => Shape::BandPass,
            8 => Shape::Tilt,
            _ => Shape::Off,
        }
    }

    /// Whether the band has a gain (and can be dynamic).
    pub fn has_gain(self) -> bool {
        matches!(self, Shape::Bell | Shape::LowShelf | Shape::HighShelf | Shape::Tilt)
    }

    /// Whether the slope chooser applies.
    pub fn has_slope(self) -> bool {
        matches!(self, Shape::LowCut | Shape::HighCut)
    }
}

macro_rules! band {
    ($n:literal) => {
        [
            ParamSpec::choice(concat!("Shape ", $n), concat!("Band ", $n), SHAPES, 0),
            ParamSpec::log(concat!("Freq ", $n), concat!("Band ", $n), 10.0, 30000.0, 1000.0, Unit::Hz),
            ParamSpec::lin(concat!("Gain ", $n), concat!("Band ", $n), -30.0, 30.0, 0.0, Unit::Db),
            ParamSpec::log(concat!("Q ", $n), concat!("Band ", $n), 0.025, 40.0, 1.0, Unit::Ratio),
            ParamSpec::choice(concat!("Slope ", $n), concat!("Band ", $n), SLOPES, 3),
            ParamSpec::choice(concat!("Stereo ", $n), concat!("Band ", $n), STEREO_MODES, 0),
            ParamSpec::toggle(concat!("Active ", $n), concat!("Band ", $n), true),
            ParamSpec::lin(concat!("Dyn Range ", $n), concat!("Band ", $n), -30.0, 30.0, 0.0, Unit::Db),
            ParamSpec::lin(concat!("Threshold ", $n), concat!("Band ", $n), -60.0, 0.0, -30.0, Unit::Db),
        ]
    };
}

const BAND_SPECS: [[ParamSpec; PER_BAND]; BANDS] = [band!(1), band!(2), band!(3), band!(4), band!(5), band!(6), band!(7), band!(8), band!(9), band!(10), band!(11), band!(12)];

const fn all_specs() -> [ParamSpec; BANDS * PER_BAND + 4] {
    let mut out = [ParamSpec::lin("", "", 0.0, 1.0, 0.0, Unit::None); BANDS * PER_BAND + 4];
    let mut b = 0;
    while b < BANDS {
        let mut k = 0;
        while k < PER_BAND {
            out[b * PER_BAND + k] = BAND_SPECS[b][k];
            k += 1;
        }
        b += 1;
    }
    out[OUTPUT] = ParamSpec::lin("Output", "Global", -36.0, 36.0, 0.0, Unit::Db);
    out[GAIN_SCALE] = ParamSpec::lin("Gain Scale", "Global", 0.0, 2.0, 1.0, Unit::Percent);
    out[ANALYZER] = ParamSpec::choice("Analyzer", "Global", ANALYZER_MODES, 3);
    out[DISPLAY] = ParamSpec::choice("Display", "Global", DISPLAY_RANGES, 2);
    out
}

static SPECS_ARRAY: [ParamSpec; BANDS * PER_BAND + 4] = all_specs();
pub static SPECS: &[ParamSpec] = &SPECS_ARRAY;

/// The filter of one band at the given settings.
pub fn design(shape: Shape, f: f64, gain_db: f64, q: f64, slope: usize, sr: f64) -> Cascade {
    match shape {
        Shape::Off => Cascade::NONE,
        Shape::Bell => Cascade::one(biquad::bell(f, sr, q, gain_db)),
        Shape::LowShelf => Cascade::one(biquad::low_shelf(f, sr, q, gain_db)),
        Shape::HighShelf => Cascade::one(biquad::high_shelf(f, sr, q, gain_db)),
        Shape::Tilt => Cascade::one(biquad::tilt(f, sr, q, gain_db)),
        Shape::LowCut => biquad::butterworth(SLOPE_ORDER[slope.min(8)], true, f, sr, q),
        Shape::HighCut => biquad::butterworth(SLOPE_ORDER[slope.min(8)], false, f, sr, q),
        Shape::Notch => Cascade::one(biquad::notch(f, sr, q)),
        Shape::BandPass => Cascade::one(biquad::bandpass(f, sr, q)),
    }
}

/// Settings of band `b` from the parameters: (shape, freq, gain, q, slope,
/// stereo mode), with the global gain scale applied. `None` when the band is
/// off or bypassed.
pub fn band_settings(p: &[f32], b: usize) -> Option<(Shape, f64, f64, f64, usize, usize)> {
    if p.len() < SPECS.len() {
        return None;
    }
    let shape = Shape::from_param(p[idx(b, SHAPE)]);
    if shape == Shape::Off || p[idx(b, ACTIVE)] < 0.5 {
        return None;
    }
    let scale = p[GAIN_SCALE].clamp(0.0, 2.0) as f64;
    Some((
        shape,
        p[idx(b, FREQ)].clamp(10.0, 30000.0) as f64,
        p[idx(b, GAIN)].clamp(-30.0, 30.0) as f64 * scale,
        p[idx(b, Q)].clamp(0.025, 40.0) as f64,
        p[idx(b, SLOPE)].round().clamp(0.0, 8.0) as usize,
        p[idx(b, STEREO)].round().clamp(0.0, 4.0) as usize,
    ))
}

/// Response of band `b` in dB at `hz`, with `dyn_db` added to its gain.
pub fn band_response_db(p: &[f32], b: usize, hz: f64, sr: f64, dyn_db: f64) -> f64 {
    match band_settings(p, b) {
        Some((shape, f, g, q, slope, _)) => design(shape, f, g + if shape.has_gain() { dyn_db } else { 0.0 }, q, slope, sr).mag_db(2.0 * std::f64::consts::PI * hz / sr),
        None => 0.0,
    }
}

const BLOCK: usize = 32;

#[derive(Clone, Copy)]
struct BandRt {
    /// Smoothed (ln freq, gain dB, ln q) and what the filter was made for.
    cur: [f64; 3],
    made: Option<(Shape, usize, [f64; 3])>,
    filter: Cascade,
    state: [[State; 8]; 2],
    /// Dynamic EQ: detector filter, envelope (power) and the applied gain.
    detector: Coefs,
    det_state: State,
    env: f64,
    dyn_db: f64,
}

impl Default for BandRt {
    fn default() -> Self {
        Self {
            cur: [0.0; 3],
            made: None,
            filter: Cascade::NONE,
            state: [[State::default(); 8]; 2],
            detector: Coefs::IDENTITY,
            det_state: State::default(),
            env: 0.0,
            dyn_db: 0.0,
        }
    }
}

pub struct LensEqState {
    bands: [BandRt; BANDS],
    out_gain: f64,
    pub taps: DeviceTaps,
}

impl Default for LensEqState {
    fn default() -> Self {
        Self { bands: [BandRt::default(); BANDS], out_gain: 1.0, taps: DeviceTaps::full() }
    }
}

#[inline]
fn run_cascade(f: &Cascade, st: &mut [State; 8], x: f64) -> f64 {
    let mut y = x;
    for (c, s) in f.sections[..f.n].iter().zip(st.iter_mut()) {
        y = s.run(c, y);
    }
    y
}

impl LensEqState {
    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], sr: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        let sr = sr as f64;
        let n = l.len().min(r.len());
        let smooth = 1.0 - (-(BLOCK as f64) / (0.02 * sr)).exp();
        let att = 1.0 - (-1.0 / (0.003 * sr)).exp();
        let rel = 1.0 - (-1.0 / (0.08 * sr)).exp();
        let target_out = 10f64.powf(p[OUTPUT].clamp(-36.0, 36.0) as f64 / 20.0);
        let settings: [Option<(Shape, f64, f64, f64, usize, usize)>; BANDS] = std::array::from_fn(|b| band_settings(p, b));
        let mut start = 0;
        while start < n {
            let end = (start + BLOCK).min(n);
            // Control rate: smooth the settings, follow the dynamics, redesign.
            for (b, band) in self.bands.iter_mut().enumerate() {
                let Some((shape, f, g, q, slope, _)) = settings[b] else {
                    band.made = None;
                    continue;
                };
                let target = [f.ln(), g, q.ln()];
                let fresh = !matches!(band.made, Some((s, sl, _)) if s == shape && sl == slope);
                if fresh {
                    band.cur = target;
                    band.state = [[State::default(); 8]; 2];
                } else {
                    for k in 0..3 {
                        band.cur[k] += (target[k] - band.cur[k]) * smooth;
                    }
                }
                let range = p[idx(b, DYN_RANGE)] as f64;
                let dynamic = shape.has_gain() && range.abs() > 0.01;
                if dynamic {
                    let level = 10.0 * (2.0 * band.env + 1e-12).log10();
                    let over = (level - p[idx(b, DYN_THRESH)] as f64).max(0.0);
                    band.dyn_db = range * (1.0 - (-over / 8.0).exp());
                    band.detector = biquad::bandpass(band.cur[0].exp(), sr, band.cur[2].exp());
                } else {
                    band.dyn_db = 0.0;
                }
                let want = [band.cur[0], band.cur[1] + band.dyn_db, band.cur[2]];
                let stale = match band.made {
                    Some((_, _, m)) => (0..3).any(|k| (m[k] - want[k]).abs() > 1e-6),
                    None => true,
                };
                if stale || fresh {
                    band.filter = design(shape, want[0].exp(), want[1], want[2].exp(), slope, sr);
                    band.made = Some((shape, slope, want));
                }
            }
            let out_step = (target_out - self.out_gain) / (end - start) as f64;
            for i in start..end {
                let (mut a, mut c) = (l[i] as f64, r[i] as f64);
                for (b, band) in self.bands.iter_mut().enumerate() {
                    let Some((shape, .., stereo)) = settings[b] else { continue };
                    let dynamic = shape.has_gain() && p[idx(b, DYN_RANGE)].abs() > 0.01;
                    match stereo {
                        1 | 2 => {
                            let x = if stereo == 1 { &mut a } else { &mut c };
                            if dynamic {
                                let d = band.det_state.run(&band.detector, *x);
                                band.env += (d * d - band.env) * if d * d > band.env { att } else { rel };
                            }
                            *x = run_cascade(&band.filter, &mut band.state[0], *x);
                        }
                        3 | 4 => {
                            let (mut m, mut s) = ((a + c) * 0.5, (a - c) * 0.5);
                            let x = if stereo == 3 { &mut m } else { &mut s };
                            if dynamic {
                                let d = band.det_state.run(&band.detector, *x);
                                band.env += (d * d - band.env) * if d * d > band.env { att } else { rel };
                            }
                            *x = run_cascade(&band.filter, &mut band.state[0], *x);
                            a = m + s;
                            c = m - s;
                        }
                        _ => {
                            if dynamic {
                                let d = band.det_state.run(&band.detector, (a + c) * 0.5);
                                band.env += (d * d - band.env) * if d * d > band.env { att } else { rel };
                            }
                            a = run_cascade(&band.filter, &mut band.state[0], a);
                            c = run_cascade(&band.filter, &mut band.state[1], c);
                        }
                    }
                }
                self.out_gain += out_step;
                l[i] = (a * self.out_gain) as f32;
                r[i] = (c * self.out_gain) as f32;
            }
            self.out_gain = target_out;
            start = end;
        }
        if let Some(m) = &mut self.taps.meters {
            for (b, band) in self.bands.iter().enumerate() {
                m.values[b] = band.dyn_db as f32;
            }
        }
    }
}

/// Parameters with one band set up, for presets and tests.
pub fn with_band(mut p: Vec<f32>, b: usize, shape: Shape, f: f32, gain: f32, q: f32) -> Vec<f32> {
    p[idx(b, SHAPE)] = shape as i32 as f32;
    p[idx(b, FREQ)] = f;
    p[idx(b, GAIN)] = gain;
    p[idx(b, Q)] = q;
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn tone_gain_db(p: &[f32], hz: f64, left_only: bool) -> (f64, f64) {
        let sr = 48000.0;
        let mut st = LensEqState::default();
        let n = 24000;
        let x: Vec<f32> = (0..n).map(|i| (0.25 * (2.0 * PI * hz * i as f64 / sr).sin()) as f32).collect();
        let (mut l, mut r) = (x.clone(), if left_only { vec![0.0; n] } else { x.clone() });
        for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            st.process(p, cl, cr, sr as f32);
        }
        // RMS, so the phase of a high tone against the samples does not matter.
        let rms = |v: &[f32]| (v[n / 2..].iter().map(|s| (*s as f64).powi(2)).sum::<f64>() / (n / 2) as f64).sqrt();
        let in_rms = 0.25 / 2f64.sqrt();
        (20.0 * (rms(&l) / in_rms).log10(), 20.0 * (rms(&r).max(1e-9) / in_rms).log10())
    }

    fn defaults() -> Vec<f32> {
        crate::params::defaults(SPECS)
    }

    #[test]
    fn audio_matches_the_drawn_curve() {
        let mut p = defaults();
        p = with_band(p, 0, Shape::Bell, 1000.0, 9.0, 1.5);
        p = with_band(p, 1, Shape::LowCut, 80.0, 0.0, 0.7071);
        p = with_band(p, 2, Shape::HighShelf, 8000.0, -6.0, 0.7);
        for hz in [50.0, 200.0, 1000.0, 3000.0, 12000.0] {
            let drawn: f64 = (0..BANDS).map(|b| band_response_db(&p, b, hz, 48000.0, 0.0)).sum();
            let (heard, _) = tone_gain_db(&p, hz, false);
            assert!((heard - drawn).abs() < 0.15, "{hz} Hz: heard {heard:.2} dB, drawn {drawn:.2} dB");
        }
    }

    #[test]
    fn bands_can_work_on_one_side_or_mid() {
        let mut p = defaults();
        p = with_band(p, 0, Shape::Bell, 1000.0, 12.0, 1.0);
        p[idx(0, STEREO)] = 1.0; // left only
        let (l, r) = tone_gain_db(&p, 1000.0, false);
        assert!((l - 12.0).abs() < 0.2 && r.abs() < 0.2, "left {l:.2} right {r:.2}");
        // Mid: a signal only on the left is half mid, half side.
        p[idx(0, STEREO)] = 3.0;
        p[idx(0, GAIN)] = -60.0f32.max(-30.0);
        let (l, r) = tone_gain_db(&p, 1000.0, true);
        // Mid cut by 30 dB leaves the side: -6 dB on the left, -6 dB (inverted) on the right.
        assert!((l + 6.0).abs() < 0.5 && (r + 6.0).abs() < 0.5, "left {l:.2} right {r:.2}");
    }

    #[test]
    fn bypass_gain_scale_and_output() {
        let mut p = defaults();
        p = with_band(p, 0, Shape::Bell, 1000.0, 10.0, 1.0);
        p[GAIN_SCALE] = 0.5;
        p[OUTPUT] = -3.0;
        let (g, _) = tone_gain_db(&p, 1000.0, false);
        assert!((g - 2.0).abs() < 0.2, "{g:.2}");
        p[idx(0, ACTIVE)] = 0.0;
        let (g, _) = tone_gain_db(&p, 1000.0, false);
        assert!((g + 3.0).abs() < 0.1);
    }

    #[test]
    fn a_dynamic_band_cuts_only_loud_signals() {
        let mut p = defaults();
        p = with_band(p, 0, Shape::Bell, 1000.0, 0.0, 1.0);
        p[idx(0, DYN_RANGE)] = -12.0;
        p[idx(0, DYN_THRESH)] = -30.0;
        let level = |amp: f32| {
            let sr = 48000.0;
            let mut st = LensEqState::default();
            let n = 48000;
            let mut l: Vec<f32> = (0..n).map(|i| amp * (2.0 * PI * 1000.0 * i as f64 / sr).sin() as f32).collect();
            let mut r = l.clone();
            for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
                st.process(&p, cl, cr, sr as f32);
            }
            let out = l[n / 2..].iter().fold(0.0f32, |m, s| m.max(s.abs()));
            (20.0 * (out / amp).log10(), st.taps.meters.as_ref().unwrap().values[0])
        };
        let (quiet, _) = level(0.003); // -50 dB: under the threshold
        let (loud, shown) = level(0.5); // -6 dB: well over it
        assert!(quiet.abs() < 0.3, "quiet signal untouched: {quiet:.2}");
        assert!(loud < -8.0, "loud signal cut: {loud:.2}");
        assert!(shown < -8.0, "the meter shows the cut: {shown}");
    }
}
