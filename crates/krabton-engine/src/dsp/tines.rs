//! Tines: the electric piano of the seventies. A hammer strikes a small steel
//! tine: a ping that is gone at once, a sine that rings long (longest in the
//! bass) and a bar next to it, an octave and a bit up, that dies sooner. A coil
//! beside the tine hears it, unevenly: the harder it swings the more it barks.
//! Then the suitcase's tremolo, going from left to right.

use super::{fast_tanh, FilterMode, Svf};
use crate::params::{ParamSpec, Unit};

pub const BARK: usize = 0;
pub const HAMMER: usize = 1;
pub const BELL: usize = 2;
pub const DECAY: usize = 3;
pub const RELEASE: usize = 4;
pub const TREMOLO: usize = 5;
pub const RATE: usize = 6;
pub const TONE: usize = 7;
pub const DRIFT: usize = 8;
pub const VOLUME: usize = 9;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Bark", "Tine", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Hammer", "Tine", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Bell", "Tine", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::log("Decay", "Tine", 0.2, 4.0, 1.0, Unit::Percent),
    ParamSpec::log("Release", "Tine", 20.0, 1500.0, 140.0, Unit::Ms),
    ParamSpec::lin("Tremolo", "Suitcase", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::log("Rate", "Suitcase", 0.5, 10.0, 4.5, Unit::Hz),
    ParamSpec::log("Tone", "Suitcase", 500.0, 16000.0, 7000.0, Unit::Hz),
    ParamSpec::lin("Drift", "Suitcase", 0.0, 1.0, 0.1, Unit::Percent),
    ParamSpec::lin("Volume", "Output", 0.0, 1.0, 0.8, Unit::Percent),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Suitcase", "Clean Stage", "Hard Bark", "Bell Ballad", "Dream Keys", "Warm Felt", "Worn Out", "Glass"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let set: &[(usize, f32)] = match name {
        "Clean Stage" => &[(BARK, 0.15), (TREMOLO, 0.0), (HAMMER, 0.25)],
        "Hard Bark" => &[(BARK, 0.9), (HAMMER, 0.6), (TREMOLO, 0.0), (TONE, 10000.0)],
        "Bell Ballad" => &[(BELL, 0.9), (BARK, 0.1), (HAMMER, 0.5), (TREMOLO, 0.2), (DECAY, 1.6)],
        "Dream Keys" => &[(TREMOLO, 0.7), (RATE, 2.8), (TONE, 3500.0), (RELEASE, 700.0), (DECAY, 2.0), (BARK, 0.1)],
        "Warm Felt" => &[(HAMMER, 0.05), (BARK, 0.05), (BELL, 0.05), (TONE, 1800.0), (TREMOLO, 0.15)],
        "Worn Out" => &[(DRIFT, 0.7), (TONE, 2800.0), (TREMOLO, 0.4), (BARK, 0.3)],
        "Glass" => &[(BELL, 1.0), (HAMMER, 0.8), (BARK, 0.0), (DECAY, 0.5), (TONE, 14000.0), (TREMOLO, 0.0)],
        _ => &[],
    };
    for &(i, v) in set {
        p[i] = v;
    }
    p
}

const VOICES: usize = 24;

#[derive(Clone, Copy, Default)]
struct Voice {
    note: u8,
    held: bool,
    alive: bool,
    phase: [f32; 3],
    /// The tine, the bar, the ping.
    level: [f32; 3],
    damp: f32,
    velocity: f32,
    age: u64,
}

pub struct TinesState {
    sr: f32,
    pub pitch_bend: f32,
    voices: [Voice; VOICES],
    clock: u64,
    tremolo: f32,
    drift: f32,
    tone: [Svf; 2],
}

