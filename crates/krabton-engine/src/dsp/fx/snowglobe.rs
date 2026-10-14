//! Snowglobe: a small world of loops, grains, glitches and echoes made from
//! what comes in. It keeps the last ten seconds and plays them back in
//! voices: pieces of the recording at their own speed and direction, each
//! shaped by an envelope. Eleven modes in four families decide which pieces
//! play when, on the song's beat (Time), each with four variations:
//!
//! - Loops: *Tiles* (the last bars looped in layers at other speeds and
//!   backwards), *Steps* (slices in a sequence of pitches), *Slide* (a loop
//!   gliding in pitch).
//! - Grains: *Mist* (a cloud of grains), *Cave* (grains through a ringing
//!   tube), *Harp* (grains strummed up a chord).
//! - Glitch: *Shuffle* (blocks from earlier, in another order), *Chop*
//!   (the sound itself, stuttering now and then), *Climb* (repeats that
//!   climb an arpeggio).
//! - Delays: *Rhythm* (echoes in a pattern), *Bend* (echoes that bend in
//!   pitch).
//!
//! Activity is how busy it is, Repeats how much comes round again, Shape
//! the envelope of every piece (plucked to swelling), Filter darkens to the
//! left and thins to the right, Space is its reverb. Hold stops listening
//! and keeps playing what it has; Reverse turns every piece around.

use super::sky::Sky;
use crate::dsp::effects::FxCtx;
use crate::dsp::Svf;
use crate::params::{ParamSpec, Unit};
use std::f32::consts::{FRAC_PI_2, PI};

pub const MODE: usize = 0;
pub const VARIATION: usize = 1;
pub const ACTIVITY: usize = 2;
pub const REPEATS: usize = 3;
pub const SHAPE: usize = 4;
pub const FILTER: usize = 5;
pub const SPACE: usize = 6;
pub const TIME: usize = 7;
pub const HOLD: usize = 8;
pub const REVERSE: usize = 9;
pub const MIX: usize = 10;

pub const MODES: &[&str] = &["Tiles", "Steps", "Slide", "Mist", "Cave", "Harp", "Shuffle", "Chop", "Climb", "Rhythm", "Bend"];
pub const VARIATIONS: &[&str] = &["A", "B", "C", "D"];
pub const TIMES: &[&str] = &["1/32", "1/16", "1/8", "1/4", "1/2", "1 Bar"];
const TIME_BEATS: [f32; 6] = [0.125, 0.25, 0.5, 1.0, 2.0, 4.0];

const TILES: usize = 0;
const STEPS: usize = 1;
const SLIDE: usize = 2;
const MIST: usize = 3;
const CAVE: usize = 4;
const HARP: usize = 5;
const SHUFFLE: usize = 6;
const CHOP: usize = 7;
const CLIMB: usize = 8;
const RHYTHM: usize = 9;

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Mode", "Snowglobe", MODES, 0),
    ParamSpec::choice("Variation", "Snowglobe", VARIATIONS, 0),
    ParamSpec::lin("Activity", "Snowglobe", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Repeats", "Snowglobe", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Shape", "Snowglobe", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Filter", "Tone", -1.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Space", "Tone", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::choice("Time", "Snowglobe", TIMES, 3),
    ParamSpec::toggle("Hold", "Snowglobe", false),
    ParamSpec::toggle("Reverse", "Snowglobe", false),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 0.5, Unit::Percent),
];

const BUF_S: f32 = 10.0;
const VOICES: usize = 40;
const CHUNK: usize = 512;

#[derive(Clone, Copy, Default)]
struct Voice {
    active: bool,
    /// Samples to wait before it starts.
    wait: u32,
    /// Where it reads in the buffer (absolute, wrapping), and how fast.
    pos: f64,
    rate: f64,
    /// A glide of the rate per sample (Slide, Bend).
    rate_k: f64,
    len: u32,
    age: u32,
    gain: f32,
    gl: f32,
    gr: f32,
    /// How much of its length goes into the attack.
    attack: f32,
}

pub struct Snowglobe {
    sr: f32,
    buf: [Vec<f32>; 2],
    w: usize,
    voices: [Voice; VOICES],
    /// Samples to the next event, and how many events so far.
    next: f64,
    count: u64,
    rng: u32,
    fb: [f32; 2],
    tube: [Vec<f32>; 2],
    tube_w: usize,
    filt: [Svf; 2],
    sky: Box<Sky>,
    sky_p: Vec<f32>,
    wet_l: Vec<f32>,
    wet_r: Vec<f32>,
    rev_l: Vec<f32>,
    rev_r: Vec<f32>,
    gate: f32,
    chop_until: u64,
    clock: u64,
}

