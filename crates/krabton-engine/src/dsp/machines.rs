//! Machines: drum voices after classic analog drum machines, built from how
//! their circuits work. The 808's bridged-T resonators (a damped sine with a
//! little pitch drop, plus the click of the trigger pulse), its six square
//! oscillators through band-passes for hats, cymbal and cowbell, and its
//! clap of three noise bursts and a tail; the 909's swept triangle
//! oscillators with a separate attack; the CR-78's gentler resonators and
//! gated noise; and the bending toms of the first electronic kits.
//! Original code, not samples.

use super::drums::DrumVoice;
use super::*;

/// The 808's metal: six square oscillators (Hz).
const METAL_808: [f32; 6] = [205.3, 304.4, 369.6, 522.7, 540.0, 800.0];
/// A brighter, denser set for the 909-style hats.
const METAL_909: [f32; 6] = [312.0, 448.0, 587.0, 727.0, 871.0, 1063.0];

/// A machine drum voice while it sounds.
#[derive(Clone, Copy)]
pub struct MachineVoice {
    t: u32,
    sr: f32,
    vel: f32,
    phase: [f32; 6],
    /// Body, second body and noise envelopes, and their per-sample decays.
    env: [f32; 3],
    coef: [f32; 3],
    /// Pitch bend (fraction above the base pitch) and its decay.
    bend: f32,
    bend_coef: f32,
    click: f32,
    click_coef: f32,
    filt: [Svf; 3],
    lp: OnePole,
    /// Base frequencies (Hz).
    freq: [f32; 2],
    tone: f32,
    macro_a: f32,
    active: bool,
}

