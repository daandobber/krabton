//! Grains: a granular processor in the spirit of the eurorack texture
//! machines. The input runs into a buffer (up to 8 seconds) and grains are
//! read back from it: Time says how far back, Size how long (from clicks to
//! two seconds; all the way up it becomes a delay), Shape their envelope
//! (from square through smooth and plucked to swelling and reversed) and
//! Pitch their pitch. Density sets how many: nothing in the middle, more and
//! more random grains to the right, regular ones to the left (on the song's
//! tempo with Sync). Each of Time, Size, Shape and Pitch has a Random
//! amount that varies it per grain (Pitch in octaves and fifths). The grains
//! go back into the buffer with Feedback, through a hall with Reverb, and
//! Freeze stops recording. Quality picks the buffer: clean stereo (4 s),
//! long mono (8 s), cassette (wow, flutter, warmth) or lo-fi (8-bit,
//! 12 kHz). Original code.

use super::hall::{self, Hall};
use super::PitchShifter;
use crate::dsp::effects::FxCtx;
use crate::dsp::{fast_tanh, OnePole, Rng};
use crate::params::{ParamSpec, Unit};
use crate::scope::{DeviceTaps, MeterTap, ScopeTap};
use std::f32::consts::{FRAC_PI_2, TAU};

pub const TIME: usize = 0;
pub const SIZE: usize = 1;
pub const SHAPE: usize = 2;
pub const PITCH: usize = 3;
pub const DENSITY: usize = 4;
pub const TIME_RAND: usize = 5;
pub const SIZE_RAND: usize = 6;
pub const SHAPE_RAND: usize = 7;
pub const PITCH_RAND: usize = 8;
pub const SPREAD: usize = 9;
pub const SYNC: usize = 10;
pub const FREEZE: usize = 11;
pub const QUALITY: usize = 12;
pub const FEEDBACK: usize = 13;
pub const REVERB: usize = 14;
pub const MIX: usize = 15;