impl Snowglobe {
    pub fn new(sr: f32) -> Self {
        let n = (BUF_S * sr) as usize;
        let mut sky_p = crate::params::defaults(super::sky::SPECS);
        sky_p[super::sky::TYPE] = 3.0;
        sky_p[super::sky::MIX] = 1.0;
        sky_p[super::sky::PREDELAY] = 10.0;
        Self {
            sr,
            buf: [vec![0.0; n], vec![0.0; n]],
            w: 0,
            voices: [Voice::default(); VOICES],
            next: 0.0,
            count: 0,
            rng: 0x5e0_91be,
            fb: [0.0; 2],
            tube: [vec![0.0; 4096], vec![0.0; 4096]],
            tube_w: 0,
            filt: [Svf::new(), Svf::new()],
            sky: Box::new(Sky::new(sr)),
            sky_p,
            wet_l: vec![0.0; CHUNK],
            wet_r: vec![0.0; CHUNK],
            rev_l: vec![0.0; CHUNK],
            rev_r: vec![0.0; CHUNK],
            gate: 1.0,
            chop_until: 0,
            clock: 0,
        }
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Starts a voice that plays `len` samples of a piece of the recording
    /// that ended `back` samples ago, at `rate` (negative: backwards, from
    /// its end).
    #[allow(clippy::too_many_arguments)]
    fn spawn(&mut self, back: f64, len: f64, rate: f64, gain: f32, pan: f32, attack: f32, wait: u32, rate_k: f64) {
        let n = self.buf[0].len() as f64;
        let len = len.clamp(64.0, n * 0.4);
        let span = len * rate.abs();
        let back = back.clamp(1.0, (n - span - 64.0).max(1.0));
        let end = (self.w as f64 - back).rem_euclid(n);
        let start = if rate >= 0.0 { (end - span).rem_euclid(n) } else { end };
        let Some(v) = self.voices.iter_mut().find(|v| !v.active) else { return };
        let p = pan.clamp(-1.0, 1.0);
        let (gl, gr) = (((1.0 - p) * 0.25 * PI).cos(), ((1.0 + p) * 0.25 * PI).cos());
        *v = Voice { active: true, wait, pos: start, rate, rate_k, len: len as u32, age: 0, gain, gl, gr, attack: attack.clamp(0.005, 0.95) };
    }

    fn event(&mut self, mode: usize, var: usize, p: &[f32], unit: f64, reverse: bool) {
        let act = p[ACTIVITY].clamp(0.0, 1.0);
        let shape = p[SHAPE].clamp(0.0, 1.0);
        let attack = 0.02 + 0.6 * shape;
        let dir = if reverse { -1.0 } else { 1.0 };
        let k = self.count;
        let semi = |s: f32| 2f64.powf(s as f64 / 12.0);
        match mode {
            TILES => {
                // A loop of one, two or four units, layered at other speeds.
                let len = unit * [2.0, 1.0, 4.0, 2.0][var];
                let layers: &[(f64, f32)] = match var {
                    0 => &[(1.0, 0.0), (0.5, -0.6)],
                    1 => &[(1.0, 0.0), (2.0, 0.6)],
                    2 => &[(1.0, 0.0), (-1.0, 0.7)],
                    _ => &[(1.0, 0.0), (0.5, -0.7), (2.0, 0.7), (-1.0, 0.2)],
                };
                for (i, &(rate, pan)) in layers.iter().enumerate() {
                    if i == 0 || self.rand() < 0.3 + 0.7 * act {
                        // The most recent stretch, looped.
                        self.spawn(64.0, len, rate * dir, if i == 0 { 0.7 } else { 0.45 }, pan, attack.min(0.3), 0, 0.0);
                    }
                }
                self.next = len * if act > 0.5 { 0.5 } else { 1.0 };
            }
            STEPS => {
                let pattern: [f32; 8] = match var {
                    0 => [0.0, 0.0, 12.0, 0.0, 7.0, 0.0, 12.0, 5.0],
                    1 => [0.0, 7.0, 12.0, 7.0, 0.0, 7.0, 12.0, 19.0],
                    2 => [0.0, -12.0, 0.0, 12.0, 0.0, -5.0, 0.0, 7.0],
                    _ => [0.0, 3.0, 5.0, 7.0, 10.0, 12.0, 7.0, 3.0],
                };
                let per = [1.0, 0.5, 0.25][((act * 2.99) as usize).min(2)];
                let len = unit * per;
                let s = pattern[(k % 8) as usize];
                self.spawn(64.0, len * 0.95, semi(s) * dir, 0.75, if k % 2 == 0 { -0.4 } else { 0.4 }, attack.min(0.4), 0, 0.0);
                self.next = len;
            }
            SLIDE => {
                let len = unit * 2.0;
                // Glide over the loop: wobble, up an octave, down, anywhere.
                let (from, to) = match var {
                    0 => (-0.3, 0.3),
                    1 => (0.0, 12.0),
                    2 => (0.0, -12.0),
                    _ => {
                        let a = (self.rand() * 24.0 - 12.0).round();
                        (a, a + (self.rand() * 14.0 - 7.0).round())
                    }
                };
                let r0 = semi(from);
                let r1 = semi(to);
                let kk = (r1 / r0).powf(1.0 / len.max(1.0));
                // Gliding up it reads faster: it starts that much further back.
                let ahead = len * ((r1 / r0).max(1.0) - 1.0) + 64.0;
                self.spawn(ahead, len, r0 * dir, 0.7, 0.0, attack.min(0.3), 0, kk);
                if act > 0.5 {
                    self.spawn(ahead + len * 0.5, len, r0 * dir * 0.5, 0.4, if k % 2 == 0 { -0.7 } else { 0.7 }, 0.4, (len * 0.5) as u32, kk);
                }
                self.next = len * 0.5;
            }
            MIST => {
                // A cloud: many grains, here and there in the last two seconds.
                let size = (0.03 + 0.25 * shape) as f64 * self.sr as f64;
                let pitch: f32 = match var {
                    0 => 0.0,
                    1 => [-12.0, 0.0, 12.0][(self.rand() * 2.99) as usize],
                    2 => [12.0, 19.0, 24.0][(self.rand() * 2.99) as usize],
                    _ => [-12.0, -24.0, 0.0][(self.rand() * 2.99) as usize],
                };
                let back = 64.0 + self.rand() as f64 * 2.0 * self.sr as f64;
                let pan = self.rand() * 2.0 - 1.0;
                self.spawn(back, size, semi(pitch) * dir, 0.35, pan, 0.5, 0, 0.0);
                let rate = 4.0 + 80.0 * act * act;
                self.next = self.sr as f64 / rate as f64 * (0.5 + self.rand() as f64);
            }
            CAVE => {
                let size = (0.08 + 0.4 * shape) as f64 * self.sr as f64;
                let back = 64.0 + self.rand() as f64 * self.sr as f64;
                let pitch = [0.0, -12.0, 7.0, 12.0][var];
                let pan = self.rand() * 1.6 - 0.8;
                self.spawn(back, size, semi(pitch) * dir, 0.5, pan, 0.5, 0, 0.0);
                let rate = 2.0 + 20.0 * act;
                self.next = self.sr as f64 / rate as f64 * (0.5 + self.rand() as f64);
            }
            HARP => {
                // On every unit a strum of grains up a chord.
                let chord: &[f32] = match var {
                    0 => &[0.0, 4.0, 7.0, 12.0, 16.0, 19.0, 24.0, 28.0],
                    1 => &[0.0, 3.0, 7.0, 12.0, 15.0, 19.0, 24.0, 27.0],
                    2 => &[0.0, 7.0, 12.0, 19.0, 24.0, 31.0, 36.0, 43.0],
                    _ => &[0.0, 12.0, 24.0, 12.0, 0.0, 12.0, 24.0, 36.0],
                };
                let n = 3 + (act * 5.0) as usize;
                let gap = (0.025 + 0.04 * (1.0 - act)) * self.sr;
                let size = (0.06 + 0.3 * shape) as f64 * self.sr as f64;
                for (i, &s) in chord.iter().take(n).enumerate() {
                    let pan = -0.8 + 1.6 * i as f32 / n as f32;
                    self.spawn(64.0, size, semi(s - 12.0) * dir, 0.4, pan, 0.03 + 0.3 * shape, (i as f32 * gap) as u32, 0.0);
                }
                self.next = unit;
            }
            SHUFFLE => {
                // A block from the last four units, in another place, maybe turned.
                let len = unit * [0.5, 0.25, 1.0, 0.5][var];
                let back = len * (self.rand() * 7.0).floor() as f64 + 64.0;
                let turn = var >= 2 && self.rand() < 0.4;
                let pitch = if var == 3 && self.rand() < 0.3 { [-12.0, 12.0][(self.rand() * 1.99) as usize] } else { 0.0 };
                if self.rand() < 0.35 + 0.65 * act {
                    let pan = self.rand() - 0.5;
                    self.spawn(back, len, semi(pitch) * dir * if turn { -1.0 } else { 1.0 }, 0.8, pan, attack.min(0.1), 0, 0.0);
                }
                self.next = len;
            }
            CHOP => {
                // Mostly the sound itself; now and then the last slice again and again.
                let step = unit * 0.25;
                if self.rand() < act * 0.8 {
                    let slice = step * [0.5, 0.25, 0.5, 0.125][var];
                    let times = [2, 3, 4, 6][var];
                    for t in 0..times {
                        let pitch = if var == 2 { -(t as f32) * 2.0 } else { 0.0 };
                        self.spawn(1.0, slice, semi(pitch) * dir, 0.9, 0.0, 0.03, (t as f64 * slice) as u32, 0.0);
                    }
                    self.chop_until = self.clock + (slice * times as f64) as u64;
                }
                self.next = step * if self.chop_until > self.clock { 2.0 } else { 1.0 };
            }
            CLIMB => {
                // The last stretch again, each time a step higher.
                let steps: &[f32] = match var {
                    0 => &[0.0, 4.0, 7.0, 12.0, 16.0, 19.0, 24.0],
                    1 => &[0.0, 3.0, 7.0, 12.0, 15.0, 19.0, 24.0],
                    2 => &[0.0, 7.0, 12.0, 7.0, 0.0, -5.0, -12.0],
                    _ => &[0.0, 5.0, -7.0, 12.0, 2.0, 9.0, -12.0],
                };
                let n = 1 + (p[REPEATS].clamp(0.0, 1.0) * 6.0) as usize;
                let len = unit * [0.5, 0.5, 0.25, 0.25][var] * (0.5 + act as f64);
                for (i, &s) in steps.iter().take(n).enumerate() {
                    self.spawn(1.0, len * 0.9, semi(s) * dir, 0.8 * 0.85f32.powi(i as i32), if i % 2 == 0 { -0.3 } else { 0.3 }, attack.min(0.3), ((i + 1) as f64 * len) as u32, 0.0);
                }
                self.next = len * (n + 1) as f64;
            }
            RHYTHM => {
                // Echoes at the pattern's taps: windows of half a unit, every
                // quarter, that add up to one stream.
                let taps: &[(f64, f32)] = match var {
                    0 => &[(1.0, 0.7), (1.5, 0.5), (2.75, 0.4)],
                    1 => &[(0.75, 0.6), (1.5, 0.5), (2.25, 0.4)],
                    2 => &[(2.0 / 3.0, 0.6), (4.0 / 3.0, 0.5), (2.0, 0.4)],
                    _ => &[(0.5, 0.5), (1.25, 0.5), (1.75, 0.4), (3.0, 0.35)],
                };
                let len = unit * 0.25;
                for (i, &(t, g)) in taps.iter().enumerate() {
                    let pan = if i % 2 == 0 { -0.6 } else { 0.6 };
                    // What was played the tap's time ago: an echo.
                    self.spawn(t * unit - 2.0 * len, len * 2.0, dir, g * (0.5 + act), pan, 0.5, 0, 0.0);
                }
                self.next = len;
            }
            _ => {
                // Bend: echoes whose pitch bends while they play.
                let len = unit * 0.25;
                let bend = [0.5, -0.5, 1.0, -1.0][var] as f64 * (0.3 + act as f64);
                let kk = semi(bend as f32).powf(1.0 / (len * 2.0).max(1.0));
                self.spawn(unit - 2.0 * len, len * 2.0, dir, 0.6, if k % 2 == 0 { -0.5 } else { 0.5 }, 0.5, 0, kk);
                self.spawn(unit * 2.0 - 2.0 * len, len * 2.0, dir, 0.4, if k % 2 == 0 { 0.5 } else { -0.5 }, 0.5, 0, kk * kk);
                self.next = len;
            }
        }
        self.count += 1;
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        for (cl, cr) in l[..n].chunks_mut(CHUNK).zip(r[..n].chunks_mut(CHUNK)) {
            self.process_chunk(p, cl, cr, ctx);
        }
    }

    fn process_chunk(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let sr = self.sr;
        let mode = (p[MODE].round().max(0.0) as usize).min(MODES.len() - 1);
        let var = (p[VARIATION].round().max(0.0) as usize).min(3);
        let beats = TIME_BEATS[(p[TIME].round().max(0.0) as usize).min(TIMES.len() - 1)];
        let unit = (beats * 60.0 / ctx.bpm.max(20.0) * sr) as f64;
        let hold = p[HOLD] >= 0.5;
        let reverse = p[REVERSE] >= 0.5;
        let repeats = p[REPEATS].clamp(0.0, 1.0);
        let fb = if mode == CLIMB { 0.15 * repeats } else { 0.85 * repeats };
        let mix = p[MIX].clamp(0.0, 1.0);
        let space = p[SPACE].clamp(0.0, 1.0);
        let f = p[FILTER].clamp(-1.0, 1.0);
        let len = self.buf[0].len();
        for c in 0..2 {
            if f < 0.0 {
                self.filt[c].set_q(18000.0 * (300.0f32 / 18000.0).powf(-f), 0.7 + 1.5 * f * f, sr);
            } else {
                self.filt[c].set_q(20.0 * (3000.0f32 / 20.0).powf(f), 0.7 + 1.5 * f * f, sr);
            }
        }
        let tube_len = [220usize, 440, 147, 330][var].min(4000);
        for i in 0..n {
            let (xl, xr) = (l[i], r[i]);
            // What comes in (and round again) is kept.
            if !hold {
                self.w = (self.w + 1) % len;
                self.buf[0][self.w] = xl + fb * self.fb[0];
                self.buf[1][self.w] = xr + fb * self.fb[1];
            }
            self.next -= 1.0;
            if self.next <= 0.0 {
                self.event(mode, var, p, unit.max(256.0), reverse);
            }
            // The voices.
            let (mut wl, mut wr) = (0.0f32, 0.0f32);
            for v in self.voices.iter_mut() {
                if !v.active {
                    continue;
                }
                if v.wait > 0 {
                    v.wait -= 1;
                    continue;
                }
                let t = v.age as f32 / v.len.max(1) as f32;
                if t >= 1.0 {
                    v.active = false;
                    continue;
                }
                // Rises and falls as a sine squared: overlapping halves add up to one.
                let e = if t < v.attack { (FRAC_PI_2 * t / v.attack).sin() } else { (FRAC_PI_2 * (1.0 - t) / (1.0 - v.attack)).sin() };
                let env = e * e;
                let i0 = v.pos.floor();
                let fr = (v.pos - i0) as f32;
                let a = i0 as usize % len;
                let b = (a + 1) % len;
                let sl = self.buf[0][a] + (self.buf[0][b] - self.buf[0][a]) * fr;
                let sr_ = self.buf[1][a] + (self.buf[1][b] - self.buf[1][a]) * fr;
                let g = v.gain * env;
                wl += g * (sl * v.gl + sr_ * (1.0 - v.gl) * 0.3);
                wr += g * (sr_ * v.gr + sl * (1.0 - v.gr) * 0.3);
                v.pos = (v.pos + v.rate).rem_euclid(len as f64);
                if v.rate_k != 0.0 {
                    v.rate *= v.rate_k;
                }
                v.age += 1;
            }
            // Cave: through a ringing tube.
            if mode == CAVE {
                let tl = self.tube[0].len();
                self.tube_w = (self.tube_w + 1) % tl;
                for (c, w) in [&mut wl, &mut wr].into_iter().enumerate() {
                    let d = tube_len + c * 7;
                    let back = self.tube[c][(self.tube_w + tl - d) % tl];
                    let y = *w + (0.55 + 0.4 * repeats) * back;
                    self.tube[c][self.tube_w] = y.clamp(-4.0, 4.0) * 0.98;
                    *w = 0.5 * y;
                }
            }
            // Chop: the sound itself, but not while it stutters.
            if mode == CHOP {
                let target = if self.chop_until > self.clock { 0.0 } else { 1.0 };
                self.gate += (target - self.gate) * 0.01;
                wl += self.gate * xl;
                wr += self.gate * xr;
            }
            self.clock += 1;
            if f.abs() > 0.02 {
                let yl = self.filt[0].tick(wl);
                let yr = self.filt[1].tick(wr);
                wl = if f < 0.0 { yl.low } else { yl.high };
                wr = if f < 0.0 { yr.low } else { yr.high };
            }
            self.fb = [wl.clamp(-2.0, 2.0), wr.clamp(-2.0, 2.0)];
            self.wet_l[i] = wl;
            self.wet_r[i] = wr;
        }
        // Space: its reverb on the wet.
        if space > 0.0 {
            self.sky_p[super::sky::DECAY] = 1.5 + 10.0 * space;
            self.rev_l[..n].copy_from_slice(&self.wet_l[..n]);
            self.rev_r[..n].copy_from_slice(&self.wet_r[..n]);
            self.sky.process(&self.sky_p, &mut self.rev_l[..n], &mut self.rev_r[..n], ctx);
            for i in 0..n {
                self.wet_l[i] = self.wet_l[i] * (1.0 - 0.5 * space) + self.rev_l[i] * space;
                self.wet_r[i] = self.wet_r[i] * (1.0 - 0.5 * space) + self.rev_r[i] * space;
            }
        }
        for i in 0..n {
            l[i] = l[i] * (1.0 - mix) + self.wet_l[i] * mix;
            r[i] = r[i] * (1.0 - mix) + self.wet_r[i] * mix;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::*;
    use super::*;

    fn run(f: impl Fn(&mut Vec<f32>), input_secs: f32, secs: f32) -> (Vec<f32>, Vec<f32>) {
        let mut p = crate::params::defaults(SPECS);
        p[MIX] = 1.0;
        f(&mut p);
        let mut g = Snowglobe::new(48000.0);
        let n = (secs * 48000.0) as usize;
        let src = tone(330.0, input_secs, 0.4);
        let mut l = vec![0.0; n];
        l[..src.len().min(n)].copy_from_slice(&src[..src.len().min(n)]);
        let mut r = l.clone();
        for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            g.process(&p, cl, cr, &ctx());
        }
        (l, r)
    }

    #[test]
    fn every_mode_and_variation_plays_and_stays_bounded() {
        for mode in 0..MODES.len() {
            for var in 0..4 {
                let (l, r) = run(
                    |p| {
                        p[MODE] = mode as f32;
                        p[VARIATION] = var as f32;
                        p[SPACE] = 0.3;
                    },
                    2.0,
                    4.0,
                );
                assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() < 6.0), "{} {}", MODES[mode], VARIATIONS[var]);
                // (A long loop is heard once it has been recorded.)
                assert!(rms(&l[48000..]) > 0.01, "{} {} is silent", MODES[mode], VARIATIONS[var]);
            }
        }
    }

