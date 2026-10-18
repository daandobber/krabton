//! Warps' vocoder: two banks of 20 band-pass filters a third of an octave
//! apart (the low bands run at a twelfth of the rate, the middle ones at a
//! third), envelope followers on the modulator's bands shaping the
//! carrier's, a formant shift that reads the followers shifted, and a
//! limiter (`warps/dsp/vocoder.cc`, `filter_bank.cc`, `limiter.h`).
//!
//! Copyright 2014 Emilie Gillet, MIT license (see the crate's `lib.rs`).

use crate::dsp::{semitones_to_ratio, soft_limit, CrossMode, CrossoverSvf, Down, Up};
use crate::resources::{FILTER_BANK, SRC_DOWN_3_36, SRC_DOWN_4_48, SRC_UP_3_36, SRC_UP_4_48};

pub const NUM_BANDS: usize = 20;
const LOW_FACTOR: usize = 4;
const MID_FACTOR: usize = 3;
/// The largest block (at 96 kHz) the bank works on.
pub const MAX_BLOCK: usize = 96;

struct DelayLine {
    buf: Vec<f32>,
    head: usize,
}

impl DelayLine {
    fn new(delay: usize) -> Self {
        Self { buf: vec![0.0; delay + 1], head: 0 }
    }

    fn read_write(&mut self, v: f32) -> f32 {
        self.buf[self.head] = v;
        self.head = (self.head + 1) % self.buf.len();
        self.buf[self.head]
    }
}

struct Band {
    group: usize,
    sample_rate: f32,
    post_gain: f32,
    svf: [CrossoverSvf; 2],
    decimation: usize,
    samples: Vec<f32>,
    delay_line: DelayLine,
}

pub struct FilterBank {
    mid_down: Down,
    mid_up: Up,
    low_down: Down,
    low_up: Up,
    tmp: [Vec<f32>; 2],
    bands: Vec<Band>,
}

impl FilterBank {
    pub fn new(sample_rate: f32) -> Self {
        let (mid_down, mid_up) = (Down::new(MID_FACTOR, &SRC_DOWN_3_36), Up::new(MID_FACTOR, &SRC_UP_3_36));
        let (low_down, low_up) = (Down::new(LOW_FACTOR, &SRC_DOWN_4_48), Up::new(LOW_FACTOR, &SRC_UP_4_48));
        let mut group = 0usize;
        let mut last_decimation = 0usize;
        let mut first = true;
        let mut delays = Vec::new();
        let mut bands: Vec<Band> = Vec::new();
        for c in FILTER_BANK.iter() {
            let decimation = c[0] as usize;
            if decimation != last_decimation {
                if !first {
                    group += 1;
                }
                first = false;
                last_decimation = decimation;
            }
            let delay = c[1] as usize * decimation;
            delays.push(delay);
            let mut svf = [CrossoverSvf::default(); 2];
            for (pass, s) in svf.iter_mut().enumerate() {
                s.set_f_fq(c[pass * 2 + 3], c[pass * 2 + 4]);
            }
            bands.push(Band { group, sample_rate: sample_rate / decimation as f32, post_gain: c[2], svf, decimation, samples: vec![0.0; MAX_BLOCK / decimation], delay_line: DelayLine::new(0) });
        }
        let max_delay = delays.iter().copied().max().unwrap_or(0).min(256) as i32;
        for (b, &d) in bands.iter_mut().zip(&delays) {
            let mut comp = max_delay - d as i32;
            if b.group == 0 {
                comp -= (LOW_FACTOR * (low_down.delay() + low_up.delay())) as i32;
                comp -= mid_down.delay() as i32;
                comp -= mid_up.delay() as i32;
            } else if b.group == 1 {
                comp -= mid_down.delay() as i32;
                comp -= mid_up.delay() as i32;
            }
            let comp = (comp - b.decimation as i32 / 2).max(0) as usize;
            b.delay_line = DelayLine::new(comp / b.decimation);
        }
        Self { mid_down, mid_up, low_down, low_up, tmp: [vec![0.0; MAX_BLOCK], vec![0.0; MAX_BLOCK]], bands }
    }

