//! Machines: drum voices after classic analog drum machines, built from how
//! their circuits work (after the published analyses of the 808 by Werner,
//! Abel and Smith, and the machines' service notes). Original code, not
//! samples.
//!
//! - The 808's voices ring resonators (its bridged-T networks) with the
//!   trigger's 1 ms pulse, as loud as the accent. The bass drum's resonator
//!   sits more than an octave higher for its first 6 ms, gets a second pulse
//!   as it drops to its note, and sighs: it rings a little sharp while it is
//!   loud. A hit while the last still rings adds to it, as on the machine.
//! - Its hats, cymbal and cowbell are six square oscillators (205.3, 304.4,
//!   369.6, 522.7, 540 and 800 Hz) through band-passes at 3440 and 7100 Hz,
//!   "swing" VCAs (the envelope is a rail the signal clips against, which
//!   gives them their grit), resonant high-passes near 10.5 kHz and an
//!   output that leans to the highs.
//! - The 909's bass drum, snare and toms are swept oscillators; its hats,
//!   ride and crash were 6-bit samples of real cymbals, here a cymbal made
//!   and played back at 6 bits and a lower rate before its analog VCA.
//! - The 606's are tighter versions of the 808 circuits; the CR-78's are
//!   gentler resonators and gated noise; then the bending toms of the first
//!   electronic kits, and two-operator FM drums.

use super::drums::DrumVoice;
use super::*;

/// The 808's metal: six square oscillators (Hz).
const METAL_808: [f32; 6] = [205.3, 304.4, 369.6, 522.7, 540.0, 800.0];
/// The cymbals the 909's hats were sampled from: a brighter, denser set.
const METAL_909: [f32; 6] = [312.0, 448.0, 587.0, 727.0, 871.0, 1063.0];
/// The 909 ride's bell: its partials over its lowest.
const RIDE_PARTIALS: [f32; 6] = [1.0, 2.36, 3.62, 5.11, 6.87, 8.41];

/// A machine drum voice while it sounds.
#[derive(Clone, Copy)]
pub struct MachineVoice {
    t: u32,
    sr: f32,
    vel: f32,
    phase: [f32; 6],
    /// Envelopes and their per-sample decays (what each is depends on the voice).
    env: [f32; 4],
    coef: [f32; 4],
    /// Pitch bends (fractions above the base pitch) and their decays: a fast and a slow one.
    bend: f32,
    bend_coef: f32,
    bend2: f32,
    bend2_coef: f32,
    click: f32,
    click_coef: f32,
    filt: [Svf; 6],
    /// Resonators (the bridged-T networks): they ring on into the next hit.
    res: [Svf; 2],
    q: [f32; 2],
    lp: OnePole,
    hp: OnePole,
    /// Base frequencies (Hz).
    freq: [f32; 2],
    tone: f32,
    macro_a: f32,
    /// FM drums: peak modulation index (radians).
    fm_index: f32,
    /// How loud the resonator rings now (the 808 kick's sigh follows it).
    amp: f32,
    /// The pulse's strength (the accent).
    pulse: f32,
    /// A 909 cymbal's sample, held between the ROM's samples.
    held: f32,
    hold_ph: f32,
    /// The output's last sample (the 808's output stage leans to the highs).
    last: f32,
    active: bool,
}

impl Default for MachineVoice {
    fn default() -> Self {
        Self {
            t: 0,
            sr: 48000.0,
            vel: 1.0,
            phase: [0.0; 6],
            env: [0.0; 4],
            coef: [0.0; 4],
            bend: 0.0,
            bend_coef: 0.0,
            bend2: 0.0,
            bend2_coef: 0.0,
            click: 0.0,
            click_coef: 0.0,
            filt: [Svf::new(); 6],
            res: [Svf::new(); 2],
            q: [1.0; 2],
            lp: OnePole::default(),
            hp: OnePole::default(),
            freq: [0.0; 2],
            tone: 0.5,
            macro_a: 0.5,
            fm_index: 0.0,
            amp: 0.0,
            pulse: 1.0,
            held: 0.0,
            hold_ph: 0.0,
            last: 0.0,
            active: false,
        }
    }
}

#[inline]
fn sine(p: f32) -> f32 {
    (p * TAU).sin()
}

#[inline]
fn square(p: f32) -> f32 {
    if p < 0.5 {
        1.0
    } else {
        -1.0
    }
}

#[inline]
fn tri(p: f32) -> f32 {
    4.0 * (p - (p + 0.25).floor() + 0.25).abs() - 1.0
}

/// The correction around a step of a band-limited rectangle (polyBLEP).
#[inline]
fn blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

/// A square without aliasing: phase `p`, `dt` its frequency over the rate.
#[inline]
fn square_bl(p: f32, dt: f32) -> f32 {
    square(p) + blep(p, dt) - blep((p + 0.5).fract(), dt)
}

/// The 808's swing-type VCA: its envelope is the rail the signal swings
/// against. While the envelope is high the signal passes; as it falls the
/// signal clips against it, and under a diode's drop it shuts.
#[inline]
fn swing(x: f32, env: f32) -> f32 {
    let e = env - 0.015;
    if e <= 0.0 {
        0.0
    } else {
        e * fast_tanh(x / e)
    }
}

/// A sample at 6 bits.
#[inline]
fn six_bit(x: f32) -> f32 {
    (x.clamp(-1.0, 1.0) * 31.0).round() / 31.0
}

/// Q of a resonator at `f` Hz that falls 40 dB in `secs`.
fn ring_q(f: f32, secs: f32) -> f32 {
    std::f32::consts::PI * f * secs / 4.6
}

/// Pulse strength that rings a resonator at `f` Hz to about 1.
fn ring_gain(f: f32) -> f32 {
    1.0 / (TAU * f.max(1.0) * 0.001)
}

/// The 909's ROM rate for its cymbals (Hz).
const ROM_RATE: f32 = 32000.0;