    #[test]
    fn hold_keeps_playing_what_it_has() {
        // Hold after a second of sound: the loops go on in the silence.
        let mut p = crate::params::defaults(SPECS);
        p[MIX] = 1.0;
        p[SPACE] = 0.0;
        p[REPEATS] = 0.0;
        let mut g = Snowglobe::new(48000.0);
        let src = tone(330.0, 1.0, 0.4);
        let mut l = vec![0.0; 4 * 48000];
        l[..src.len()].copy_from_slice(&src);
        let mut r = l.clone();
        for (k, (cl, cr)) in l.chunks_mut(256).zip(r.chunks_mut(256)).enumerate() {
            p[HOLD] = if k * 256 > 46000 { 1.0 } else { 0.0 };
            g.process(&p, cl, cr, &ctx());
        }
        assert!(rms(&l[3 * 48000..]) > 0.02, "{}", rms(&l[3 * 48000..]));
        // Without Hold, silence after the sound has passed through.
        let (l, _) = run(
            |p| {
                p[SPACE] = 0.0;
                p[REPEATS] = 0.0;
            },
            1.0,
            6.0,
        );
        assert!(rms(&l[5 * 48000..]) < 1e-3);
    }

    #[test]
    fn rhythm_echoes_on_the_beat() {
        // A click; Rhythm A at 120 bpm with a quarter as the unit: the first
        // echo a beat (half a second) later.
        let mut p = crate::params::defaults(SPECS);
        p[MODE] = RHYTHM as f32;
        p[MIX] = 1.0;
        p[SPACE] = 0.0;
        p[REPEATS] = 0.0;
        let mut g = Snowglobe::new(48000.0);
        let mut l = vec![0.0; 48000 * 2];
        for v in l[100..340].iter_mut() {
            *v = 0.5;
        }
        let mut r = l.clone();
        for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            g.process(&p, cl, cr, &ctx());
        }
        let first = (4800..48000 + 12000).find(|&i| l[i].abs() > 0.05).unwrap_or(0);
        assert!((first as i64 - 24000).abs() < 2400, "first echo at {first}");
    }
}