    pub fn band_sample_rate(&self, i: usize) -> f32 {
        self.bands[i].sample_rate
    }

    pub fn band_decimation(&self, i: usize) -> usize {
        self.bands[i].decimation
    }

    /// Splits `input` (a multiple of 12 long) into the bands.
    pub fn analyze(&mut self, input: &[f32]) {
        let size = input.len();
        let [t0, t1] = &mut self.tmp;
        self.mid_down.process(input, &mut t0[..size / MID_FACTOR]);
        self.low_down.process(&t0[..size / MID_FACTOR], &mut t1[..size / MID_FACTOR / LOW_FACTOR]);
        let last = self.bands.len() - 1;
        for (i, b) in self.bands.iter_mut().enumerate() {
            let n = size / b.decimation;
            let source: &[f32] = match b.group {
                0 => &t1[..n],
                1 => &t0[..n],
                _ => &input[..n],
            };
            let mode = if i == 0 {
                CrossMode::LowPass
            } else if i == last {
                CrossMode::HighPass
            } else {
                CrossMode::BandPassNormalized
            };
            let mut buf = std::mem::take(&mut b.samples);
            b.svf[0].process(mode, 0, source, &mut buf[..n]);
            let copy: Vec<f32> = buf[..n].to_vec();
            b.svf[1].process(mode, 1, &copy, &mut buf[..n]);
            for v in buf[..n].iter_mut() {
                *v *= b.post_gain;
            }
            b.samples = buf;
        }
    }

    pub fn band_samples(&mut self, i: usize, n: usize) -> &mut [f32] {
        &mut self.bands[i].samples[..n]
    }

    /// Puts the bands back together into `out`.
    pub fn synthesize(&mut self, out: &mut [f32]) {
        let size = out.len();
        let [t0, t1] = &mut self.tmp;
        let d0 = self.bands[0].decimation;
        t1[..size / d0].fill(0.0);
        let groups: Vec<usize> = self.bands.iter().map(|b| b.group).collect();
        for i in 0..self.bands.len() {
            let b = &mut self.bands[i];
            let n = size / b.decimation;
            let target: &mut [f32] = match b.group {
                0 => &mut t1[..n],
                1 => &mut t0[..n],
                _ => &mut out[..n],
            };
            for j in 0..n {
                target[j] += b.delay_line.read_write(b.samples[j]);
            }
            let next_group = groups.get(i + 1).copied().unwrap_or(usize::MAX);
            if next_group != b.group {
                if b.group == 0 {
                    self.low_up.process(&t1[..n], &mut t0[..n * LOW_FACTOR]);
                } else if b.group == 1 {
                    self.mid_up.process(&t0[..n], &mut out[..n * MID_FACTOR]);
                }
            }
        }
    }
}

const FOLLOWER_GAIN: f32 = 4.472_136; // sqrt(20)

#[derive(Clone, Copy)]
struct Follower {
    attack: f32,
    decay: f32,
    envelope: f32,
    peak: f32,
    freeze: bool,
}

impl Follower {
    fn new() -> Self {
        Self { attack: 0.1, decay: 0.1, envelope: 0.0, peak: 0.0, freeze: false }
    }

    fn process(&mut self, input: &[f32], out: &mut [f32]) {
        let mut env = self.envelope;
        let (attack, decay) = if self.freeze { (0.0, 0.0) } else { (self.attack, self.decay) };
        let mut peak = 0.0f32;
        for (o, &x) in out.iter_mut().zip(input.iter()) {
            let error = (x * FOLLOWER_GAIN).abs() - env;
            env += if error > 0.0 { attack } else { decay } * error;
            peak = peak.max(env);
            *o = env;
        }
        self.envelope = env;
        let error = peak - self.peak;
        self.peak += if error > 0.0 { 0.5 } else { 0.1 } * error;
    }
}

struct Limiter {
    peak: f32,
}