impl MachineVoice {
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Hits `voice`. `tune` is a frequency ratio, `decay` stretches the
    /// decay (1 = as built), `tone` and `a` are the pad's Tone and extra knob.
    #[allow(clippy::too_many_arguments)]
    pub fn trigger(&mut self, voice: DrumVoice, vel: f32, tune: f32, decay: f32, tone: f32, a: f32, sr: f32) {
        use DrumVoice::*;
        // A hit while the last still rings: its resonators ring on (no machine-gun restarts).
        let ringing = self.active && self.sr == sr;
        self.t = 0;
        self.sr = sr;
        self.vel = vel.clamp(0.0, 1.0);
        self.tone = tone.clamp(0.0, 1.0);
        self.macro_a = a.clamp(0.0, 1.0);
        self.active = true;
        self.phase = [0.0, 0.13, 0.31, 0.47, 0.61, 0.83];
        self.env = [1.0; 4];
        self.coef = [0.0; 4];
        self.bend = 0.0;
        self.bend_coef = 0.0;
        self.bend2 = 0.0;
        self.bend2_coef = 0.0;
        self.click = 1.0;
        self.click_coef = decay_coef(0.004, sr);
        for f in &mut self.filt {
            f.reset();
        }
        if !ringing {
            for r in &mut self.res {
                r.reset();
            }
            self.amp = 0.0;
            self.last = 0.0;
        }
        self.lp.z = 0.0;
        self.hp.z = 0.0;
        self.held = 0.0;
        self.hold_ph = 0.0;
        // The trigger: 4 to 14 V as the accent is (on a scale where 14 V is 1).
        self.pulse = (4.0 + 10.0 * self.vel) / 14.0;
        let d = decay.max(0.05);
        let dc = |s: f32| decay_coef(s, sr);
        match voice {
            M808Kick | M606Kick => {
                let (f0, len) = if voice == M808Kick { (49.5, 0.7) } else { (63.0, 0.3) };
                self.freq = [f0 * tune, 0.0];
                self.q = [ring_q(f0 * tune, len * d), 0.0];
                self.coef = [dc(len * d), 0.0, 0.0, 0.0];
                self.lp.set(300.0 + 3500.0 * self.tone * self.tone, sr);
                // The pulse shaper passes the edge and swings back after it.
                self.hp.set(160.0, sr);
            }
            M808Snare => {
                // The low network and the high one (335 Hz in the revised circuit).
                self.freq = [180.0 * tune, 335.0 * tune];
                self.q = [ring_q(self.freq[0], 0.16 * d.sqrt()), ring_q(self.freq[1], 0.1 * d.sqrt())];
                self.coef = [dc(0.16 * d.sqrt()), 0.0, dc(0.28 * d), 0.0];
                self.filt[0].set_q(1800.0, 0.7, sr);
                self.filt[1].set(9000.0, 0.0, sr);
            }
            M606Snare => {
                self.freq = [235.0 * tune, 410.0 * tune];
                self.q = [ring_q(self.freq[0], 0.08 * d.sqrt()), ring_q(self.freq[1], 0.05 * d.sqrt())];
                self.coef = [dc(0.08 * d.sqrt()), 0.0, dc(0.17 * d), 0.0];
                self.filt[0].set_q(2200.0, 0.7, sr);
                self.filt[1].set(11000.0, 0.0, sr);
            }
            M808Tom | M808Conga | M606LowTom | M606HighTom => {
                let (base, len, bend) = match voice {
                    M808Tom => (120.0, 0.45, 0.18),
                    M808Conga => (250.0, 0.2, 0.1),
                    M606LowTom => (142.0, 0.3, 0.12),
                    _ => (212.0, 0.3, 0.12),
                };
                self.freq = [base * tune, 0.0];
                self.q = [ring_q(base * tune, len * d), 0.0];
                self.bend = bend;
                self.bend_coef = dc(0.12);
                self.coef = [dc(len * d), 0.0, dc(0.09), 0.0];
                self.filt[0].set(900.0 * tune, 0.0, sr);
            }
            M808Rim => {
                self.freq = [455.0 * tune, 1667.0 * tune];
                self.q = [ring_q(self.freq[0], 0.03 * d), ring_q(self.freq[1], 0.02 * d)];
                self.coef = [dc(0.03 * d), 0.0, 0.0, 0.0];
                self.filt[0].set_q(900.0, 0.7, sr);
            }
            M808Claves => {
                self.freq = [2500.0 * tune, 0.0];
                self.q = [ring_q(self.freq[0], 0.06 * d), 0.0];
                self.coef = [dc(0.06 * d), 0.0, 0.0, 0.0];
            }
            M808Clap | M909Clap => {
                let (f, q) = if voice == M808Clap { (1000.0, 2.5) } else { (1150.0, 1.6) };
                self.filt[0].set_q(f * tune, q, sr);
                self.filt[1].set_q(4000.0, 0.7, sr);
                // The reverb's own band, a little lower and wider.
                self.filt[2].set_q(f * 0.85 * tune, 1.2, sr);
                let tail = if voice == M808Clap { 0.35 } else { 0.5 };
                self.coef = [dc(tail * d), 0.0, 0.0, 0.0];
            }
            M808Maracas => {
                self.filt[0].set_q(5000.0 * tune, 0.7, sr);
                self.coef = [dc(0.07 * d), 0.0, 0.0, 0.0];
            }
            M808Cowbell => {
                self.freq = [540.0 * tune, 800.0 * tune];
                self.filt[0].set_q(2640.0 * tune, 1.0, sr);
                self.filt[1].set_q(400.0, 0.7, sr);
                // A quick drop, then the long ring.
                self.coef = [dc(0.04), dc(0.6 * d), 0.0, 0.0];
            }
            M808Cymbal | M606Cymbal => {
                let (lo, hi, k) = if voice == M808Cymbal { (3440.0, 7100.0, 1.0) } else { (4200.0, 8800.0, 0.65) };
                self.filt[0].set_q(lo * tune, 1.6, sr);
                self.filt[1].set_q(hi * tune, 1.6, sr);
                self.filt[2].set_q(3000.0, 0.7, sr);
                self.filt[3].set_q(5000.0, 0.7, sr);
                self.filt[4].set_q(10500.0, 1.4, sr);
                // Three bands: the low one long, a short middle, the high one longest.
                self.coef = [dc(1.1 * k * d), dc(0.25 * k), dc(2.0 * k * d), 0.0];
                self.hp.set(12000.0, sr);
            }
            M808ClosedHat | M808OpenHat | M606ClosedHat | M606OpenHat => {
                let open = matches!(voice, M808OpenHat | M606OpenHat);
                let m808 = matches!(voice, M808ClosedHat | M808OpenHat);
                let len = match (open, m808) {
                    (true, true) => 0.6,
                    (true, false) => 0.4,
                    (false, true) => 0.07,
                    (false, false) => 0.05,
                };
                let f = if m808 { 7100.0 } else { 8200.0 };
                self.filt[0].set_q(f * tune.sqrt(), 1.6, sr);
                self.filt[1].set_q(if m808 { 7500.0 } else { 9000.0 }, 1.2, sr);
                self.coef = [dc(len * d), 0.0, 0.0, 0.0];
                self.hp.set(12000.0, sr);
            }
            M909Kick => {
                self.freq = [52.0 * tune, 0.0];
                // A fast drop in the first milliseconds, then the long slide to the note.
                self.bend = 2.0;
                self.bend_coef = decay_coef(0.06, sr);
                self.bend2 = 0.45;
                self.bend2_coef = decay_coef(0.3, sr);
                self.coef = [dc(0.6 * d), 0.0, dc(0.012), 0.0];
                self.filt[0].set_q(3500.0, 0.7, sr);
                self.filt[1].set_q(300.0, 0.7, sr);
                self.hp.set(25.0, sr);
            }
            M909Snare => {
                self.freq = [180.0 * tune, 330.0 * tune];
                self.bend = 0.35;
                self.bend_coef = dc(0.025);
                self.coef = [dc(0.17 * d.sqrt()), dc(0.11 * d.sqrt()), dc(0.3 * d), dc(0.18 * d)];
                self.filt[0].set(3000.0 + 9000.0 * self.tone, 0.1, sr);
                self.filt[1].set_q(2000.0, 0.7, sr);
            }
            M909Tom => {
                self.freq = [110.0 * tune, 0.0];
                self.bend = 0.5;
                self.bend_coef = dc(0.2);
                self.coef = [dc(0.6 * d), 0.0, dc(0.07), 0.0];
                self.filt[0].set(2000.0, 0.0, sr);
            }
            M909Rim => {
                self.freq = [500.0 * tune, 1700.0 * tune];
                self.q = [ring_q(self.freq[0], 0.025 * d), ring_q(self.freq[1], 0.018 * d)];
                self.coef = [dc(0.025 * d), 0.0, dc(0.01), 0.0];
                self.filt[0].set_q(4000.0, 0.7, sr);
            }
            M909ClosedHat | M909OpenHat | M909Ride | M909Crash => {
                // env[0]: the analog VCA; env[1]: the sample's own fade (as recorded).
                let (vca, rec, hpf, lpf) = match voice {
                    M909ClosedHat => (0.06 * d, 0.9, 6500.0, 12000.0),
                    M909OpenHat => (0.55 * d, 0.9, 6500.0, 12000.0),
                    M909Ride => (3.0 * d, 2.8, 900.0, 13000.0),
                    _ => (2.2 * d, 2.0, 1800.0, 13000.0),
                };
                self.freq = [520.0 * tune, 0.0];
                self.filt[0].set_q(hpf * tune.sqrt(), 0.8, sr);
                self.filt[1].set_q((lpf - 5000.0 + 6000.0 * self.tone).min(sr * 0.45), 0.7, sr);
                self.filt[2].set_q(3000.0 * tune, 0.6, sr);
                self.filt[3].set_q(8000.0 * tune.sqrt(), 1.0, sr);
                self.coef = [dc(vca), dc(rec), 0.0, 0.0];
            }
            Cr78Kick => {
                self.freq = [66.0 * tune, 0.0];
                self.bend = 0.08;
                self.bend_coef = dc(0.03);
                self.coef = [dc(0.32 * d), 0.0, 0.0, 0.0];
                self.lp.set(800.0 + 1200.0 * self.tone, sr);
            }
            Cr78Snare => {
                self.freq = [300.0 * tune, 0.0];
                self.coef = [dc(0.09 * d.sqrt()), 0.0, dc(0.16 * d), 0.0];
                self.filt[0].set_q(3200.0, 1.0, sr);
            }
            Cr78Hat => {
                self.filt[0].set_q(7000.0 * tune, 0.7, sr);
                self.filt[1].set_q(10500.0 * tune.sqrt(), 2.0, sr);
                self.coef = [dc(0.06 * d), 0.0, 0.0, 0.0];
            }
            Cr78Cymbal => {
                self.freq = [1150.0 * tune, 1590.0 * tune];
                self.filt[0].set_q(5200.0 * tune, 1.4, sr);
                self.filt[1].set_q(4000.0, 0.7, sr);
                self.coef = [dc(1.1 * d), 0.0, 0.0, 0.0];
            }
            Cr78Bongo => {
                self.freq = [420.0 * tune, 0.0];
                self.bend = 0.05;
                self.bend_coef = dc(0.02);
                self.coef = [dc(0.12 * d), 0.0, 0.0, 0.0];
            }
            Cr78Guiro => {
                self.filt[0].set_q(2800.0 * tune, 3.0, sr);
                self.coef = [dc(0.22 * d), 0.0, 0.0, 0.0];
            }
            Cr78Tambourine => {
                self.filt[0].set_q(6500.0 * tune, 0.7, sr);
                self.filt[1].set_q(9000.0 * tune.sqrt(), 1.5, sr);
                self.coef = [dc(0.25 * d), 0.0, 0.0, 0.0];
            }
            Cr78Metal => {
                self.freq = [1100.0 * tune, 1530.0 * tune];
                self.filt[0].set_q(3000.0 * tune, 2.0, sr);
                self.coef = [dc(0.16 * d), 0.0, 0.0, 0.0];
            }
            ElectroTom => {
                self.freq = [110.0 * tune, 0.0];
                // Bend: how far above the note the drum starts (up to an octave and a half).
                self.bend = 2f32.powf(1.5 * self.macro_a) - 1.0;
                self.bend_coef = dc(0.35 * d);
                self.coef = [dc(0.7 * d), 0.0, 0.0, 0.0];
                self.filt[0].set_q(3000.0, 0.7, sr);
            }
            FmKick | FmSnare | FmTom | FmHat | FmBell | FmZap | FmPerc | FmNoise => {
                // (carrier Hz, modulator ratio, index, body s, index s, pitch bend, bend s)
                let (fc, ratio, index, body, idx_len, bend, bend_len) = match voice {
                    FmKick => (48.0, 1.0, 3.0, 0.45, 0.03, 3.0, 0.03),
                    FmSnare => (185.0, 1.47, 2.5, 0.12, 0.08, 0.4, 0.02),
                    FmTom => (105.0, 1.0, 1.6, 0.4, 0.1, 1.0, 0.1),
                    FmHat => (1700.0, 2.83, 7.0, 0.06, 0.2, 0.0, 0.01),
                    FmBell => (560.0, 1.4, 3.0, 0.9, 0.35, 0.0, 0.01),
                    FmZap => (90.0, 0.5, 1.2, 0.25, 0.2, 24.0, 0.025),
                    FmPerc => (330.0, 3.5, 2.5, 0.16, 0.06, 0.05, 0.01),
                    _ => (3100.0, 7.13, 12.0, 0.18, 0.3, 0.0, 0.01),
                };
                self.freq = [fc * tune, ratio];
                // The pad's FM knob sets the index (0.5 = as designed).
                self.fm_index = index * 4f32.powf(self.macro_a - 0.5);
                self.bend = bend;
                self.bend_coef = dc(bend_len);
                self.coef = [dc(body * d), dc(idx_len * d.sqrt()), dc(0.15 * d), 0.0];
                self.filt[0].set_q(
                    match voice {
                        FmHat | FmNoise => 6000.0,
                        _ => 2500.0,
                    },
                    0.7,
                    sr,
                );
            }
            _ => {
                self.active = false;
            }
        }
    }

