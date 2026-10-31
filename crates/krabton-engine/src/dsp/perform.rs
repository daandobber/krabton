//! The Key drawer's effect buttons: effects on the whole song, on while a
//! button is held and off when it is let go. They work on the master, after
//! its devices and before its fader.
//!
//! - *Tape Stop*: the song slows to a stop over two beats, as a tape whose
//!   motor is switched off; let go and it is back at once.
//! - *Stutter*: the eighth from the press, again and again.
//! - *Roll*: a sixteenth again and again, halving every beat it is held (a
//!   build-up), down to a 128th.
//! - *Reverse*: the beat before the press, backwards, over and over.
//! - *Filter*: a low-pass closing over a bar, down to a muffle.
//! - *Thin*: a high-pass opening over a bar, up to a radio.
//! - *Echo*: what plays goes into a dotted-eighth dub echo while held; its
//!   tail rings on after.
//! - *Crush*: fewer bits and a lower sample rate.
//!
//! The time effects (tape, stutter, roll, reverse) read the song as it came
//! in, from a history of four seconds; stutter, roll and reverse keep their
//! slice apart from it, so they go on as long as they are held. A later one
//! wins over an earlier one.

use super::{FilterMode, OnePole, Svf};

pub const TAPE: usize = 0;
pub const STUTTER: usize = 1;
pub const ROLL: usize = 2;
pub const REVERSE: usize = 3;
pub const FILTER: usize = 4;
pub const THIN: usize = 5;
pub const ECHO: usize = 6;
pub const CRUSH: usize = 7;
pub const COUNT: usize = 8;

pub const NAMES: [&str; COUNT] = ["Tape Stop", "Stutter", "Roll", "Reverse", "Filter", "Thin", "Echo", "Crush"];
pub const HINTS: [&str; COUNT] = [
    "The song slows to a stop, as a tape whose motor goes off",
    "The eighth from the press, again and again",
    "A sixteenth again and again, getting faster every beat",
    "The beat before the press, backwards",
    "A low-pass closing over a bar",
    "A high-pass opening over a bar",
    "Into a dub echo while held; it rings on after",
    "Fewer bits, a lower sample rate",
];

/// How long the history is (seconds).
const HISTORY: f32 = 4.0;

/// A time effect's state: how long it has been on.
#[derive(Clone, Copy, Default)]
struct Hold {
    t: u64,
    mix: f32,
}

pub struct Perform {
    sr: f32,
    held: u16,
    hist: Vec<[f32; 2]>,
    w: usize,
    holds: [Hold; 4],
    /// The slices of stutter, roll and reverse (by `k - STUTTER`), kept.
    slices: [Vec<[f32; 2]>; 3],
    /// The samples per beat last block (for the reverse's slice at a press).
    spb: f64,
    /// The tape's read place and how far it lags.
    tape_read: f64,
    /// The sweeps (0: open, 1: all the way).
    filter: f32,
    thin: f32,
    filters: [Svf; 4],
    tick: u32,
    echo: Vec<[f32; 2]>,
    echo_w: usize,
    echo_send: f32,
    echo_tone: [OnePole; 2],
    echo_level: f32,
    /// The echo's line has something in it.
    echo_dirty: bool,
    crush: f32,
    crushed: [f32; 2],
    crush_ph: f32,
}

impl Perform {
    pub fn new(sr: f32) -> Self {
        let n = (HISTORY * sr) as usize + 2;
        Self {
            sr,
            held: 0,
            hist: vec![[0.0; 2]; n],
            w: 0,
            holds: [Hold::default(); 4],
            slices: [vec![[0.0; 2]; n / 2], vec![[0.0; 2]; n / 2], vec![[0.0; 2]; n / 2]],
            spb: sr as f64 / 2.0,
            tape_read: 0.0,
            filter: 0.0,
            thin: 0.0,
            filters: [Svf::new(); 4],
            tick: 0,
            echo: vec![[0.0; 2]; n],
            echo_w: 0,
            echo_send: 0.0,
            echo_tone: {
                let mut t = OnePole::default();
                t.set(2_500.0, sr);
                [t; 2]
            },
            echo_level: 0.0,
            echo_dirty: false,
            crush: 0.0,
            crushed: [0.0; 2],
            crush_ph: 0.0,
        }
    }

