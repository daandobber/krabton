//! Wind Chimes: a wind chime that plays by itself. The wind, the clapper
//! and the tubes are simulated (see `chime_sim`); every time the clapper
//! meets a tube, that tube rings, as hard as they met. A tube sounds with
//! the modes of a free bar (1 : 2.756 : 5.404 : 8.933 : 13.34 : 18.64 times
//! its fundamental), each fading at its own rate, the lowest two split into
//! a slow beat as on real tubes, plus the knock of the clapper. The
//! material sets how long it rings, how bright it is and how loud the knock.
//!
//! Play a key to touch the tube nearest to it. The wind blows while the
//! transport plays, always, or while a key is held (Blows). Original code.

use super::chime_sim::{lengths_for, ChimeSim, Shape, MAX_TUBES};
use super::effects::FxCtx;
use super::fx::hall::{self, Hall};
use super::{db_to_gain, midi_to_freq, pan_gains};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::TAU;

pub const BLOWS: usize = 0;
pub const WIND: usize = 1;
pub const GUSTS: usize = 2;
pub const TUBES: usize = 3;
pub const MATERIAL: usize = 4;
pub const BASE_NOTE: usize = 5;
pub const TUNING: usize = 6;
pub const SPACING: usize = 7;
pub const CLAPPER: usize = 8;
pub const SAIL: usize = 9;
pub const THICKNESS: usize = 10;
pub const RING: usize = 11;
pub const WIDTH: usize = 12;
pub const SPACE: usize = 13;
pub const VOLUME: usize = 14;

pub const BLOW_MODES: &[&str] = &["While Playing", "Always", "While a Key Is Held"];
pub const MATERIALS: &[&str] = &["Aluminium", "Brass", "Steel", "Bamboo", "Glass", "Wood"];
pub const TUNINGS: &[&str] = &["Pentatonic", "Minor Pentatonic", "Major 6/9", "Major 7", "Minor 7", "Suspended", "Whole Tone", "In (Japanese)", "Harmonic Series"];

/// Semitones above the lowest tube, for up to eight tubes.
const TUNING_STEPS: [[f32; MAX_TUBES]; 9] = [
    [0.0, 2.0, 4.0, 7.0, 9.0, 12.0, 14.0, 16.0],
    [0.0, 3.0, 5.0, 7.0, 10.0, 12.0, 15.0, 17.0],
    [0.0, 4.0, 7.0, 9.0, 14.0, 16.0, 19.0, 21.0],
    [0.0, 4.0, 7.0, 11.0, 12.0, 16.0, 19.0, 23.0],
    [0.0, 3.0, 7.0, 10.0, 12.0, 15.0, 19.0, 22.0],
    [0.0, 5.0, 7.0, 12.0, 14.0, 17.0, 19.0, 24.0],
    [0.0, 2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0],
    [0.0, 1.0, 5.0, 7.0, 8.0, 12.0, 13.0, 17.0],
    [0.0, 12.0, 19.02, 24.0, 27.86, 31.02, 33.69, 36.0],
];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Blows", "Wind", BLOW_MODES, 0),
    ParamSpec::lin("Wind", "Wind", 0.0, 1.5, 0.4, Unit::Percent),
    ParamSpec::lin("Gusts", "Wind", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::int("Tubes", "Chime", 3.0, 8.0, 6.0, Unit::None),
    ParamSpec::choice("Material", "Chime", MATERIALS, 0),
    ParamSpec::int("Lowest Note", "Chime", 36.0, 96.0, 72.0, Unit::None),
    ParamSpec::choice("Tuning", "Chime", TUNINGS, 0),
    ParamSpec::lin("Spacing", "Build", 0.0, 1.0, 0.45, Unit::Percent),
    ParamSpec::lin("Clapper", "Build", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Sail", "Build", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Thickness", "Build", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Ring", "Sound", 0.2, 3.0, 1.0, Unit::None),
    ParamSpec::lin("Width", "Sound", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Space", "Sound", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Volume", "Sound", -24.0, 12.0, 0.0, Unit::Db),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Garden Aluminium", "Temple Brass", "Deep Steel", "Bamboo Grove", "Glass Porch", "Wooden Clatter", "Storm", "Still Evening", "Harmonic Tower"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let mut set = |pairs: &[(usize, f32)]| {
        for &(i, v) in pairs {
            p[i] = v;
        }
    };
    match name {
        "Temple Brass" => set(&[(MATERIAL, 1.0), (BASE_NOTE, 62.0), (TUNING, 5.0), (TUBES, 5.0), (CLAPPER, 0.7), (THICKNESS, 0.7), (SPACE, 0.35)]),
        "Deep Steel" => set(&[(MATERIAL, 2.0), (BASE_NOTE, 48.0), (TUNING, 4.0), (TUBES, 6.0), (WIND, 0.35), (SAIL, 0.7), (RING, 1.6), (SPACE, 0.3)]),
        "Bamboo Grove" => set(&[(MATERIAL, 3.0), (BASE_NOTE, 67.0), (TUNING, 1.0), (TUBES, 8.0), (WIND, 0.55), (SPACING, 0.25), (SPACE, 0.15)]),
        "Glass Porch" => set(&[(MATERIAL, 4.0), (BASE_NOTE, 79.0), (TUNING, 0.0), (TUBES, 7.0), (CLAPPER, 0.3), (THICKNESS, 0.3), (SPACE, 0.25)]),
        "Wooden Clatter" => set(&[(MATERIAL, 5.0), (BASE_NOTE, 60.0), (TUNING, 0.0), (TUBES, 8.0), (WIND, 0.7), (SPACING, 0.2)]),
        "Storm" => set(&[(WIND, 1.2), (GUSTS, 1.0), (SAIL, 0.8), (BASE_NOTE, 65.0), (TUNING, 6.0), (SPACE, 0.3)]),
        "Still Evening" => set(&[(WIND, 0.22), (GUSTS, 0.8), (BASE_NOTE, 69.0), (TUNING, 2.0), (RING, 1.8), (SPACE, 0.4)]),
        "Harmonic Tower" => set(&[(MATERIAL, 2.0), (BASE_NOTE, 41.0), (TUNING, 8.0), (TUBES, 8.0), (RING, 2.0), (SPACE, 0.35)]),
        _ => {}
    }
    p
}

