//! Free reeds, as in a melodica, a harmonica, an accordion: a brass tongue
//! swings through a slot and lets the air through in a pulse once a cycle.
//! What is heard is how fast that flow changes: the pulse's spectrum tilted
//! up (Radiate: from the soft pulse itself to its sharp edges, buzzy), the
//! pulse narrower and brighter when blown harder, the highest harmonics
//! softened by the reed itself. The breath is not a hiss on top: the air
//! only rushes while the reed is out of the slot, so it pulses with the
//! tone, plus a puff as the note starts. The tongue's second mode (at 6.27
//! times the note, a cantilever's) pings as it starts. A reed takes a moment
//! to start, sinks a little flat when blown hard, and a second reed a few
//! cents off makes it beat (tremolo, musette).
//!
//! The reed here is shared by the Melodica and the Harmonica, which decide
//! which reeds sound and how they are blown. Original code.

use super::{OnePole, Rng};
use std::f32::consts::{LN_2, PI, TAU};

/// Harmonics at most (fewer when they would pass the Nyquist frequency).
pub const MAX_H: usize = 64;
/// A reed that belongs to no key.
pub const NONE: u8 = 255;

/// The harmonics of a reed's air pulse. The pulse opens sharply and closes
/// more softly, so it falls like a sawtooth (1/k) with a gentle ripple where
/// a square-edged pulse would have holes; `duty` is the share of the cycle
/// it is open (narrower: brighter, the ripple further up), `bright` how much
/// each harmonic keeps of the one below, `radiate` how much of the pulse's
/// change is heard rather than the pulse (0..1, up to a flat buzz). Their
/// power is kept the same, so a brighter reed is not a louder one.
pub fn reed_spectrum(out: &mut [f32; MAX_H], f: f32, sr: f32, duty: f32, bright: f32, radiate: f32) -> usize {
    let lim = 0.45 * sr;
    let mut n = 0;
    let mut tilt = 1.0;
    let mut power = 0.0;
    let step = bright.powf((duty / 0.3).clamp(0.3, 3.0));
    for (k, a) in out.iter_mut().enumerate() {
        let h = (k + 1) as f32;
        if h * f >= lim || tilt < 2e-4 {
            break;
        }
        let ripple = 1.0 - 0.22 * (1.0 - (TAU * h * duty).cos());
        *a = h.powf(radiate - 1.0) * tilt * ripple;
        power += *a * *a;
        tilt *= step;
        n = k + 1;
    }
    let norm = 0.8 / power.sqrt().max(1e-6);
    for a in out[..n].iter_mut() {
        *a *= norm;
    }
    n
}

/// How a block of reeds is blown.
pub struct Blow<'a> {
    pub sr: f32,
    pub att: f32,
    pub rel: f32,
    pub glide_k: f32,
    /// The reed's brightness and how open its pulse is (0..1 each).
    pub bright: f32,
    pub duty: f32,
    pub breath: f32,
    /// A second reed, this many semitones off (0: none).
    pub beat: f32,
    /// The breath's vibrato for each sample (-1..1, already scaled by depth),
    /// and how much of it is loudness and how much pitch.
    pub vib: &'a [f32],
    pub vib_amp: f32,
    pub vib_pitch: f32,
    pub pitch_bend: f32,
    /// How much of the flow's change is heard (0 soft, 1 buzzy).
    pub radiate: f32,
    /// The tongue's ping as it starts.
    pub ping: f32,
    /// What each reed gets of the air (one breath for several).
    pub share: f32,
    /// A tongued start: a push of pressure on a hard note ("pah").
    pub tongue: f32,
    /// A puff of air as a key lets go and its valve shuts.
    pub puff_off: f32,
    /// An instrument's own measured tone instead of the plain reed: the
    /// level (dB) of a harmonic at a frequency, given its number, the key,
    /// how hard it is blown (0..1), how far it has started (0..1) and how
    /// long it has sounded (seconds).
    pub shape: Option<&'a dyn Fn(f32, usize, u8, f32, f32, f32) -> f32>,
}

/// One reed sounding.
pub struct Reed {
    pub note: u8,
    /// The key that made it.
    pub owner: u8,
    pub gate: bool,
    pub f: f32,
    pub target: f32,
    pub env: f32,
    pub t: f32,
    pub vel: f32,
    /// Its share of the sound.
    pub amp: f32,
    /// Darker (below 1) or thinner (above) than a plain reed.
    pub dark: f32,
    /// A wobble in the pitch (an overblow), 0..1.
    pub rough: f32,
    /// It starts this many semitones off and gets there in `bend_t` seconds.
    pub bend_from: f32,
    pub bend_t: f32,
    /// As it dies away its pitch drifts up this far (a bend let go).
    pub rise: f32,
    /// More breath than tone (a bend, an overblow).
    pub airy: f32,
    pub pan: f32,
    pub level: f32,
    /// Where it is on a harmonica: the hole (1..10, 0: none), drawn or
    /// blown, and how it is played (0 plain, 1 bent, 2 overblown).
    pub hole: u8,
    pub draw: bool,
    pub tech: u8,
    phase: f32,
    phase2: f32,
    ping_ph: f32,
    wobble: f32,
    /// Seconds since it was let go.
    off_t: f32,
    was_gate: bool,
    amps: [f32; MAX_H],
    n_h: usize,
    hp: OnePole,
    lp: OnePole,
}

