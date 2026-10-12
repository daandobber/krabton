//! Acoustic drums, modelled. A drum is the sum of the ways its head or its
//! metal vibrates: the modes of a circular membrane (1, 1.59, 2.14, 2.30,
//! 2.65, 2.92 ... times the fundamental) for kicks, snares and toms, with
//! the pitch that a hard hit bends up for a moment (the tension of the
//! head), the beater or stick's click, and on the snare the wires, whose
//! buzz follows the head as it swings. Hats and cymbals are many
//! inharmonic partials over a wash of noise; a crash blooms, a ride builds
//! up as you keep playing it. A new hit adds to what still rings instead of
//! starting over, the way a real drum does, and no two hits are quite the
//! same (velocity changes the brightness, each hit varies a little).
//! Original code.

use super::drums::DrumVoice;
use super::{decay_coef, fast_tanh, FilterMode, Rng, Svf};
use std::f32::consts::TAU;

/// The modes of a circular membrane, relative to the fundamental.
const MEMBRANE: [f32; 8] = [1.0, 1.594, 2.136, 2.296, 2.653, 2.918, 3.156, 3.501];
const MAX_PARTS: usize = 24;

/// A membrane mode: a sine whose pitch follows the head's tension.
#[derive(Clone, Copy, Default)]
struct Mode {
    ph: f32,
    f: f32,
    amp: f32,
    coef: f32,
}

/// A metal partial: a rotating phasor (no pitch bend, so no sine needed).
#[derive(Clone, Copy, Default)]
struct Partial {
    re: f32,
    im: f32,
    c: f32,
    s: f32,
}

impl Partial {
    /// Adds `amp` of a partial at `f` Hz decaying with `coef`, keeping the
    /// phase of what already rings.
    fn strike(&mut self, f: f32, amp: f32, coef: f32, sr: f32) {
        let w = TAU * f / sr;
        self.c = coef * w.cos();
        self.s = coef * w.sin();
        let mag = (self.re * self.re + self.im * self.im).sqrt();
        if mag > 1e-6 {
            let k = (mag + amp).min(2.0) / mag;
            self.re *= k;
            self.im *= k;
        } else {
            self.re = 0.0;
            self.im = amp;
        }
    }

    #[inline]
    fn tick(&mut self) -> f32 {
        let re = self.re * self.c - self.im * self.s;
        self.im = self.re * self.s + self.im * self.c;
        self.re = re;
        self.im
    }
}

pub struct AcousticVoice {
    modes: [Mode; 8],
    n_modes: usize,
    parts: [Partial; MAX_PARTS],
    n_parts: usize,
    /// How far the head's pitch is bent up now, and how fast it settles.
    bend: f32,
    bend_coef: f32,
    /// The stick or beater: a short burst of filtered noise.
    click: f32,
    click_coef: f32,
    click_bp: Svf,
    /// A noise wash (snare wires, hat sizzle, cymbal wash) and its filters.
    wash: f32,
    wash_coef: f32,
    /// A swell for cymbals that bloom (0 = none).
    bloom: f32,
    bloom_coef: f32,
    wash_hp: Svf,
    wash_bp: Svf,
    wires: f32,
    /// The hat's wobble after an open hit.
    wobble: f32,
    wobble_ph: f32,
    gain: f32,
    sr: f32,
    rng: Rng,
    live: u32,
}

impl Default for AcousticVoice {
    fn default() -> Self {
        Self {
            modes: [Mode::default(); 8],
            n_modes: 0,
            parts: [Partial::default(); MAX_PARTS],
            n_parts: 0,
            bend: 0.0,
            bend_coef: 0.0,
            click: 0.0,
            click_coef: 0.0,
            click_bp: Svf::new(),
            wash: 0.0,
            wash_coef: 0.0,
            bloom: 0.0,
            bloom_coef: 0.0,
            wash_hp: Svf::new(),
            wash_bp: Svf::new(),
            wires: 0.0,
            wobble: 0.0,
            wobble_ph: 0.0,
            gain: 1.0,
            sr: 48000.0,
            rng: Rng::new(0xD2_7A_11),
            live: 0,
        }
    }
}

/// How a membrane drum is built.
struct Head {
    f0: f32,
    amps: [f32; 8],
    /// Ring time of each mode (to -40 dB), seconds.
    rings: [f32; 8],
    /// How far a full hit bends the pitch up, and how fast it settles.
    bend: f32,
    bend_time: f32,
    click: f32,
    click_hz: f32,
    gain: f32,
}

