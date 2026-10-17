//! Resonant drums: a port of the drums of Mutable Instruments Peaks by
//! Emilie Gillet (MIT, <https://github.com/pichenettes/eurorack>; the notice
//! is at the top of `peaks_tables`). A bass drum and a snare as the 808
//! makes them, pulses ringing a resonant filter (a state variable filter
//! here, its frequency pushed up by its own output for punch), the snare's
//! two bodies and band-passed noise; a hi-hat of six square waves through a
//! band pass and a VCA that lets only the top half through; and an FM drum,
//! a sine whose pitch an envelope and its own output bend, with noise or
//! overdrive. Fixed point as on the module, at its 48 kHz, resampled to
//! Krabton's rate.

use super::peaks_tables::*;

fn clip(x: i32) -> i32 {
    x.clamp(-32767, 32767)
}

fn interpolate824(table: &[u16], phase: u32) -> u32 {
    let i = (phase >> 24) as usize;
    let a = table[i] as u32;
    let b = table[i + 1] as u32;
    // (wrapping as the module's 16-bit arithmetic does)
    a.wrapping_add(b.wrapping_sub(a).wrapping_mul((phase >> 8) & 0xffff) >> 16) as u16 as u32
}

fn interpolate1022(table: &[i16], phase: u32) -> i32 {
    let i = (phase >> 22) as usize;
    let a = table[i] as i32;
    let b = table[i + 1] as i32;
    a + ((b - a) * ((phase >> 6) & 0xffff) as i32 >> 16)
}

fn mix(a: i32, b: i32, balance: u32) -> i32 {
    (a * (65535 - balance as i32) + b * balance as i32) >> 16
}

/// The module's noise: a linear congruential generator.
#[derive(Clone, Copy)]
struct Random(u32);

impl Random {
    fn sample(&mut self) -> i32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 16) as i16 as i32
    }
}

#[derive(Clone, Copy, Default)]
struct Excitation {
    delay: u32,
    decay: u32,
    counter: i32,
    state: i32,
    level: i32,
}

impl Excitation {
    fn new(delay: u32, decay: u32) -> Self {
        Self { delay, decay, ..Self::default() }
    }

    fn trigger(&mut self, level: i32) {
        self.level = level;
        self.counter = self.delay as i32 + 1;
    }

    fn done(&self) -> bool {
        self.counter == 0
    }