pub const QUALITIES: &[&str] = &["Clean", "Long Mono", "Cassette", "Lo-Fi"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Time", "Grains", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Size", "Grains", 0.0, 1.0, 0.45, Unit::Percent),
    ParamSpec::lin("Shape", "Grains", 0.0, 1.0, 0.4, Unit::Percent),
    ParamSpec::lin("Pitch", "Grains", -24.0, 24.0, 0.0, Unit::Semi),
    ParamSpec::lin("Density", "Grains", -1.0, 1.0, 0.45, Unit::Percent),
    ParamSpec::lin("Time Rand", "Random", 0.0, 1.0, 0.15, Unit::Percent),
    ParamSpec::lin("Size Rand", "Random", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Shape Rand", "Random", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Pitch Rand", "Random", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Spread", "Random", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::toggle("Sync", "Buffer", false),
    ParamSpec::toggle("Freeze", "Buffer", false),
    ParamSpec::choice("Quality", "Buffer", QUALITIES, 0),
    ParamSpec::lin("Feedback", "Output", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Reverb", "Output", 0.0, 1.0, 0.2, Unit::Percent),
    ParamSpec::lin("Mix", "Output", 0.0, 1.0, 0.5, Unit::Percent),
];

const MAX_GRAINS: usize = 32;
/// Frames per processing step.
const CHUNK: usize = 256;
const BUFFER_SECONDS: f32 = 8.0;
/// Grains shown on the display.
pub const SHOWN: usize = 9;

#[derive(Clone, Copy, Default)]
struct Grain {
    active: bool,
    /// Read position in the buffer (frames), and frames per output frame.
    pos: f64,
    inc: f64,
    len: u32,
    age: u32,
    shape: f32,
    gain: (f32, f32),
    semis: f32,
}

/// The envelope at `t` (0..1) of a grain of shape `s`: square, smooth
/// (Hann), plucked, then swelling.
fn envelope(s: f32, t: f32) -> f32 {
    let hann = 0.5 - 0.5 * (TAU * t).cos();
    let square = (t * 40.0).min((1.0 - t) * 40.0).min(1.0);
    let pluck = (t * 25.0).min(1.0) * (-5.0 * t).exp();
    let swell = (t * t) * ((1.0 - t) * 25.0).min(1.0);
    if s < 1.0 / 3.0 {
        let k = s * 3.0;
        square + (hann - square) * k
    } else if s < 2.0 / 3.0 {
        let k = (s - 1.0 / 3.0) * 3.0;
        hann + (pluck - hann) * k
    } else {
        let k = (s - 2.0 / 3.0) * 3.0;
        pluck + (swell - pluck) * k
    }
}

pub struct Grains {
    sr: f32,
    buf: [Vec<f32>; 2],
    write: usize,
    /// Frames recorded so far (until the buffer is full).
    filled: usize,
    grains: [Grain; MAX_GRAINS],
    rng: Rng,
    /// Frames to the next grain.
    countdown: f64,
    last_division: i64,
    hall: Box<Hall>,
    hall_p: Vec<f32>,
    /// Delay mode: two pitch shifters, and the read time gliding.
    shifters: [PitchShifter; 2],
    delay_time: f32,
    wet_prev: (f32, f32),
    /// Cassette: wow and flutter phases, warmth.
    wow: f32,
    flutter: f32,
    tone: [OnePole; 2],
    /// Lo-fi: held sample and counter.
    held: (f32, f32),
    hold: u32,
    pub taps: DeviceTaps,
}

impl Grains {
    pub fn new(sr: f32) -> Self {
        let len = (sr * BUFFER_SECONDS) as usize + 4;
        Self {
            sr,
            buf: [vec![0.0; len], vec![0.0; len]],
            write: 0,
            filled: 0,
            grains: [Grain::default(); MAX_GRAINS],
            rng: Rng::new(0x6A1_4E5),
            countdown: 0.0,
            last_division: i64::MIN,
            hall: Box::new(Hall::new(sr)),
            hall_p: crate::params::defaults(hall::SPECS),
            shifters: [PitchShifter::new(sr, 0.1), PitchShifter::new(sr, 0.1)],
            delay_time: -1.0,
            wet_prev: (0.0, 0.0),
            wow: 0.0,
            flutter: 0.0,
            tone: {
                let mut t = [OnePole::default(); 2];
                for f in t.iter_mut() {
                    // Tape warmth.
                    f.set(9000.0, sr);
                }
                t
            },
            held: (0.0, 0.0),
            hold: 0,
            taps: DeviceTaps { pre: None, post: ScopeTap::default(), meters: Some(MeterTap::default()) },
        }
    }

    fn unit(&mut self) -> f32 {
        (self.rng.bipolar() + 1.0) * 0.5
    }

    /// Seconds of buffer this quality has.
    fn span(quality: usize) -> f32 {
        match quality {
            0 => 4.0,
            2 => 6.0,
            _ => 8.0,
        }
    }

    /// Reads the buffer at a fractional frame (wrapping).
    fn read(&self, pos: f64) -> (f32, f32) {
        let n = self.buf[0].len();
        let p = pos.rem_euclid(n as f64);
        let i = p as usize;
        let f = (p - i as f64) as f32;
        let j = (i + 1) % n;
        (self.buf[0][i] + (self.buf[0][j] - self.buf[0][i]) * f, self.buf[1][i] + (self.buf[1][j] - self.buf[1][i]) * f)
    }

    fn spawn(&mut self, p: &[f32], quality: usize, rate: f32) {
        let Some(slot) = self.grains.iter().position(|g| !g.active) else { return };
        let r = [self.rng.bipolar(), self.rng.bipolar(), self.rng.bipolar(), self.rng.bipolar(), self.rng.bipolar()];
        let size = (p[SIZE] + p[SIZE_RAND] * r[0] * 0.5).clamp(0.0, 0.97);
        let len_s = 0.01 * 200f32.powf(size / 0.97);
        let len = (len_s * self.sr) as u32;
        // Pitch: the knob, and random octaves and fifths.
        let mut semis = p[PITCH];
        let pr = p[PITCH_RAND].clamp(0.0, 1.0);
        if pr > 0.0 && self.unit() < pr {
            let choices = [12.0, -12.0, 7.0, -5.0, 19.0, 24.0, -24.0];
            let k = ((self.unit() * pr * choices.len() as f32) as usize).min(choices.len() - 1);
            semis += choices[k];
        }
        let shape = (p[SHAPE] + p[SHAPE_RAND] * r[1] * 0.5).clamp(0.0, 1.0);
        let reverse = shape > 0.85;
        let ratio = 2f64.powf(semis as f64 / 12.0) * if reverse { -1.0 } else { 1.0 };
        // Where it starts: Time back, never running into the write head.
        let span = (Self::span(quality) * self.sr) as f64;
        let avail = (self.filled as f64).min(span).max(1.0);
        let time = (p[TIME] + p[TIME_RAND] * r[2] * 0.5).clamp(0.0, 1.0) as f64;
        let ahead = len as f64 * (ratio - 1.0).max(0.0) + 64.0;
        let back = (time * avail).max(ahead).max(if reverse { len as f64 * ratio.abs() + 64.0 } else { 0.0 }).min(avail);
        let start = self.write as f64 - back;
        // Overlapping grains share the level.
        let overlap = (rate * len_s).max(1.0);
        let pan = p[SPREAD].clamp(0.0, 1.0) * r[3];
        let level = 1.0 / overlap.sqrt();
        let gain = ((1.0 - pan).min(1.0) * level, (1.0 + pan).min(1.0) * level);
        self.grains[slot] = Grain { active: true, pos: start, inc: ratio, len: len.max(16), age: 0, shape, gain, semis };
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        if p.len() < SPECS.len() {
            return;
        }
        let n = l.len().min(r.len());
        let mut done = 0;
        while done < n {
            let len = CHUNK.min(n - done);
            let beat = ctx.beat + done as f64 / (60.0 / ctx.bpm.max(1.0) as f64 * self.sr as f64);
            let c = FxCtx { beat, ..*ctx };
            self.process_chunk(p, &mut l[done..done + len], &mut r[done..done + len], &c);
            done += len;
        }
    }

    fn process_chunk(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], ctx: &FxCtx) {
        let n = l.len().min(r.len());
        let quality = (p[QUALITY].round().max(0.0) as usize).min(QUALITIES.len() - 1);
        let freeze = p[FREEZE] >= 0.5;
        let feedback = p[FEEDBACK].clamp(0.0, 1.0) * 0.95;
        let density = p[DENSITY].clamp(-1.0, 1.0);
        let rate = 0.5 * 160f32.powf(density.abs());
        let delay_mode = p[SIZE] >= 0.98;
        let mix = p[MIX].clamp(0.0, 1.0);
        let (dry_g, wet_g) = ((mix * FRAC_PI_2).cos(), (mix * FRAC_PI_2).sin());
        let blen = self.buf[0].len();
        let spb = 60.0 / ctx.bpm.max(1.0) * self.sr;
        let divisions = [4.0, 2.0, 1.0, 0.5, 0.25, 0.125, 0.0625, 0.03125];
        let (mut wl, mut wr) = ([0.0f32; CHUNK], [0.0f32; CHUNK]);
        for i in 0..n {
            let (x, y) = (l[i], r[i]);
            // Record (with feedback), coloured by the quality.
            if !freeze {
                let (mut a, mut b) = (x + self.wet_prev.0 * feedback, y + self.wet_prev.1 * feedback);
                if feedback > 0.0 {
                    a = fast_tanh(a);
                    b = fast_tanh(b);
                }
                match quality {
                    1 => {
                        let m = (a + b) * 0.5;
                        (a, b) = (m, m);
                    }
                    2 => {
                        a = self.tone[0].lp(fast_tanh(a * 1.3) / 1.1);
                        b = self.tone[1].lp(fast_tanh(b * 1.3) / 1.1);
                    }
                    3 => {
                        if self.hold == 0 {
                            let crush = |v: f32| {
                                // 8-bit mu-law.
                                let m = v.clamp(-1.0, 1.0);
                                let c = m.signum() * (1.0 + 255.0 * m.abs()).ln() / 256f32.ln();
                                let q = (c * 127.0).round() / 127.0;
                                q.signum() * ((256f32).powf(q.abs()) - 1.0) / 255.0
                            };
                            self.held = (crush((a + b) * 0.5), crush((a + b) * 0.5));
                        }
                        self.hold = (self.hold + 1) % 4;
                        (a, b) = self.held;
                    }
                    _ => {}
                }
                self.buf[0][self.write] = a;
                self.buf[1][self.write] = b;
                self.write = (self.write + 1) % blen;
                self.filled = (self.filled + 1).min(blen);
            }
            // Cassette wow and flutter move the read heads a little.
            let wobble = if quality == 2 {
                self.wow = (self.wow + 0.6 / self.sr).fract();
                self.flutter = (self.flutter + 7.5 / self.sr).fract();
                ((self.wow * TAU).sin() * 0.0025 + (self.flutter * TAU).sin() * 0.0004) as f64 * self.sr as f64
            } else {
                0.0
            };
            let (mut gl, mut gr) = (0.0f32, 0.0f32);
            if delay_mode {
                // One read head: Time sets the delay, Pitch shifts it.
                let span = Self::span(quality) * self.sr;
                let want = (0.01 * self.sr + p[TIME].clamp(0.0, 1.0) * (span - 0.02 * self.sr)).min(self.filled.max(1) as f32);
                if self.delay_time < 0.0 {
                    self.delay_time = want;
                }
                self.delay_time += (want - self.delay_time) * 0.0005;
                let (a, b) = self.read(self.write as f64 - self.delay_time as f64 + wobble);
                let ratio = 2f32.powf(p[PITCH] / 12.0);
                let win = 0.08 * self.sr;
                (gl, gr) = if (ratio - 1.0).abs() < 1e-4 { (a, b) } else { (self.shifters[0].tick(a, ratio, win), self.shifters[1].tick(b, ratio, win)) };
            } else {
                // When the next grain starts.
                if density.abs() > 0.02 {
                    if density < 0.0 && p[SYNC] >= 0.5 && ctx.playing {
                        let div = divisions[((density.abs() * (divisions.len() - 1) as f32).round() as usize).min(divisions.len() - 1)];
                        let beat = ctx.beat + i as f64 / spb as f64;
                        let k = (beat / div as f64).floor() as i64;
                        if k != self.last_division {
                            self.last_division = k;
                            self.spawn(p, quality, (1.0 / (div * 60.0 / ctx.bpm.max(1.0))).max(0.1));
                        }
                    } else {
                        self.countdown -= 1.0;
                        if self.countdown <= 0.0 {
                            self.spawn(p, quality, rate);
                            self.countdown += if density > 0.0 {
                                // Random: exponential gaps.
                                -(self.unit().max(1e-4) as f64).ln() * self.sr as f64 / rate as f64
                            } else {
                                self.sr as f64 / rate as f64
                            };
                        }
                    }
                }
                for g in 0..MAX_GRAINS {
                    if !self.grains[g].active {
                        continue;
                    }
                    let gr_ = self.grains[g];
                    let t = gr_.age as f32 / gr_.len as f32;
                    let env = envelope(gr_.shape, t);
                    let (a, b) = self.read(gr_.pos + wobble);
                    gl += a * env * gr_.gain.0;
                    gr += b * env * gr_.gain.1;
                    let gm = &mut self.grains[g];
                    gm.pos += gm.inc;
                    gm.age += 1;
                    if gm.age >= gm.len {
                        gm.active = false;
                    }
                }
            }
            self.wet_prev = (gl, gr);
            wl[i] = gl;
            wr[i] = gr;
        }
        // The hall on the grains.
        let rv = p[REVERB].clamp(0.0, 1.0);
        if rv > 0.001 {
            let hp = &mut self.hall_p;
            hp[hall::PREDELAY] = 20.0;
            hp[hall::SIZE] = 0.8;
            hp[hall::DECAY] = 4.0;
            hp[hall::MIX] = rv;
            self.hall.process(hp, &mut wl[..n], &mut wr[..n], ctx, None);
        }
        for i in 0..n {
            l[i] = l[i] * dry_g + wl[i] * wet_g;
            r[i] = r[i] * dry_g + wr[i] * wet_g;
        }
        // For the display: grains as (where they read, how long, envelope and pitch).
        if let Some(m) = &mut self.taps.meters {
            let span = (Self::span(quality) * self.sr).max(1.0);
            m.values = [0.0; crate::scope::METER_VALUES];
            m.values[0] = self.grains.iter().filter(|g| g.active).count() as f32;
            m.values[1] = freeze as u8 as f32;
            m.values[2] = (self.filled as f32 / span).min(1.0);
            for (k, g) in self.grains.iter().filter(|g| g.active).take(SHOWN).enumerate() {
                let back = (self.write as f64 - g.pos).rem_euclid(blen as f64) as f32;
                let env = envelope(g.shape, g.age as f32 / g.len as f32);
                m.values[3 + k * 3] = (back / span).clamp(0.0, 1.0);
                m.values[4 + k * 3] = (g.len as f32 * g.inc.abs() as f32 / span).clamp(0.0, 1.0);
                m.values[5 + k * 3] = (g.semis.round() + 48.0) + env.clamp(0.0, 1.0) * 0.99;
            }
            if delay_mode {
                m.values[30] = (self.delay_time / span).clamp(0.0, 1.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(p: &[f32], secs: f32) -> (Vec<f32>, Grains) {
        let sr = 48000.0;
        let mut g = Grains::new(sr);
        let ctx = FxCtx { sr, bpm: 120.0, beat: 0.0, playing: true };
        let n = (secs * sr) as usize;
        let mut out = Vec::with_capacity(n);
        let mut t = 0usize;
        while t < n {
            let len = 256.min(n - t);
            // A 220 Hz tone for the first second, then silence.
            let mut l: Vec<f32> = (t..t + len).map(|i| if i < 48000 { (TAU * 220.0 * i as f32 / sr).sin() * 0.5 } else { 0.0 }).collect();
            let mut r = l.clone();
            g.process(p, &mut l, &mut r, &ctx);
            out.extend_from_slice(&l);
            t += len;
        }
        (out, g)
    }

    fn rms(v: &[f32]) -> f32 {
        (v.iter().map(|x| x * x).sum::<f32>() / v.len().max(1) as f32).sqrt()
    }

    #[test]
    fn grains_keep_sounding_after_the_input_stops() {
        let mut p = crate::params::defaults(SPECS);
        p[MIX] = 1.0;
        p[REVERB] = 0.0;
        p[TIME] = 0.6;
        let (out, _) = run(&p, 3.0);
        assert!(out.iter().all(|x| x.is_finite()));
        // Two seconds later the buffer still holds the tone.
        assert!(rms(&out[(2.0 * 48000.0) as usize..(2.5 * 48000.0) as usize]) > 0.01);
    }

    #[test]
    fn no_density_no_grains() {
        let mut p = crate::params::defaults(SPECS);
        p[MIX] = 1.0;
        p[REVERB] = 0.0;
        p[DENSITY] = 0.0;
        let (out, g) = run(&p, 1.0);
        assert!(rms(&out) < 1e-4);
        assert!(g.grains.iter().all(|g| !g.active));
    }

    #[test]
    fn freeze_holds_the_buffer() {
        let mut p = crate::params::defaults(SPECS);
        p[FREEZE] = 1.0;
        p[MIX] = 1.0;
        p[REVERB] = 0.0;
        let (out, g) = run(&p, 1.0);
        assert_eq!(g.filled, 0, "nothing recorded");
        assert!(rms(&out) < 1e-4);
    }

    #[test]
    fn pitch_moves_the_grains() {
        // An octave up: the tone comes back at 440 Hz (zero crossings double).
        let mut p = crate::params::defaults(SPECS);
        p[MIX] = 1.0;
        p[REVERB] = 0.0;
        p[TIME_RAND] = 0.0;
        p[SHAPE] = 0.4;
        p[DENSITY] = -0.6;
        p[SIZE] = 0.6;
        p[TIME] = 0.1;
        let crossings = |p: &[f32]| {
            let (out, _) = run(p, 1.0);
            let s = &out[24000..47000];
            s.windows(2).filter(|w| (w[0] < 0.0) != (w[1] < 0.0)).count()
        };
        let base = crossings(&p);
        p[PITCH] = 12.0;
        let up = crossings(&p);
        assert!((up as f32) > base as f32 * 1.6, "{base} vs {up}");
    }

    #[test]
    fn biggest_size_is_a_delay() {
        let mut p = crate::params::defaults(SPECS);
        p[SIZE] = 1.0;
        p[MIX] = 1.0;
        p[REVERB] = 0.0;
        p[TIME] = 0.1;
        let (out, g) = run(&p, 2.0);
        assert!(g.grains.iter().all(|g| !g.active), "no grains in delay mode");
        // The tone ends at 1 s and comes back a little later.
        assert!(rms(&out[(1.2 * 48000.0) as usize..(1.4 * 48000.0) as usize]) > 0.05);
    }

    #[test]
    fn every_quality_works() {
        for q in 0..QUALITIES.len() {
            let mut p = crate::params::defaults(SPECS);
            p[QUALITY] = q as f32;
            p[MIX] = 1.0;
            p[FEEDBACK] = 0.5;
            p[DENSITY] = 0.8;
            let (out, _) = run(&p, 1.5);
            assert!(out.iter().all(|x| x.is_finite() && x.abs() < 4.0), "{}", QUALITIES[q]);
            assert!(rms(&out) > 0.005, "{}", QUALITIES[q]);
        }
    }
}