impl Default for MachineVoice {
    fn default() -> Self {
        Self {
            t: 0,
            sr: 48000.0,
            vel: 1.0,
            phase: [0.0; 6],
            env: [0.0; 3],
            coef: [0.0; 3],
            bend: 0.0,
            bend_coef: 0.0,
            click: 0.0,
            click_coef: 0.0,
            filt: [Svf::new(); 3],
            lp: OnePole::default(),
            freq: [0.0; 2],
            tone: 0.5,
            macro_a: 0.5,
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

impl MachineVoice {
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Hits `voice`. `tune` is a frequency ratio, `decay` stretches the
    /// decay (1 = as built), `tone` and `a` are the pad's Tone and extra knob.
    #[allow(clippy::too_many_arguments)]
    pub fn trigger(&mut self, voice: DrumVoice, vel: f32, tune: f32, decay: f32, tone: f32, a: f32, sr: f32) {
        use DrumVoice::*;
        self.t = 0;
        self.sr = sr;
        self.vel = vel.clamp(0.0, 1.0);
        self.tone = tone.clamp(0.0, 1.0);
        self.macro_a = a.clamp(0.0, 1.0);
        self.active = true;
        self.phase = [0.0, 0.13, 0.31, 0.47, 0.61, 0.83];
        self.env = [1.0, 1.0, 1.0];
        self.bend = 0.0;
        self.bend_coef = 0.0;
        self.click = 1.0;
        self.click_coef = decay_coef(0.004, sr);
        for f in &mut self.filt {
            f.reset();
        }
        self.lp.z = 0.0;
        let d = decay.max(0.05);
        let dc = |s: f32| decay_coef(s, sr);
        match voice {
            M808Kick => {
                self.freq = [56.0 * tune, 0.0];
                self.bend = 0.22;
                self.bend_coef = dc(0.035);
                self.coef = [dc(0.7 * d), 0.0, 0.0];
                self.lp.set(300.0 + 3500.0 * self.tone * self.tone, sr);
            }
            M808Snare => {
                self.freq = [185.0 * tune, 330.0 * tune];
                self.coef = [dc(0.16 * d.sqrt()), dc(0.1 * d.sqrt()), dc(0.28 * d)];
                self.filt[0].set_q(1800.0, 0.7, sr);
                self.filt[1].set(9000.0, 0.0, sr);
            }
            M808Tom | M808Conga => {
                let (base, len) = if voice == M808Tom { (120.0, 0.45) } else { (250.0, 0.2) };
                self.freq = [base * tune, 0.0];
                self.bend = 0.18;
                self.bend_coef = dc(0.12);
                self.coef = [dc(len * d), 0.0, dc(0.09)];
                self.filt[0].set(900.0 * tune, 0.0, sr);
            }
            M808Rim => {
                self.freq = [455.0 * tune, 1667.0 * tune];
                self.coef = [dc(0.03 * d), dc(0.02 * d), 0.0];
                self.filt[0].set_q(900.0, 0.7, sr);
            }
            M808Claves => {
                self.freq = [2500.0 * tune, 0.0];
                self.coef = [dc(0.06 * d), 0.0, 0.0];
            }
            M808Clap | M909Clap => {
                let (f, q) = if voice == M808Clap { (1000.0, 2.5) } else { (1250.0, 1.6) };
                self.filt[0].set_q(f * tune, q, sr);
                self.filt[1].set_q(4000.0, 0.7, sr);
                let tail = if voice == M808Clap { 0.35 } else { 0.5 };
                self.coef = [dc(tail * d), 0.0, 0.0];
            }
            M808Maracas => {
                self.filt[0].set_q(5000.0 * tune, 0.7, sr);
                self.coef = [dc(0.07 * d), 0.0, 0.0];
            }
            M808Cowbell => {
                self.freq = [540.0 * tune, 800.0 * tune];
                self.filt[0].set_q(2640.0 * tune, 2.4, sr);
                self.coef = [dc(0.05), dc(0.6 * d), 0.0];
            }
            M808Cymbal => {
                self.filt[0].set_q(3440.0 * tune, 1.5, sr);
                self.filt[1].set_q(7100.0 * tune, 0.8, sr);
                self.filt[2].set_q(6000.0, 0.7, sr);
                self.coef = [dc(0.9 * d), dc(2.2 * d), 0.0];
            }
            M808ClosedHat | M808OpenHat | M909ClosedHat | M909OpenHat => {
                let open = matches!(voice, M808OpenHat | M909OpenHat);
                let len = if open { 0.7 } else { 0.09 };
                let f = if matches!(voice, M808ClosedHat | M808OpenHat) { 7100.0 } else { 9500.0 };
                self.filt[0].set_q(f * tune.sqrt(), 1.2, sr);
                self.filt[1].set_q(6000.0, 0.7, sr);
                self.coef = [dc(len * d), 0.0, 0.0];
            }
            M909Kick => {
                self.freq = [52.0 * tune, 0.0];
                self.bend = 2.6;
                self.bend_coef = dc(0.07);
                self.coef = [dc(0.75 * d), 0.0, dc(0.02)];
                self.filt[0].set_q(2500.0, 0.7, sr);
            }
            M909Snare => {
                self.freq = [175.0 * tune, 334.0 * tune];
                self.bend = 0.25;
                self.bend_coef = dc(0.03);
                self.coef = [dc(0.18 * d.sqrt()), dc(0.12 * d.sqrt()), dc(0.3 * d)];
                self.filt[0].set(2500.0 + 8000.0 * self.tone, 0.1, sr);
                self.filt[1].set_q(3000.0, 0.7, sr);
            }
            M909Tom => {
                self.freq = [110.0 * tune, 0.0];
                self.bend = 0.45;
                self.bend_coef = dc(0.15);
                self.coef = [dc(0.6 * d), 0.0, dc(0.07)];
                self.filt[0].set(2000.0, 0.0, sr);
            }
            M909Rim => {
                self.freq = [500.0 * tune, 1700.0 * tune];
                self.coef = [dc(0.025 * d), dc(0.018 * d), dc(0.01)];
                self.filt[0].set_q(4000.0, 0.7, sr);
            }
            Cr78Kick => {
                self.freq = [66.0 * tune, 0.0];
                self.bend = 0.08;
                self.bend_coef = dc(0.03);
                self.coef = [dc(0.32 * d), 0.0, 0.0];
                self.lp.set(800.0 + 1200.0 * self.tone, sr);
            }
            Cr78Snare => {
                self.freq = [300.0 * tune, 0.0];
                self.coef = [dc(0.09 * d.sqrt()), 0.0, dc(0.16 * d)];
                self.filt[0].set_q(3200.0, 1.0, sr);
            }
            Cr78Hat => {
                self.filt[0].set_q(7000.0 * tune, 0.7, sr);
                self.filt[1].set_q(10500.0 * tune.sqrt(), 2.0, sr);
                self.coef = [dc(0.06 * d), 0.0, 0.0];
            }
            Cr78Cymbal => {
                self.freq = [1150.0 * tune, 1590.0 * tune];
                self.filt[0].set_q(5200.0 * tune, 1.4, sr);
                self.filt[1].set_q(4000.0, 0.7, sr);
                self.coef = [dc(1.1 * d), 0.0, 0.0];
            }
            Cr78Bongo => {
                self.freq = [420.0 * tune, 0.0];
                self.bend = 0.05;
                self.bend_coef = dc(0.02);
                self.coef = [dc(0.12 * d), 0.0, 0.0];
            }
            Cr78Guiro => {
                self.filt[0].set_q(2800.0 * tune, 3.0, sr);
                self.coef = [dc(0.22 * d), 0.0, 0.0];
            }
            Cr78Tambourine => {
                self.filt[0].set_q(6500.0 * tune, 0.7, sr);
                self.filt[1].set_q(9000.0 * tune.sqrt(), 1.5, sr);
                self.coef = [dc(0.25 * d), 0.0, 0.0];
            }
            Cr78Metal => {
                self.freq = [1100.0 * tune, 1530.0 * tune];
                self.filt[0].set_q(3000.0 * tune, 2.0, sr);
                self.coef = [dc(0.16 * d), 0.0, 0.0];
            }
            ElectroTom => {
                self.freq = [110.0 * tune, 0.0];
                // Bend: how far above the note the drum starts (up to an octave and a half).
                self.bend = 2f32.powf(1.5 * self.macro_a) - 1.0;
                self.bend_coef = dc(0.35 * d);
                self.coef = [dc(0.7 * d), 0.0, 0.0];
                self.filt[0].set_q(3000.0, 0.7, sr);
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

    /// Six squares at `freqs` scaled by `tune`, summed (-1..1).
    #[inline]
    fn metal(&mut self, freqs: &[f32; 6], tune: f32) -> f32 {
        let mut s = 0.0;
        for (k, f) in freqs.iter().enumerate() {
            s += square(self.advance(k, f * tune));
        }
        s / 6.0
    }

    #[inline]
    pub fn tick(&mut self, voice: DrumVoice, tune: f32, rng: &mut Rng) -> f32 {
        use DrumVoice::*;
        let sr = self.sr;
        let ts = self.t as f32 / sr;
        let bendf = 1.0 + self.bend;
        let (tone, a) = (self.tone, self.macro_a);
        let out = match voice {
            M808Kick => {
                let body = sine(self.advance(0, self.freq[0] * bendf)) * self.env[0];
                // The trigger pulse leaks through, softened by the tone filter.
                let pulse = self.lp.lp(if ts < 0.0012 { 1.0 } else { 0.0 });
                // Accent (velocity) brings out the click, as on the machine.
                fast_tanh((body + pulse * (0.4 + 0.8 * tone) * (0.5 + 0.5 * self.vel)) * 1.3)
            }
            M808Snare => {
                let b1 = sine(self.advance(0, self.freq[0])) * self.env[0];
                let b2 = sine(self.advance(1, self.freq[1])) * self.env[1];
                let n = self.filt[0].process(rng.bipolar(), FilterMode::High);
                let n = self.filt[1].process(n, FilterMode::Low);
                let snappy = 0.25 + 1.1 * a;
                b1 * (0.75 - 0.35 * tone) + b2 * (0.3 + 0.6 * tone) + n * self.env[2] * snappy
            }
            M808Tom | M808Conga => {
                let body = sine(self.advance(0, self.freq[0] * bendf)) * self.env[0];
                let noise = if voice == M808Tom { self.filt[0].process(rng.bipolar(), FilterMode::Low) * self.env[2] * (0.1 + 0.3 * tone) } else { 0.0 };
                body + noise
            }
            M808Rim => {
                let x = sine(self.advance(0, self.freq[0])) * self.env[0] + sine(self.advance(1, self.freq[1])) * self.env[1];
                let y = self.filt[0].process(x, FilterMode::High);
                fast_tanh(y * (2.0 + 3.0 * tone)) * 0.8
            }
            M808Claves => {
                let p = self.advance(0, self.freq[0]);
                sine(p) * self.env[0] * (0.9 + 0.2 * tone)
            }
            M808Clap | M909Clap => {
                // Three quick bursts, then the tail.
                let gap = if voice == M808Clap { 0.0105 } else { 0.0085 };
                let burst = if ts < 3.0 * gap { (-(ts % gap) / 0.0025).exp() } else { 0.0 };
                let tail = if ts >= 2.0 * gap { self.env[0] * 0.7 } else { 0.0 };
                let n = rng.bipolar();
                let band = self.filt[0].process(n, FilterMode::Band);
                let bright = self.filt[1].process(n, FilterMode::High) * (0.1 + 0.4 * tone);
                (band * 2.4 + bright) * burst.max(tail)
            }
            M808Maracas => {
                let attack = (ts / 0.004).min(1.0);
                self.filt[0].process(rng.bipolar(), FilterMode::High) * self.env[0] * attack * 1.2
            }
            M808Cowbell => {
                let x = (square(self.advance(0, self.freq[0])) + square(self.advance(1, self.freq[1]))) * 0.5;
                let y = self.filt[0].process(x, FilterMode::Band);
                y * (self.env[0] * 0.55 + self.env[1] * (0.35 + 0.3 * tone)) * 1.6
            }
            M808Cymbal => {
                let m = self.metal(&METAL_808, tune);
                let lo = self.filt[0].process(m, FilterMode::Band) * self.env[0];
                let hi = self.filt[1].process(m, FilterMode::Band) * self.env[1];
                let y = lo * (0.8 - 0.5 * tone) + hi * (0.4 + 0.6 * tone);
                self.filt[2].process(y, FilterMode::High) * 2.0
            }
            M808ClosedHat | M808OpenHat => {
                let m = self.metal(&METAL_808, tune);
                let y = self.filt[0].process(m, FilterMode::Band);
                let y = self.filt[1].process(y, FilterMode::High);
                y * self.env[0] * (2.2 + 1.2 * tone)
            }
            M909ClosedHat | M909OpenHat => {
                let m = self.metal(&METAL_909, tune) * (1.0 - 0.5 * tone) + rng.bipolar() * (0.3 + 0.5 * tone);
                let y = self.filt[0].process(m, FilterMode::Band);
                let y = self.filt[1].process(y, FilterMode::High);
                y * self.env[0] * 2.4
            }
            M909Kick => {
                let p = self.advance(0, self.freq[0] * bendf);
                // A triangle bent towards a sine, driven a little.
                let t = tri(p);
                let body = (t * (1.0 + 0.5 * tone) * std::f32::consts::FRAC_PI_2).sin() * self.env[0];
                let noise = self.filt[0].process(rng.bipolar(), FilterMode::High) * self.env[2];
                let pulse = if ts < 0.0008 { 1.0 } else { 0.0 };
                let attack = (noise * 0.6 + pulse * 0.8) * (0.15 + 0.85 * a) * (0.5 + 0.5 * self.vel);
                fast_tanh((body + attack) * 1.4)
            }
            M909Snare => {
                let b1 = tri(self.advance(0, self.freq[0] * bendf)) * self.env[0];
                let b2 = tri(self.advance(1, self.freq[1] * bendf)) * self.env[1];
                let n = rng.bipolar();
                let body_noise = self.filt[0].process(n, FilterMode::Low) * self.env[2] * 0.5;
                let snappy = self.filt[1].process(n, FilterMode::High) * self.env[2] * (0.2 + 1.2 * a);
                (b1 * 0.6 + b2 * 0.35) * 0.9 + body_noise + snappy
            }
            M909Tom => {
                let body = tri(self.advance(0, self.freq[0] * bendf));
                let body = (body * std::f32::consts::FRAC_PI_2).sin() * self.env[0];
                let noise = self.filt[0].process(rng.bipolar(), FilterMode::Low) * self.env[2] * (0.1 + 0.3 * tone);
                body + noise
            }
            M909Rim => {
                let x = sine(self.advance(0, self.freq[0])) * self.env[0] + sine(self.advance(1, self.freq[1])) * self.env[1];
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
            ElectroTom => {
                let body = sine(self.advance(0, self.freq[0] * bendf)) * self.env[0];
                let click = self.filt[0].process(rng.bipolar(), FilterMode::High) * self.click * (0.2 + 0.8 * tone);
                body + click * 0.6
            }
            _ => 0.0,
        };
        for k in 0..3 {
            self.env[k] *= self.coef[k];
        }
        self.bend *= self.bend_coef;
        self.click *= self.click_coef;
        self.t = self.t.saturating_add(1);
        // A voice is over once all its envelopes are.
        if self.env.iter().all(|e| *e < 1e-4) && ts > 0.05 {
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
        M808Cymbal => 1.8,
        M808ClosedHat => 1.6,
        M808OpenHat => 1.3,
        M909ClosedHat | M909OpenHat => 0.8,
        M808Maracas => 0.6,
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(voice: DrumVoice, secs: f32) -> Vec<f32> {
        let sr = 48000.0;
        let mut v = MachineVoice::default();
        let mut rng = Rng::new(3);
        v.trigger(voice, 1.0, 1.0, 1.0, 0.5, 0.5, sr);
        (0..(secs * sr) as usize).map(|_| v.tick(voice, 1.0, &mut rng)).collect()
    }

    /// Frequency from upward zero crossings between `a` and `b` seconds.
    fn pitch(x: &[f32], a: f32, b: f32) -> f32 {
        let (i, j) = ((a * 48000.0) as usize, (b * 48000.0) as usize);
        let n = x[i..j].windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
        n as f32 / (b - a)
    }

    #[test]
    fn kicks_sit_at_their_pitch() {
        let k = run(DrumVoice::M808Kick, 0.5);
        let f = pitch(&k, 0.1, 0.4);
        assert!((50.0..62.0).contains(&f), "808 kick at {f} Hz");
        let k = run(DrumVoice::Cr78Kick, 0.3);
        let f = pitch(&k, 0.05, 0.25);
        assert!((60.0..72.0).contains(&f), "CR-78 kick at {f} Hz");
    }

    #[test]
    fn the_909_kick_starts_high_and_falls() {
        let k = run(DrumVoice::M909Kick, 0.5);
        let start = pitch(&k, 0.002, 0.03);
        let end = pitch(&k, 0.25, 0.45);
        assert!(start > end * 1.8, "{start} Hz falls to {end} Hz");
        assert!((45.0..60.0).contains(&end), "settles at {end} Hz");
    }
}
