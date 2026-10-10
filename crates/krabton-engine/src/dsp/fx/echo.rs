//! Echo: a tape-style delay. The tape wobbles (wow) and flutters, the
//! repeats saturate and lose their highs and lows, and the feedback can go
//! past 100 % into self-oscillation, held in check by the saturation.

use super::DelayLine;
use crate::dsp::effects::FxCtx;
use crate::dsp::{fast_tanh, OnePole, Rng};
use crate::params::{ParamSpec, Unit};
use std::f32::consts::TAU;

pub const SYNC: usize = 0;
pub const TIME: usize = 1;
pub const FEEDBACK: usize = 2;
pub const WOW: usize = 3;
pub const FLUTTER: usize = 4;
pub const SATURATION: usize = 5;
pub const TONE: usize = 6;
pub const LOW_CUT: usize = 7;
pub const SPREAD: usize = 8;
pub const MIX: usize = 9;

pub const SYNCS: &[&str] = &["Free", "1/16", "1/16 D", "1/8", "1/8 D", "1/4", "1/4 D", "1/2", "1 Bar"];
const SYNC_BEATS: [f32; 9] = [0.0, 0.25, 0.375, 0.5, 0.75, 1.0, 1.5, 2.0, 4.0];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Sync", "Time", SYNCS, 4),
    ParamSpec::log("Time", "Time", 10.0, 2000.0, 375.0, Unit::Ms),
    ParamSpec::lin("Feedback", "Time", 0.0, 1.1, 0.45, Unit::Percent),
    ParamSpec::lin("Wow", "Tape", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::lin("Flutter", "Tape", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Saturation", "Tape", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::log("Tone", "Tape", 500.0, 16000.0, 4500.0, Unit::Hz),
    ParamSpec::log("Low Cut", "Tape", 20.0, 1000.0, 120.0, Unit::Hz),
    ParamSpec::lin("Spread", "Output", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 0.35, Unit::Percent),
];

pub struct Echo {
    lines: [DelayLine; 2],
    tone: [OnePole; 2],
    low: [OnePole; 2],
    /// The delay time glides to new settings like a tape speed change.
    time: f32,
    wow_phase: f32,
    flutter_phase: f32,
    drift: f32,
    rng: Rng,
}

impl Echo {
    pub fn new(sr: f32) -> Self {
        let len = (sr * 4.2) as usize;
        Self {
            lines: [DelayLine::new(len), DelayLine::new(len)],
            tone: [OnePole::default(); 2],
            low: [OnePole::default(); 2],
            time: -1.0,
            wow_phase: 0.0,
            flutter_phase: 0.0,
            drift: 0.0,
            rng: Rng::new(0xEC40),
        }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let sr = ctx.sr;
        let k = (p[SYNC].round().max(0.0) as usize).min(SYNC_BEATS.len() - 1);
        let secs = if k == 0 { p[TIME] * 0.001 } else { SYNC_BEATS[k] * 60.0 / ctx.bpm.max(1.0) };
        let max = (self.lines[0].len() - 8) as f32;
        let target = (secs * sr).clamp(4.0, max * 0.95);
        if self.time < 0.0 {
            self.time = target;
        }
        let fb = p[FEEDBACK].clamp(0.0, 1.1);
        let (wow, flutter) = (p[WOW].clamp(0.0, 1.0), p[FLUTTER].clamp(0.0, 1.0));
        let sat = p[SATURATION].clamp(0.0, 1.0);
        let drive = 1.0 + 4.0 * sat;
        for c in self.tone.iter_mut() {
            c.set(p[TONE].clamp(200.0, 0.45 * sr), sr);
        }
        for c in self.low.iter_mut() {
            c.set(p[LOW_CUT].clamp(10.0, 2000.0), sr);
        }
        let spread = p[SPREAD].clamp(0.0, 1.0);
        let mix = p[MIX].clamp(0.0, 1.0);
        for i in 0..n {
            self.time += (target - self.time) * (3.0 / sr);
            self.wow_phase = super::wrap(self.wow_phase + 0.55 / sr);
            self.flutter_phase = super::wrap(self.flutter_phase + 7.3 / sr);
            // A slow random walk on top of the wow.
            self.drift = (self.drift + self.rng.bipolar() * 0.002).clamp(-1.0, 1.0) * 0.9999;
            let wobble = wow * (0.006 * sr) * (0.7 * (self.wow_phase * TAU).sin() + 0.3 * self.drift) + flutter * (0.0006 * sr) * (self.flutter_phase * TAU).sin();
            let (xl, xr) = (l[i], r[i]);
            let mut wet = [0.0f32; 2];
            for ch in 0..2 {
                // The right side runs a little later when spread.
                let d = self.time * (1.0 + if ch == 1 { 0.25 * spread } else { 0.0 }) + wobble;
                let y = self.lines[ch].read(d.clamp(2.0, max));
                wet[ch] = y;
            }
            for ch in 0..2 {
                let x = if ch == 0 { xl } else { xr };
                // Repeats cross over a little when spread, like ping-pong.
                let back = wet[ch] * (1.0 - 0.5 * spread) + wet[1 - ch] * 0.5 * spread;
                let mut w = x + fb * back;
                // Unity gain for quiet repeats, squashed when loud.
                w = fast_tanh(w * drive) / drive;
                w = self.low[ch].hp(self.tone[ch].lp(w));
                if !w.is_finite() {
                    w = 0.0;
                    self.lines[ch].clear();
                }
                self.lines[ch].write(w);
            }
            l[i] = xl * (1.0 - mix) + wet[0] * mix;
            r[i] = xr * (1.0 - mix) + wet[1] * mix;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::*;
    use super::*;

    #[test]
    fn a_click_comes_back_later_and_dies_away() {
        let mut e = Echo::new(48000.0);
        let mut p = crate::params::defaults(SPECS);
        p[SYNC] = 0.0;
        p[TIME] = 100.0;
        p[MIX] = 1.0;
        p[WOW] = 0.0;
        p[FLUTTER] = 0.0;
        let mut l = vec![0.0f32; 48000 * 4];
        l[0] = 1.0;
        let mut r = l.clone();
        for (cl, cr) in l.chunks_mut(512).zip(r.chunks_mut(512)) {
            e.process(&p, cl, cr, &ctx());
        }
        let first = (0..20000).find(|&i| l[i].abs() > 0.05).unwrap();
        assert!((4700..5000).contains(&first), "{first}");
        assert!(peak(&l[48000 * 3..]) < 0.01);
    }

    #[test]
    fn past_full_feedback_it_stays_bounded() {
        let mut e = Echo::new(48000.0);
        let mut p = crate::params::defaults(SPECS);
        p[FEEDBACK] = 1.1;
        p[SATURATION] = 1.0;
        let mut l = tone(500.0, 5.0, 0.8);
        let mut r = l.clone();
        for (cl, cr) in l.chunks_mut(512).zip(r.chunks_mut(512)) {
            e.process(&p, cl, cr, &ctx());
        }
        assert!(l.iter().all(|x| x.is_finite()) && peak(&l) < 4.0);
    }
}