impl AcousticVoice {
    pub fn is_active(&self) -> bool {
        self.live > 0
    }

    fn jitter(&mut self, amount: f32) -> f32 {
        1.0 + amount * self.rng.bipolar()
    }

    fn strike_head(&mut self, h: &Head, vel: f32, decay: f32, tone: f32) {
        let sr = self.sr;
        self.n_modes = 8;
        for k in 0..8 {
            // Harder hits bring out the higher modes; each hit a little different.
            let bright = vel.powf(0.25 * k as f32) * (0.6 + 0.8 * tone).powf(k as f32 * 0.35);
            let amp = h.amps[k] * bright * self.jitter(0.12);
            let ring = h.rings[k] * decay * if k == 0 { 1.0 } else { 0.7 + 0.6 * tone };
            let m = &mut self.modes[k];
            m.f = h.f0 * MEMBRANE[k] * (1.0 + 0.004 * self.rng.bipolar());
            // The stick damps what rang a moment before, then adds its own.
            m.amp = m.amp * 0.4 + amp;
            m.coef = decay_coef(ring.max(0.005), sr);
        }
        self.bend = h.bend * vel;
        self.bend_coef = decay_coef(h.bend_time, sr);
        self.click = h.click * vel * vel.sqrt();
        self.click_coef = decay_coef(0.004 + 0.003 * (1.0 - tone), sr);
        self.click_bp.set(h.click_hz * (0.7 + 0.6 * tone), 0.35, sr);
        self.gain = h.gain;
    }