    #[inline]
    fn advance(&mut self, k: usize, f: f32) -> f32 {
        self.phase[k] = wrap_phase(self.phase[k] + f / self.sr);
        self.phase[k]
    }

    /// Six band-limited squares at `freqs` scaled by `tune`, summed (-1..1).
    #[inline]
    fn metal(&mut self, freqs: &[f32; 6], tune: f32) -> f32 {
        let mut s = 0.0;
        for (k, f) in freqs.iter().enumerate() {
            let hz = f * tune;
            let p = self.advance(k, hz);
            s += square_bl(p, hz / self.sr);
        }
        s / 6.0
    }

    /// Resonator `k` at `f` Hz with Q `q`, rung by `x`.
    #[inline]
    fn ring(&mut self, k: usize, x: f32, f: f32, q: f32) -> f32 {
        self.res[k].set_q(f, q, self.sr);
        self.res[k].tick(x).band
    }

    /// The trigger's 1 ms pulse at `ts` seconds (`at` later for a second one).
    #[inline]
    fn pulse_at(&self, ts: f32, at: f32) -> f32 {
        if ts >= at && ts < at + 0.001 {
            self.pulse
        } else {
            0.0
        }
    }

    /// The 808 (or 606) bass drum: its resonator rung by the shaped pulse,
    /// sharp during the attack, with a second pulse as it drops to its note.
    fn bridged_t_kick(&mut self, ts: f32, m808: bool) -> f32 {
        let (shift, attack, sigh) = if m808 { (2.3, 0.006, 0.17) } else { (1.7, 0.003, 0.05) };
        let f0 = self.freq[0];
        // During the attack: more than an octave up, and a higher Q; then down over a millisecond.
        let up = if ts < attack { 1.0 } else { (-(ts - attack) / 0.0007).exp() };
        // The sigh: a little sharp while it rings loud.
        let f = f0 * (1.0 + (shift - 1.0) * up) * (1.0 + sigh * self.amp.min(1.2));
        let q = self.q[0] * (1.0 + 0.6 * up);
        let raw = self.pulse_at(ts, 0.0) + 0.8 * self.pulse_at(ts, attack - 0.001);
        let shaped = self.hp.hp(raw);
        let body = self.ring(0, shaped * ring_gain(f0) * 0.95, f, q);
        self.amp = body.abs().max(self.amp * decay_coef(0.03, self.sr));
        // The output stage: the tone knob lets the pulse through with the body.
        let click = self.lp.lp(shaped);
        let k = if m808 { 0.4 + 0.8 * self.tone } else { 0.5 + 0.7 * self.tone };
        fast_tanh((body + click * k * (0.5 + 0.5 * self.vel)) * 1.2)
    }

