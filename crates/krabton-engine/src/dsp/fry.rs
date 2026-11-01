//! The FRY section of Analog Princess, shared by the machine (its master
//! section, see `princess_machine`), the Fryer effect (`fx::fryer`) and
//! Flipside: what is sent to it through a ring modulator, a drive and a
//! crusher ([`Bus::fry`]); then the time effects and the ceiling
//! ([`Bus::tick`]): a stutter (a slice repeated), a tape stop (slowing to a
//! stop), channel surfing (the sound flipping between a radio band, a
//! muffled room, a tinny speaker, an octave down or up and static for a
//! moment), and a clip into the ceiling (soft or hard) behind a limiter that
//! never lets a peak over it.
//!
//! Beside it the stages of its own that the Fryer adds to any sound: the
//! 808 into another 808 (a saturation into a hard clip, then into a second,
//! as an 808 put into another 808 is; [`cascade`]) and the subliminal shadow
//! under it (a quiet copy, backwards and pitched down, low-passed;
//! [`Shadow`]).

use super::*;

/// A time effect starting at a sample of the block.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Fx {
    /// A slice this long repeated for so long (samples).
    Stutter(u32, u32),
    /// Slowing to a stop over so long.
    Tape(u32),
    /// Flipping channels for so long.
    Surf(u32),
}

/// The FRY section's knobs for a block.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BusKnobs {
    pub(crate) drive: f32,
    pub(crate) steps: f32,
    pub(crate) rate: f32,
    pub(crate) crush: bool,
    pub(crate) ring: f32,
    pub(crate) ring_inc: f32,
    pub(crate) push: f32,
    pub(crate) hard: bool,
    pub(crate) ceiling: f32,
    pub(crate) volume: f32,
    pub(crate) lim_rel: f32,
    pub(crate) stutter_hold: bool,
    pub(crate) stutter_slice: u32,
    pub(crate) tape_hold: bool,
    pub(crate) tape_len: u32,
    pub(crate) surf_hold: bool,
}

/// The FRY section and the kit's time effects.
pub(crate) struct Bus {
    pub(crate) sr: f32,
    pub(crate) rng: Rng,
    pub(crate) ring_ph: f32,
    pub(crate) crushed: [f32; 2],
    pub(crate) crush_ph: f32,
    /// What came in, for the stutter.
    pub(crate) cap: Vec<[f32; 2]>,
    pub(crate) cap_w: usize,
    pub(crate) stut_len: u32,
    pub(crate) stut_t: u32,
    pub(crate) stut_left: u32,
    pub(crate) stut_start: usize,
    pub(crate) stut_mix: f32,
    pub(crate) stut_held: bool,
    /// What the stutter let through, for the tape and the channels.
    pub(crate) vari: Vec<[f32; 2]>,
    pub(crate) vari_w: usize,
    pub(crate) tape_len: u32,
    pub(crate) tape_t: u32,
    pub(crate) tape_left: u32,
    pub(crate) tape_read: f64,
    pub(crate) tape_mix: f32,
    pub(crate) tape_held: bool,
    pub(crate) surf_left: u32,
    pub(crate) surf_chan: u32,
    pub(crate) surf_t: u32,
    pub(crate) surf_kind: u8,
    pub(crate) surf_rate: f64,
    pub(crate) surf_read: f64,
    pub(crate) surf_mix: f32,
    pub(crate) surf_held: bool,
    pub(crate) surf_on: bool,
    pub(crate) surf_filt: [Svf; 2],
    pub(crate) surf_ph: f32,
    pub(crate) lim: f32,
}