impl Limiter {
    fn process(&mut self, buf: &mut [f32], pre_gain: f32) {
        for x in buf.iter_mut() {
            let s = *x * pre_gain;
            let error = s.abs() - self.peak;
            self.peak += if error > 0.0 { 0.05 } else { 0.00002 } * error;
            let gain = if self.peak <= 1.0 { 1.0 } else { 1.0 / self.peak };
            *x = soft_limit(s * gain * 0.8);
        }
    }
}

#[derive(Clone, Copy, Default)]
struct BandGain {
    carrier: f32,
    vocoder: f32,
}

pub struct Vocoder {
    modulator_bank: FilterBank,
    carrier_bank: FilterBank,
    limiter: Limiter,
    release_time: f32,
    formant_shift: f32,
    previous_gain: [BandGain; NUM_BANDS],
    gain: [BandGain; NUM_BANDS],
    followers: [Follower; NUM_BANDS],
    tmp: Vec<f32>,
}

impl Vocoder {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            modulator_bank: FilterBank::new(sample_rate),
            carrier_bank: FilterBank::new(sample_rate),
            limiter: Limiter { peak: 0.5 },
            release_time: 0.5,
            formant_shift: 0.5,
            previous_gain: [BandGain::default(); NUM_BANDS],
            gain: [BandGain::default(); NUM_BANDS],
            followers: [Follower::new(); NUM_BANDS],
            tmp: vec![0.0; MAX_BLOCK],
        }
    }

    pub fn set_release_time(&mut self, t: f32) {
        self.release_time = t;
    }

    pub fn set_formant_shift(&mut self, s: f32) {
        self.formant_shift = s;
    }

    pub fn process(&mut self, modulator: &[f32], carrier: &[f32], out: &mut [f32]) {
        let size = out.len();
        self.modulator_bank.analyze(modulator);
        self.carrier_bank.analyze(carrier);
        let mut f = 80.0 * semitones_to_ratio(-72.0 * self.release_time);
        for i in 0..NUM_BANDS {
            let decay = f / self.modulator_bank.band_sample_rate(i);
            self.followers[i].attack = decay * 2.0;
            self.followers[i].decay = decay * 0.5;
            self.followers[i].freeze = self.release_time > 0.995;
            f *= 1.2599;
        }
        let mut amount = 2.0 * (self.formant_shift - 0.5).abs();
        amount *= 2.0 - amount;
        amount *= 2.0 - amount;
        let envelope_increment = 4.0 * semitones_to_ratio(-48.0 * self.formant_shift);
        let mut envelope = 0.0f32;
        let last_band = NUM_BANDS as f32 - 1.0001;
        for i in 0..NUM_BANDS {
            let source = envelope.clamp(0.0, last_band);
            let si = source as usize;
            let sf = source - si as f32;
            let a = self.followers[si].peak;
            let b = self.followers[si + 1].peak;
            let mut band_gain = a + (b - a) * sf;
            let attenuation = envelope - last_band;
            if attenuation >= 0.0 {
                band_gain *= 1.0 / (1.0 + attenuation);
            }
            envelope += envelope_increment;
            self.gain[i] = BandGain { carrier: band_gain * amount, vocoder: 1.0 - amount };
        }
        for i in 0..NUM_BANDS {
            let n = size / self.modulator_bank.band_decimation(i);
            let step = 1.0 / n as f32;
            let m: Vec<f32> = self.modulator_bank.band_samples(i, n).to_vec();
            self.followers[i].process(&m, &mut self.tmp[..n]);
            let (mut vg, mut cg) = (self.previous_gain[i].vocoder, self.previous_gain[i].carrier);
            let vi = (self.gain[i].vocoder - vg) * step;
            let ci = (self.gain[i].carrier - cg) * step;
            let env = &self.tmp;
            for (j, c) in self.carrier_bank.band_samples(i, n).iter_mut().enumerate() {
                *c *= cg + vg * env[j];
                vg += vi;
                cg += ci;
            }
            self.previous_gain[i] = self.gain[i];
        }
        self.carrier_bank.synthesize(out);
        self.limiter.process(out, 1.4);
    }
}
