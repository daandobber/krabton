//! Step Gate: the volume chopped to a pattern of steps that follows the song.
//! Each step has its own level, the steps can go from side to side, and the
//! edges are as hard or as soft as asked. A pad becomes a rhythm.

use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};

pub const RATE: usize = 0;
pub const STEPS: usize = 1;
pub const DEPTH: usize = 2;
pub const SMOOTH: usize = 3;
pub const SPREAD: usize = 4;
pub const FIRST_STEP: usize = 5;
pub const MAX_STEPS: usize = 16;

pub const RATES: &[&str] = &["1/32", "1/16 T", "1/16", "1/8 T", "1/8", "1/4"];
const RATE_BEATS: [f64; 6] = [0.125, 1.0 / 6.0, 0.25, 1.0 / 3.0, 0.5, 1.0];

const fn step(name: &'static str, on: f32) -> ParamSpec {
    ParamSpec::lin(name, "Steps", 0.0, 1.0, on, Unit::Percent)
}

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Rate", "Gate", RATES, 2),
    ParamSpec::int("Steps", "Gate", 2.0, 16.0, 16.0, Unit::None),
    ParamSpec::lin("Depth", "Gate", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::log("Smooth", "Gate", 0.5, 60.0, 4.0, Unit::Ms),
    ParamSpec::lin("Spread", "Gate", 0.0, 1.0, 0.0, Unit::Percent),
    step("Step 1", 1.0),
    step("Step 2", 0.0),
    step("Step 3", 1.0),
    step("Step 4", 1.0),
    step("Step 5", 0.0),
    step("Step 6", 1.0),
    step("Step 7", 0.0),
    step("Step 8", 1.0),
    step("Step 9", 1.0),
    step("Step 10", 0.0),
    step("Step 11", 1.0),
    step("Step 12", 1.0),
    step("Step 13", 0.0),
    step("Step 14", 1.0),
    step("Step 15", 0.6),
    step("Step 16", 0.3),
];

/// Which step a place in the song is on.
pub fn step_at(p: &[f32], beat: f64) -> usize {
    let rate = RATE_BEATS[(p[RATE].round().max(0.0) as usize).min(RATE_BEATS.len() - 1)];
    let steps = (p[STEPS].round().max(1.0) as i64).min(MAX_STEPS as i64);
    ((beat / rate).floor() as i64).rem_euclid(steps) as usize
}

pub struct StepGate {
    sr: f32,
    gain: [f32; 2],
    /// The beat when the transport is stopped (it keeps chopping at the tempo).
    free_beat: f64,
    /// The step sounding, for the panel.
    pub now: usize,
}

impl StepGate {
    pub fn new(sr: f32) -> Self {
        Self { sr, gain: [1.0; 2], free_beat: 0.0, now: 0 }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let depth = p[DEPTH].clamp(0.0, 1.0);
        let spread = p[SPREAD].clamp(0.0, 1.0);
        let k = 1.0 - (-1.0 / (p[SMOOTH].max(0.1) * 0.001 * self.sr)).exp();
        let per_frame = ctx.bpm as f64 / 60.0 / self.sr as f64;
        let start = if ctx.playing { ctx.beat } else { self.free_beat };
        for i in 0..n {
            let s = step_at(p, start + i as f64 * per_frame);
            let level = p.get(FIRST_STEP + s).copied().unwrap_or(1.0).clamp(0.0, 1.0);
            let g = 1.0 - depth * (1.0 - level);
            // Odd steps lean one way and even ones the other.
            let (a, b) = if s % 2 == 0 { (1.0, 1.0 - spread) } else { (1.0 - spread, 1.0) };
            self.gain[0] += (g * a - self.gain[0]) * k;
            self.gain[1] += (g * b - self.gain[1]) * k;
            l[i] *= self.gain[0];
            r[i] *= self.gain[1];
            self.now = s;
        }
        self.free_beat = start + n as f64 * per_frame;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::{Scale, ScaleKind};

    const SR: f32 = 48000.0;

    fn ctx(beat: f64, playing: bool) -> FxCtx {
        FxCtx { sr: SR, bpm: 120.0, beat, playing, scale: Scale::new(0, ScaleKind::Minor), note: None }
    }

    fn pattern(on: &[f32]) -> Vec<f32> {
        let mut p = crate::params::defaults(SPECS);
        p[STEPS] = on.len() as f32;
        p[SMOOTH] = 0.5;
        for (k, v) in on.iter().enumerate() {
            p[FIRST_STEP + k] = *v;
        }
        p
    }

    /// One second of ones through it, from the start of the song.
    fn run(p: &[f32]) -> (Vec<f32>, Vec<f32>) {
        let n = SR as usize;
        let (mut l, mut r) = (vec![1.0; n], vec![1.0; n]);
        let mut g = StepGate::new(SR);
        let mut done = 0;
        while done < n {
            let m = (n - done).min(256);
            let beat = done as f64 / SR as f64 * 2.0;
            let (a, b) = (&mut l[done..done + m], &mut r[done..done + m]);
            g.process(p, a, b, &ctx(beat, true));
            done += m;
        }
        (l, r)
    }

    #[test]
    fn it_opens_and_closes_on_the_steps_of_the_song() {
        // A sixteenth at 120 is 6000 frames.
        let (l, _) = run(&pattern(&[1.0, 0.0, 1.0, 0.25]));
        assert!(l[5000] > 0.99, "{}", l[5000]);
        assert!(l[11000] < 0.01, "{}", l[11000]);
        assert!(l[17000] > 0.99, "{}", l[17000]);
        assert!((l[23000] - 0.25).abs() < 0.01, "{}", l[23000]);
        // And round again.
        assert!(l[29000] > 0.99 && l[35000] < 0.01);
    }

    #[test]
    fn depth_says_how_far_it_closes() {
        let mut p = pattern(&[1.0, 0.0]);
        p[DEPTH] = 0.5;
        let (l, _) = run(&p);
        assert!((l[11000] - 0.5).abs() < 0.01, "{}", l[11000]);
    }

    #[test]
    fn spread_sends_the_steps_from_side_to_side() {
        let mut p = pattern(&[1.0, 1.0]);
        p[SPREAD] = 1.0;
        let (l, r) = run(&p);
        assert!(l[5000] > 0.99 && r[5000] < 0.01);
        assert!(l[11000] < 0.01 && r[11000] > 0.99);
    }

    #[test]
    fn its_edges_do_not_click() {
        let mut p = pattern(&[1.0, 0.0]);
        p[SMOOTH] = 10.0;
        let (l, _) = run(&p);
        let jump = l.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max);
        assert!(jump < 0.01, "{jump}");
    }

    #[test]
    fn stopped_it_keeps_going() {
        let p = pattern(&[1.0, 0.0]);
        let mut g = StepGate::new(SR);
        let n = 12000;
        let (mut l, mut r) = (vec![1.0; n], vec![1.0; n]);
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            g.process(&p, a, b, &ctx(0.0, false));
        }
        assert!(l[5000] > 0.99 && l[11000] < 0.01, "{} {}", l[5000], l[11000]);
    }

    #[test]
    fn a_long_run_lands_on_the_right_step() {
        let p = pattern(&[1.0, 0.0, 0.0, 0.0]);
        assert_eq!(step_at(&p, 100.0), 0);
        assert_eq!(step_at(&p, 100.25), 1);
        assert_eq!(step_at(&p, -0.25), 3, "before the song starts");
    }
}