impl TinesState {
    pub fn new(sr: f32) -> Self {
        Self { sr, pitch_bend: 0.0, voices: [Voice::default(); VOICES], clock: 0, tremolo: 0.0, drift: 0.0, tone: [Svf::new(); 2] }
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        let get = |i: usize| p.get(i).copied().unwrap_or(SPECS[i].default);
        self.clock += 1;
        let k = (0..VOICES)
            .find(|&i| self.voices[i].alive && self.voices[i].note == note)
            .or_else(|| (0..VOICES).find(|&i| !self.voices[i].alive))
            .unwrap_or_else(|| (0..VOICES).min_by_key(|&i| (self.voices[i].held, self.voices[i].age)).unwrap_or(0));
        let v = &mut self.voices[k];
        let vel = vel.clamp(0.0, 1.0);
        let top = (note as f32 / 127.0).clamp(0.0, 1.0);
        *v = Voice {
            note,
            held: true,
            alive: true,
            phase: [0.0; 3],
            // The bar and the ping come through harder, higher up and when struck hard.
            level: [1.0, get(BELL) * (0.15 + 0.7 * vel) * (0.5 + top), get(HAMMER) * vel * vel * (0.4 + 1.2 * top)],
            damp: 1.0,
            velocity: 0.25 + 0.75 * vel,
            age: self.clock,
        };
    }

    pub fn note_off(&mut self, _p: &[f32], note: u8) {
        for v in self.voices.iter_mut().filter(|v| v.held && v.note == note) {
            v.held = false;
        }
    }