impl Bus {
    pub(crate) fn new(sr: f32) -> Self {
        Self {
            sr,
            rng: Rng::new(0xF2_1ED5),
            ring_ph: 0.0,
            crushed: [0.0; 2],
            crush_ph: 0.0,
            cap: vec![[0.0; 2]; (2.0 * sr) as usize + 2],
            cap_w: 0,
            stut_len: 0,
            stut_t: 0,
            stut_left: 0,
            stut_start: 0,
            stut_mix: 0.0,
            stut_held: false,
            vari: vec![[0.0; 2]; (3.0 * sr) as usize + 2],
            vari_w: 0,
            tape_len: 0,
            tape_t: 0,
            tape_left: 0,
            tape_read: 0.0,
            tape_mix: 0.0,
            tape_held: false,
            surf_left: 0,
            surf_chan: 0,
            surf_t: 0,
            surf_kind: 0,
            surf_rate: 1.0,
            surf_read: 0.0,
            surf_mix: 0.0,
            surf_held: false,
            surf_on: false,
            surf_filt: [Svf::new(); 2],
            surf_ph: 0.0,
            lim: 1.0,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.stut_len = 0;
        self.stut_left = 0;
        self.stut_mix = 0.0;
        self.tape_len = 0;
        self.tape_left = 0;
        self.tape_mix = 0.0;
        self.surf_left = 0;
        self.surf_on = false;
        self.surf_mix = 0.0;
    }

    pub(crate) fn start(&mut self, fx: Fx) {
        match fx {
            Fx::Stutter(slice, len) => {
                if self.stut_left == 0 && !self.stut_held {
                    self.stut_len = slice.clamp(16, self.cap.len() as u32 - 2);
                    self.stut_t = 0;
                    self.stut_start = self.cap_w;
                }
                self.stut_left = self.stut_left.max(len);
            }
            Fx::Tape(len) => {
                if self.tape_left == 0 && !self.tape_held {
                    self.start_tape(len);
                }
            }
            Fx::Surf(len) => {
                if !self.surf_on {
                    self.surf_on = true;
                    self.new_channel();
                }
                self.surf_left = self.surf_left.max(len);
            }
        }
    }

    pub(crate) fn start_tape(&mut self, len: u32) {
        // At most so long that the reader stays in the buffer.
        self.tape_len = len.clamp(64, (self.vari.len() as u32 - 4) * 2 - 8);
        self.tape_left = self.tape_len;
        self.tape_t = 0;
        self.tape_read = self.vari_w as f64;
    }

    /// The buttons to hold: what they start comes on as one goes down and
    /// lets go as it comes up.
    pub(crate) fn holds(&mut self, k: &BusKnobs) {
        if k.stutter_hold && !self.stut_held {
            self.start(Fx::Stutter(k.stutter_slice, 0));
        }
        self.stut_held = k.stutter_hold;
        if k.tape_hold && !self.tape_held {
            self.start_tape(k.tape_len);
        }
        self.tape_held = k.tape_hold;
        if k.surf_hold && !self.surf_held {
            self.start(Fx::Surf(0));
        }
        self.surf_held = k.surf_hold;
        if !k.surf_hold && self.surf_left == 0 {
            self.surf_on = false;
        }
    }

    /// Whether a stutter, a tape stop or a channel flip is going (for the
    /// lights of the faces), and how much the limiter takes (0..1).
    pub(crate) fn lights(&self) -> [f32; 4] {
        let on = |b: bool| if b { 1.0 } else { 0.0 };
        [on(self.stut_len > 0 && (self.stut_left > 0 || self.stut_held)), on(self.tape_len > 0), on(self.surf_on), 1.0 - self.lim]
    }

    /// Flips to another channel.
    pub(crate) fn new_channel(&mut self) {
        let sr = self.sr;
        self.surf_t = 0;
        self.surf_chan = ((0.04 + 0.13 * self.rng.unipolar()) * sr) as u32;
        let kind = (self.rng.unipolar() * 6.0) as u8 % 6;
        // Not the same channel twice.
        self.surf_kind = if kind == self.surf_kind { (kind + 1) % 6 } else { kind };
        let vlen = self.vari.len() as f64;
        match self.surf_kind {
            0 => {
                let f = 900.0 + 1800.0 * self.rng.unipolar();
                self.surf_filt[0].set_q(f, 2.5, sr);
                self.surf_filt[1].set_q(f, 2.5, sr);
            }
            1 => {
                let f = 250.0 + 450.0 * self.rng.unipolar();
                self.surf_filt[0].set_q(f, 0.9, sr);
                self.surf_filt[1].set_q(f, 0.9, sr);
            }
            2 => {
                self.surf_filt[0].set_q(1800.0, 0.8, sr);
                self.surf_filt[1].set_q(1800.0, 0.8, sr);
            }
            3 => {
                // An octave or a fifth down: read more slowly than it is written.
                self.surf_rate = if self.rng.unipolar() < 0.5 { 0.5 } else { 0.667 };
                self.surf_read = self.vari_w as f64;
            }
            4 => {
                // Up: read faster, from far enough back to never catch up.
                self.surf_rate = if self.rng.unipolar() < 0.5 { 2.0 } else { 1.5 };
                let back = (self.surf_chan as f64 * (self.surf_rate - 1.0)).min(vlen - 4.0);
                self.surf_read = (self.vari_w as f64 - back).rem_euclid(vlen);
            }
            _ => {
                self.surf_filt[0].set_q(3000.0 + 3000.0 * self.rng.unipolar(), 1.2, sr);
                self.surf_filt[1].set_q(3000.0, 1.2, sr);
            }
        }
    }

    #[inline]
    pub(crate) fn read(buf: &[[f32; 2]], at: f64) -> [f32; 2] {
        let n = buf.len();
        let i = at.floor();
        let f = (at - i) as f32;
        let i0 = (i as i64).rem_euclid(n as i64) as usize;
        let i1 = (i0 + 1) % n;
        [buf[i0][0] + (buf[i1][0] - buf[i0][0]) * f, buf[i0][1] + (buf[i1][1] - buf[i0][1]) * f]
    }

    /// The fry chain on what is sent to it.
    #[inline]
    pub(crate) fn fry(&mut self, x: [f32; 2], k: &BusKnobs) -> [f32; 2] {
        let mut y = x;
        if k.ring > 0.0 {
            let m = 1.0 - k.ring + k.ring * (self.ring_ph * TAU).sin();
            self.ring_ph = (self.ring_ph + k.ring_inc).fract();
            y = [y[0] * m, y[1] * m];
        }
        if k.drive > 0.0 {
            let g = 1.0 + 14.0 * k.drive * k.drive;
            let comp = 1.0 / (1.0 + 0.6 * k.drive);
            y = [fast_tanh(y[0] * g) * comp, fast_tanh(y[1] * g) * comp];
        }
        if k.crush {
            self.crush_ph += k.rate;
            if self.crush_ph >= 1.0 {
                self.crush_ph -= 1.0;
                self.crushed = [(y[0] * k.steps).round() / k.steps, (y[1] * k.steps).round() / k.steps];
            }
            y = self.crushed;
        }
        y
    }

    /// The time effects, then the clip into the ceiling.
    #[inline]
    pub(crate) fn tick(&mut self, x: [f32; 2], k: &BusKnobs) -> [f32; 2] {
        let sr = self.sr;
        let smooth = 1.0 - (-1.0 / (0.002 * sr)).exp();
        // The stutter: the slice as it came in, then again and again.
        let clen = self.cap.len();
        self.cap[self.cap_w] = x;
        let mut y = x;
        if self.stut_len > 0 {
            let on = self.stut_left > 0 || self.stut_held;
            let len = self.stut_len;
            let ph = self.stut_t % len;
            let s = self.cap[(self.stut_start + ph as usize) % clen];
            // Short fades where it jumps back.
            let fade = (0.002 * sr).max(1.0);
            let mut g = ((len - ph) as f32 / fade).min(1.0);
            if self.stut_t >= len {
                g = g.min((ph as f32 + 1.0) / fade);
            }
            self.stut_mix += ((on as u8 as f32) - self.stut_mix) * smooth;
            y = [x[0] + (s[0] * g - x[0]) * self.stut_mix, x[1] + (s[1] * g - x[1]) * self.stut_mix];
            self.stut_t = self.stut_t.wrapping_add(1);
            self.stut_left = self.stut_left.saturating_sub(1);
            if !on && self.stut_mix < 1e-3 {
                self.stut_len = 0;
            }
        }
        self.cap_w = (self.cap_w + 1) % clen;
        // Tape and channels read what the stutter let through.
        let vlen = self.vari.len();
        self.vari[self.vari_w] = y;
        if self.tape_len > 0 {
            let on = self.tape_left > 0 || self.tape_held;
            let rate = (1.0 - self.tape_t as f64 / self.tape_len as f64).max(0.0);
            let s = Self::read(&self.vari, self.tape_read);
            let g = (rate as f32).sqrt();
            self.tape_read = (self.tape_read + rate).rem_euclid(vlen as f64);
            self.tape_mix += ((on as u8 as f32) - self.tape_mix) * smooth;
            y = [y[0] + (s[0] * g - y[0]) * self.tape_mix, y[1] + (s[1] * g - y[1]) * self.tape_mix];
            self.tape_t = (self.tape_t + 1).min(self.tape_len);
            self.tape_left = self.tape_left.saturating_sub(1);
            if !on && self.tape_mix < 1e-3 {
                self.tape_len = 0;
            }
        } else if self.surf_on || self.surf_mix > 1e-3 {
            let n = self.rng.bipolar();
            let s = match self.surf_kind {
                // A radio: a narrow band and a little hiss.
                0 => [self.surf_filt[0].process(y[0], FilterMode::Band) * 2.2 + n * 0.05, self.surf_filt[1].process(y[1], FilterMode::Band) * 2.2 + n * 0.05],
                // Through the wall.
                1 => [self.surf_filt[0].process(y[0], FilterMode::Low) * 1.3, self.surf_filt[1].process(y[1], FilterMode::Low) * 1.3],
                // A tinny speaker, ringing.
                2 => {
                    self.surf_ph = (self.surf_ph + 900.0 / sr).fract();
                    let m = 0.5 + 0.5 * (self.surf_ph * TAU).sin();
                    let mono = (y[0] + y[1]) * 0.5;
                    let t = self.surf_filt[0].process(mono, FilterMode::High) * (0.6 + 0.8 * m) * 1.8;
                    [t, t]
                }
                // Pitched down or up.
                3 | 4 => {
                    let s = Self::read(&self.vari, self.surf_read);
                    self.surf_read = (self.surf_read + self.surf_rate).rem_euclid(vlen as f64);
                    s
                }
                // Mostly static.
                _ => {
                    let st = self.surf_filt[0].process(n, FilterMode::Band) * 0.5;
                    [st + y[0] * 0.25, st + y[1] * 0.25]
                }
            };
            // The flip itself: a burst of static.
            let burst_len = 0.012 * sr;
            let burst = if (self.surf_t as f32) < burst_len { n * 0.3 * (1.0 - self.surf_t as f32 / burst_len) } else { 0.0 };
            let target = if self.surf_on { 1.0 } else { 0.0 };
            self.surf_mix += (target - self.surf_mix) * smooth * 2.0;
            let m = self.surf_mix;
            y = [y[0] + (s[0] + burst - y[0]) * m, y[1] + (s[1] + burst - y[1]) * m];
            self.surf_t = self.surf_t.saturating_add(1);
            if self.surf_on {
                self.surf_left = self.surf_left.saturating_sub(1);
                if self.surf_left == 0 && !self.surf_held {
                    self.surf_on = false;
                } else if self.surf_t >= self.surf_chan {
                    self.new_channel();
                }
            }
        }
        self.vari_w = (self.vari_w + 1) % vlen;
        // Pushed into a limiter a little over the ceiling, then clipped to it.
        let ceil = k.ceiling;
        let pre = [y[0] * k.push, y[1] * k.push];
        let peak = pre[0].abs().max(pre[1].abs());
        let thr = ceil * 1.4;
        if peak * self.lim > thr {
            self.lim = thr / peak.max(1e-9);
        } else {
            self.lim += (1.0 - self.lim) * k.lim_rel;
        }
        let z = [pre[0] * self.lim, pre[1] * self.lim];
        let out = if k.hard { [z[0].clamp(-ceil, ceil), z[1].clamp(-ceil, ceil)] } else { [ceil * fast_tanh(z[0] / ceil), ceil * fast_tanh(z[1] / ceil)] };
        let out = [out[0].clamp(-ceil, ceil) * k.volume, out[1].clamp(-ceil, ceil) * k.volume];
        if out[0].is_finite() && out[1].is_finite() {
            out
        } else {
            self.lim = 1.0;
            [0.0, 0.0]
        }
    }
}

/// A stutter's slice (beats) at Stutter Rate `rate` (Mixed: any of them).
pub(crate) fn stutter_slice(rate: f32, rng: &mut Rng) -> f32 {
    let i = rate.round().max(0.0) as usize;
    let i = if i >= 3 { (rng.unipolar() * 3.0) as usize % 3 } else { i };
    [0.5, 0.25, 0.125][i]
}

/// The time effects now and then, thrown at the sixteenth `step` of a bar
/// (0 to 15): a tape stop into the bar line (by chance `amounts[1]`), else
/// a stutter at the start of a beat (`amounts[0]`), else a flip of the
/// channels (`amounts[2]`, less often), each in the song's time (`spb`:
/// samples a beat) at Tape Time `tape_time` and Stutter Rate `rate`.
pub(crate) fn now_and_then(rng: &mut Rng, step: i64, amounts: [f32; 3], tape_time: f32, rate: f32, spb: f32) -> Option<Fx> {
    let [stut, tape, surf] = amounts;
    let tape_steps = [1, 2, 4, 8][(tape_time.round().max(0.0) as usize).min(3)];
    if tape > 0.0 && step == 16 - tape_steps && rng.unipolar() < tape {
        Some(Fx::Tape((tape_steps as f32 * 0.25 * spb) as u32))
    } else if stut > 0.0 && step % 4 == 0 && rng.unipolar() < stut {
        let slice = stutter_slice(rate, rng) * spb;
        let steps = if rng.unipolar() < 0.5 { 2.0 } else { 4.0 };
        let len = (steps * 0.25 * spb).max(2.0 * slice);
        Some(Fx::Stutter(slice as u32, len as u32))
    } else if surf > 0.0 && rng.unipolar() < surf * 0.3 {
        let steps = 1.0 + (rng.unipolar() * 3.0).floor();
        Some(Fx::Surf((steps * 0.25 * spb) as u32))
    } else {
        None
    }
}

// ------------------------------------------------------------ 808 into 808

/// The most 808s a sound goes through, one into the next.
pub const MAX_STAGES: usize = 3;

/// A sound put into an 808's distortion and that into another's, as the
/// machine's 808 goes into its track and the track into the FRY section:
/// each stage the 808's own distortion (soft, hard into the ceiling, folded
/// or lopsided), its top rounded off a little as a box does (so three in a
/// row grind rather than fizz), its offset taken out. The heat drives the
/// first hardest and the others a little less; each stage's loss of level
/// is made up so a cascade is about as loud as one.
pub struct Cascade {
    lp: [[OnePole; 2]; MAX_STAGES],
    dc: [[OnePole; 2]; MAX_STAGES],
    block: [[DcBlock; 2]; MAX_STAGES],
}

impl Cascade {
    pub fn new(sr: f32) -> Self {
        let mut lp = [[OnePole::default(); 2]; MAX_STAGES];
        let mut dc = [[OnePole::default(); 2]; MAX_STAGES];
        for (k, (l, d)) in lp.iter_mut().zip(dc.iter_mut()).enumerate() {
            for c in 0..2 {
                l[c].set([14_000.0, 9_000.0, 7_000.0][k], sr);
                d[c].set(12.0, sr);
            }
        }
        Self { lp, dc, block: [[DcBlock::default(); 2]; MAX_STAGES] }
    }