    /// The 808's (or 606's) cymbal: three bands of the metal, each with its
    /// swing VCA and high-pass; Tone sets the highest.
    fn cymbal_808(&mut self, tune: f32, m808: bool) -> f32 {
        let m = if m808 { self.metal(&METAL_808, tune) } else { self.metal(&METAL_808, tune * 1.08) };
        let b1 = self.filt[0].process(m, FilterMode::Band) * 3.5;
        let b2 = self.filt[1].process(m, FilterMode::Band) * 4.5;
        let lo = self.filt[2].process(swing(b1, self.env[0] * self.pulse), FilterMode::High);
        let mid = self.filt[3].process(swing(b2, self.env[1] * self.pulse), FilterMode::High);
        let hi = self.filt[4].process(swing(b2, self.env[2] * self.pulse), FilterMode::High);
        let y = lo * 0.8 + mid * 0.5 + hi * (0.15 + 0.85 * self.tone);
        self.lean(y)
    }

    /// The 808's output stage leans to the highs (a differentiator in the
    /// audio band), gently: `y` and its change.
    #[inline]
    fn lean(&mut self, y: f32) -> f32 {
        let d = y - self.last;
        self.last = y;
        self.hp.hp(y) * 0.6 + d * 0.9
    }

    /// A 909 cymbal (hats, ride, crash): a cymbal as recorded into its ROM,
    /// played back at 6 bits and a lower rate, through the analog VCA.
    fn rom_cymbal(&mut self, voice: DrumVoice, tune: f32, rng: &mut Rng) -> f32 {
        let n = rng.bipolar();
        let rec = self.env[1];
        let src = match voice {
            DrumVoice::M909Ride => {
                // The bell's partials over a wash of the metal.
                let mut bell = 0.0;
                for (k, r) in RIDE_PARTIALS.iter().enumerate() {
                    bell += sine(self.advance(k, self.freq[0] * r)) / (1.0 + k as f32 * 0.6);
                }
                let wash = self.filt[3].process(n, FilterMode::Band);
                bell * 0.35 * (0.3 + 0.7 * rec) + wash * 0.9
            }
            DrumVoice::M909Crash => {
                // Noise and metal; the crash swells in over its first milliseconds.
                let swell = (self.t as f32 / self.sr / 0.012).min(1.0);
                let m = self.metal(&METAL_909, tune * 0.8);
                let body = self.filt[2].process(m, FilterMode::Band);
                (n * 0.8 + body * 1.2) * swell
            }
            _ => self.metal(&METAL_909, tune) * 0.55 + n * 0.6,
        };
        let x = self.filt[0].process(src, FilterMode::High) * rec * 1.6;
        // The ROM: 6 bits, at its own rate (held between its samples).
        self.hold_ph += ROM_RATE / self.sr;
        if self.hold_ph >= 1.0 {
            self.hold_ph -= 1.0;
            self.held = six_bit(x);
        }
        let y = self.held * self.env[0];
        self.filt[1].process(y, FilterMode::Low)
    }