/// Mode ratios of a free bar.
const RATIOS: [f32; 6] = [1.0, 2.756, 5.404, 8.933, 13.344, 18.638];

/// Per material: ring time (s, for 440 Hz), how fast higher modes fade
/// faster, the modes' levels, the beat between split modes (Hz), the knock
/// and its brightness (Hz), and a slight stretch of the overtones.
struct Material {
    t60: f32,
    tilt: f32,
    amps: [f32; 6],
    split: f32,
    knock: f32,
    knock_hz: f32,
    stretch: f32,
}

const MATERIAL_DATA: [Material; 6] = [
    Material { t60: 9.0, tilt: 0.7, amps: [1.0, 0.55, 0.35, 0.22, 0.12, 0.06], split: 0.45, knock: 0.04, knock_hz: 6000.0, stretch: 1.0 },
    Material { t60: 5.5, tilt: 0.9, amps: [1.0, 0.7, 0.5, 0.3, 0.16, 0.08], split: 0.6, knock: 0.07, knock_hz: 4000.0, stretch: 0.995 },
    Material { t60: 12.0, tilt: 0.6, amps: [1.0, 0.6, 0.45, 0.35, 0.25, 0.15], split: 0.3, knock: 0.05, knock_hz: 7000.0, stretch: 1.005 },
    Material { t60: 0.55, tilt: 1.2, amps: [1.0, 0.45, 0.2, 0.08, 0.04, 0.02], split: 0.0, knock: 0.45, knock_hz: 2500.0, stretch: 0.98 },
    Material { t60: 3.0, tilt: 0.8, amps: [0.8, 0.7, 0.6, 0.45, 0.35, 0.25], split: 0.8, knock: 0.03, knock_hz: 9000.0, stretch: 1.01 },
    Material { t60: 0.35, tilt: 1.4, amps: [1.0, 0.4, 0.15, 0.05, 0.02, 0.01], split: 0.0, knock: 0.35, knock_hz: 1800.0, stretch: 0.97 },
];

/// The tubes' pitches (Hz) for these params.
pub fn tube_freqs(p: &[f32]) -> [f32; MAX_TUBES] {
    let t = (p[TUNING].round().max(0.0) as usize).min(TUNINGS.len() - 1);
    std::array::from_fn(|i| midi_to_freq(p[BASE_NOTE] + TUNING_STEPS[t][i]))
}