    /// `x` through `stages` 808s (1 to 3) at `heat` (0..1).
    #[inline]
    pub fn tick(&mut self, x: [f32; 2], heat: f32, stages: usize, dist: super::princess::Dist) -> [f32; 2] {
        let mut y = x;
        for k in 0..stages.clamp(1, MAX_STAGES) {
            let drive = heat * [1.0, 0.8, 0.65][k];
            for c in 0..2 {
                let v = super::princess::shape(y[c] * (1.0 + 2.0 * drive), drive, dist, &mut self.dc[k][c]);
                let v = if k > 0 { self.lp[k][c].lp(v) } else { v };
                y[c] = self.block[k][c].tick(v);
            }
        }
        y
    }
}

// ------------------------------------------------------------ the shadow

/// How long the shadow can look back (s).
const SHADOW_SECS: f32 = 2.0;

/// The subliminal layer: a quiet copy of a sound under it, backwards (or
/// not) and pitched down, low-passed, so it is felt more than heard. Two
/// readers, half a grain apart, each read a grain from where the sound is
/// now back into what it was (at the pitch's speed), faded in and out over
/// it in a raised sine; the two always sum to one, so it never clicks.
pub struct Shadow {
    sr: f32,
    buf: Vec<f32>,
    w: usize,
    /// How far the first reader is through its grain (0..1; the second is
    /// half a grain on), and where each one's grain began.
    ph: f32,
    from: [f64; 2],
    lp: [Svf; 2],
}

impl Shadow {
    pub fn new(sr: f32) -> Self {
        let mut s = Self { sr, buf: vec![0.0; (SHADOW_SECS * sr) as usize + 4], w: 0, ph: 0.0, from: [0.0; 2], lp: [Svf::new(); 2] };
        s.set_tone(900.0);
        s
    }