    /// A 909 clap or an 808 clap: quick bursts of noise (each a sawtooth
    /// envelope: sharp, then falling), then the reverb's tail.
    fn clap(&mut self, ts: f32, m808: bool, rng: &mut Rng) -> f32 {
        let (gap, bursts) = if m808 { (0.0105, 3.0) } else { (0.0085, 4.0) };
        let burst = if ts < bursts * gap { 1.0 - (ts % gap) / gap } else { 0.0 };
        let tail = if ts >= (bursts - 1.0) * gap { self.env[0] * 0.75 } else { 0.0 };
        let n = rng.bipolar();
        let band = self.filt[0].process(n, FilterMode::Band);
        let verb = self.filt[2].process(n, FilterMode::Band);
        let bright = self.filt[1].process(n, FilterMode::High) * (0.1 + 0.4 * self.tone);
        (band * 2.4 + bright) * burst * burst + (verb * 2.2 + bright * 0.5) * tail
    }

    #[inline]
    pub fn tick(&mut self, voice: DrumVoice, tune: f32, rng: &mut Rng) -> f32 {
        use DrumVoice::*;
        let sr = self.sr;
        let ts = self.t as f32 / sr;
        let bendf = 1.0 + self.bend;
        let (tone, a) = (self.tone, self.macro_a);
        let out = match voice {
            M808Kick => self.bridged_t_kick(ts, true),
            M606Kick => self.bridged_t_kick(ts, false),
            M808Snare | M606Snare => {
                let p = self.pulse_at(ts, 0.0);
                let (f1, f2) = (self.freq[0], self.freq[1]);
                let (q1, q2) = (self.q[0], self.q[1]);
                let b1 = self.ring(0, p * ring_gain(f1) * 0.9, f1, q1);
                let b2 = self.ring(1, p * ring_gain(f2) * 0.5, f2, q2);
                let n = self.filt[0].process(rng.bipolar(), FilterMode::High);
                let n = self.filt[1].process(n, FilterMode::Low);
                let snappy = 0.25 + 1.1 * a;
                if voice == M808Snare {
                    b1 * (0.75 - 0.35 * tone) + b2 * (0.3 + 0.6 * tone) + n * self.env[2] * snappy
                } else {
                    b1 * (0.7 - 0.3 * tone) + b2 * 0.3 + n * self.env[2] * (0.3 + 1.1 * a)
                }
            }
            M808Tom | M808Conga | M606LowTom | M606HighTom => {
                let p = self.pulse_at(ts, 0.0);
                let f = self.freq[0] * bendf;
                let q = self.q[0];
                let body = self.ring(0, p * ring_gain(self.freq[0]) * 0.9, f, q);
                let noise = if matches!(voice, M808Tom | M606LowTom | M606HighTom) {
                    self.filt[0].process(rng.bipolar(), FilterMode::Low) * self.env[2] * (0.1 + 0.3 * tone)
                } else {
                    0.0
                };
                body + noise
            }
            M808Rim => {
                let p = self.pulse_at(ts, 0.0);
                let (f1, f2, q1, q2) = (self.freq[0], self.freq[1], self.q[0], self.q[1]);
                let x = self.ring(0, p * ring_gain(f1), f1, q1) + self.ring(1, p * ring_gain(f2) * 0.8, f2, q2);
                let y = self.filt[0].process(x, FilterMode::High);
                fast_tanh(y * (2.0 + 3.0 * tone)) * 0.8
            }
            M808Claves => {
                let p = self.pulse_at(ts, 0.0);
                let (f, q) = (self.freq[0], self.q[0]);
                self.ring(0, p * ring_gain(f), f, q) * (0.9 + 0.2 * tone)
            }
            M808Clap => self.clap(ts, true, rng),
            M909Clap => self.clap(ts, false, rng),
            M808Maracas => {
                let attack = (ts / 0.004).min(1.0);
                self.filt[0].process(rng.bipolar(), FilterMode::High) * self.env[0] * attack * 1.2
            }
            M808Cowbell => {
                let (f1, f2) = (self.freq[0], self.freq[1]);
                let x = (square_bl(self.advance(0, f1), f1 / sr) + square_bl(self.advance(1, f2), f2 / sr)) * 0.5;
                let y = self.filt[0].process(x, FilterMode::Band) * 2.2;
                let env = (self.env[0] * 0.6 + self.env[1] * (0.35 + 0.3 * tone)) * self.pulse;
                self.filt[1].process(swing(y, env), FilterMode::High) * 1.4
            }
            M808Cymbal => self.cymbal_808(tune, true),
            M606Cymbal => self.cymbal_808(tune, false),
            M808ClosedHat | M808OpenHat | M606ClosedHat | M606OpenHat => {
                let m808 = matches!(voice, M808ClosedHat | M808OpenHat);
                let m = if m808 { self.metal(&METAL_808, tune) } else { self.metal(&METAL_808, tune * 1.08) + rng.bipolar() * 0.12 };
                let b = self.filt[0].process(m, FilterMode::Band) * 4.5;
                let v = swing(b, self.env[0] * self.pulse);
                let y = self.filt[1].process(v, FilterMode::High);
                self.lean(y) * (1.0 + 0.6 * tone)
            }
            M909ClosedHat | M909OpenHat | M909Ride | M909Crash => self.rom_cymbal(voice, tune, rng),
            M909Kick => {
                let p = self.advance(0, self.freq[0] * (1.0 + self.bend + self.bend2));
                // A triangle bent towards a sine (more of its corners with Tone), driven a little.
                let t = tri(p);
                let body = (t * (1.0 + 0.5 * tone) * std::f32::consts::FRAC_PI_2).sin() * self.env[0];
                // The attack: a short burst of noise and the trigger's pulse.
                let n = self.filt[0].process(rng.bipolar(), FilterMode::Low);
                let n = self.filt[1].process(n, FilterMode::High) * self.env[2];
                let pulse = if ts < 0.0008 { 1.0 } else { 0.0 };
                let attack = (n * 0.9 + pulse * 0.8) * (0.15 + 0.85 * a) * (0.5 + 0.5 * self.vel);
                self.hp.hp(fast_tanh((body + attack) * 1.4))
            }
            M909Snare => {
                // Two oscillators bent towards sines, and the noise: through the tone's low-pass, and snappy through a high-pass.
                let s1 = (tri(self.advance(0, self.freq[0] * bendf)) * std::f32::consts::FRAC_PI_2).sin() * self.env[0];
                let s2 = (tri(self.advance(1, self.freq[1] * bendf)) * std::f32::consts::FRAC_PI_2).sin() * self.env[1];
                let n = self.filt[0].process(rng.bipolar(), FilterMode::Low);
                let body_noise = n * self.env[3] * 0.4;
                let snappy = self.filt[1].process(n, FilterMode::High) * self.env[2] * (0.2 + 1.2 * a);
                (s1 * 0.65 + s2 * 0.4) * 0.9 + body_noise + snappy
            }
            M909Tom => {
                let body = tri(self.advance(0, self.freq[0] * bendf));
                let body = (body * std::f32::consts::FRAC_PI_2).sin() * self.env[0];
                let noise = self.filt[0].process(rng.bipolar(), FilterMode::Low) * self.env[2] * (0.1 + 0.3 * tone);
                body + noise
            }
            M909Rim => {
                let p = self.pulse_at(ts, 0.0);
                let (f1, f2, q1, q2) = (self.freq[0], self.freq[1], self.q[0], self.q[1]);
                let x = self.ring(0, p * ring_gain(f1), f1, q1) + self.ring(1, p * ring_gain(f2) * 0.8, f2, q2);
                let click = self.filt[0].process(rng.bipolar(), FilterMode::High) * self.env[2];
                fast_tanh((x + click * (0.5 + tone)) * 2.5) * 0.8
            }
            Cr78Kick => {
                let body = sine(self.advance(0, self.freq[0] * bendf)) * self.env[0];
                let pulse = self.lp.lp(if ts < 0.002 { 1.0 } else { 0.0 });
                (body + pulse * 0.3 * (0.5 + 0.5 * self.vel)) * 0.95
            }
            Cr78Snare => {
                let body = sine(self.advance(0, self.freq[0])) * self.env[0];
                let n = self.filt[0].process(rng.bipolar(), FilterMode::Band) * self.env[2];
                body * (0.8 - 0.4 * tone) + n * (0.4 + 1.2 * a)
            }
            Cr78Hat => {
                let y = self.filt[0].process(rng.bipolar(), FilterMode::High);
                let y = self.filt[1].process(y, FilterMode::Band);
                y * self.env[0] * (1.6 + tone)
            }
            Cr78Cymbal => {
                let ring = square(self.advance(0, self.freq[0])) * square(self.advance(1, self.freq[1]));
                let x = ring * (0.5 - 0.3 * tone) + rng.bipolar() * (0.5 + 0.3 * tone);
                let y = self.filt[0].process(x, FilterMode::Band);
                self.filt[1].process(y, FilterMode::High) * self.env[0] * 2.0
            }
            Cr78Bongo => sine(self.advance(0, self.freq[0] * bendf)) * self.env[0],
            Cr78Guiro => {
                // The scrape: noise gated by a fast square.
                let rate = 32.0 + 20.0 * tone;
                let gate = if (ts * rate).fract() < 0.55 { 1.0 } else { 0.15 };
                self.filt[0].process(rng.bipolar(), FilterMode::Band) * gate * self.env[0] * 3.0
            }
            Cr78Tambourine => {
                let gap = 0.011;
                let burst = if ts < 3.0 * gap { (-(ts % gap) / 0.004).exp().max(self.env[0]) } else { self.env[0] };
                let y = self.filt[0].process(rng.bipolar(), FilterMode::High);
                self.filt[1].process(y, FilterMode::Band) * burst * (1.6 + tone)
            }
            Cr78Metal => {
                let ring = square(self.advance(0, self.freq[0])) * square(self.advance(1, self.freq[1]));
                self.filt[0].process(ring, FilterMode::Band) * self.env[0] * (1.4 + tone)
            }
            FmKick | FmSnare | FmTom | FmHat | FmBell | FmZap | FmPerc | FmNoise => {
                let fc = self.freq[0] * bendf;
                let pm = self.advance(1, fc * self.freq[1]);
                let pc = self.advance(0, fc);
                let idx = self.fm_index * self.env[1];
                let y = sine(wrap_phase(pc + idx * sine(pm) / std::f32::consts::TAU)) * self.env[0];
                match voice {
                    FmSnare => y * (0.8 - 0.3 * tone) + self.filt[0].process(rng.bipolar(), FilterMode::High) * self.env[2] * (0.3 + 0.8 * tone),
                    FmHat | FmNoise => self.filt[0].process(y, FilterMode::High) * (1.5 + tone),
                    FmKick => fast_tanh(y * (1.2 + tone)),
                    _ => y,
                }
            }
            ElectroTom => {
                let body = sine(self.advance(0, self.freq[0] * bendf)) * self.env[0];
                let click = self.filt[0].process(rng.bipolar(), FilterMode::High) * self.click * (0.2 + 0.8 * tone);
                body + click * 0.6
            }
            _ => 0.0,
        };
        for k in 0..4 {
            self.env[k] *= self.coef[k];
        }
        self.bend *= self.bend_coef;
        self.bend2 *= self.bend2_coef;
        self.click *= self.click_coef;
        self.t = self.t.saturating_add(1);
        // A voice is over once all its envelopes are (and its resonators have rung out).
        if self.env.iter().all(|e| *e < 1e-4) && self.amp < 1e-4 && ts > 0.05 {
            self.active = false;
        }
        out * machine_trim(voice)
    }
}