impl Default for Reed {
    fn default() -> Self {
        Self {
            note: 60,
            owner: NONE,
            gate: false,
            f: 261.6,
            target: 261.6,
            env: 0.0,
            t: 0.0,
            vel: 0.0,
            amp: 1.0,
            dark: 1.0,
            rough: 0.0,
            bend_from: 0.0,
            bend_t: 0.05,
            rise: 0.0,
            airy: 1.0,
            pan: 0.0,
            level: 0.0,
            hole: 0,
            draw: false,
            tech: 0,
            phase: 0.0,
            phase2: 0.37,
            ping_ph: 0.0,
            wobble: 0.0,
            off_t: 0.0,
            was_gate: false,
            amps: [0.0; MAX_H],
            n_h: 0,
            hp: OnePole::default(),
            lp: OnePole::default(),
        }
    }
}

impl Reed {
    pub fn quiet(&self) -> bool {
        !self.gate && self.env < 1e-4
    }

    /// Starts it on `note` at `f` Hz, plain.
    pub fn start(&mut self, note: u8, owner: u8, f: f32, vel: f32, amp: f32, pan: f32) {
        self.note = note;
        self.owner = owner;
        self.f = f;
        self.target = f;
        self.t = 0.0;
        self.vel = vel;
        self.amp = amp;
        self.dark = 1.0;
        self.rough = 0.0;
        self.bend_from = 0.0;
        self.rise = 0.0;
        self.airy = 1.0;
        self.pan = pan;
        self.hole = 0;
        self.draw = false;
        self.tech = 0;
        self.gate = true;
    }