    /// The buttons held now, a bit each (bit `TAPE` and on).
    pub fn set_held(&mut self, held: u16) {
        let pressed = held & !self.held;
        for k in 0..4 {
            if pressed & (1 << k) != 0 {
                let h = &mut self.holds[k];
                h.t = 0;
                if k == TAPE {
                    self.tape_read = self.w as f64;
                }
                if k == REVERSE {
                    // The beat before the press, backwards.
                    let n = self.hist.len();
                    let slice = &mut self.slices[REVERSE - STUTTER];
                    let len = (self.spb as usize).clamp(16, slice.len());
                    for (j, s) in slice[..len].iter_mut().enumerate() {
                        *s = self.hist[(self.w + n - 1 - j) % n];
                    }
                }
            }
        }
        self.held = held;
    }

    pub fn held(&self) -> u16 {
        self.held
    }

    fn on(&self, k: usize) -> bool {
        self.held & (1 << k) != 0
    }

    /// Whether anything is on or still ringing.
    fn busy(&self) -> bool {
        self.held != 0 || self.holds.iter().any(|h| h.mix > 1e-4) || self.filter > 0.0 || self.thin > 0.0 || self.echo_dirty || self.crush > 1e-4
    }

    #[inline]
    fn read(&self, at: f64) -> [f32; 2] {
        let n = self.hist.len();
        let i = at.floor();
        let f = (at - i) as f32;
        let a = self.hist[(i as i64).rem_euclid(n as i64) as usize];
        let b = self.hist[(i as i64 + 1).rem_euclid(n as i64) as usize];
        [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f]
    }