    /// Partials of metal: `n` of them from `f_lo` up by `spread`, ringing `ring` seconds.
    #[allow(clippy::too_many_arguments)]
    fn strike_metal(&mut self, n: usize, f_lo: f32, spread: f32, ring: f32, ring_hi: f32, vel: f32, tone: f32, fresh: bool) {
        let sr = self.sr;
        self.n_parts = n.min(MAX_PARTS);
        for k in 0..self.n_parts {
            let x = k as f32 / n.max(1) as f32;
            // Cymbal modes crowd together as they rise.
            let f = f_lo * (1.0 + k as f32).powf(spread) * if fresh { 1.0 + 0.006 * self.rng.bipolar() } else { 1.0 };
            let f = f.min(0.45 * sr);
            let amp = (0.5 + 0.5 * self.rng.unipolar()) / (1.0 + 0.12 * k as f32) * (0.4 + vel * (0.6 + 0.6 * tone * x));
            let r = ring + (ring_hi - ring) * x;
            self.parts[k].strike(f, amp * 0.18, decay_coef(r.max(0.01), sr), sr);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn trigger(&mut self, voice: DrumVoice, vel: f32, tr: f32, decay: f32, tone: f32, a: f32, b: f32, sr: f32) {
        self.sr = sr;
        let vel = vel.clamp(0.05, 1.0);
        let d = decay.max(0.05);
        let fresh = self.live == 0;
        self.live = (sr * 12.0) as u32;
        // Silence the parts this voice does not use.
        let reset_parts = |s: &mut Self| {
            if fresh {
                for p in s.parts.iter_mut() {
                    *p = Partial::default();
                }
                for m in s.modes.iter_mut() {
                    m.amp = 0.0;
                }
            }
        };
        reset_parts(self);
        self.n_modes = 0;
        self.n_parts = 0;
        self.bloom = 0.0;
        self.wobble = 0.0;
        match voice {
            DrumVoice::AcKick => {
                let h = Head {
                    f0: 52.0 * tr,
                    amps: [1.0, 0.32, 0.2, 0.14, 0.1, 0.07, 0.05, 0.03],
                    rings: [0.55, 0.16, 0.11, 0.09, 0.07, 0.06, 0.05, 0.04],
                    bend: 0.55,
                    bend_time: 0.03,
                    click: 0.5 + 1.2 * a,
                    click_hz: 3200.0,
                    gain: 1.1,
                };
                self.strike_head(&h, vel, d, tone);
                // The shell and the air in it.
                self.wash = 0.25 * vel;
                self.wash_coef = decay_coef(0.06 * d, sr);
                self.wash_bp.set(110.0 * tr, 0.5, sr);
                self.wires = 0.0;
            }
            DrumVoice::AcSnare | DrumVoice::AcBrush => {
                let brush = voice == DrumVoice::AcBrush;
                let h = Head {
                    f0: 188.0 * tr,
                    amps: [1.0, 0.62, 0.48, 0.38, 0.3, 0.25, 0.2, 0.16],
                    rings: [0.28, 0.2, 0.14, 0.12, 0.1, 0.09, 0.08, 0.07],
                    bend: if brush { 0.02 } else { 0.14 },
                    bend_time: 0.012,
                    click: if brush { 0.0 } else { 0.8 },
                    click_hz: 4200.0,
                    gain: if brush { 0.35 } else { 0.62 },
                };
                self.strike_head(&h, vel, d, tone);
                // The wires: their buzz follows the head.
                self.wires = (0.4 + 1.2 * a) * if brush { 0.4 } else { 1.0 };
                self.wash = vel * if brush { 0.0 } else { 1.0 };
                self.wash_coef = decay_coef(0.32 * d, sr);
                self.wash_hp.set(1800.0 * (0.8 + 0.4 * tone), 0.1, sr);
                self.wash_bp.set(5200.0 * (0.8 + 0.5 * tone), 0.25, sr);
                if brush {
                    // A swish: the brush slides across the head.
                    self.bloom = 1.0;
                    self.bloom_coef = decay_coef(0.03, sr);
                    self.wash = 0.9 * vel;
                    self.wash_coef = decay_coef(0.35 * d, sr);
                }
            }
            DrumVoice::AcStick => {
                // A cross stick: the stick knocks the rim, the head barely sounds.
                let h = Head {
                    f0: 470.0 * tr,
                    amps: [1.0, 0.5, 0.15, 0.5, 0.1, 0.25, 0.05, 0.05],
                    rings: [0.07, 0.05, 0.04, 0.05, 0.03, 0.03, 0.02, 0.02],
                    bend: 0.02,
                    bend_time: 0.005,
                    click: 1.2,
                    click_hz: 2500.0,
                    gain: 0.6,
                };
                self.strike_head(&h, vel, d, tone);
                self.wires = 0.15;
                self.wash = 0.2 * vel;
                self.wash_coef = decay_coef(0.08 * d, sr);
                self.wash_hp.set(1500.0, 0.1, sr);
                self.wash_bp.set(4000.0, 0.3, sr);
            }
            DrumVoice::AcTomHigh | DrumVoice::AcTomMid | DrumVoice::AcTomFloor => {
                let (f0, ring) = match voice {
                    DrumVoice::AcTomHigh => (172.0, 0.75),
                    DrumVoice::AcTomMid => (128.0, 0.9),
                    _ => (86.0, 1.25),
                };
                let h = Head {
                    f0: f0 * tr,
                    amps: [1.0, 0.45, 0.28, 0.2, 0.14, 0.1, 0.07, 0.05],
                    rings: [ring, ring * 0.45, ring * 0.3, ring * 0.25, 0.15, 0.12, 0.1, 0.08],
                    bend: 0.08 + 0.3 * a,
                    bend_time: 0.06,
                    click: 0.6,
                    click_hz: 3000.0,
                    gain: 0.75,
                };
                self.strike_head(&h, vel, d, tone);
                self.wires = 0.0;
                self.wash = 0.15 * vel;
                self.wash_coef = decay_coef(0.04, sr);
                self.wash_hp.set(900.0, 0.1, sr);
                self.wash_bp.set(2400.0, 0.3, sr);
            }
            DrumVoice::AcHatClosed | DrumVoice::AcHatOpen | DrumVoice::AcHatPedal => {
                let (ring, wash_t, click) = match voice {
                    DrumVoice::AcHatClosed => (0.07, 0.09, 0.9),
                    DrumVoice::AcHatOpen => (0.9, 1.1, 0.6),
                    _ => (0.035, 0.05, 0.25),
                };
                let pedal = voice == DrumVoice::AcHatPedal;
                self.strike_metal(10, 3100.0 * tr, 0.55, ring * d, ring * d * 0.6, vel, tone, fresh);
                if pedal {
                    for p in self.parts.iter_mut().take(self.n_parts) {
                        p.re *= 0.3;
                        p.im *= 0.3;
                    }
                }
                self.wash = vel * if pedal { 0.5 } else { 1.0 };
                self.wash_coef = decay_coef(wash_t * d, sr);
                self.wash_hp.set(if pedal { 1200.0 } else { 6500.0 * (0.8 + 0.4 * tone) }, 0.1, sr);
                self.wash_bp.set(if pedal { 2600.0 } else { 9500.0 * (0.85 + 0.3 * tone) }, 0.3, sr);
                self.click = click * vel;
                self.click_coef = decay_coef(0.003, sr);
                self.click_bp.set(if pedal { 1800.0 } else { 7500.0 }, 0.4, sr);
                self.wires = 0.0;
                self.gain = 0.55;
                if voice == DrumVoice::AcHatOpen {
                    self.wobble = 0.35;
                    self.wobble_ph = 0.0;
                }
            }
            DrumVoice::AcRide | DrumVoice::AcRideBell => {
                let bell = voice == DrumVoice::AcRideBell || a > 0.75;
                self.strike_metal(20, if bell { 640.0 } else { 380.0 } * tr, if bell { 0.8 } else { 1.05 }, 4.0 * d, 1.6 * d, vel, tone, fresh);
                if bell {
                    // The bell: a few strong, long partials.
                    for p in self.parts.iter_mut().take(5) {
                        p.re *= 2.5;
                        p.im *= 2.5;
                    }
                }
                self.wash = vel * if bell { 0.15 } else { 0.4 };
                self.wash_coef = decay_coef(2.6 * d, sr);
                self.wash_hp.set(4000.0, 0.1, sr);
                self.wash_bp.set(7000.0 * (0.8 + 0.4 * tone), 0.2, sr);
                self.click = 0.7 * vel;
                self.click_coef = decay_coef(0.004, sr);
                self.click_bp.set(5200.0, 0.5, sr);
                self.wires = 0.0;
                self.gain = 0.42;
            }
            DrumVoice::AcCrash | DrumVoice::AcChina => {
                let china = voice == DrumVoice::AcChina;
                let size = 0.7 + 0.6 * a;
                self.strike_metal(24, 260.0 * tr / size, if china { 1.25 } else { 1.4 }, 2.4 * d * size, 1.4 * d * size, vel, tone, fresh);
                self.wash = vel * 1.3;
                self.wash_coef = decay_coef(2.0 * d * size, sr);
                // The crash blooms: its wash swells for a moment.
                self.bloom = 1.0;
                self.bloom_coef = decay_coef(0.025, sr);
                self.wash_hp.set(if china { 2000.0 } else { 3200.0 }, 0.1, sr);
                self.wash_bp.set(if china { 4500.0 } else { 7500.0 } * (0.8 + 0.4 * tone), if china { 0.6 } else { 0.25 }, sr);
                self.click = 0.8 * vel;
                self.click_coef = decay_coef(0.004, sr);
                self.click_bp.set(4000.0, 0.4, sr);
                self.wires = 0.0;
                self.gain = 0.5;
            }
            _ => {
                self.live = 0;
            }
        }
        let _ = b;
    }

    #[inline]
    pub fn tick(&mut self, tr: f32) -> f32 {
        let _ = tr;
        if self.live == 0 {
            return 0.0;
        }
        self.live -= 1;
        let sr = self.sr;
        let mut head = 0.0;
        let bend = 1.0 + self.bend;
        for m in self.modes.iter_mut().take(self.n_modes) {
            m.ph += m.f * bend / sr;
            if m.ph >= 1.0 {
                m.ph -= 1.0;
            }
            head += (m.ph * TAU).sin() * m.amp;
            m.amp *= m.coef;
        }
        self.bend *= self.bend_coef;
        let mut metal = 0.0;
        for p in self.parts.iter_mut().take(self.n_parts) {
            metal += p.tick();
        }
        let n = self.rng.bipolar();
        let click = self.click_bp.process(n, FilterMode::Band) * self.click;
        self.click *= self.click_coef;
        // The wash: a noise band; snare wires buzz with the head's swing.
        let mut wash = 0.0;
        if self.wash > 1e-5 {
            let hp = self.wash_hp.process(self.rng.bipolar(), FilterMode::High);
            let band = self.wash_bp.process(hp, FilterMode::Band) + 0.35 * hp;
            let mut env = self.wash;
            if self.bloom > 0.0 {
                // Swelling up while the wash itself fades.
                env *= 1.0 - self.bloom;
                self.bloom *= self.bloom_coef;
                if self.bloom < 1e-4 {
                    self.bloom = 0.0;
                }
            }
            if self.wires > 0.0 {
                let swing = (head.abs() * 1.4).min(1.0);
                env *= self.wires * (0.35 + 0.65 * swing);
            }
            if self.wobble > 0.0 {
                self.wobble_ph = (self.wobble_ph + 7.0 / sr).fract();
                env *= 1.0 + self.wobble * (self.wobble_ph * TAU).sin();
                self.wobble *= 0.99995;
            }
            wash = band * env;
            self.wash *= self.wash_coef;
        }
        let y = (head + metal) * self.gain + wash * 0.5 + click * 0.6;
        if self.live % 512 == 0 {
            // Done when everything has faded.
            let modes = self.modes.iter().take(self.n_modes).fold(0.0f32, |m, x| m.max(x.amp));
            let parts = self.parts.iter().take(self.n_parts).fold(0.0f32, |m, p| m.max(p.re.abs() + p.im.abs()));
            if modes < 1e-4 && parts < 1e-4 && self.wash < 1e-4 && self.click < 1e-4 {
                self.live = 0;
            }
        }
        fast_tanh(y * 1.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn hit(voice: DrumVoice, vel: f32, secs: f32) -> Vec<f32> {
        let mut v = AcousticVoice::default();
        v.trigger(voice, vel, 1.0, 1.0, 0.5, 0.5, 0.5, SR);
        (0..(secs * SR) as usize).map(|_| v.tick(1.0)).collect()
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    /// The strongest frequency (by counting zero crossings of a low-passed copy).
    fn pitch(x: &[f32]) -> f32 {
        let mut lp = 0.0;
        let k = 1.0 - (-TAU * 300.0 / SR).exp();
        let y: Vec<f32> = x.iter().map(|v| {
            lp += (v - lp) * k;
            lp
        }).collect();
        let crossings = y.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
        crossings as f32 * SR / x.len() as f32
    }

    #[test]
    fn every_acoustic_voice_sounds_and_fades() {
        for v in DrumVoice::ACOUSTIC {
            let x = hit(v, 0.9, 6.0);
            assert!(x.iter().all(|s| s.is_finite() && s.abs() <= 1.0), "{v:?}");
            let start = rms(&x[..1200]);
            assert!(start > 0.02, "{v:?} {start}");
            let end = rms(&x[x.len() - 4800..]);
            assert!(end < start * 0.05, "{v:?} {start} {end}");
        }
    }

    #[test]
    fn the_kick_is_low_and_the_toms_go_down_the_kit() {
        let kick = pitch(&hit(DrumVoice::AcKick, 0.8, 0.3)[2400..]);
        let floor = pitch(&hit(DrumVoice::AcTomFloor, 0.8, 0.3)[2400..]);
        let high = pitch(&hit(DrumVoice::AcTomHigh, 0.8, 0.3)[2400..]);
        assert!(kick > 35.0 && kick < 80.0, "{kick}");
        assert!(floor < high, "{floor} {high}");
    }

    #[test]
    fn harder_hits_are_louder_and_brighter() {
        let soft = hit(DrumVoice::AcSnare, 0.3, 0.2);
        let hard = hit(DrumVoice::AcSnare, 1.0, 0.2);
        let hf = |x: &[f32]| rms(&x.windows(2).map(|w| w[1] - w[0]).collect::<Vec<_>>()) / rms(x).max(1e-9);
        assert!(rms(&hard) > rms(&soft));
        assert!(hf(&hard) > hf(&soft), "{} {}", hf(&hard), hf(&soft));
    }

    #[test]
    fn a_ride_hit_again_rings_on_without_a_jump() {
        let mut v = AcousticVoice::default();
        v.trigger(DrumVoice::AcRide, 0.7, 1.0, 1.0, 0.5, 0.5, 0.5, SR);
        let mut x: Vec<f32> = (0..12000).map(|_| v.tick(1.0)).collect();
        v.trigger(DrumVoice::AcRide, 0.7, 1.0, 1.0, 0.5, 0.5, 0.5, SR);
        x.extend((0..12000).map(|_| v.tick(1.0)));
        // Louder after the second hit, and still ringing.
        assert!(rms(&x[12000..16000]) > rms(&x[8000..12000]));
    }
}
