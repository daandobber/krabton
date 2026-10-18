//! Warps: the meta-modulator of Mutable Instruments (see `krabton_warps`):
//! the track against an internal oscillator through crossfading, folding,
//! diode or digital ring modulation, XOR, a comparator, or a vocoder whose
//! formants shift. One Warps a side (the right one's carrier a little
//! detuned for width); the dry signal delayed as much as Warps delays, so
//! the mix does not comb.

use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};

pub const ALGORITHM: usize = 0;
pub const TIMBRE: usize = 1;
pub const DRIVE: usize = 2;
pub const NOTE: usize = 3;
pub const SHAPE: usize = 4;
pub const DETUNE: usize = 5;
pub const MIX: usize = 6;
pub const OUTPUT: usize = 7;

pub const SHAPES: &[&str] = &["Sine", "Triangle", "Saw"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Algorithm", "Warps", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Timbre", "Warps", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Drive", "Warps", 0.0, 1.0, 0.6, Unit::Percent),
    ParamSpec::lin("Note", "Carrier", 12.0, 108.0, 48.0, Unit::Semi),
    ParamSpec::choice("Shape", "Carrier", SHAPES, 0),
    ParamSpec::lin("Detune", "Carrier", 0.0, 50.0, 0.0, Unit::Cents),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Output", "Output", -24.0, 12.0, 0.0, Unit::Db),
];

/// What the algorithm knob is on now.
pub fn algorithm_name(a: f32) -> &'static str {
    if a > 0.7 {
        return "Vocoder";
    }
    ["Crossfade", "Fold", "Analog Ring", "Digital Ring", "XOR", "Comparator"][((a * 8.0).min(5.999) + 0.5) as usize % 6]
}

pub struct Warps {
    w: [Box<krabton_warps::Warps>; 2],
    dry: [Vec<f32>; 2],
    pos: usize,
    wet: [Vec<f32>; 2],
}

impl Warps {
    pub fn new(sr: f32) -> Self {
        let w = [Box::new(krabton_warps::Warps::new(sr)), Box::new(krabton_warps::Warps::new(sr))];
        let latency = w[0].latency().max(1);
        Self { w, dry: [vec![0.0; latency], vec![0.0; latency]], pos: 0, wet: [vec![0.0; 1024], vec![0.0; 1024]] }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], _ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let get = |i: usize| p.get(i).copied().unwrap_or(SPECS[i].default);
        let shape = get(SHAPE).round().clamp(0.0, 2.0) as i32 + 1;
        let note = get(NOTE);
        let detune = get(DETUNE) / 100.0;
        let base = krabton_warps::Params { algorithm: get(ALGORITHM), timbre: get(TIMBRE), drive: get(DRIVE), note, shape };
        let params = [base, krabton_warps::Params { note: note + detune, ..base }];
        let mix = get(MIX).clamp(0.0, 1.0);
        let out = 10f32.powf(get(OUTPUT) / 20.0);
        let mut done = 0;
        while done < n {
            let m = (n - done).min(1024);
            for (c, buf) in [&*l, &*r].into_iter().enumerate() {
                self.w[c].process(&params[c], &buf[done..done + m], &mut self.wet[c][..m]);
            }
            let len = self.dry[0].len();
            for k in 0..m {
                let i = done + k;
                for (c, buf) in [&mut *l, &mut *r].into_iter().enumerate() {
                    // The dry signal, as late as the wet one.
                    let delayed = self.dry[c][self.pos];
                    self.dry[c][self.pos] = buf[i];
                    let wet = self.wet[c][k];
                    buf[i] = (delayed + (wet - delayed) * mix) * out;
                }
                self.pos = (self.pos + 1) % len;
            }
            done += m;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_processes_stereo_and_names_its_algorithms() {
        let ctx = FxCtx { sr: 48000.0, bpm: 120.0, beat: 0.0, playing: false };
        let mut fx = Warps::new(48000.0);
        let p = crate::params::defaults(SPECS);
        let (mut l, mut r): (Vec<f32>, Vec<f32>) = (0..9600).map(|i| ((i as f32 * 0.03).sin() * 0.5, (i as f32 * 0.031).sin() * 0.5)).unzip();
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            fx.process(&p, a, b, &ctx);
        }
        assert!(l.iter().chain(&r).all(|x| x.is_finite()));
        assert!(l[4800..].iter().map(|x| x * x).sum::<f32>() > 1.0);
        assert_eq!(algorithm_name(0.0), "Crossfade");
        assert_eq!(algorithm_name(0.375), "Digital Ring");
        assert_eq!(algorithm_name(0.9), "Vocoder");
    }
}