    pub fn all_off(&mut self) {
        for v in self.voices.iter_mut() {
            v.held = false;
        }
    }

    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        let get = |i: usize| p.get(i).copied().unwrap_or(SPECS[i].default);
        let sr = self.sr;
        let n = l.len().min(r.len());
        l[..n].fill(0.0);
        r[..n].fill(0.0);
        let bark = get(BARK);
        let decay = get(DECAY).max(0.05);
        let damp_k = (-6.9 / (get(RELEASE).max(1.0) * 0.001 * sr)).exp();
        let trem = get(TREMOLO);
        let rate = get(RATE);
        let drift = get(DRIFT);
        let volume = get(VOLUME);
        let bend = if self.pitch_bend == 0.0 { 1.0 } else { 2f32.powf(self.pitch_bend / 12.0) };
        let cutoff = get(TONE).min(sr * 0.45);
        for f in self.tone.iter_mut() {
            f.set_q(cutoff, 0.707, sr);
        }
        let tau = std::f32::consts::TAU;
        for i in 0..n {
            self.drift = (self.drift + 0.21 / sr).fract();
            let wow = 1.0 + drift * 0.004 * (self.drift * tau).sin() + drift * 0.0015 * (self.drift * 11.3 * tau).sin();
            let mut x = 0.0;
            for v in self.voices.iter_mut().filter(|v| v.alive) {
                let hz = 440.0 * 2f32.powf((v.note as f32 - 69.0) / 12.0) * bend * wow;
                // The low tines ring for seconds; the top ones not.
                let top = (v.note as f32 - 24.0).max(0.0) / 84.0;
                let t60 = (7.0 * (1.0 - top * 0.85)).max(0.5) * decay;
                v.level[0] *= (-6.9 / (t60 * sr)).exp();
                v.level[1] *= (-6.9 / (t60 * 0.3 * sr)).exp();
                v.level[2] *= (-6.9 / (0.03 * sr)).exp();
                if !v.held {
                    v.damp *= damp_k;
                }
                let incs = [hz, (hz * 4.07).min(sr * 0.45), (hz * 12.3).min(sr * 0.45)];
                let mut s = [0.0f32; 3];
                for k in 0..3 {
                    v.phase[k] = (v.phase[k] + incs[k] / sr).fract();
                    s[k] = (v.phase[k] * tau).sin() * v.level[k];
                }
                // The coil: a swing nearer it counts more than a swing away from it.
                let swing = (s[0] + 0.5 * s[1]) * v.velocity;
                let heard = swing + bark * 0.9 * swing * swing + bark * 0.4 * swing * swing * swing;
                x += (heard + s[2] * 0.25) * v.damp;
                if v.damp < 1e-4 || v.level[0] < 1e-4 {
                    v.alive = false;
                }
            }
            x *= 0.32;
            let y = fast_tanh(x);
            let y = self.tone[0].process(y, FilterMode::Low);
            self.tremolo = (self.tremolo + rate / sr).fract();
            let side = (self.tremolo * tau).sin() * trem;
            l[i] += y * (1.0 - 0.5 * trem + 0.5 * side) * volume;
            r[i] += y * (1.0 - 0.5 * trem - 0.5 * side) * volume;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn play(s: &mut TinesState, p: &[f32], seconds: f32) -> (Vec<f32>, Vec<f32>) {
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

    /// How much of a tone is in it.
    fn at(x: &[f32], hz: f32) -> f32 {
        let (mut re, mut im) = (0.0f32, 0.0f32);
        for (i, v) in x.iter().enumerate() {
            let a = std::f32::consts::TAU * hz * i as f32 / SR;
            re += v * a.cos();
            im += v * a.sin();
        }
        (re * re + im * im).sqrt() / x.len() as f32
    }

    #[test]
    fn it_rings_on_its_note() {
        let mut s = TinesState::new(SR);
        let p = crate::params::defaults(SPECS);
        s.note_on(&p, 69, 0.8);
        let (x, _) = play(&mut s, &p, 0.5);
        let body = &x[4800..];
        assert!(at(body, 440.0) > 3.0 * at(body, 400.0), "{} {}", at(body, 440.0), at(body, 400.0));
    }

    #[test]
    fn struck_harder_it_barks() {
        // The first overtone, against the note: more of it from a hard blow.
        let ratio = |vel: f32| {
            let mut p = crate::params::defaults(SPECS);
            p[BARK] = 0.9;
            p[BELL] = 0.0;
            p[HAMMER] = 0.0;
            p[TREMOLO] = 0.0;
            let mut s = TinesState::new(SR);
            s.note_on(&p, 57, vel);
            let (x, _) = play(&mut s, &p, 0.3);
            at(&x[2400..], 440.0) / at(&x[2400..], 220.0)
        };
        let (soft, hard) = (ratio(0.15), ratio(1.0));
        assert!(hard > soft * 1.5, "{soft} {hard}");
    }

    #[test]
    fn low_notes_ring_longer_than_high_ones() {
        let tail = |note: u8| {
            let p = crate::params::defaults(SPECS);
            let mut s = TinesState::new(SR);
            s.note_on(&p, note, 0.8);
            let (x, _) = play(&mut s, &p, 2.5);
            rms(&x[x.len() - 4800..]) / rms(&x[..4800])
        };
        assert!(tail(36) > tail(96) * 2.0, "{} {}", tail(36), tail(96));
    }

    #[test]
    fn the_tremolo_goes_from_side_to_side() {
        let mut p = crate::params::defaults(SPECS);
        p[TREMOLO] = 1.0;
        p[RATE] = 4.0;
        let mut s = TinesState::new(SR);
        s.note_on(&p, 60, 0.8);
        let (l, r) = play(&mut s, &p, 0.5);
        // A quarter turn in the left is loudest and the right quietest.
        let (a, b) = (rms(&l[2000..4000]), rms(&r[2000..4000]));
        assert!(a > b * 2.0, "{a} {b}");
    }

    #[test]
    fn letting_go_ends_it_and_every_preset_plays() {
        for name in preset_names() {
            let p = preset(name);
            assert_eq!(p.len(), SPECS.len());
            let mut s = TinesState::new(SR);
            for n in [48, 60, 64, 67, 71] {
                s.note_on(&p, n, 1.0);
            }
            let (x, _) = play(&mut s, &p, 0.3);
            let top = x.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(top > 0.05 && top < 1.2 && x.iter().all(|v| v.is_finite()), "{name}: {top}");
            s.all_off();
            let (x, _) = play(&mut s, &p, 3.0);
            assert!(rms(&x[x.len() - 2400..]) < 0.002, "{name}");
            assert!(s.voices.iter().all(|v| !v.alive), "{name}");
        }
    }
}