/// The build for the simulation.
pub fn shape(p: &[f32]) -> Shape {
    let n = (p[TUBES].round() as usize).clamp(3, MAX_TUBES);
    let f = tube_freqs(p);
    Shape { tubes: n, spacing: p[SPACING], clapper: p[CLAPPER], sail: p[SAIL], wind: p[WIND].clamp(0.0, 1.5), gusts: p[GUSTS].clamp(0.0, 1.0), lengths: lengths_for(&f[..n]) }
}

/// A decaying sine: a phasor turned and shrunk every sample.
#[derive(Clone, Copy, Default)]
struct Mode {
    re: f32,
    im: f32,
    c: f32,
    s: f32,
}

impl Mode {
    #[inline]
    fn tick(&mut self) -> f32 {
        let re = self.re * self.c - self.im * self.s;
        self.im = self.re * self.s + self.im * self.c;
        self.re = re;
        self.im
    }
}

#[derive(Clone, Copy, Default)]
struct Tube {
    modes: [Mode; 8],
    amps: [f32; 8],
    n: usize,
    knock: f32,
    knock_lp: f32,
    pan: (f32, f32),
}

pub struct WindChimesState {
    /// The song plays (set by the engine every block).
    pub playing: bool,
    sr: f32,
    sim: ChimeSim,
    tubes: [Tube; MAX_TUBES],
    /// The params the modes were set up for.
    key: [f32; 8],
    held: usize,
    rng: u32,
    knock_a: f32,
    hall: Box<Hall>,
    hall_p: Vec<f32>,
    wl: Vec<f32>,
    wr: Vec<f32>,
    /// How strongly each tube rings now (for the screen).
    energy: [f32; MAX_TUBES],
}

/// Samples per simulation run.
const CHUNK: usize = 64;

impl WindChimesState {
    pub fn new(sr: f32) -> Self {
        Self {
            playing: false,
            sr: sr.max(1000.0),
            sim: ChimeSim::new(0x7ABE_11ED),
            tubes: [Tube::default(); MAX_TUBES],
            key: [f32::NAN; 8],
            held: 0,
            rng: 0x2545_F491,
            knock_a: 0.5,
            hall: Box::new(Hall::new(sr)),
            hall_p: crate::params::defaults(hall::SPECS),
            wl: vec![0.0; CHUNK],
            wr: vec![0.0; CHUNK],
            energy: [0.0; MAX_TUBES],
        }
    }

    /// Sets the modes up for the material, pitches and build.
    fn tune(&mut self, p: &[f32]) {
        let mat = (p[MATERIAL].round().max(0.0) as usize).min(MATERIALS.len() - 1);
        let key = [mat as f32, p[BASE_NOTE], p[TUNING].round(), p[THICKNESS], p[RING], p[WIDTH], p[TUBES].round(), self.sr];
        if key == self.key {
            return;
        }
        self.key = key;
        let m = &MATERIAL_DATA[mat];
        let f = tube_freqs(p);
        let n = (p[TUBES].round() as usize).clamp(3, MAX_TUBES);
        let s = shape(p);
        let thick = p[THICKNESS].clamp(0.0, 1.0);
        let ring = p[RING].clamp(0.2, 3.0) * (0.6 + 0.8 * thick);
        self.knock_a = 1.0 - (-TAU * m.knock_hz / self.sr).exp();
        for i in 0..MAX_TUBES {
            let tube = &mut self.tubes[i];
            let mut k = 0;
            for (j, &ratio) in RATIOS.iter().enumerate() {
                let fr = f[i] * ratio * m.stretch.powf(j as f32);
                if fr > 0.45 * self.sr {
                    break;
                }
                // Lower pitches ring longer, higher modes shorter.
                let t60 = (m.t60 * ring * (440.0 / f[i]).powf(0.5) / (j as f32 + 1.0).powf(m.tilt * (1.2 - 0.4 * thick))).max(0.02);
                let r = (-6.91 / (t60 * self.sr)).exp();
                // The lowest two split in two, a slow beat.
                let parts: &[f32] = if j < 2 && m.split > 0.0 { &[-0.5, 0.5] } else { &[0.0] };
                for &d in parts {
                    if k < tube.modes.len() {
                        let w = TAU * (fr + d * m.split) / self.sr;
                        let old = tube.modes[k];
                        tube.modes[k] = Mode { re: old.re, im: old.im, c: r * w.cos(), s: r * w.sin() };
                        tube.amps[k] = m.amps[j] * (1.0 - 0.35 * thick * j as f32 / 5.0) / parts.len() as f32;
                        k += 1;
                    }
                }
            }
            tube.n = k;
            let pos = s.rest(i.min(n.max(1) - 1));
            tube.pan = pan_gains(pos[0] * p[WIDTH].clamp(0.0, 1.0));
        }
    }