/// Output trim so the machine voices play at similar levels.
fn machine_trim(voice: DrumVoice) -> f32 {
    use DrumVoice::*;
    match voice {
        M909Snare => 0.55,
        Cr78Guiro => 0.4,
        Cr78Tambourine => 0.55,
        Cr78Metal => 0.35,
        Cr78Hat => 0.5,
        M808Clap => 0.65,
        M909Clap => 0.55,
        M808Cowbell => 0.55,
        M808Claves => 5.0,
        M808Rim => 1.3,
        M808ClosedHat => 0.8,
        M808OpenHat => 0.75,
        M909ClosedHat => 1.1,
        M909Ride | M909Crash => 0.6,
        M606ClosedHat => 0.7,
        M606OpenHat => 0.55,
        M606Cymbal => 0.8,
        M808Maracas => 0.6,
        FmKick => 1.4,
        M606Kick => 0.75,
        M606Snare => 0.75,
        FmHat | FmNoise => 0.5,
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn run_with(voice: DrumVoice, vel: f32, decay: f32, secs: f32) -> Vec<f32> {
        let mut v = MachineVoice::default();
        let mut rng = Rng::new(3);
        v.trigger(voice, vel, 1.0, decay, 0.5, 0.5, SR);
        (0..(secs * SR) as usize).map(|_| v.tick(voice, 1.0, &mut rng)).collect()
    }

    fn run(voice: DrumVoice, secs: f32) -> Vec<f32> {
        run_with(voice, 1.0, 1.0, secs)
    }

    fn at(x: &[f32], a: f32, b: f32) -> &[f32] {
        &x[(a * SR) as usize..((b * SR) as usize).min(x.len())]
    }

    /// Frequency from upward zero crossings between `a` and `b` seconds:
    /// the crossings found between samples, the periods between the first
    /// and the last averaged.
    fn pitch(x: &[f32], a: f32, b: f32) -> f32 {
        let y = at(x, a, b);
        let ups: Vec<f32> = y.windows(2).enumerate().filter(|(_, w)| w[0] <= 0.0 && w[1] > 0.0).map(|(i, w)| i as f32 + w[0] / (w[0] - w[1])).collect();
        if ups.len() < 2 {
            return ups.len() as f32 / (b - a);
        }
        (ups.len() - 1) as f32 * SR / (ups[ups.len() - 1] - ups[0])
    }

    fn peak(x: &[f32]) -> f32 {
        x.iter().fold(0.0f32, |m, v| m.max(v.abs()))
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    /// How much of `x` lies above `hz` (0..1): the energy a first difference
    /// passes, against the whole, as a crude brightness.
    fn bright(x: &[f32]) -> f32 {
        let d: f32 = x.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum();
        let e: f32 = x.iter().map(|v| v * v).sum();
        d / (4.0 * e.max(1e-12))
    }

    /// Magnitude at `hz` (Goertzel), over `x`.
    fn magnitude(x: &[f32], hz: f32) -> f32 {
        let w = TAU * hz / SR;
        let (mut s1, mut s2) = (0.0f32, 0.0f32);
        for &v in x {
            let s0 = v + 2.0 * w.cos() * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        (s1 * s1 + s2 * s2 - 2.0 * w.cos() * s1 * s2).sqrt() / x.len() as f32
    }

    #[test]
    fn the_808_kick_sits_at_its_note_and_sighs_down_to_it() {
        let k = run(DrumVoice::M808Kick, 0.8);
        let early = pitch(&k, 0.02, 0.12);
        let late = pitch(&k, 0.35, 0.75);
        assert!((46.0..54.0).contains(&late), "settles at {late} Hz");
        assert!(early > late * 1.04 && early < late * 1.25, "sighs from {early} Hz to {late} Hz");
        // The attack rings more than an octave up for a few milliseconds.
        let first = pitch(&k, 0.0005, 0.0055);
        assert!(first > 90.0, "attack at {first} Hz");
        // A soft hit sighs less (it rings less loud).
        let soft = run_with(DrumVoice::M808Kick, 0.0, 1.0, 0.8);
        let soft_early = pitch(&soft, 0.02, 0.12);
        assert!(soft_early < early, "soft {soft_early} Hz, hard {early} Hz");
        assert!(peak(&soft) < peak(&k) * 0.8);
    }

    #[test]
    fn a_kick_hit_again_rings_on() {
        let mut v = MachineVoice::default();
        let mut rng = Rng::new(1);
        v.trigger(DrumVoice::M808Kick, 1.0, 1.0, 1.0, 0.5, 0.5, SR);
        let first: Vec<f32> = (0..(0.1 * SR) as usize).map(|_| v.tick(DrumVoice::M808Kick, 1.0, &mut rng)).collect();
        let before = rms(&first[first.len() - 960..]);
        v.trigger(DrumVoice::M808Kick, 1.0, 1.0, 1.0, 0.5, 0.5, SR);
        let next: Vec<f32> = (0..96).map(|_| v.tick(DrumVoice::M808Kick, 1.0, &mut rng)).collect();
        assert!(rms(&next) > before * 0.3, "no silence when it is hit again: {} after {before}", rms(&next));
    }

    #[test]
    fn the_909_kick_starts_high_and_falls() {
        let k = run(DrumVoice::M909Kick, 0.5);
        let start = pitch(&k, 0.002, 0.03);
        let end = pitch(&k, 0.25, 0.45);
        assert!(start > end * 1.8, "{start} Hz falls to {end} Hz");
        assert!((45.0..60.0).contains(&end), "settles at {end} Hz");
    }

    #[test]
    fn cr78_kick_sits_at_its_pitch() {
        let k = run(DrumVoice::Cr78Kick, 0.3);
        let f = pitch(&k, 0.05, 0.25);
        assert!((60.0..72.0).contains(&f), "CR-78 kick at {f} Hz");
    }

    #[test]
    fn the_808_metal_is_bright_and_the_hats_close_quickly() {
        let ch = run(DrumVoice::M808ClosedHat, 0.6);
        let oh = run(DrumVoice::M808OpenHat, 0.6);
        // A 7 kHz sine scores 0.2 on it, white noise 0.5.
        assert!(bright(at(&ch, 0.0, 0.05)) > 0.2, "bright hat: {}", bright(at(&ch, 0.0, 0.05)));
        assert!(rms(at(&ch, 0.15, 0.25)) < rms(at(&ch, 0.0, 0.03)) * 0.05, "the closed hat closes");
        assert!(rms(at(&oh, 0.15, 0.25)) > rms(at(&ch, 0.15, 0.25)) * 4.0, "the open one rings on");
        let cy = run(DrumVoice::M808Cymbal, 1.5);
        assert!(rms(at(&cy, 0.8, 1.0)) > 0.0005, "the cymbal sustains");
        assert!(bright(at(&cy, 0.0, 0.1)) > 0.2);
    }

    #[test]
    fn the_cowbell_rings_at_its_two_notes() {
        let cb = run(DrumVoice::M808Cowbell, 0.3);
        let x = at(&cb, 0.0, 0.2);
        let (a, b) = (magnitude(x, 540.0), magnitude(x, 800.0));
        let off = magnitude(x, 660.0).max(magnitude(x, 430.0));
        assert!(a > off * 3.0 && b > off * 3.0, "540 {a}, 800 {b}, between {off}");
    }

    #[test]
    fn claps_clap_in_bursts_then_a_tail() {
        for (voice, bursts) in [(DrumVoice::M808Clap, 3), (DrumVoice::M909Clap, 4)] {
            let c = run(voice, 0.4);
            // Peaks per 2 ms window over the first 40 ms: the bursts' starts stand out.
            let w = (0.002 * SR) as usize;
            let env: Vec<f32> = c[..(0.045 * SR) as usize].chunks(w).map(peak).collect();
            let rises = env.windows(2).filter(|p| p[1] > p[0] * 1.6).count();
            assert!(rises >= bursts - 1, "{voice:?}: {rises} bursts in {env:?}");
            assert!(rms(at(&c, 0.08, 0.15)) > 0.01, "{voice:?} has a tail");
        }
    }

    #[test]
    fn the_909_cymbals_are_six_bit_samples_through_a_vca() {
        for voice in [DrumVoice::M909ClosedHat, DrumVoice::M909OpenHat, DrumVoice::M909Ride, DrumVoice::M909Crash] {
            let x = run(voice, 2.0);
            assert!(peak(&x) > 0.1 && peak(&x) < 1.5, "{voice:?} peak {}", peak(&x));
        }
        let ch = run(DrumVoice::M909ClosedHat, 0.5);
        let oh = run(DrumVoice::M909OpenHat, 0.5);
        assert!(rms(at(&oh, 0.2, 0.3)) > rms(at(&ch, 0.2, 0.3)) * 4.0, "the open hat rings on");
        let ride = run(DrumVoice::M909Ride, 2.0);
        let crash = run(DrumVoice::M909Crash, 2.0);
        assert!(rms(at(&ride, 1.0, 1.5)) > 0.002 && rms(at(&crash, 0.8, 1.2)) > 0.002, "ride and crash ring on");
        // The ROM's steps: before the VCA's last smoothing, values sit on 63 levels; the output is
        // low-passed, so look at the held sample itself.
        let mut v = MachineVoice::default();
        let mut rng = Rng::new(2);
        v.trigger(DrumVoice::M909OpenHat, 1.0, 1.0, 1.0, 0.5, 0.5, SR);
        for _ in 0..2000 {
            v.tick(DrumVoice::M909OpenHat, 1.0, &mut rng);
            let k = v.held * 31.0;
            assert!((k - k.round()).abs() < 1e-3, "6-bit: {}", v.held);
        }
    }

    #[test]
    fn every_machine_voice_plays_and_ends() {
        use DrumVoice::*;
        let all = [
            M808Kick, M808Snare, M808Tom, M808Conga, M808Rim, M808Clap, M808Claves, M808Maracas, M808Cowbell, M808Cymbal, M808ClosedHat, M808OpenHat, M909Kick, M909Snare, M909Tom, M909Rim, M909Clap, M909ClosedHat, M909OpenHat, M909Ride, M909Crash,
            M606Kick, M606Snare, M606LowTom, M606HighTom, M606ClosedHat, M606OpenHat, M606Cymbal,
        ];
        for voice in all {
            let mut v = MachineVoice::default();
            let mut rng = Rng::new(5);
            v.trigger(voice, 1.0, 1.0, 1.0, 0.5, 0.5, SR);
            let x: Vec<f32> = (0..(12.0 * SR) as usize).map(|_| v.tick(voice, 1.0, &mut rng)).collect();
            let p = peak(&x);
            assert!(p > 0.3 && p < 1.3, "{voice:?} peak {p}");
            assert!(x.iter().all(|s| s.is_finite()), "{voice:?}");
            assert!(!v.is_active(), "{voice:?} ends");
        }
    }

    /// Levels of every machine voice, to set their trims (cargo test -- --ignored --nocapture levels).
    #[test]
    #[ignore]
    fn levels() {
        use DrumVoice::*;
        for voice in [
            M808Kick, M808Snare, M808Tom, M808Conga, M808Rim, M808Clap, M808Claves, M808Maracas, M808Cowbell, M808Cymbal, M808ClosedHat, M808OpenHat, M909Kick, M909Snare, M909Tom, M909Rim, M909Clap, M909ClosedHat, M909OpenHat, M909Ride, M909Crash,
            M606Kick, M606Snare, M606LowTom, M606HighTom, M606ClosedHat, M606OpenHat, M606Cymbal,
        ] {
            let x = run(voice, 1.0);
            println!("{voice:?}: peak {:.3}, rms(0..0.1) {:.3}", peak(&x), rms(at(&x, 0.0, 0.1)));
        }
    }
}
