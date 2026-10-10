//! Beat Repeat: in time with the song, grabs a slice of what comes in and
//! repeats it (stutters, rolls), at an interval, with a chance, for a while.
//! Mix plays the repeats over the sound, Insert replaces it while they
//! play, Gate plays only the repeats.

use crate::dsp::effects::FxCtx;
use crate::dsp::Rng;
use crate::params::{ParamSpec, Unit};

pub const INTERVAL: usize = 0;
pub const OFFSET: usize = 1;
pub const GATE: usize = 2;
pub const GRID: usize = 3;
pub const VARIATION: usize = 4;
pub const CHANCE: usize = 5;
pub const DECAY: usize = 6;
pub const PITCH: usize = 7;
pub const MODE: usize = 8;

pub const INTERVALS: &[&str] = &["1/4", "1/2", "1 Bar", "2 Bars", "4 Bars"];
const INTERVAL_BEATS: [f64; 5] = [1.0, 2.0, 4.0, 8.0, 16.0];
pub const GRIDS: &[&str] = &["1/32", "1/16 T", "1/16", "1/8 T", "1/8", "1/4", "1/2"];
const GRID_BEATS: [f64; 7] = [0.125, 1.0 / 6.0, 0.25, 1.0 / 3.0, 0.5, 1.0, 2.0];
pub const MODES: &[&str] = &["Mix", "Insert", "Gate"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Interval", "Repeat", INTERVALS, 2),
    ParamSpec::int("Offset", "Repeat", 0.0, 15.0, 8.0, Unit::None),
    ParamSpec::int("Gate", "Repeat", 1.0, 32.0, 4.0, Unit::None),
    ParamSpec::choice("Grid", "Repeat", GRIDS, 2),
    ParamSpec::lin("Variation", "Repeat", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Chance", "Repeat", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Decay", "Output", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Pitch Decay", "Output", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Mode", "Output", MODES, 1),
];

/// Longest slice: a half note at 30 BPM.
const MAX_SECS: f32 = 4.0;

pub struct BeatRepeat {
    buf: [Vec<f32>; 2],
    rng: Rng,
    /// The current repeat: where it started (beats), how long it lasts.
    active: bool,
    start: f64,
    end: f64,
    grid: f64,
    /// Samples recorded into the slice so far, and the slice length.
    recorded: usize,
    slice: usize,
    /// Read position in the slice (fractional, for pitch decay) and speed.
    read: f64,
    speed: f64,
    gain: f32,
    /// The interval the last decision was made for.
    decided: i64,
    fade: f32,
}