    /// The master's block, `spb` samples to a beat.
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32], spb: f64) {
        let n = self.hist.len();
        self.spb = spb.clamp(1.0, (n / 4) as f64);
        if !self.busy() {
            // Only remember what came by, for a press to come.
            for (a, b) in l.iter().zip(r.iter()) {
                self.hist[self.w] = [*a, *b];
                self.w = (self.w + 1) % n;
            }
            self.echo_level = 0.0;
            return;
        }
        let sr = self.sr;
        let smooth = 1.0 - (-1.0 / (0.004 * sr)).exp();
        let spb = spb.clamp(1.0, (n / 4) as f64);
        self.spb = spb;
        let max_len = self.slices[0].len() as u64;
        let fade = (0.002 * sr).max(1.0);
        let bar = (4.0 * spb) as f32;
        let back = 1.0 / (0.25 * sr);
        let echo_len = (0.75 * spb) as usize;
        for i in 0..l.len() {
            let x = [l[i], r[i]];
            self.hist[self.w] = x;
            let mut y = x;

            // Stutter and roll: a slice from the press, again and again.
            for k in [STUTTER, ROLL] {
                let on = self.on(k);
                let h = self.holds[k];
                let target = on as u8 as f32;
                let mix = h.mix + (target - h.mix) * smooth;
                self.holds[k].mix = mix;
                if mix < 1e-4 {
                    continue;
                }
                let len = if k == STUTTER {
                    0.5 * spb
                } else {
                    let beats = (h.t as f64 / spb).floor().min(3.0);
                    0.25 * spb / 2f64.powf(beats)
                };
                let len = (len as u64).clamp(16, max_len);
                let ph = h.t % len;
                // The first time round it is kept as it comes in.
                let slice = &mut self.slices[k - STUTTER];
                if h.t < len {
                    slice[ph as usize] = x;
                }
                let s = slice[ph as usize];
                let mut g = ((len - ph) as f32 / fade).min(1.0);
                if h.t >= len {
                    g = g.min((ph as f32 + 1.0) / fade);
                }
                y = [y[0] + (s[0] * g - y[0]) * mix, y[1] + (s[1] * g - y[1]) * mix];
                if on {
                    self.holds[k].t += 1;
                }
            }

            // Reverse: the beat before the press, backwards.
            {
                let on = self.on(REVERSE);
                let h = self.holds[REVERSE];
                let mix = h.mix + ((on as u8 as f32) - h.mix) * smooth;
                self.holds[REVERSE].mix = mix;
                if mix > 1e-4 {
                    let len = (spb as u64).clamp(16, max_len);
                    let ph = h.t % len;
                    let s = self.slices[REVERSE - STUTTER][ph as usize];
                    let mut g = ((len - ph) as f32 / fade).min(1.0);
                    if h.t >= len {
                        g = g.min((ph as f32 + 1.0) / fade);
                    }
                    y = [y[0] + (s[0] * g - y[0]) * mix, y[1] + (s[1] * g - y[1]) * mix];
                    if on {
                        self.holds[REVERSE].t += 1;
                    }
                }
            }

            // Tape stop: reading slower and slower, quieter as it goes.
            {
                let on = self.on(TAPE);
                let h = self.holds[TAPE];
                let mix = h.mix + ((on as u8 as f32) - h.mix) * smooth;
                self.holds[TAPE].mix = mix;
                if mix > 1e-4 {
                    let len = 2.0 * spb;
                    let rate = (1.0 - h.t as f64 / len).max(0.0);
                    let s = self.read(self.tape_read);
                    let g = (rate as f32).sqrt();
                    y = [y[0] + (s[0] * g - y[0]) * mix, y[1] + (s[1] * g - y[1]) * mix];
                    if on {
                        self.tape_read = (self.tape_read + rate).rem_euclid(n as f64);
                        self.holds[TAPE].t += 1;
                    }
                }
            }
            self.w = (self.w + 1) % n;

            // The sweeps: over a bar while held, back in a quarter second.
            self.filter = if self.on(FILTER) { (self.filter + 1.0 / bar).min(1.0) } else { (self.filter - back).max(0.0) };
            self.thin = if self.on(THIN) { (self.thin + 1.0 / bar).min(1.0) } else { (self.thin - back).max(0.0) };
            if self.tick == 0 {
                let lo = 20_000.0 * (220.0f32 / 20_000.0).powf(self.filter);
                let hi = 20.0 * (1_500.0f32 / 20.0).powf(self.thin);
                for f in &mut self.filters[..2] {
                    f.set(lo, 0.45 * self.filter, sr);
                }
                for f in &mut self.filters[2..] {
                    f.set(hi, 0.4 * self.thin, sr);
                }
            }
            self.tick = (self.tick + 1) % 16;
            if self.filter > 0.0 {
                y = [self.filters[0].process(y[0], FilterMode::Low), self.filters[1].process(y[1], FilterMode::Low)];
            } else {
                self.filters[0].reset();
                self.filters[1].reset();
            }
            if self.thin > 0.0 {
                y = [self.filters[2].process(y[0], FilterMode::High), self.filters[3].process(y[1], FilterMode::High)];
            } else {
                self.filters[2].reset();
                self.filters[3].reset();
            }

            // Crush: held, fewer bits and a lower rate.
            self.crush += ((self.on(CRUSH) as u8 as f32) - self.crush) * smooth;
            if self.crush > 1e-4 {
                self.crush_ph += 1.0 / 6.0;
                if self.crush_ph >= 1.0 {
                    self.crush_ph -= 1.0;
                    let steps = 24.0;
                    self.crushed = [(y[0] * steps).round() / steps, (y[1] * steps).round() / steps];
                }
                y = [y[0] + (self.crushed[0] - y[0]) * self.crush, y[1] + (self.crushed[1] - y[1]) * self.crush];
            }

            // Echo: fed while held, a darker repeat each time round.
            self.echo_send += ((self.on(ECHO) as u8 as f32) - self.echo_send) * smooth;
            let en = self.echo.len();
            let d = self.echo[(self.echo_w + en - echo_len.clamp(1, en - 1)) % en];
            let fb = [self.echo_tone[0].lp(d[0]), self.echo_tone[1].lp(d[1])];
            // The repeats cross over, ping-pong.
            let w = [y[1] * self.echo_send * 0.8 + fb[1] * 0.6, y[0] * self.echo_send * 0.8 + fb[0] * 0.6];
            self.echo[self.echo_w] = [super::fast_tanh(w[0]), super::fast_tanh(w[1])];
            self.echo_dirty |= self.echo_send > 1e-4;
            self.echo_w = (self.echo_w + 1) % en;
            self.echo_level = self.echo_level * 0.9995 + (d[0].abs() + d[1].abs() + self.echo_send) * 0.0005;
            y = [y[0] + d[0], y[1] + d[1]];

            l[i] = if y[0].is_finite() { y[0] } else { 0.0 };
            r[i] = if y[1].is_finite() { y[1] } else { 0.0 };
        }
        if self.echo_dirty && self.echo_level < 1e-5 && self.echo_send < 1e-4 {
            // Rung out: a clean line for the next time.
            self.echo.iter_mut().for_each(|s| *s = [0.0; 2]);
            self.echo_tone = [OnePole { z: 0.0, ..self.echo_tone[0] }; 2];
            self.echo_level = 0.0;
            self.echo_dirty = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;
    const SPB: f64 = 24_000.0;

    /// A tone that goes up in steps, so where it reads from can be heard.
    fn tone(t: usize) -> f32 {
        let f = 200.0 + (t / 4800) as f32 * 50.0;
        (std::f32::consts::TAU * f * t as f32 / SR).sin() * 0.5
    }

    fn run(p: &mut Perform, from: usize, frames: usize) -> Vec<f32> {
        let mut out = Vec::new();
        for b in (from..from + frames).step_by(256) {
            let mut l: Vec<f32> = (b..b + 256).map(tone).collect();
            let mut r = l.clone();
            p.process(&mut l, &mut r, SPB);
            out.extend_from_slice(&l);
        }
        out
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    #[test]
    fn nothing_held_leaves_the_song_alone() {
        let mut p = Perform::new(SR);
        let out = run(&mut p, 0, 48_000);
        let want: Vec<f32> = (0..48_000).map(tone).collect();
        assert!(out.iter().zip(&want).all(|(a, b)| (a - b).abs() < 1e-6));
    }

    #[test]
    fn the_stutter_repeats_the_eighth_from_the_press() {
        let mut p = Perform::new(SR);
        run(&mut p, 0, 48_128);
        p.set_held(1 << STUTTER);
        let out = run(&mut p, 48_128, 48_000);
        // An eighth is 12000 samples: the second time round is the first again.
        let a = &out[2000..10_000];
        let b = &out[14_000..22_000];
        let diff = a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0f32, f32::max);
        assert!(diff < 0.02, "repeats: {diff}");
    }

    #[test]
    fn held_long_they_go_on() {
        // Longer than the history: the stutter and the reverse still repeat their slice.
        for k in [STUTTER, REVERSE] {
            let mut p = Perform::new(SR);
            run(&mut p, 0, 48_128);
            p.set_held(1 << k);
            let out = run(&mut p, 48_128, 480_000);
            let len = if k == STUTTER { 12_000 } else { 24_000 };
            let a = &out[400_000..400_000 + 4_000];
            let b = &out[400_000 + len..400_000 + len + 4_000];
            let diff = a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0f32, f32::max);
            assert!(diff < 0.02, "{}: {diff}", NAMES[k]);
            let live: Vec<f32> = (48_128 + 400_000..48_128 + 404_000).map(tone).collect();
            let off = a.iter().zip(&live).map(|(x, y)| (x - y).abs()).fold(0.0f32, f32::max);
            assert!(off > 0.1, "{} is not the song as it plays", NAMES[k]);
        }
    }

    #[test]
    fn the_tape_stops_and_comes_back_when_let_go() {
        let mut p = Perform::new(SR);
        run(&mut p, 0, 48_128);
        p.set_held(1 << TAPE);
        let out = run(&mut p, 48_128, 72_192);
        // Two beats (48000 samples) to a stop: silent after.
        assert!(rms(&out[50_000..]) < 1e-3, "stopped: {}", rms(&out[50_000..]));
        p.set_held(0);
        let back = run(&mut p, 120_320, 9_600);
        let want: Vec<f32> = (120_320 + 4_800..120_320 + 9_600).map(tone).collect();
        let diff = back[4_800..].iter().zip(&want).map(|(x, y)| (x - y).abs()).fold(0.0f32, f32::max);
        assert!(diff < 1e-3, "back: {diff}");
    }

    #[test]
    fn the_filter_closes_and_opens_again() {
        let mut p = Perform::new(SR);
        p.set_held(1 << FILTER);
        let out = run(&mut p, 0, 96_000);
        assert!(rms(&out[90_000..]) < 0.5 * rms(&out[..4_800]), "muffled");
        p.set_held(0);
        let out = run(&mut p, 96_000, 48_000);
        let want: Vec<f32> = (96_000 + 24_000..96_000 + 48_000).map(tone).collect();
        let diff = out[24_000..].iter().zip(&want).map(|(x, y)| (x - y).abs()).fold(0.0f32, f32::max);
        assert!(diff < 1e-6, "open again: {diff}");
    }

    #[test]
    fn the_echo_rings_on_after_it_is_let_go() {
        let mut p = Perform::new(SR);
        p.set_held(1 << ECHO);
        run(&mut p, 0, 24_064);
        p.set_held(0);
        // The song stops: only the echo is left.
        let mut l = vec![0.0f32; 24_000];
        let mut r = l.clone();
        p.process(&mut l, &mut r, SPB);
        assert!(rms(&l) > 0.02, "rings: {}", rms(&l));
    }

    #[test]
    fn everything_at_once_stays_in_bounds() {
        let mut p = Perform::new(SR);
        p.set_held((1 << COUNT) - 1);
        let out = run(&mut p, 0, 192_000);
        assert!(out.iter().all(|v| v.is_finite() && v.abs() < 3.0));
        p.set_held(0);
        let out = run(&mut p, 192_000, 48_000);
        assert!(out.iter().all(|v| v.is_finite()));
    }
}