    fn process(&mut self) -> i32 {
        self.state = ((self.state as i64 * self.decay as i64) >> 12) as i32;
        if self.counter > 0 {
            self.counter -= 1;
            if self.counter == 0 {
                self.state += self.level.abs();
            }
        }
        if self.level < 0 {
            -self.state
        } else {
            self.state
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Bp,
    Hp,
}

#[derive(Clone, Copy)]
struct Svf {
    dirty: bool,
    frequency: i16,
    resonance: i16,
    punch: i32,
    f: i32,
    damp: i32,
    lp: i32,
    bp: i32,
}

impl Default for Svf {
    fn default() -> Self {
        Self { dirty: true, frequency: 33 << 7, resonance: 16384, punch: 0, f: 0, damp: 0, lp: 0, bp: 0 }
    }
}

impl Svf {
    fn set_frequency(&mut self, f: i16) {
        let f = f.max(0);
        self.dirty = self.dirty || self.frequency != f;
        self.frequency = f;
    }

    fn set_resonance(&mut self, r: i16) {
        self.resonance = r.max(0);
        self.dirty = true;
    }

    fn set_punch(&mut self, punch: u16) {
        self.punch = ((punch as u32 * punch as u32) >> 24) as i32;
    }

    fn process(&mut self, mode: Mode, input: i32) -> i32 {
        if self.dirty {
            self.f = interpolate824(&LUT_SVF_CUTOFF, (self.frequency as u32) << 17) as i32;
            self.damp = interpolate824(&LUT_SVF_DAMP, (self.resonance as u32) << 17) as i32;
            self.dirty = false;
        }
        let mut f = self.f;
        let mut damp = self.damp;
        if self.punch != 0 {
            let punch_signal = if self.lp > 4096 { self.lp } else { 2048 };
            f += ((punch_signal >> 4) * self.punch) >> 9;
            damp += (punch_signal - 2048) >> 3;
        }
        let notch = input - ((self.bp as i64 * damp as i64) >> 15) as i32;
        self.lp = clip(self.lp + ((f as i64 * self.bp as i64) >> 15) as i32);
        let hp = notch - self.lp;
        self.bp = clip(self.bp + ((f as i64 * hp as i64) >> 15) as i32);
        match mode {
            Mode::Bp => self.bp,
            Mode::Hp => hp,
        }
    }
}

/// Which of the drums.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Kick,
    Snare,
    Hat,
    FmKick,
    FmSnare,
}

/// The settings of a hit, 0 to 65535 (frequency: -32768 to 32767) as on the module.
#[derive(Clone, Copy, Debug)]
pub struct Knobs {
    pub frequency: i32,
    pub tone: u32,
    pub decay: u32,
    pub a: u32,
    pub b: u32,
}

#[derive(Clone, Copy)]
pub struct PeaksVoice {
    kind: Kind,
    // Bass drum.
    pulse_up: Excitation,
    pulse_down: Excitation,
    attack_fm: Excitation,
    resonator: Svf,
    frequency: i32,
    lp_coefficient: i32,
    lp_state: i32,
    // Snare.
    ex1_up: Excitation,
    ex1_down: Excitation,
    ex2: Excitation,
    ex_noise: Excitation,
    body1: Svf,
    body2: Svf,
    noise: Svf,
    gain1: i32,
    gain2: i32,
    snappy: i32,
    // Hi-hat.
    hat_phase: [u32; 6],
    vca_coloration: Svf,
    vca_env: Excitation,
    // FM drum.
    aux_strength: u32,
    fm_frequency: u32,
    fm_amount: u32,
    am_decay: u32,
    fm_decay: u32,
    fm_noise: u32,
    overdrive: u32,
    previous: i32,
    phase: u32,
    fm_env_phase: u32,
    am_env_phase: u32,
    aux_env_phase: u32,
    phase_inc: u32,
    count: u32,
    accent: f32,
    rng: Random,
    // Running at 48 kHz, read at the host's rate.
    step: f64,
    acc: f64,
    prev: f32,
    cur: f32,
    /// The module's output is AC coupled: a DC blocker.
    dc_x: f32,
    dc_y: f32,
    dc_k: f32,
    quiet: u32,
    active: bool,
}

impl Default for PeaksVoice {
    fn default() -> Self {
        let mut noise = Svf::default();
        noise.set_resonance(2000);
        Self {
            kind: Kind::Kick,
            pulse_up: Excitation::new(0, 3340),
            pulse_down: Excitation::new(48, 3072),
            attack_fm: Excitation::new(192, 4093),
            resonator: Svf::default(),
            frequency: 31 << 7,
            lp_coefficient: 0,
            lp_state: 0,
            ex1_up: Excitation::new(0, 1536),
            ex1_down: Excitation::new(48, 3072),
            ex2: Excitation::new(48, 1200),
            ex_noise: Excitation::new(0, 4093),
            body1: Svf::default(),
            body2: Svf::default(),
            noise,
            gain1: 22000,
            gain2: 22000,
            snappy: 512,
            hat_phase: [0; 6],
            vca_coloration: Svf::default(),
            vca_env: Excitation::new(0, 4093),
            aux_strength: 0,
            fm_frequency: 0,
            fm_amount: 0,
            am_decay: 0,
            fm_decay: 0,
            fm_noise: 0,
            overdrive: 0,
            previous: 0,
            phase: 0,
            fm_env_phase: u32::MAX,
            am_env_phase: u32::MAX,
            aux_env_phase: u32::MAX,
            phase_inc: 0,
            count: 0,
            accent: 1.0,
            rng: Random(0x2152_4111),
            step: 1.0,
            acc: 0.0,
            prev: 0.0,
            cur: 0.0,
            dc_x: 0.0,
            dc_y: 0.0,
            dc_k: 0.999,
            quiet: 0,
            active: false,
        }
    }
}

const K_HIGHEST_NOTE: i32 = 128 * 128;
const K_PITCH_TABLE_START: i32 = 116 * 128;
const K_OCTAVE: i32 = 128 * 12;

fn env_increment(decay: u32) -> u32 {
    let i = (decay >> 8) as usize;
    let a = LUT_ENV_INCREMENTS[i];
    let b = LUT_ENV_INCREMENTS[i + 1];
    a.wrapping_sub(((a.wrapping_sub(b)) as u64 * (decay & 0xff) as u64 >> 8) as u32)
}

fn phase_increment(midi_pitch: i32) -> u32 {
    let mut ref_pitch = midi_pitch.min(K_HIGHEST_NOTE - 1) - K_PITCH_TABLE_START;
    let mut shifts = 0;
    while ref_pitch < 0 {
        ref_pitch += K_OCTAVE;
        shifts += 1;
    }
    let i = (ref_pitch >> 4) as usize;
    let a = LUT_OSCILLATOR_INCREMENTS[i] as i64;
    let b = LUT_OSCILLATOR_INCREMENTS[i + 1] as i64;
    let inc = (a + ((b - a) * (ref_pitch & 0xf) as i64 >> 4)) as u32;
    inc >> shifts
}

impl PeaksVoice {
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// A hit, with the knobs as the module takes them.
    pub fn trigger(&mut self, kind: Kind, k: Knobs, vel: f32, sr: f32) {
        if self.kind != kind {
            *self = Self { kind, ..Self::default() };
        }
        self.step = 48000.0 / sr.max(1000.0) as f64;
        self.dc_k = 1.0 - std::f32::consts::TAU * 8.0 / sr.max(1000.0);
        self.active = true;
        self.quiet = 0;
        let accent = (0.35 + 0.65 * vel.clamp(0.0, 1.0)) as f64;
        match kind {
            Kind::Kick => {
                self.frequency = (31 << 7) + (k.frequency * 896 >> 15);
                let scaled = 65535 - k.decay.min(65535);
                let squared = scaled * scaled >> 16;
                let scaled = squared * scaled >> 18;
                self.resonator.set_resonance((32768 - 128 - scaled as i32).clamp(0, 32767) as i16);
                let c = k.tone.min(65535);
                let c = c * c >> 16;
                self.lp_coefficient = 512 + (c >> 2) as i32 * 3;
                let punch = k.a.min(65535);
                self.resonator.set_punch((punch * punch >> 16) as u16);
                self.pulse_up.trigger((12.0 * 32768.0 * 0.7 * accent) as i32);
                self.pulse_down.trigger((-19662.0 * 0.7 * accent) as i32);
                self.attack_fm.trigger(18000);
            }
            Kind::Snare => {
                let tone = k.tone.min(65535) as i32;
                self.gain1 = 22000 - (tone >> 2);
                self.gain2 = 22000 + (tone >> 2);
                let snappy = ((k.a.min(65535) >> 1) as i32).min(28672);
                self.snappy = 512 + snappy;
                let decay = k.decay.min(65535) as i32;
                self.body1.set_resonance((29000 + (decay >> 5)) as i16);
                self.body2.set_resonance((26500 + (decay >> 5)) as i16);
                self.ex_noise.decay = (4092 + (decay >> 14)) as u32;
                let base = (52 << 7) + (k.frequency * 896 >> 15);
                self.body1.set_frequency(base as i16);
                self.body2.set_frequency((base + (12 << 7)) as i16);
                self.noise.set_frequency((base + (48 << 7)).min(32767) as i16);
                self.ex1_up.trigger((15.0 * 32768.0 * accent) as i32);
                self.ex1_down.trigger((-32768.0 * accent) as i32);
                self.ex2.trigger((13107.0 * accent) as i32);
                self.ex_noise.trigger((self.snappy as f64 * accent) as i32);
            }
            Kind::Hat => {
                // The module's hat has no knobs; here Tone moves the band, Decay the VCA.
                let f = 105.0 + (k.frequency as f32 / 32768.0) * 7.0 + (k.tone as f32 / 65535.0 - 0.5) * 8.0;
                self.noise.set_frequency(((f.clamp(80.0, 124.0)) * 128.0) as i16);
                self.noise.set_resonance(24000);
                self.vca_coloration.set_frequency(110 << 7);
                self.vca_coloration.set_resonance(0);
                self.vca_env.decay = (4093 + ((k.decay.min(65535) as i32 - 32768) >> 13)).clamp(4080, 4095) as u32;
                self.vca_env.trigger((32768.0 * 15.0 * accent) as i32);
            }
            Kind::FmKick | Kind::FmSnare => {
                let frequency = k.frequency.clamp(0, 65535) as u32;
                self.aux_strength = if frequency <= 16384 {
                    1024
                } else if frequency <= 32768 {
                    2048 - (frequency >> 4)
                } else {
                    0
                };
                self.fm_frequency = (24 << 7) + ((72 << 7) * frequency >> 16);
                self.fm_amount = ((k.a.min(65535) >> 2) * 3) >> 2;
                let decay = k.decay.min(65535);
                self.am_decay = 16384 + (decay >> 1);
                self.fm_decay = 8192 + (decay >> 2);
                let n = k.b.min(65535);
                let noise = if n >= 32768 { (n - 32768) * (n - 32768) >> 15 } else { 0 };
                self.fm_noise = (noise >> 2) * 5;
                self.overdrive = if n <= 32767 { (32767 - n) * (32767 - n) >> 14 } else { 0 };
                self.fm_env_phase = 0;
                self.am_env_phase = 0;
                self.aux_env_phase = 0;
                self.phase = 0x3fff * self.fm_amount >> 16;
                self.count = 0;
                self.accent = accent as f32;
            }
        }
    }

