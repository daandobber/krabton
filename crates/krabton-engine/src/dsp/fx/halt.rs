//! Halt: someone leans on the reel. Press *Stop* and the sound slows down to
//! nothing, as a tape or a record would; let go and it spins back up and then
//! joins what the song is doing by now. Made for a modulator, a key or a move on
//! the *Stop* switch.

use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};

pub const STOP: usize = 0;
pub const DOWN: usize = 1;
pub const UP: usize = 2;
pub const CURVE: usize = 3;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::toggle("Stop", "Reel", false),
    ParamSpec::log("Down", "Reel", 50.0, 6000.0, 900.0, Unit::Ms),
    ParamSpec::log("Up", "Reel", 20.0, 3000.0, 300.0, Unit::Ms),
    ParamSpec::lin("Curve", "Reel", 0.0, 1.0, 0.5, Unit::Percent),
];

/// How long it can stay held, in seconds.
const LINE_SECONDS: f32 = 30.0;
/// How long the join with the song takes when it is back up to speed.
const JOIN_SECONDS: f32 = 0.03;

pub struct Halt {
    sr: f32,
    line: [Vec<f32>; 2],
    write: usize,
    /// Where the head reads, how far from full speed it is along the way.
    read: f64,
    speed: f32,
    /// 1 down at full speed; 0 halted (then shaped by the curve).
    along: f32,
    /// Off the live sound: reading what is on the tape.
    behind: bool,
    /// Fading from the tape back to the song (1..0).
    join: f32,
}

impl Halt {
    pub fn new(sr: f32) -> Self {
        let size = (LINE_SECONDS * sr) as usize + 4;
        Self { sr, line: [vec![0.0; size], vec![0.0; size]], write: 0, read: 0.0, speed: 1.0, along: 1.0, behind: false, join: 0.0 }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], _ctx: &FxCtx) {
        let sr = self.sr;
        let n = l.len().min(r.len());
        let size = self.line[0].len();
        let stop = p[STOP] >= 0.5;
        let down = 1.0 / (p[DOWN].max(1.0) * 0.001 * sr);
        let up = 1.0 / (p[UP].max(1.0) * 0.001 * sr);
        // A little: hangs on, then goes (a heavy reel). A lot: drops at once.
        let shape = 0.6 + 2.4 * p[CURVE].clamp(0.0, 1.0);
        let join_step = 1.0 / (JOIN_SECONDS * sr);
        for i in 0..n {
            let dry = [l[i], r[i]];
            self.line[0][self.write] = dry[0];
            self.line[1][self.write] = dry[1];
            if stop && !self.behind {
                // It leaves the live sound here.
                self.behind = true;
                self.read = self.write as f64;
                self.join = 0.0;
            }
            if stop {
                self.along = (self.along - down).max(0.0);
            } else {
                self.along = (self.along + up).min(1.0);
            }
            self.speed = self.along.powf(shape);
            let write = self.write;
            self.write = (self.write + 1) % size;
            if !self.behind && self.join <= 0.0 {
                continue;
            }
            let mut tape = [0.0f32; 2];
            // How far behind the song the head is; it can't fall off the tape.
            let lag = (write as f64 - self.read).rem_euclid(size as f64);
            if lag < size as f64 - 8.0 {
                let i0 = self.read.floor() as usize % size;
                let f = self.read.fract() as f32;
                for (side, t) in tape.iter_mut().enumerate() {
                    let line = &self.line[side];
                    *t = line[i0] + (line[(i0 + 1) % size] - line[i0]) * f;
                }
            }
            self.read = (self.read + self.speed as f64).rem_euclid(size as f64);
            // Slowing down, it goes quiet too.
            let level = self.speed.powf(0.5).min(1.0);
            if self.behind {
                if !stop && self.along >= 1.0 {
                    // Back up to speed: into what the song is doing by now.
                    self.behind = false;
                    self.join = 1.0;
                }
                if self.behind {
                    l[i] = tape[0] * level;
                    r[i] = tape[1] * level;
                    continue;
                }
            }
            self.join = (self.join - join_step).max(0.0);
            l[i] = dry[0] + (tape[0] - dry[0]) * self.join;
            r[i] = dry[1] + (tape[1] - dry[1]) * self.join;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::{Scale, ScaleKind};

    const SR: f32 = 48000.0;

    fn ctx() -> FxCtx {
        FxCtx { sr: SR, bpm: 120.0, beat: 0.0, playing: true, scale: Scale::new(0, ScaleKind::Minor) }
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    fn ups(x: &[f32]) -> usize {
        (1..x.len()).filter(|&i| x[i - 1] < 0.0 && x[i] >= 0.0).count()
    }

    struct Run {
        h: Halt,
        p: Vec<f32>,
        at: usize,
    }

    impl Run {
        fn new() -> Self {
            let mut p = crate::params::defaults(SPECS);
            p[DOWN] = 1000.0;
            p[UP] = 200.0;
            Self { h: Halt::new(SR), p, at: 0 }
        }

        fn go(&mut self, seconds: f32) -> Vec<f32> {
            let n = (seconds * SR) as usize;
            let at = self.at;
            let mut l: Vec<f32> = (0..n).map(|i| (std::f32::consts::TAU * 1000.0 * (at + i) as f32 / SR).sin() * 0.5).collect();
            let mut r = l.clone();
            for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
                self.h.process(&self.p, a, b, &ctx());
            }
            self.at += n;
            l
        }
    }

    #[test]
    fn left_alone_it_changes_nothing() {
        let mut run = Run::new();
        let x = run.go(0.5);
        for (i, v) in x.iter().enumerate() {
            let want = (std::f32::consts::TAU * 1000.0 * i as f32 / SR).sin() * 0.5;
            assert!((v - want).abs() < 1e-5, "{i}");
        }
    }

    #[test]
    fn stopped_it_slows_to_nothing() {
        let mut run = Run::new();
        run.go(0.2);
        run.p[STOP] = 1.0;
        let a = run.go(0.2);
        let b = run.go(0.3);
        let c = run.go(0.3);
        // Lower and lower.
        let (fa, fb) = (ups(&a) as f32 / 0.2, ups(&b) as f32 / 0.3);
        assert!(fa > fb * 1.3, "{fa} {fb}");
        assert!(fa < 1000.0, "{fa}");
        run.go(0.3);
        // And then nothing at all.
        let silent = run.go(0.5);
        assert!(rms(&silent) < 1e-3, "{}", rms(&silent));
        assert!(rms(&c) < rms(&a));
    }

    #[test]
    fn let_go_it_comes_back_up_and_joins_the_song() {
        let mut run = Run::new();
        run.go(0.2);
        run.p[STOP] = 1.0;
        run.go(1.5);
        run.p[STOP] = 0.0;
        let rising = run.go(0.2);
        // Coming up to speed, not at it yet.
        assert!((ups(&rising) as f32 / 0.2) < 900.0, "{}", ups(&rising));
        run.go(0.1);
        // Now on the song, exactly.
        let at = run.at;
        let x = run.go(0.2);
        for (i, v) in x.iter().enumerate() {
            let want = (std::f32::consts::TAU * 1000.0 * (at + i) as f32 / SR).sin() * 0.5;
            assert!((v - want).abs() < 1e-3, "{i}: {v} {want}");
        }
    }

    #[test]
    fn it_stays_quiet_for_as_long_as_it_is_held() {
        let mut run = Run::new();
        run.go(0.2);
        run.p[STOP] = 1.0;
        run.go(2.0);
        let x = run.go(40.0);
        assert!(x.iter().all(|v| v.is_finite() && v.abs() < 1e-3));
    }
}
