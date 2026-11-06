//! String Machine: the string keyboards of the seventies. One bright saw per
//! key at three registers, all opening and closing with one slow hand, and then
//! what made them what they were: three delay lines swayed by a slow and a fast
//! wobble, a third apart, so that a thin organ-like sound turns into a whole
//! section. Bed of half the lo-fi records there are.

use super::{fast_tanh, FilterMode, Rng, Svf};
use crate::params::{ParamSpec, Unit};

pub const CELLO: usize = 0;
pub const VIOLA: usize = 1;
pub const VIOLIN: usize = 2;
pub const ATTACK: usize = 3;
pub const RELEASE: usize = 4;
pub const ENSEMBLE: usize = 5;
pub const SPEED: usize = 6;
pub const TONE: usize = 7;
pub const BITE: usize = 8;
pub const AGE: usize = 9;
pub const VOLUME: usize = 10;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Cello", "Registers", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Viola", "Registers", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Violin", "Registers", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::log("Attack", "Hand", 1.0, 3000.0, 180.0, Unit::Ms),
    ParamSpec::log("Release", "Hand", 10.0, 6000.0, 700.0, Unit::Ms),
    ParamSpec::lin("Ensemble", "Ensemble", 0.0, 1.0, 0.8, Unit::Percent),
    ParamSpec::lin("Speed", "Ensemble", 0.2, 2.0, 1.0, Unit::Percent),
    ParamSpec::log("Tone", "Tone", 400.0, 16000.0, 5500.0, Unit::Hz),
    ParamSpec::lin("Bite", "Tone", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Age", "Tone", 0.0, 1.0, 0.1, Unit::Percent),
    ParamSpec::lin("Volume", "Output", 0.0, 1.0, 0.8, Unit::Percent),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Full Section", "Violins", "Low Cellos", "Slow Bed", "Worn Tape", "Thin Organ", "Bright Stabs", "Dream"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Violins" => &[(CELLO, 0.0), (VIOLA, 0.3), (VIOLIN, 1.0), (TONE, 8000.0)],
        "Low Cellos" => &[(CELLO, 1.0), (VIOLA, 0.4), (VIOLIN, 0.0), (TONE, 2500.0)],
        "Slow Bed" => &[(ATTACK, 1200.0), (RELEASE, 3000.0), (TONE, 3500.0), (BITE, 0.15)],
        "Worn Tape" => &[(AGE, 0.7), (TONE, 2800.0), (BITE, 0.1), (ENSEMBLE, 0.9)],
        "Thin Organ" => &[(ENSEMBLE, 0.0), (ATTACK, 5.0), (RELEASE, 80.0), (CELLO, 0.0), (BITE, 0.0)],
        "Bright Stabs" => &[(ATTACK, 5.0), (RELEASE, 200.0), (TONE, 12000.0), (BITE, 0.7)],
        "Dream" => &[(ATTACK, 700.0), (RELEASE, 4000.0), (SPEED, 0.5), (ENSEMBLE, 1.0), (TONE, 3000.0), (AGE, 0.3)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

const VOICES: usize = 24;
const LINE_SECONDS: f32 = 0.05;

#[derive(Clone, Copy, Default)]
struct Voice {
    note: u8,
    on: bool,
    level: f32,
    velocity: f32,
    phase: [f32; 3],
    age: u64,
}

pub struct StringMachineState {
    sr: f32,
    pub pitch_bend: f32,
    voices: [Voice; VOICES],
    clock: u64,
    line: Vec<f32>,
    write: usize,
    slow: f32,
    fast: f32,
    wander: f32,
    wander_goal: f32,
    wander_in: u32,
    rng: Rng,
    low: Svf,
    bite: [Svf; 2],
    high: Svf,
}

/// A bright saw, its corner rounded where it would alias.
#[inline]
fn saw(phase: f32, dt: f32) -> f32 {
    let mut y = 2.0 * phase - 1.0;
    if phase < dt {
        let t = phase / dt;
        y -= t + t - t * t - 1.0;
    } else if phase > 1.0 - dt {
        let t = (phase - 1.0) / dt;
        y -= t * t + t + t + 1.0;
    }
    y
}

impl StringMachineState {
    pub fn new(sr: f32) -> Self {
        let mut bite = [Svf::new(); 2];
        bite[0].set_q(420.0, 1.2, sr);
        bite[1].set_q(1600.0, 1.4, sr);
        let mut high = Svf::new();
        high.set_q(110.0, 0.7, sr);
        Self {
            sr,
            pitch_bend: 0.0,
            voices: [Voice::default(); VOICES],
            clock: 0,
            line: vec![0.0; (LINE_SECONDS * sr) as usize + 4],
            write: 0,
            slow: 0.0,
            fast: 0.0,
            wander: 0.0,
            wander_goal: 0.0,
            wander_in: 0,
            rng: Rng::new(0x57A1),
            low: Svf::new(),
            bite,
            high,
        }
    }

    pub fn note_on(&mut self, _p: &[f32], note: u8, vel: f32) {
        self.clock += 1;
        // The key still ringing, a free one, the one gone quietest.
        let k = (0..VOICES)
            .find(|&i| self.voices[i].note == note && self.voices[i].level > 0.0)
            .or_else(|| (0..VOICES).find(|&i| !self.voices[i].on && self.voices[i].level <= 0.0))
            .unwrap_or_else(|| (0..VOICES).min_by_key(|&i| (self.voices[i].on, self.voices[i].age)).unwrap_or(0));
        let v = &mut self.voices[k];
        if !(v.note == note && v.level > 0.0) {
            // Free-running, as a divider's are: never in step with each other.
            let f = self.rng.unipolar();
            v.phase = [f, (f * 3.1).fract(), (f * 7.3).fract()];
            v.level = 0.0;
        }
        v.note = note;
        v.on = true;
        v.velocity = 0.55 + 0.45 * vel.clamp(0.0, 1.0);
        v.age = self.clock;
    }

    pub fn note_off(&mut self, _p: &[f32], note: u8) {
        for v in self.voices.iter_mut().filter(|v| v.on && v.note == note) {
            v.on = false;
        }
    }

    pub fn all_off(&mut self) {
        for v in self.voices.iter_mut() {
            v.on = false;
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        let get = |i: usize| p.get(i).copied().unwrap_or(SPECS[i].default);
        let sr = self.sr;
        let n = l.len().min(r.len());
        l[..n].fill(0.0);
        r[..n].fill(0.0);
        let regs = [get(CELLO), get(VIOLA), get(VIOLIN)];
        let octave = [0.5f32, 1.0, 2.0];
        let attack = 1.0 / (get(ATTACK).max(1.0) * 0.001 * sr);
        let release = (-6.9 / (get(RELEASE).max(1.0) * 0.001 * sr)).exp();
        let ensemble = get(ENSEMBLE);
        let speed = get(SPEED);
        let bite = get(BITE);
        let age = get(AGE);
        let volume = get(VOLUME);
        self.low.set_q(get(TONE).min(sr * 0.45), 0.707, sr);
        let (slow_inc, fast_inc) = (0.61 * speed / sr, 5.9 * speed / sr);
        let size = self.line.len();
        for i in 0..n {
            // Its age: the pitch of everything wanders a little, together.
            if self.wander_in == 0 {
                self.wander_goal = self.rng.bipolar();
                self.wander_in = (sr * 0.4) as u32;
            }
            self.wander_in -= 1;
            self.wander += (self.wander_goal - self.wander) * (3.0 / sr);
            let wobble = 1.0 + self.wander * age * 0.012 + self.pitch_bend_ratio_minus_one();
            let mut x = 0.0;
            for v in self.voices.iter_mut() {
                if v.on {
                    v.level = (v.level + attack).min(1.0);
                } else if v.level > 0.0 {
                    v.level *= release;
                    if v.level < 1e-4 {
                        v.level = 0.0;
                    }
                }
                if v.level <= 0.0 {
                    continue;
                }
                let hz = 440.0 * 2f32.powf((v.note as f32 - 69.0) / 12.0) * wobble;
                let mut s = 0.0;
                for k in 0..3 {
                    if regs[k] <= 0.0 {
                        continue;
                    }
                    let dt = (hz * octave[k] / sr).min(0.45);
                    v.phase[k] = (v.phase[k] + dt).fract();
                    s += saw(v.phase[k], dt) * regs[k];
                }
                x += s * v.level * v.velocity;
            }
            x *= 0.16;
            let hp = self.high.tick(x).high;
            let tone = self.low.process(hp, FilterMode::Low);
            let a = self.bite[0].tick(tone).band;
            let b = self.bite[1].tick(tone).band;
            let voiced = tone + bite * 0.35 * (a + b);
            // Three lines, a third of a turn apart, swayed slowly and fast.
            self.slow = (self.slow + slow_inc).fract();
            self.fast = (self.fast + fast_inc).fract();
            self.line[self.write] = voiced;
            let mut taps = [0.0f32; 3];
            for (k, t) in taps.iter_mut().enumerate() {
                let turn = k as f32 / 3.0;
                let sway = 0.0035 * ((self.slow + turn).fract() * std::f32::consts::TAU).sin() + 0.0006 * ((self.fast + turn).fract() * std::f32::consts::TAU).sin();
                let delay = (0.012 + sway) * sr;
                let at = self.write as f32 + size as f32 - delay;
                let i0 = at.floor() as usize % size;
                let i1 = (i0 + 1) % size;
                let f = at.fract();
                *t = self.line[i0] + (self.line[i1] - self.line[i0]) * f;
            }
            self.write = (self.write + 1) % size;
            let wet_l = (taps[0] + 0.7 * taps[1] + 0.3 * taps[2]) * 0.6;
            let wet_r = (taps[2] + 0.7 * taps[1] + 0.3 * taps[0]) * 0.6;
            l[i] += fast_tanh((voiced * (1.0 - ensemble) + wet_l * ensemble) * 1.2) * volume;
            r[i] += fast_tanh((voiced * (1.0 - ensemble) + wet_r * ensemble) * 1.2) * volume;
        }
    }

    #[inline]
    fn pitch_bend_ratio_minus_one(&self) -> f32 {
        if self.pitch_bend == 0.0 {
            0.0
        } else {
            2f32.powf(self.pitch_bend / 12.0) - 1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn play(s: &mut StringMachineState, p: &[f32], seconds: f32) -> (Vec<f32>, Vec<f32>) {
        let n = (seconds * SR) as usize;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            s.render(p, a, b);
        }
        (l, r)
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    #[test]
    fn it_swells_and_fades_with_one_slow_hand() {
        let mut p = crate::params::defaults(SPECS);
        p[ATTACK] = 500.0;
        p[RELEASE] = 300.0;
        let mut s = StringMachineState::new(SR);
        s.note_on(&p, 60, 1.0);
        let (a, _) = play(&mut s, &p, 0.1);
        let (b, _) = play(&mut s, &p, 0.6);
        assert!(rms(&a) < rms(&b[b.len() / 2..]) * 0.6, "{} {}", rms(&a), rms(&b));
        s.note_off(&p, 60);
        let (c, _) = play(&mut s, &p, 1.2);
        assert!(rms(&c[c.len() - 2400..]) < 0.002, "{}", rms(&c[c.len() - 2400..]));
    }

    #[test]
    fn the_ensemble_makes_it_wide() {
        let mut p = crate::params::defaults(SPECS);
        p[ENSEMBLE] = 0.0;
        let mut s = StringMachineState::new(SR);
        s.note_on(&p, 60, 1.0);
        play(&mut s, &p, 0.5);
        let (l, r) = play(&mut s, &p, 0.5);
        let side = |l: &[f32], r: &[f32]| rms(&l.iter().zip(r).map(|(a, b)| a - b).collect::<Vec<_>>());
        assert!(side(&l, &r) < 1e-4, "dry: {}", side(&l, &r));
        p[ENSEMBLE] = 1.0;
        let (l, r) = play(&mut s, &p, 0.5);
        assert!(side(&l, &r) > rms(&l) * 0.3, "wet: {} of {}", side(&l, &r), rms(&l));
    }

    #[test]
    fn registers_sit_an_octave_apart() {
        let centre = |reg: usize| {
            let mut p = crate::params::defaults(SPECS);
            p[CELLO] = 0.0;
            p[VIOLA] = 0.0;
            p[VIOLIN] = 0.0;
            p[reg] = 1.0;
            p[ENSEMBLE] = 0.0;
            p[BITE] = 0.0;
            p[ATTACK] = 1.0;
            let mut s = StringMachineState::new(SR);
            s.note_on(&p, 60, 1.0);
            play(&mut s, &p, 0.2);
            let (x, _) = play(&mut s, &p, 0.5);
            let ups = (1..x.len()).filter(|&i| x[i - 1] < 0.0 && x[i] >= 0.0).count() as f32;
            ups / 0.5
        };
        let (low, mid, high) = (centre(CELLO), centre(VIOLA), centre(VIOLIN));
        assert!(low < mid && mid < high, "{low} {mid} {high}");
    }

    #[test]
    fn a_full_chord_stays_in_bounds_in_every_preset() {
        for name in preset_names() {
            let p = preset(name);
            assert_eq!(p.len(), SPECS.len());
            let mut s = StringMachineState::new(SR);
            for n in [48, 55, 60, 64, 67, 72, 76, 79] {
                s.note_on(&p, n, 1.0);
            }
            let (x, y) = play(&mut s, &p, 2.0);
            let top = x.iter().chain(&y).fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(top > 0.05 && top <= 1.0 && x.iter().all(|v| v.is_finite()), "{name}: {top}");
        }
    }
}