    fn render48(&mut self) -> i32 {
        match self.kind {
            Kind::Kick => {
                let mut excitation = self.pulse_up.process();
                excitation += if !self.pulse_down.done() { 16384 } else { 0 };
                excitation += self.pulse_down.process();
                self.attack_fm.process();
                let f = self.frequency + if self.attack_fm.done() { 0 } else { 17 << 7 };
                self.resonator.set_frequency(f as i16);
                let out = (excitation >> 4) + self.resonator.process(Mode::Bp, excitation);
                self.lp_state += ((out - self.lp_state) as i64 * self.lp_coefficient as i64 >> 15) as i32;
                clip(self.lp_state)
            }
            Kind::Snare => {
                let mut e1 = self.ex1_up.process();
                e1 += self.ex1_down.process();
                e1 += if !self.ex1_down.done() { 2621 } else { 0 };
                let body1 = self.body1.process(Mode::Bp, e1) + (e1 >> 4);
                let mut e2 = self.ex2.process();
                e2 += if !self.ex2.done() { 13107 } else { 0 };
                let body2 = self.body2.process(Mode::Bp, e2) + (e2 >> 4);
                let ns = self.rng.sample();
                let noise = self.noise.process(Mode::Bp, ns);
                let env = self.ex_noise.process();
                let sd = (body1 * self.gain1 >> 15) + (body2 * self.gain2 >> 15) + ((env as i64 * noise as i64) >> 15) as i32;
                clip(sd)
            }
            Kind::Hat => {
                const INC: [u32; 6] = [48_318_382, 71_582_788, 37_044_092, 54_313_440, 66_214_079, 93_952_409];
                let mut noise: i32 = 0;
                for (p, inc) in self.hat_phase.iter_mut().zip(INC) {
                    *p = p.wrapping_add(inc);
                    noise += (*p >> 31) as i32;
                }
                noise <<= 12;
                // The band pass twice a sample, for stability.
                let mut filtered = self.noise.process(Mode::Bp, noise);
                filtered += self.noise.process(Mode::Bp, noise);
                // The VCA passes only the top half, as the 808's does.
                let filtered = filtered.clamp(0, 32767);
                let env = self.vca_env.process() >> 4;
                let vca = clip(((env as i64 * filtered as i64) >> 14) as i32);
                let mut hh = self.vca_coloration.process(Mode::Hp, vca);
                hh += self.vca_coloration.process(Mode::Hp, vca);
                clip(hh << 1)
            }
            Kind::FmKick | Kind::FmSnare => {
                let am_inc = env_increment(self.am_decay);
                let fm_inc = env_increment(self.fm_decay);
                self.fm_env_phase = self.fm_env_phase.wrapping_add(fm_inc);
                if self.fm_env_phase < fm_inc {
                    self.fm_env_phase = u32::MAX;
                }
                self.aux_env_phase = self.aux_env_phase.wrapping_add(4_473_924);
                if self.aux_env_phase < 4_473_924 {
                    self.aux_env_phase = u32::MAX;
                }
                if self.count % 4 == 0 {
                    let aux = 65535 - interpolate824(&LUT_ENV_EXPO, self.aux_env_phase);
                    let fm = 65535 - interpolate824(&LUT_ENV_EXPO, self.fm_env_phase);
                    let pitch = self.fm_frequency as i32 + (fm * self.fm_amount >> 16) as i32 + (aux * self.aux_strength >> 15) as i32 + (self.previous >> 6);
                    self.phase_inc = phase_increment(pitch);
                }
                self.count = self.count.wrapping_add(1);
                self.phase = self.phase.wrapping_add(self.phase_inc);
                let mut m = interpolate1022(&WAV_SINE, self.phase);
                if self.fm_noise != 0 {
                    let n = self.rng.sample();
                    m = mix(m, n, self.fm_noise);
                }
                self.am_env_phase = self.am_env_phase.wrapping_add(am_inc);
                if self.am_env_phase < am_inc {
                    self.am_env_phase = u32::MAX;
                }
                let am = 65535 - interpolate824(&LUT_ENV_EXPO, self.am_env_phase);
                m = (m * am as i32) >> 16;
                if self.overdrive != 0 {
                    let phi = ((m << 16) as u32).wrapping_add(1 << 31);
                    let od = interpolate1022(&WAV_OVERDRIVE, phi);
                    m = mix(m, od, self.overdrive);
                }
                self.previous = m;
                (m as f32 * self.accent) as i32
            }
        }
    }