impl BeatRepeat {
    pub fn new(sr: f32) -> Self {
        let len = (sr * MAX_SECS) as usize;
        Self {
            buf: [vec![0.0; len], vec![0.0; len]],
            rng: Rng::new(0xBEA7),
            active: false,
            start: 0.0,
            end: 0.0,
            grid: 0.25,
            recorded: 0,
            slice: 1,
            read: 0.0,
            speed: 1.0,
            gain: 1.0,
            decided: i64::MIN,
            fade: 0.0,
        }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let mode = p[MODE].round() as i32;
        if !ctx.playing {
            self.active = false;
            self.decided = i64::MIN;
            if mode == 2 {
                l[..n].fill(0.0);
                r[..n].fill(0.0);
            }
            return;
        }
        let interval = INTERVAL_BEATS[(p[INTERVAL].round().max(0.0) as usize).min(INTERVAL_BEATS.len() - 1)];
        let offset = (p[OFFSET].round().clamp(0.0, 15.0) as f64 * 0.25).min(interval - 0.25).max(0.0);
        let gate = p[GATE].round().clamp(1.0, 32.0) as f64 * 0.25;
        let grid_idx = (p[GRID].round().max(0.0) as usize).min(GRID_BEATS.len() - 1);
        let beats_per_sample = ctx.bpm.max(1.0) as f64 / 60.0 / ctx.sr as f64;
        let max_len = self.buf[0].len();
        let fade_step = 1.0 / (0.002 * ctx.sr);
        for i in 0..n {
            let beat = ctx.beat + i as f64 * beats_per_sample;
            let k = (beat / interval).floor() as i64;
            let at = k as f64 * interval + offset;
            // One decision per interval, at its offset.
            if k != self.decided && beat >= at {
                self.decided = k;
                if self.rng.unipolar() < p[CHANCE].clamp(0.0, 1.0) {
                    // Variation picks a random grid around the set one.
                    let var = p[VARIATION].clamp(0.0, 1.0);
                    let g = if var > 0.0 {
                        let spread = (var * 3.0).round() as i64;
                        let pick = grid_idx as i64 + (self.rng.unipolar() * (2 * spread + 1) as f32) as i64 - spread;
                        GRID_BEATS[pick.clamp(0, GRID_BEATS.len() as i64 - 1) as usize]
                    } else {
                        GRID_BEATS[grid_idx]
                    };
                    self.active = true;
                    self.start = beat;
                    self.end = beat + gate;
                    self.grid = g;
                    self.slice = ((g / beats_per_sample) as usize).clamp(16, max_len);
                    self.recorded = 0;
                    self.read = 0.0;
                    self.speed = 1.0;
                    self.gain = 1.0;
                }
            }
            if self.active && beat >= self.end {
                self.active = false;
            }
            let (xl, xr) = (l[i], r[i]);
            let mut wet = (0.0, 0.0);
            if self.active {
                // First the slice is recorded (and heard as it comes in), then it loops.
                if self.recorded < self.slice {
                    self.buf[0][self.recorded] = xl;
                    self.buf[1][self.recorded] = xr;
                    self.recorded += 1;
                    wet = (xl, xr);
                } else {
                    let len = self.slice as f64;
                    let pos = self.read;
                    let j = (pos as usize).min(self.slice - 1);
                    // Short fades at the slice edges against clicks.
                    let edge = ((pos.min(len - pos)) as f32 * fade_step).min(1.0);
                    wet = (self.buf[0][j] * self.gain * edge, self.buf[1][j] * self.gain * edge);
                    self.read += self.speed;
                    if self.read >= len {
                        self.read -= len;
                        self.gain *= 1.0 - p[DECAY].clamp(0.0, 1.0) * 0.5;
                        self.speed = (self.speed * (1.0 - p[PITCH].clamp(0.0, 1.0) as f64 * 0.12)).max(0.25);
                    }
                }
            }
            // Fade between the dry sound and the repeats when they start and stop.
            let target = if self.active && self.recorded >= self.slice { 1.0 } else { 0.0 };
            self.fade += (target - self.fade).clamp(-fade_step, fade_step);
            let (out_l, out_r) = match mode {
                0 => (xl + wet.0 * self.fade, xr + wet.1 * self.fade),
                2 => (wet.0 * self.fade, wet.1 * self.fade),
                _ => (xl * (1.0 - self.fade) + wet.0 * self.fade, xr * (1.0 - self.fade) + wet.1 * self.fade),
            };
            l[i] = out_l;
            r[i] = out_r;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::*;
    use super::*;

    #[test]
    fn it_repeats_a_slice_in_time() {
        let mut b = BeatRepeat::new(48000.0);
        let mut p = crate::params::defaults(SPECS);
        p[OFFSET] = 0.0;
        // A ramp makes every sample different, so repeats are easy to spot.
        let n = 48000 * 2;
        let mut l: Vec<f32> = (0..n).map(|i| i as f32 / n as f32).collect();
        let mut r = l.clone();
        let mut c = ctx();
        c.playing = true;
        let bps = 120.0 / 60.0 / 48000.0;
        for (k, (cl, cr)) in l.chunks_mut(480).zip(r.chunks_mut(480)).enumerate() {
            c.beat = (k * 480) as f64 * bps;
            b.process(&p, cl, cr, &c);
        }
        // A 1/16 at 120 BPM is 6000 samples: the slice 0..6000 plays again at 6000..12000.
        let slice = 6000;
        for i in [slice + 2000, slice + 3000, 2 * slice + 2500] {
            assert!((l[i] - (i % slice) as f32 / n as f32).abs() < 1e-3, "{i}: {}", l[i]);
        }
        // After the gate (a quarter note) the input comes back.
        assert!((l[30000] - 30000.0 / n as f32).abs() < 1e-4);
    }
}