    fn strike(&mut self, i: usize, speed: f32, p: &[f32]) {
        let mat = (p[MATERIAL].round().max(0.0) as usize).min(MATERIALS.len() - 1);
        let m = &MATERIAL_DATA[mat];
        let vel = (speed / 2.0).sqrt().min(1.0);
        // A heavier clapper strikes softer: the high modes less.
        let soft = 0.3 + 0.7 * p[CLAPPER].clamp(0.0, 1.0);
        let t = &mut self.tubes[i];
        for k in 0..t.n {
            let bright = (1.0 - soft * 0.5).powf(k as f32 * 0.5) * (0.4 + 0.6 * vel);
            t.modes[k].im += t.amps[k] * vel * bright;
        }
        t.knock += m.knock * vel * (0.5 + soft);
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        self.held += 1;
        // The tube nearest the key.
        let f = tube_freqs(p);
        let s = shape(p);
        let want = midi_to_freq(note as f32);
        let i = (0..s.tubes).min_by(|&a, &b| (f[a] / want).ln().abs().total_cmp(&(f[b] / want).ln().abs())).unwrap_or(0);
        self.tune(p);
        self.strike(i, 2.0 * vel * vel, p);
        self.sim.nudge(&s, i, vel);
    }

    pub fn note_off(&mut self, _p: &[f32], _note: u8) {
        self.held = self.held.saturating_sub(1);
    }

    pub fn all_off(&mut self) {
        self.held = 0;
    }