    /// A block of the reed, added into `out`; returns its peak.
    pub fn block(&mut self, b: &Blow, rng: &mut Rng, out: &mut [f32]) -> f32 {
        let sr = b.sr;
        self.hp.set(900.0, sr);
        self.lp.set(5200.0, sr);
        // The spectrum for this block, from how hard it is blown now: harder
        // is a narrower pulse and a brighter reed.
        let blown = (self.env * (0.2 + 0.8 * self.vel) * b.share).min(1.0);
        let bright = (b.bright * self.dark * (0.88 + 0.12 * blown)).clamp(0.3, 0.985);
        let duty = (b.duty * (1.0 - 0.45 * blown) * (2.0 - self.dark).clamp(0.6, 1.4)).clamp(0.08, 0.5);
        self.n_h = match b.shape {
            Some(shape) => {
                // The instrument's own tone, its power kept.
                let lim = 0.45 * sr;
                let mut n = 0;
                let mut power = 0.0;
                for (k, a) in self.amps.iter_mut().enumerate() {
                    let fk = (k + 1) as f32 * self.f;
                    if fk >= lim {
                        break;
                    }
                    *a = 10f32.powf(shape(fk, k + 1, self.note, blown, self.env, self.t) / 20.0);
                    power += *a * *a;
                    n = k + 1;
                }
                let norm = 0.8 / power.sqrt().max(1e-6);
                for a in self.amps[..n].iter_mut() {
                    *a *= norm;
                }
                n
            }
            None => reed_spectrum(&mut self.amps, self.f, sr, duty, bright, b.radiate),
        };
        // The air rushes while the reed is out of the slot: this much of the cycle.
        let gap = (PI * duty).cos();
        let ping_f = self.f * 6.27;
        let ping = if ping_f < 0.45 * sr { b.ping } else { 0.0 };
        let beat = if b.beat > 0.0 { (b.beat * LN_2 / 12.0).exp() } else { 0.0 };
        // A soft note swells in, a hard one speaks at once.
        let att = (b.att * (0.5 + 0.8 * self.vel)).min(1.0);
        if self.was_gate && !self.gate {
            self.off_t = 0.0;
        }
        self.was_gate = self.gate;
        let mut peak = 0.0f32;
        for (j, o) in out.iter_mut().enumerate() {
            self.t += 1.0 / sr;
            if !self.gate {
                self.off_t += 1.0 / sr;
            }
            self.f += (self.target - self.f) * b.glide_k;
            self.env += ((if self.gate { 1.0 } else { 0.0 }) - self.env) * if self.gate { att } else { b.rel };
            let vib = b.vib.get(j).copied().unwrap_or(0.0);
            // Tongued: a push of pressure as a hard note starts.
            let accent = 1.0 + b.tongue * self.vel * self.vel * (-self.t / 0.05).exp();
            let press = self.env * (0.45 + 0.55 * self.vel) * b.share * self.amp * (1.0 + vib * b.vib_amp) * accent;
            if self.rough > 0.0 {
                self.wobble += 0.004 * (rng.bipolar() - self.wobble * 0.02);
                self.wobble = self.wobble.clamp(-1.0, 1.0);
            }
            // Free reeds sink a little when blown hard.
            let mut semis = self.bend_from * (-self.t / self.bend_t.max(0.002)).exp() + b.pitch_bend + vib * b.vib_pitch - 0.06 * press * press + self.rough * 0.15 * self.wobble;
            if !self.gate {
                semis += self.rise * (1.0 - self.env);
            }
            let f = self.f * (semis * (LN_2 / 12.0)).exp();
            self.phase = (self.phase + f / sr).fract();
            let mut y = harmonics(&self.amps[..self.n_h], self.phase);
            if beat > 0.0 {
                self.phase2 = (self.phase2 + f * beat / sr).fract();
                y = 0.62 * (y + harmonics(&self.amps[..self.n_h], self.phase2));
            }
            // The air through the slot, pulsing with the reed, and a puff as
            // the note starts.
            let open = (((TAU * self.phase).cos() - gap) / (1.0 - gap)).max(0.0);
            let puff = (-self.t / 0.03).exp();
            let nz = self.lp.lp(self.hp.hp(rng.bipolar()));
            let mut air = nz * (b.breath * self.airy * (0.1 + 0.6 * press) * open * 1.6 + b.breath.max(0.1) * 0.5 * puff) * self.env;
            // The valve shuts with a puff.
            if !self.gate && b.puff_off > 0.0 {
                air += nz * b.puff_off * b.breath.max(0.1) * 0.6 * (-self.off_t / 0.025).exp();
            }
            // The tongue's second mode, ringing a moment as it starts.
            let mut zing = 0.0;
            if ping > 0.0 && self.t < 0.25 {
                self.ping_ph = (self.ping_ph + ping_f / sr).fract();
                zing = (TAU * self.ping_ph).sin() * ping * press * (-self.t / 0.05).exp();
            }
            let s = y * press + air * 0.5 + zing;
            peak = peak.max(s.abs());
            *o += s;
        }
        self.level = (self.level * 0.85).max(peak * 3.0).min(1.0);
        peak
    }
}

/// The sum of the harmonics at `phase`, by recurrence from one sine and cosine.
#[inline]
fn harmonics(amps: &[f32], phase: f32) -> f32 {
    let (s1, c1) = (TAU * phase).sin_cos();
    let (mut sk, mut ck) = (s1, c1);
    let mut y = 0.0;
    for &a in amps {
        y += a * sk;
        let s_next = sk * c1 + ck * s1;
        ck = ck * c1 - sk * s1;
        sk = s_next;
    }
    y
}

/// A voice to use: a quiet one, else the softest letting go, else the softest.
pub fn free_reed(reeds: &[Reed]) -> usize {
    (0..reeds.len())
        .find(|&i| reeds[i].quiet())
        .or_else(|| (0..reeds.len()).filter(|&i| !reeds[i].gate).min_by(|&a, &b| reeds[a].env.total_cmp(&reeds[b].env)))
        .or_else(|| (0..reeds.len()).min_by(|&a, &b| reeds[a].env.total_cmp(&reeds[b].env)))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_narrower_pulse_is_brighter() {
        let mut a = [0.0; MAX_H];
        let mut b = [0.0; MAX_H];
        reed_spectrum(&mut a, 220.0, 48000.0, 0.45, 0.9, 0.5);
        reed_spectrum(&mut b, 220.0, 48000.0, 0.15, 0.9, 0.5);
        let hi = |x: &[f32; MAX_H]| x[4..12].iter().map(|v| v * v).sum::<f32>() / x.iter().map(|v| v * v).sum::<f32>();
        assert!(hi(&b) > 1.5 * hi(&a));
        // Nothing above the Nyquist frequency.
        let n = reed_spectrum(&mut a, 3000.0, 48000.0, 0.3, 0.95, 0.5);
        assert!(n as f32 * 3000.0 < 0.45 * 48000.0 + 3000.0);
        // Hearing the change is buzzier than hearing the pulse.
        reed_spectrum(&mut a, 220.0, 48000.0, 0.4, 0.9, 0.0);
        reed_spectrum(&mut b, 220.0, 48000.0, 0.4, 0.9, 1.0);
        assert!(hi(&b) > 2.0 * hi(&a));
    }
}
