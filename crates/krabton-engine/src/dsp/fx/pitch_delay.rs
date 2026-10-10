//! Pitch Delay: a tempo-synced delay with a pitch shifter. In Climb mode
//! the shifter sits in the loop, so each echo climbs (or falls) one more
//! step: +12 gives echoes an octave, two octaves, three octaves up. In Fixed
//! mode every echo is shifted by the same interval.

use super::{DelayLine, PitchShifter};
use crate::dsp::effects::FxCtx;
use crate::dsp::{fast_tanh, OnePole};
use crate::params::{ParamSpec, Unit};

pub const SYNC: usize = 0;
pub const TIME: usize = 1;
pub const PITCH: usize = 2;
pub const FINE: usize = 3;
pub const MODE: usize = 4;
pub const FEEDBACK: usize = 5;
pub const TONE: usize = 6;
pub const LOW_CUT: usize = 7;
pub const PING_PONG: usize = 8;
pub const MIX: usize = 9;

pub const SYNCS: &[&str] = &["Free", "1/16", "1/16 D", "1/8", "1/8 D", "1/4", "1/4 D", "1/2", "1 Bar"];
const SYNC_BEATS: [f32; 9] = [0.0, 0.25, 0.375, 0.5, 0.75, 1.0, 1.5, 2.0, 4.0];
pub const MODES: &[&str] = &["Climb", "Fixed"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Sync", "Time", SYNCS, 5),
    ParamSpec::log("Time", "Time", 20.0, 2000.0, 400.0, Unit::Ms),
    ParamSpec::int("Pitch", "Pitch", -24.0, 24.0, 12.0, Unit::Semi),
    ParamSpec::lin("Fine", "Pitch", -50.0, 50.0, 0.0, Unit::Cents),
    ParamSpec::choice("Mode", "Pitch", MODES, 0),
    ParamSpec::lin("Feedback", "Time", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::log("Tone", "Tone", 500.0, 18000.0, 9000.0, Unit::Hz),
    ParamSpec::log("Low Cut", "Tone", 20.0, 1000.0, 80.0, Unit::Hz),
    ParamSpec::toggle("Ping Pong", "Output", false),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 0.4, Unit::Percent),
];

pub struct PitchDelay {
    lines: [DelayLine; 2],
    shifters: [PitchShifter; 2],
    tone: [OnePole; 2],
    low: [OnePole; 2],
    /// The delay time glides to new settings.
    time: f32,
}

impl PitchDelay {
    pub fn new(sr: f32) -> Self {
        let len = (sr * 4.2) as usize;
        Self {
            lines: [DelayLine::new(len), DelayLine::new(len)],
            shifters: [PitchShifter::new(sr, 0.12), PitchShifter::new(sr, 0.12)],
            tone: [OnePole::default(); 2],
            low: [OnePole::default(); 2],
            time: -1.0,
        }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let sr = ctx.sr;
        let k = (p[SYNC].round().max(0.0) as usize).min(SYNC_BEATS.len() - 1);
        let secs = if k == 0 { p[TIME] * 0.001 } else { SYNC_BEATS[k] * 60.0 / ctx.bpm.max(1.0) };
        let max = (self.lines[0].len() - 8) as f32;
        let target = (secs * sr).clamp(64.0, max * 0.95);
        if self.time < 0.0 {
            self.time = target;
        }
        let semis = p[PITCH].round() + p[FINE] * 0.01;
        let ratio = 2f32.powf(semis / 12.0);
        let shifting = semis.abs() > 1e-3;
        let every = p[MODE] < 0.5;
        let fb = p[FEEDBACK].clamp(0.0, 1.0);
        for c in self.tone.iter_mut() {
            c.set(p[TONE].clamp(200.0, 0.45 * sr), sr);
        }
        for c in self.low.iter_mut() {
            c.set(p[LOW_CUT].clamp(10.0, 2000.0), sr);
        }
        let ping = p[PING_PONG] >= 0.5;
        let mix = p[MIX].clamp(0.0, 1.0);
        // The shifter's window: shorter for short delays, so echoes stay tight.
        let window = (0.07 * sr).min(target * 0.5);
        // The shifter delays by half its window on average: read that much sooner.
        let comp = if shifting { 0.5 * window } else { 0.0 };
        for i in 0..n {
            self.time += (target - self.time) * (3.0 / sr);
            let (xl, xr) = (l[i], r[i]);
            let at = (self.time - comp).max(2.0);
            let delayed = [self.lines[0].read(at), self.lines[1].read(at)];
            let mut wet = delayed;
            if !every && shifting {
                for c in 0..2 {
                    wet[c] = self.shifters[c].tick(delayed[c], ratio, window);
                }
            }
            for c in 0..2 {
                let x = if c == 0 { xl } else { xr };
                // Ping-pong: the input goes in on the left, and the sides swap.
                let (input, back) = if ping { (if c == 0 { 0.5 * (xl + xr) } else { 0.0 }, delayed[1 - c]) } else { (x, delayed[c]) };
                let mut w = input + fb * back;
                if every && shifting {
                    w = self.shifters[c].tick(w, ratio, window);
                }
                w = self.low[c].hp(self.tone[c].lp(w));
                w = fast_tanh(w * 1.2) / 1.2;
                self.lines[c].write(if w.is_finite() { w } else { 0.0 });
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

    /// Strongest frequency in each echo of a 300 Hz burst.
    fn echoes(mode: f32, pitch: f32) -> Vec<f32> {
        let mut d = PitchDelay::new(48000.0);
        let mut p = crate::params::defaults(SPECS);
        p[SYNC] = 0.0;
        p[TIME] = 250.0;
        p[MIX] = 1.0;
        p[MODE] = mode;
        p[PITCH] = pitch;
        p[FEEDBACK] = 0.8;
        p[TONE] = 18000.0;
        let n = 48000;
        let mut l = vec![0.0f32; n];
        l[..4800].copy_from_slice(&tone(300.0, 0.1, 0.5));
        let mut r = l.clone();
        for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            d.process(&p, cl, cr, &ctx());
        }
        // Echo k sits around k * 250 ms.
        (1..=3).map(|k| strongest(&l[k * 12000 + 1200..k * 12000 + 4200], 100.0, 2600.0, 10.0)).collect()
    }

    #[test]
    fn every_repeat_climbs_an_octave_each_time() {
        let f = echoes(0.0, 12.0);
        // Within 5 % (the crossfading heads blur the spectrum a little).
        for (got, want) in f.iter().zip([600.0, 1200.0, 2400.0]) {
            assert!((got - want).abs() < 0.05 * want, "{f:?}");
        }
    }

    #[test]
    fn all_repeats_share_one_interval() {
        let f = echoes(1.0, -12.0);
        assert!(f.iter().all(|&x| (x - 150.0).abs() < 15.0), "{f:?}");
    }
}