    pub fn reset(&mut self) {
        self.buf.iter_mut().for_each(|x| *x = 0.0);
    }

    /// Sets its tone (the low-pass, Hz).
    pub fn set_tone(&mut self, hz: f32) {
        for f in self.lp.iter_mut() {
            f.set(hz, 0.1, self.sr);
        }
    }

    /// Takes in `x` (mono) and gives the shadow: `semis` down (0 to 24),
    /// backwards if `reverse`, in grains of `grain` seconds (at most half
    /// of what it can look back over).
    #[inline]
    pub fn tick(&mut self, x: f32, semis: f32, reverse: bool, grain: f32) -> f32 {
        let n = self.buf.len();
        self.buf[self.w] = x;
        let rate = 2f64.powf(-(semis.clamp(0.0, 24.0) as f64) / 12.0);
        let len = (grain.clamp(0.05, SHADOW_SECS * 0.45) * self.sr).max(16.0);
        let before = self.ph;
        self.ph += 1.0 / len;
        if self.ph >= 1.0 {
            self.ph -= 1.0;
        }
        let mut out = 0.0;
        for k in 0..2 {
            let (age, was) = ((self.ph + 0.5 * k as f32).fract(), (before + 0.5 * k as f32).fract());
            if age < was {
                // A new grain: from where the sound is now.
                self.from[k] = self.w as f64;
            }
            // Backwards it runs away from the sound at 1 + rate a sample,
            // forwards it falls behind at 1 - rate: never past the buffer.
            let t = age as f64 * len as f64;
            let at = if reverse { self.from[k] - t * rate } else { self.from[k] + t * rate };
            let i = at.floor();
            let f = (at - i) as f32;
            let i0 = (i as i64).rem_euclid(n as i64) as usize;
            let i1 = (i0 + 1) % n;
            let s = self.buf[i0] + (self.buf[i1] - self.buf[i0]) * f;
            let win = (std::f32::consts::PI * age).sin();
            out += s * win * win;
        }
        self.w = (self.w + 1) % n;
        let once = self.lp[0].process(out, FilterMode::Low);
        let y = self.lp[1].process(once, FilterMode::Low);
        if y.is_finite() {
            y
        } else {
            self.lp = [Svf::new(); 2];
            0.0
        }
    }
}
