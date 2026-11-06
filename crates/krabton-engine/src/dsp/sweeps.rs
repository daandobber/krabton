//! Sweeps: the sounds between the sounds. Risers, things falling down, impacts,
//! lasers, a sub dropping out from under it all, a swell coming up backwards.
//! A key sets one off and it runs its length by itself; the key says how high.
//! For the moment before the drop, and for going from one record to the next.

use super::{fast_tanh, Rng, Svf};
use crate::params::{ParamSpec, Unit};

pub const KIND: usize = 0;
pub const LENGTH: usize = 1;
pub const NOISE: usize = 2;
pub const RESONANCE: usize = 3;
pub const CURVE: usize = 4;
pub const WIDTH: usize = 5;
pub const DRIVE: usize = 6;
pub const VOLUME: usize = 7;

pub const KINDS: &[&str] = &["Riser", "Downlifter", "Impact", "Laser", "Sub Drop", "Backwards", "Tape Rise", "Siren"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Kind", "Sweep", KINDS, 0),
    ParamSpec::log("Length", "Sweep", 0.1, 30.0, 4.0, Unit::Sec),
    ParamSpec::lin("Noise", "Sound", 0.0, 1.0, 0.7, Unit::Percent),
    ParamSpec::lin("Resonance", "Sound", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Curve", "Sound", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Width", "Output", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Drive", "Output", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Volume", "Output", 0.0, 1.0, 0.7, Unit::Percent),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Long Riser", "Short Riser", "Falling Air", "Big Impact", "Thud", "Laser", "Sub Drop", "Swell", "Tape Up", "Siren"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Long Riser" => &[(LENGTH, 8.0)],
        "Short Riser" => &[(LENGTH, 1.5), (RESONANCE, 0.6)],
        "Falling Air" => &[(KIND, 1.0), (LENGTH, 4.0), (NOISE, 1.0)],
        "Big Impact" => &[(KIND, 2.0), (LENGTH, 2.5), (NOISE, 0.4), (DRIVE, 0.5)],
        "Thud" => &[(KIND, 2.0), (LENGTH, 0.6), (NOISE, 0.1), (DRIVE, 0.4)],
        "Laser" => &[(KIND, 3.0), (LENGTH, 0.4), (NOISE, 0.0), (RESONANCE, 0.2)],
        "Sub Drop" => &[(KIND, 4.0), (LENGTH, 3.0), (NOISE, 0.0), (DRIVE, 0.3)],
        "Swell" => &[(KIND, 5.0), (LENGTH, 2.0), (NOISE, 0.5)],
        "Tape Up" => &[(KIND, 6.0), (LENGTH, 1.2), (NOISE, 0.1)],
        "Siren" => &[(KIND, 7.0), (LENGTH, 4.0), (NOISE, 0.0), (RESONANCE, 0.5)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

const VOICES: usize = 6;

/// What a sweep does at a place along it (0..1): how loud, the filter (Hz), the
/// pitch in notes off the key, whether the tone is a saw.
pub(crate) fn shape(kind: usize, t: f32, curve: f32) -> (f32, f32, f32, bool) {
    let c = 0.5 + 3.0 * curve;
    let exp_hz = |a: f32, b: f32, x: f32| a * (b / a).powf(x.clamp(0.0, 1.0));
    match kind {
        // Up: louder, brighter, higher.
        0 => (t.powf(c), exp_hz(200.0, 14000.0, t.powf(1.5)), -24.0 + 48.0 * t, true),
        // Down.
        1 => ((1.0 - t).powf(c), exp_hz(14000.0, 150.0, t.powf(0.6)), 24.0 - 48.0 * t, true),
        // A blow, then falling away.
        2 => ((-(4.0 + 5.0 * curve) * t).exp(), exp_hz(9000.0, 70.0, t.powf(0.35)), 12.0 * (-14.0 * t).exp() - 12.0 - 6.0 * t, false),
        3 => ((-(2.0 + 3.0 * curve) * t).exp(), 16000.0, 48.0 * (-(4.0 + 8.0 * curve) * t).exp() - 12.0, false),
        4 => ((1.0 - t).powf(0.5 + curve), 3000.0, 12.0 - 36.0 * t.powf(0.6 + curve), false),
        // A blow played from its end.
        5 => {
            let k = 3.0 + 6.0 * curve;
            (((k * t).exp() - 1.0) / (k.exp() - 1.0), exp_hz(300.0, 12000.0, t), 0.0, true)
        }
        // A tape coming up to speed.
        6 => ((t * 8.0).min(1.0) * (1.0 - ((t - 0.95) / 0.05).clamp(0.0, 1.0)), 7000.0, -48.0 * (1.0 - t).powf(1.0 + 2.0 * curve), true),
        // Up and down, up and down.
        _ => {
            let turns = 2.0 + 6.0 * curve;
            let w = (t * turns * std::f32::consts::TAU).sin();
            ((t * 20.0).min(1.0) * ((1.0 - t) * 20.0).min(1.0), 8000.0, 7.0 + 7.0 * w, false)
        }
    }
}

#[derive(Clone, Copy)]
struct Voice {
    alive: bool,
    note: u8,
    velocity: f32,
    /// Along it, and by how much per frame.
    t: f32,
    step: f32,
    kind: usize,
    phase: f32,
    filter: [Svf; 2],
    rng: [Rng; 2],
    age: u64,
}

pub struct SweepsState {
    sr: f32,
    pub pitch_bend: f32,
    voices: [Voice; VOICES],
    clock: u64,
}

impl SweepsState {
    pub fn new(sr: f32) -> Self {
        let voice = Voice { alive: false, note: 60, velocity: 1.0, t: 0.0, step: 0.0, kind: 0, phase: 0.0, filter: [Svf::new(); 2], rng: [Rng::new(0x1234), Rng::new(0x9ABC)], age: 0 };
        Self { sr, pitch_bend: 0.0, voices: [voice; VOICES], clock: 0 }
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        let get = |i: usize| p.get(i).copied().unwrap_or(SPECS[i].default);
        self.clock += 1;
        let k = (0..VOICES).find(|&i| !self.voices[i].alive).unwrap_or_else(|| (0..VOICES).min_by_key(|&i| self.voices[i].age).unwrap_or(0));
        let v = &mut self.voices[k];
        v.alive = true;
        v.note = note;
        v.velocity = vel.clamp(0.0, 1.0);
        v.t = 0.0;
        v.step = 1.0 / (get(LENGTH).max(0.01) * self.sr);
        v.kind = (get(KIND).round().max(0.0) as usize).min(KINDS.len() - 1);
        v.phase = 0.0;
        v.age = self.clock;
    }

    /// It runs by itself: a key let go changes nothing.
    pub fn note_off(&mut self, _p: &[f32], _note: u8) {}

    pub fn all_off(&mut self) {
        for v in self.voices.iter_mut() {
            v.alive = false;
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        let get = |i: usize| p.get(i).copied().unwrap_or(SPECS[i].default);
        let sr = self.sr;
        let n = l.len().min(r.len());
        l[..n].fill(0.0);
        r[..n].fill(0.0);
        let noise = get(NOISE);
        let q = 0.7 + get(RESONANCE) * 9.0;
        let curve = get(CURVE);
        let width = get(WIDTH);
        let drive = 1.0 + get(DRIVE) * 6.0;
        let volume = get(VOLUME);
        let bend = self.pitch_bend;
        for v in self.voices.iter_mut().filter(|v| v.alive) {
            // The filter moves on every 32 frames.
            let mut shaped = shape(v.kind, v.t, curve);
            for i in 0..n {
                if i % 32 == 0 {
                    shaped = shape(v.kind, v.t.min(1.0), curve);
                    let hz = shaped.1.clamp(20.0, sr * 0.45);
                    v.filter[0].set_q(hz, q, sr);
                    v.filter[1].set_q(hz, q, sr);
                }
                let (amp, _, semis, is_saw) = shaped;
                let note = v.note as f32 + semis + bend;
                let hz = (440.0 * 2f32.powf((note - 69.0) / 12.0)).clamp(5.0, sr * 0.45);
                v.phase = (v.phase + hz / sr).fract();
                let tone = if is_saw { 2.0 * v.phase - 1.0 } else { (v.phase * std::f32::consts::TAU).sin() };
                let a = tone * (1.0 - noise) + v.rng[0].bipolar() * noise;
                let b = tone * (1.0 - noise) + (v.rng[1].bipolar() * width + v.rng[0].bipolar() * (1.0 - width)) * noise;
                let (fl, fr) = (v.filter[0].tick(a).low, v.filter[1].tick(b).low);
                // The filter's peak grows with its Q: not what comes out of it.
                let level = amp * v.velocity * volume / q.sqrt();
                l[i] += fast_tanh(fl * level * drive) / drive.sqrt();
                r[i] += fast_tanh(fr * level * drive) / drive.sqrt();
                v.t += v.step;
                if v.t >= 1.0 {
                    v.alive = false;
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn run(p: &[f32], seconds: f32) -> Vec<f32> {
        let mut s = SweepsState::new(SR);
        s.note_on(p, 60, 1.0);
        s.note_off(p, 60);
        let n = (seconds * SR) as usize;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            s.render(p, a, b);
        }
        l
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    fn with(kind: f32, length: f32) -> Vec<f32> {
        let mut p = crate::params::defaults(SPECS);
        p[KIND] = kind;
        p[LENGTH] = length;
        p
    }

    fn thirds(x: &[f32], len: f32) -> (f32, f32) {
        let n = (len * SR) as usize;
        (rms(&x[..n / 3]), rms(&x[n * 2 / 3..n]))
    }

    #[test]
    fn a_riser_gets_louder_and_a_downlifter_quieter() {
        let (a, b) = thirds(&run(&with(0.0, 2.0), 2.0), 2.0);
        assert!(b > a * 2.0, "{a} {b}");
        let (a, b) = thirds(&run(&with(1.0, 2.0), 2.0), 2.0);
        assert!(a > b * 2.0, "{a} {b}");
    }

    #[test]
    fn a_riser_gets_brighter() {
        // More of it changes from one frame to the next, the brighter it is.
        let x = run(&with(0.0, 2.0), 2.0);
        let edge = |a: &[f32]| rms(&a.windows(2).map(|w| w[1] - w[0]).collect::<Vec<_>>()) / rms(a).max(1e-9);
        assert!(edge(&x[80000..96000]) > edge(&x[16000..32000]) * 1.5, "{} {}", edge(&x[16000..32000]), edge(&x[80000..96000]));
    }

    #[test]
    fn it_runs_its_length_whatever_the_key_does_and_then_ends() {
        let x = run(&with(0.0, 1.0), 1.5);
        assert!(rms(&x[40000..47000]) > 0.01);
        assert!(x[48500..].iter().all(|v| *v == 0.0));
    }

    #[test]
    fn the_sub_drop_falls() {
        let mut p = with(4.0, 2.0);
        p[NOISE] = 0.0;
        let x = run(&p, 1.8);
        let ups = |a: &[f32]| (1..a.len()).filter(|&i| a[i - 1] < 0.0 && a[i] >= 0.0).count();
        assert!(ups(&x[..12000]) > ups(&x[60000..72000]) * 2, "{} {}", ups(&x[..12000]), ups(&x[60000..72000]));
    }

    #[test]
    fn every_shape_stays_sane_all_the_way_along() {
        for kind in 0..KINDS.len() {
            for k in 0..=100 {
                let (amp, hz, semis, _) = shape(kind, k as f32 / 100.0, 0.5);
                assert!(amp.is_finite() && (-0.01..=1.01).contains(&amp), "{kind} {k}: {amp}");
                assert!(hz > 10.0 && hz.is_finite(), "{kind} {k}: {hz}");
                assert!(semis.abs() <= 60.0, "{kind} {k}: {semis}");
            }
        }
    }

    #[test]
    fn every_preset_makes_a_sound_within_bounds() {
        for name in preset_names() {
            let p = preset(name);
            assert_eq!(p.len(), SPECS.len());
            let x = run(&p, p[LENGTH].min(4.0));
            let top = x.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(top > 0.02 && top <= 1.0 && x.iter().all(|v| v.is_finite()), "{name}: {top}");
        }
    }
}