    /// The screen: [0] [1] the clapper, [2] wind, [3] its direction, [4]
    /// tubes, [5..13] their swing across, [13..21] their swing in depth,
    /// [21..29] how strongly each rings.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = self.sim.clapper[0];
        out[1] = self.sim.clapper[1];
        out[2] = self.sim.wind;
        out[3] = self.sim.dir;
        out[4] = 1.0;
        for i in 0..MAX_TUBES {
            out[5 + i] = self.sim.tube[i][0];
            out[13 + i] = self.sim.tube[i][1];
            out[21 + i] = self.energy[i];
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        self.tune(p);
        let s = shape(p);
        let blowing = match (p[BLOWS].round().max(0.0) as usize).min(BLOW_MODES.len() - 1) {
            0 => self.playing,
            1 => true,
            _ => self.held > 0,
        };
        let n = l.len().min(r.len());
        let space = p[SPACE].clamp(0.0, 1.0);
        let gain = db_to_gain(p[VOLUME]) * 0.22;
        let mut done = 0;
        while done < n {
            let len = CHUNK.min(n - done);
            let mut hits = [0.0f32; MAX_TUBES];
            self.sim.run(&s, len as f32 / self.sr, blowing, &mut |i, speed, _| hits[i] = hits[i].max(speed));
            for (i, &h) in hits.iter().enumerate() {
                if h > 0.0 {
                    self.strike(i, h, p);
                }
            }
            let (wl, wr) = (&mut self.wl[..len], &mut self.wr[..len]);
            wl.fill(0.0);
            wr.fill(0.0);
            for (i, t) in self.tubes.iter_mut().enumerate().take(s.tubes) {
                let mut e = 0.0f32;
                for j in 0..len {
                    let mut y = 0.0;
                    for m in t.modes[..t.n].iter_mut() {
                        y += m.tick();
                    }
                    // The clapper's knock: a short burst of dull noise.
                    if t.knock > 1e-5 {
                        self.rng ^= self.rng << 13;
                        self.rng ^= self.rng >> 17;
                        self.rng ^= self.rng << 5;
                        let w = (self.rng >> 8) as f32 / 8_388_608.0 - 1.0;
                        t.knock_lp += self.knock_a * (w - t.knock_lp);
                        y += t.knock * t.knock_lp;
                        t.knock *= 0.993;
                    }
                    wl[j] += y * t.pan.0;
                    wr[j] += y * t.pan.1;
                    e = e.max(y.abs());
                }
                self.energy[i] = self.energy[i] * 0.9 + 0.1 * e;
                // Keep it quiet when it has rung out (no denormals).
                for m in t.modes[..t.n].iter_mut() {
                    if m.re.abs() + m.im.abs() < 1e-7 {
                        m.re = 0.0;
                        m.im = 0.0;
                    }
                }
            }
            if space > 0.001 {
                let hp = &mut self.hall_p;
                hp[hall::PREDELAY] = 20.0;
                hp[hall::SIZE] = 0.4 + 0.5 * space;
                hp[hall::DECAY] = 1.5 + 6.0 * space;
                hp[hall::LOW_DECAY] = 1.0;
                hp[hall::DAMPING] = 7000.0;
                hp[hall::DIFFUSION] = 0.8;
                hp[hall::MODULATION] = 0.15;
                hp[hall::FREEZE] = 0.0;
                hp[hall::WIDTH] = 1.0;
                hp[hall::MIX] = 0.5 * space;
                let ctx = FxCtx { sr: self.sr, bpm: 120.0, beat: 0.0, playing: false };
                self.hall.process(&self.hall_p, wl, wr, &ctx, None);
            }
            for j in 0..len {
                l[done + j] += (wl[j] * gain).clamp(-1.5, 1.5);
                r[done + j] += (wr[j] * gain).clamp(-1.5, 1.5);
            }
            done += len;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn run(st: &mut WindChimesState, p: &[f32], secs: f32) -> (Vec<f32>, Vec<f32>) {
        let n = (secs * SR) as usize;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            st.render(p, a, b);
        }
        (l, r)
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
    fn every_preset_chimes_and_stays_bounded() {
        for name in preset_names() {
            let mut p = preset(name);
            p[BLOWS] = 1.0;
            p[WIND] = p[WIND].max(0.5);
            let mut st = WindChimesState::new(SR);
            let (l, r) = run(&mut st, &p, 8.0);
            let peak = l.iter().chain(&r).fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(l.iter().chain(&r).all(|v| v.is_finite()), "{name}");
            assert!(peak > 0.01 && peak < 1.2, "{name}: {peak}");
            if std::env::var("CHIME_REPORT").is_ok() {
                let rms = (l.iter().map(|v| v * v).sum::<f32>() / l.len() as f32).sqrt();
                eprintln!("{name}: peak {peak:.3} rms {rms:.3}");
            }
        }
    }

    #[test]
    fn a_key_rings_its_tube_with_free_bar_overtones() {
        let mut p = preset("Garden Aluminium");
        p[BLOWS] = 2.0;
        let mut st = WindChimesState::new(SR);
        // The lowest tube is C5 (523 Hz): its next mode is 2.756 times up.
        st.note_on(&p, 72, 0.9);
        let (l, _) = run(&mut st, &p, 1.0);
        let f0 = midi_to_freq(72.0);
        let tail = &l[4800..];
        assert!(mag(tail, f0) > 10.0 * mag(tail, f0 * 1.5), "fundamental");
        assert!(mag(tail, f0 * 2.756) > 5.0 * mag(tail, f0 * 2.0), "free bar overtone");
    }

    #[test]
    fn bamboo_dies_away_much_sooner_than_aluminium() {
        let tail = |mat: f32| {
            let mut p = preset("Garden Aluminium");
            p[BLOWS] = 2.0;
            p[MATERIAL] = mat;
            p[SPACE] = 0.0;
            let mut st = WindChimesState::new(SR);
            st.note_on(&p, 72, 0.9);
            let (l, _) = run(&mut st, &p, 1.5);
            let rms = |x: &[f32]| (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt();
            rms(&l[(1.2 * SR) as usize..]) / rms(&l[..(0.2 * SR) as usize])
        };
        assert!(tail(3.0) < 0.1 * tail(0.0), "{} {}", tail(3.0), tail(0.0));
    }

    #[test]
    fn no_wind_without_a_key_when_it_waits_for_one() {
        let mut p = preset("Storm");
        p[BLOWS] = 2.0;
        let mut st = WindChimesState::new(SR);
        let (l, _) = run(&mut st, &p, 4.0);
        assert!(l.iter().all(|v| v.abs() < 1e-4));
        // A key held: the wind comes up and it chimes.
        st.note_on(&p, 30, 0.01);
        let (l, _) = run(&mut st, &p, 6.0);
        assert!(l.iter().any(|v| v.abs() > 0.01));
    }
}
