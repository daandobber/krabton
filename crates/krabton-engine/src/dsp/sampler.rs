//! Sampler: plays one sample chromatically across the keyboard.

use super::*;
use crate::params::{ParamSpec, Unit};
use crate::sample::SampleRef;

pub const ROOT: usize = 0;
pub const TRANSPOSE: usize = 1;
pub const FINE: usize = 2;
pub const START: usize = 3;
pub const LOOP: usize = 4;
pub const ATTACK: usize = 5;
pub const DECAY: usize = 6;
pub const SUSTAIN: usize = 7;
pub const RELEASE: usize = 8;
pub const CUTOFF: usize = 9;
pub const RESO: usize = 10;
pub const VEL_SENS: usize = 11;
pub const VOLUME: usize = 12;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::int("Root Key", "Sample", 0.0, 127.0, 60.0, Unit::None),
    ParamSpec::int("Transpose", "Sample", -48.0, 48.0, 0.0, Unit::Semi),
    ParamSpec::lin("Fine", "Sample", -50.0, 50.0, 0.0, Unit::Cents),
    ParamSpec::lin("Start", "Sample", 0.0, 0.99, 0.0, Unit::Percent),
    ParamSpec::toggle("Loop", "Sample", false),
    ParamSpec::log("Attack", "Envelope", 0.001, 5.0, 0.001, Unit::Sec),
    ParamSpec::log("Decay", "Envelope", 0.005, 10.0, 1.0, Unit::Sec),
    ParamSpec::lin("Sustain", "Envelope", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::log("Release", "Envelope", 0.005, 10.0, 0.1, Unit::Sec),
    ParamSpec::log("Cutoff", "Filter", 20.0, 20000.0, 20000.0, Unit::Hz),
    ParamSpec::lin("Resonance", "Filter", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel > Vol", "Global", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Volume", "Global", -36.0, 6.0, -3.0, Unit::Db),
];

const MAX_VOICES: usize = 16;

#[derive(Clone, Copy)]
struct Voice {
    active: bool,
    released: bool,
    note: u8,
    vel: f32,
    age: u64,
    pos: f64,
    amp: Adsr,
    filt: [Svf; 2],
}

pub struct SamplerState {
    voices: [Voice; MAX_VOICES],
    counter: u64,
}

impl Default for SamplerState {
    fn default() -> Self {
        Self::new()
    }
}

impl SamplerState {
    pub fn new() -> Self {
        let v = Voice {
            active: false,
            released: false,
            note: 0,
            vel: 0.0,
            age: 0,
            pos: 0.0,
            amp: Adsr::new(),
            filt: [Svf::new(); 2],
        };
        Self { voices: [v; MAX_VOICES], counter: 0 }
    }

    pub fn note_on(&mut self, p: &[f32], sample: Option<&SampleRef>, note: u8, vel: f32) {
        let Some(s) = sample else { return };
        self.counter += 1;
        let idx = self
            .voices
            .iter()
            .position(|v| !v.active)
            .unwrap_or_else(|| self.voices.iter().enumerate().min_by_key(|(_, v)| v.age).map(|(i, _)| i).unwrap_or(0));
        let v = &mut self.voices[idx];
        v.active = true;
        v.released = false;
        v.note = note;
        v.vel = vel;
        v.age = self.counter;
        v.pos = (p[START] as f64 * s.frames() as f64).floor();
        v.amp.reset();
        v.amp.trigger();
        for f in &mut v.filt {
            f.reset();
        }
    }

    pub fn note_off(&mut self, note: u8) {
        for v in self.voices.iter_mut() {
            if v.active && !v.released && v.note == note {
                v.amp.release();
                v.released = true;
            }
        }
    }

    pub fn all_off(&mut self) {
        for v in self.voices.iter_mut() {
            v.amp.release();
            v.released = true;
        }
    }

    pub fn render(&mut self, p: &[f32], sample: Option<&SampleRef>, l: &mut [f32], r: &mut [f32], sr: f32) {
        let Some(s) = sample else {
            for v in self.voices.iter_mut() {
                v.active = false;
            }
            return;
        };
        let n = l.len().min(r.len());
        let frames = s.frames() as f64;
        let start = (p[START] as f64 * frames).floor();
        let looping = p[LOOP] >= 0.5 && frames - start > 16.0;
        let amp_c = AdsrCoefs::new(p[ATTACK], p[DECAY], p[SUSTAIN], p[RELEASE], sr);
        let filter_on = p[CUTOFF] < 19_500.0;
        let vol = db_to_gain(p[VOLUME]);
        let vel_sens = p[VEL_SENS];
        for v in self.voices.iter_mut().filter(|v| v.active) {
            let semis = v.note as f32 - p[ROOT] + p[TRANSPOSE] + p[FINE] / 100.0;
            let rate = 2f64.powf(semis as f64 / 12.0) * s.sample_rate as f64 / sr as f64;
            if filter_on {
                v.filt[0].set(p[CUTOFF], p[RESO], sr);
                v.filt[1].set(p[CUTOFF], p[RESO], sr);
            }
            let g = vol * (1.0 - vel_sens + vel_sens * v.vel);
            for i in 0..n {
                if v.pos >= frames - 1.0 {
                    if looping {
                        v.pos = start + (v.pos - start) % (frames - 1.0 - start);
                    } else {
                        v.active = false;
                        break;
                    }
                }
                let (mut a, mut b) = s.read(v.pos);
                if filter_on {
                    a = v.filt[0].process(a, FilterMode::Low);
                    b = v.filt[1].process(b, FilterMode::Low);
                }
                let e = v.amp.next(&amp_c) * g;
                l[i] += a * e;
                r[i] += b * e;
                v.pos += rate;
                if !v.amp.is_active() {
                    v.active = false;
                    break;
                }
            }
        }
    }
}