    /// One sample at the host's rate (-1 to 1).
    pub fn tick(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }
        self.acc += self.step;
        while self.acc >= 1.0 {
            self.acc -= 1.0;
            self.prev = self.cur;
            self.cur = self.render48() as f32 / 32768.0;
        }
        let x = self.prev + (self.cur - self.prev) * self.acc as f32;
        let y = x - self.dc_x + self.dc_k * self.dc_y;
        self.dc_x = x;
        self.dc_y = y;
        // Done after a quiet tenth of a second (the fixed point never quite
        // reaches nothing).
        if y.abs() < 6e-4 {
            self.quiet += 1;
        } else {
            self.quiet = 0;
        }
        if self.quiet as f64 > 4800.0 / self.step {
            self.active = false;
        }
        y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn knobs() -> Knobs {
        Knobs { frequency: 0, tone: 32768, decay: 32768, a: 40000, b: 32768 }
    }

    fn hit(kind: Kind, k: Knobs, secs: f32) -> Vec<f32> {
        let mut v = PeaksVoice::default();
        v.trigger(kind, k, 1.0, 48000.0);
        (0..(secs * 48000.0) as usize).map(|_| v.tick()).collect()
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    #[test]
    fn every_drum_sounds_and_dies_away() {
        for kind in [Kind::Kick, Kind::Snare, Kind::Hat, Kind::FmKick, Kind::FmSnare] {
            let mut v = PeaksVoice::default();
            let mut k = knobs();
            if kind == Kind::FmSnare {
                k.frequency = 40000;
                k.b = 50000;
            }
            v.trigger(kind, k, 1.0, 44100.0);
            let out: Vec<f32> = (0..44100 * 3).map(|_| v.tick()).collect();
            assert!(rms(&out[..4410]) > 0.01, "{kind:?}: {}", rms(&out[..4410]));
            assert!(out.iter().all(|x| x.is_finite() && x.abs() <= 1.1), "{kind:?}");
            assert!(!v.is_active(), "{kind:?} rings on");
        }
    }

    #[test]
    fn the_kick_rings_low_and_the_snare_higher() {
        let zc = |x: &[f32]| x.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
        let kick = hit(Kind::Kick, knobs(), 0.2);
        let snare = hit(Kind::Snare, Knobs { a: 0, ..knobs() }, 0.2);
        let (k, s) = (zc(&kick[960..]), zc(&snare[960..]));
        assert!(k < s, "{k} {s}");
        // The kick near 50 Hz.
        let hz = k as f32 / 0.18;
        assert!((30.0..90.0).contains(&hz), "{hz}");
    }

    #[test]
    fn longer_decay_rings_longer() {
        let short = hit(Kind::Kick, Knobs { decay: 8000, ..knobs() }, 1.0);
        let long = hit(Kind::Kick, Knobs { decay: 60000, ..knobs() }, 1.0);
        assert!(rms(&long[24000..]) > 2.0 * rms(&short[24000..]));
    }
}


