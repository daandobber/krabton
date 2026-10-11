//! Bassline: a TB-303 style monophonic bass synth, a Rust port of Open303
//! by Robin Schmidt (rosic::Open303).
//!
//! The voice follows Open303's DSP closely: a band-limited saw/square blend
//! oscillator reading mip-mapped wavetables (built by FFT the way Open303
//! does, with its tanh-shaped "303 square"), the TeeBeeFilter in its TB_303
//! mode with a high-pass inside the feedback loop, the main filter envelope
//! smoothed by leaky integrators for the normal and the accent path, the
//! analog-style amp envelope, the high-pass before the filter and the
//! all-pass, high-pass and notch after it. Oscillator and filter run at 4x
//! the host rate and are band-limited by Open303's 12th order elliptic
//! quarter-band filter. Slide and accent work as in rosic::Open303: a note
//! that starts while another key is held slides there without restarting
//! the envelopes, and a note is accented when its velocity reaches 100.
//!
//! Krabton changes:
//! - A fixed-size held-note stack (last note priority; releasing the newest
//!   key slides back to the newest key still held), so the audio thread
//!   never allocates. The slide back uses the current tuning (Open303 used
//!   A4 = 440 Hz there).
//! - "Accent At": the accent velocity threshold is a parameter (default 100).
//! - "Drive": a soft tanh saturation after the filter, inside the
//!   oversampled loop. Not in Open303; 0 (the default) leaves the signal
//!   untouched.
//! - Tuning is in cents from A4 = 440 Hz (Open303: A4 in Hz).
//! - The voice goes idle once its release has died out (Open303 has this
//!   check commented out). A note started from idle resets the oscillator
//!   phase and the filters, as Open303's triggerNote intends.
//!
//! Open303's license:
//!
//! The MIT License
//!
//! Copyright (c) 2009 Robin Schmidt (www.rs-met.com)
//!
//! Permission is hereby granted, free of charge, to any person obtaining a
//! copy of this software and associated documentation files (the
//! "Software"), to deal in the Software without restriction, including
//! without limitation the rights to use, copy, modify, merge, publish,
//! distribute, sublicense, and/or sell copies of the Software, and to
//! permit persons to whom the Software is furnished to do so, subject to
//! the following conditions:
//!
//! The above copyright notice and this permission notice shall be included
//! in all copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
//! OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
//! MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
//! IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
//! CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
//! TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
//! SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use crate::params::{ParamSpec, Unit};
use std::f64::consts::{FRAC_1_SQRT_2, PI};
use std::sync::OnceLock;

/// Oscillator and filter run at this multiple of the host rate.
const OVERSAMPLING: usize = 4;
const TABLE_LEN: usize = 2048;
const NUM_TABLES: usize = 12;
/// Each table carries 4 extra samples (copies of its start) so the
/// interpolator never has to wrap.
const STRIDE: usize = TABLE_LEN + 4;
/// FLT_MIN, which rosic adds in its recursive filters against denormals.
const TINY: f64 = f32::MIN_POSITIVE as f64;
/// Every MIDI key fits on the note stack.
const STACK: usize = 128;
/// Open303's fixed amp release for accented notes (ms).
const ACCENT_AMP_RELEASE: f64 = 50.0;
/// The amp level (after the de-clicker) below which a released voice idles.
const IDLE_LEVEL: f64 = 1e-6;

pub const WAVEFORM: usize = 0;
pub const TUNING: usize = 1;
pub const CUTOFF: usize = 2;
pub const RESONANCE: usize = 3;
pub const ENV_MOD: usize = 4;
pub const DECAY: usize = 5;
pub const ACCENT: usize = 6;
pub const VOLUME: usize = 7;
pub const SLIDE_TIME: usize = 8;
pub const ACCENT_AT: usize = 9;
pub const NORMAL_ATTACK: usize = 10;
pub const ACCENT_ATTACK: usize = 11;
pub const ACCENT_DECAY: usize = 12;
pub const AMP_DECAY: usize = 13;
pub const AMP_SUSTAIN: usize = 14;
pub const AMP_RELEASE: usize = 15;
pub const PRE_HPF: usize = 16;
pub const FEEDBACK_HPF: usize = 17;
pub const POST_HPF: usize = 18;
pub const SQUARE_PHASE: usize = 19;
pub const DRIVE: usize = 20;

// Defaults are Open303's: the plugin's init patch for the eight front-panel
// knobs, the core's member defaults for the rest. (Open303's constructor
// sets the slide and attack smoothers directly to 60, 0 and 15 ms instead
// of going through its own setters; we go through the setters.)
pub static SPECS: &[ParamSpec] = &[
    ParamSpec::lin("Waveform", "Oscillator", 0.0, 1.0, 0.85, Unit::Percent),
    ParamSpec::lin("Tuning", "Oscillator", -1200.0, 1200.0, 0.0, Unit::Cents),
    ParamSpec::log("Cutoff", "Filter", 314.0, 2394.0, 500.0, Unit::Hz),
    ParamSpec::lin("Resonance", "Filter", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Env Mod", "Filter", 0.0, 1.0, 0.25, Unit::Percent),
    ParamSpec::log("Decay", "Filter", 30.0, 3000.0, 400.0, Unit::Ms),
    ParamSpec::lin("Accent", "Accent", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::lin("Volume", "Output", -48.0, 6.0, -6.0, Unit::Db),
    ParamSpec::log("Slide Time", "Oscillator", 1.0, 1000.0, 60.0, Unit::Ms),
    ParamSpec::int("Accent At", "Accent", 1.0, 127.0, 100.0, Unit::None),
    ParamSpec::lin("Normal Attack", "Filter", 0.0, 30.0, 3.0, Unit::Ms),
    ParamSpec::lin("Accent Attack", "Accent", 0.0, 30.0, 3.0, Unit::Ms),
    ParamSpec::log("Accent Decay", "Accent", 30.0, 3000.0, 200.0, Unit::Ms),
    ParamSpec::log("Amp Decay", "Amp Env", 10.0, 5000.0, 1230.0, Unit::Ms),
    ParamSpec::lin("Amp Sustain", "Amp Env", -60.0, 0.0, -60.0, Unit::Db),
    ParamSpec::log("Amp Release", "Amp Env", 0.5, 2000.0, 1.0, Unit::Ms),
    ParamSpec::log("Pre HPF", "Highpass", 10.0, 500.0, 44.486, Unit::Hz),
    ParamSpec::log("Feedback HPF", "Highpass", 10.0, 500.0, 150.0, Unit::Hz),
    ParamSpec::log("Post HPF", "Highpass", 10.0, 500.0, 24.167, Unit::Hz),
    ParamSpec::int("Square Phase", "Oscillator", 0.0, 360.0, 180.0, Unit::None),
    // Krabton addition, not part of Open303.
    ParamSpec::lin("Drive", "Output", 0.0, 1.0, 0.0, Unit::Percent),
];

pub fn preset_names() -> &'static [&'static str] {
    &["Classic", "Squelch", "Rubber", "Screamer", "Square Bass", "Deep Sub", "Talking", "Hard Acid"]
}

pub fn preset(name: &str) -> Vec<f32> {
    let mut p = crate::params::defaults(SPECS);
    let mut set = |pairs: &[(usize, f32)]| {
        for &(i, v) in pairs {
            p[i] = v;
        }
    };
    match name {
        // "Classic" is Open303's init patch.
        "Squelch" => set(&[(WAVEFORM, 0.0), (CUTOFF, 420.0), (RESONANCE, 0.88), (ENV_MOD, 0.7), (DECAY, 550.0), (ACCENT, 0.75), (VOLUME, -8.0)]),
        "Rubber" => set(&[(WAVEFORM, 0.35), (CUTOFF, 330.0), (RESONANCE, 0.62), (ENV_MOD, 0.45), (DECAY, 1100.0), (ACCENT, 0.4), (SLIDE_TIME, 160.0), (AMP_DECAY, 2500.0), (VOLUME, -6.0)]),
        "Screamer" => set(&[(WAVEFORM, 0.0), (CUTOFF, 800.0), (RESONANCE, 0.96), (ENV_MOD, 0.85), (DECAY, 320.0), (ACCENT, 1.0), (ACCENT_DECAY, 160.0), (FEEDBACK_HPF, 120.0), (DRIVE, 0.7), (VOLUME, -10.0)]),
        "Square Bass" => set(&[(WAVEFORM, 1.0), (CUTOFF, 650.0), (RESONANCE, 0.3), (ENV_MOD, 0.35), (DECAY, 280.0), (ACCENT, 0.4), (AMP_DECAY, 600.0), (VOLUME, -4.0)]),
        "Deep Sub" => set(&[(WAVEFORM, 0.6), (CUTOFF, 314.0), (RESONANCE, 0.15), (ENV_MOD, 0.1), (DECAY, 1500.0), (ACCENT, 0.25), (AMP_SUSTAIN, -4.0), (AMP_RELEASE, 40.0), (PRE_HPF, 12.0), (FEEDBACK_HPF, 60.0), (POST_HPF, 10.0), (VOLUME, -2.0)]),
        "Talking" => set(&[(WAVEFORM, 0.0), (CUTOFF, 520.0), (RESONANCE, 0.92), (ENV_MOD, 0.55), (DECAY, 1800.0), (ACCENT, 0.85), (ACCENT_ATTACK, 12.0), (ACCENT_DECAY, 500.0), (SLIDE_TIME, 110.0), (VOLUME, -8.0)]),
        "Hard Acid" => set(&[(WAVEFORM, 0.0), (CUTOFF, 1100.0), (RESONANCE, 0.82), (ENV_MOD, 1.0), (DECAY, 220.0), (ACCENT, 1.0), (ACCENT_DECAY, 110.0), (DRIVE, 0.35), (VOLUME, -10.0)]),
        _ => {}
    }
    p
}

#[inline]
fn db_to_amp(db: f64) -> f64 {
    (db * 0.115_129_254_649_702_28).exp()
}

/// MIDI note (plus cents) to Hz around A4 = 440 Hz.
#[inline]
fn note_to_freq(note: f64, cents: f64) -> f64 {
    440.0 * ((note - 69.0) / 12.0 + cents / 1200.0).exp2()
}

/// The binary exponent of a double (rosic's EXPOFDBL), which the oscillator
/// uses to pick a mip-map level.
#[inline]
fn exponent(x: f64) -> i32 {
    ((x.to_bits() >> 52) & 0x7FF) as i32 - 1023
}

// ---------------------------------------------------------------------------
// Wavetables (rosic::MipMappedWaveTable)

/// Mip-mapped saw and square, shared by all instances. Level 0 is the raw
/// prototype, level t keeps the harmonics below TABLE_LEN / 2^(t+1).
struct WaveTables {
    saw: Vec<f32>,
    /// Without Open303's phase shift; the oscillator applies it on read.
    square: Vec<f32>,
}

fn tables() -> &'static WaveTables {
    static T: OnceLock<WaveTables> = OnceLock::new();
    T.get_or_init(|| {
        let saw = saw303();
        // Open303's internal shaper settings: 36.9 dB drive, 4.37 offset.
        let drive = db_to_amp(36.9);
        let square: Vec<f64> = saw.iter().map(|&s| -(drive * s + 4.37).tanh()).collect();
        WaveTables { saw: mip_map(&saw), square: mip_map(&square) }
    })
}

/// Open303's saw: rises from 0 to 1 over the first half, jumps to -1 and
/// rises back to 0.
fn saw303() -> Vec<f64> {
    let n = TABLE_LEN;
    let n1 = ((0.5 * (n - 1) as f64 + 0.5).floor() as usize).clamp(1, n - 1);
    let (s1, s2) = (1.0 / (n1 - 1) as f64, 1.0 / (n - n1) as f64);
    (0..n).map(|i| if i < n1 { s1 * i as f64 } else { -1.0 + s2 * (i - n1) as f64 }).collect()
}

/// In-place radix-2 FFT, unscaled.
fn fft(re: &mut [f64], im: &mut [f64], inverse: bool) {
    let n = re.len();
    let bits = n.trailing_zeros();
    for i in 0..n {
        let j = i.reverse_bits() >> (usize::BITS - bits);
        if j > i {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let sign = if inverse { 1.0 } else { -1.0 };
    let mut size = 2;
    while size <= n {
        let half = size / 2;
        for k in 0..half {
            let (wi, wr) = (sign * 2.0 * PI * k as f64 / size as f64).sin_cos();
            for start in (0..n).step_by(size) {
                let (a, b) = (start + k, start + k + half);
                let tr = re[b] * wr - im[b] * wi;
                let ti = re[b] * wi + im[b] * wr;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
            }
        }
        size *= 2;
    }
}

/// Renders the band-limited levels the way MipMappedWaveTable does: take the
/// spectrum, clear DC and Nyquist, and drop one more octave per level.
fn mip_map(proto: &[f64]) -> Vec<f32> {
    let n = TABLE_LEN;
    let mut out = vec![0.0f32; NUM_TABLES * STRIDE];
    let mut store = |t: usize, x: &[f64]| {
        let table = &mut out[t * STRIDE..(t + 1) * STRIDE];
        for i in 0..STRIDE {
            table[i] = x[i % n] as f32;
        }
    };
    store(0, proto);
    let (mut re, mut im) = (proto.to_vec(), vec![0.0; n]);
    fft(&mut re, &mut im, false);
    let (mut lre, mut lim) = (vec![0.0; n], vec![0.0; n]);
    for t in 1..NUM_TABLES {
        let keep = n >> (t + 1);
        lre.fill(0.0);
        lim.fill(0.0);
        for k in 1..keep {
            lre[k] = re[k];
            lim[k] = im[k];
            lre[n - k] = re[n - k];
            lim[n - k] = im[n - k];
        }
        fft(&mut lre, &mut lim, true);
        for v in lre.iter_mut() {
            *v /= n as f64;
        }
        store(t, &lre);
    }
    out
}

// ---------------------------------------------------------------------------
// Building blocks

/// rosic::OnePoleFilter in high-pass or all-pass mode.
#[derive(Clone, Copy, Default)]
struct OnePole {
    b0: f64,
    b1: f64,
    a1: f64,
    x1: f64,
    y1: f64,
}

impl OnePole {
    fn set_highpass(&mut self, cutoff: f64, sr: f64) {
        let fc = if cutoff > 0.0 && cutoff <= 20000.0 { cutoff } else { 20000.0 };
        let x = (-2.0 * PI * fc / sr).exp();
        self.b0 = 0.5 * (1.0 + x);
        self.b1 = -self.b0;
        self.a1 = x;
    }

    fn set_allpass(&mut self, cutoff: f64, sr: f64) {
        let t = (PI * cutoff / sr).tan();
        let x = (t - 1.0) / (t + 1.0);
        self.b0 = x;
        self.b1 = 1.0;
        self.a1 = -x;
    }

    fn reset(&mut self) {
        self.x1 = 0.0;
        self.y1 = 0.0;
    }

    #[inline]
    fn tick(&mut self, x: f64) -> f64 {
        self.y1 = self.b0 * x + self.b1 * self.x1 + self.a1 * self.y1 + TINY;
        self.x1 = x;
        self.y1
    }
}

/// rosic::BiquadFilter (RBJ cookbook) in low-pass and band-reject mode.
#[derive(Clone, Copy, Default)]
struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Biquad {
    fn set_lowpass(&mut self, freq: f64, q: f64, sr: f64) {
        let (s, c) = (2.0 * PI * freq / sr).sin_cos();
        let alpha = s / (2.0 * q);
        let scale = 1.0 / (1.0 + alpha);
        self.a1 = 2.0 * c * scale;
        self.a2 = (alpha - 1.0) * scale;
        self.b1 = (1.0 - c) * scale;
        self.b0 = 0.5 * self.b1;
        self.b2 = self.b0;
    }

    /// `bandwidth` in octaves.
    fn set_band_reject(&mut self, freq: f64, bandwidth: f64, sr: f64) {
        let w = 2.0 * PI * freq / sr;
        let (s, c) = w.sin_cos();
        let alpha = s * (0.5 * 2f64.ln() * bandwidth * w / s).sinh();
        let scale = 1.0 / (1.0 + alpha);
        self.a1 = 2.0 * c * scale;
        self.a2 = (alpha - 1.0) * scale;
        self.b0 = scale;
        self.b1 = -2.0 * c * scale;
        self.b2 = scale;
    }

    fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }

    #[inline]
    fn tick(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2 + self.a1 * self.y1 + self.a2 * self.y2 + TINY;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// rosic::LeakyIntegrator: an RC low-pass with its time constant in ms.
#[derive(Clone, Copy, Default)]
struct Leaky {
    coeff: f64,
    y1: f64,
}

impl Leaky {
    fn set_tau(&mut self, tau_ms: f64, sr: f64) {
        self.coeff = if tau_ms > 0.0 { (-1.0 / (sr * 0.001 * tau_ms)).exp() } else { 0.0 };
    }

    #[inline]
    fn tick(&mut self, x: f64) -> f64 {
        self.y1 = x + self.coeff * (self.y1 - x);
        self.y1
    }
}

/// rosic::DecayEnvelope: a pure exponential decay that starts at 1.
#[derive(Clone, Copy)]
struct DecayEnv {
    c: f64,
    y: f64,
    y_init: f64,
}

impl DecayEnv {
    fn new(tau_ms: f64, sr: f64) -> Self {
        let mut e = Self { c: 1.0, y: 1.0, y_init: 1.0 };
        e.set_tau(tau_ms, sr);
        e
    }

    fn set_tau(&mut self, tau_ms: f64, sr: f64) {
        if tau_ms > 0.001 {
            self.c = (-1.0 / (0.001 * tau_ms * sr)).exp();
            self.y_init = 1.0 / self.c;
        }
    }

    fn trigger(&mut self) {
        self.y = self.y_init;
    }

    #[inline]
    fn tick(&mut self) -> f64 {
        self.y *= self.c;
        // Flush long-dead tails before they turn denormal.
        if self.y < 1e-30 {
            self.y = 0.0;
        }
        self.y
    }
}

/// rosic::AnalogEnvelope as Open303 sets it up: instant attack to 1, an RC
/// decay towards the sustain level and an RC release. Times are time
/// constants in ms.
#[derive(Clone, Copy)]
struct AmpEnv {
    sr: f64,
    /// Elapsed ms since the note started (stops counting in sustain).
    time: f64,
    /// ms per sample.
    inc: f64,
    decay_ms: f64,
    decay_coeff: f64,
    release_coeff: f64,
    sustain: f64,
    level: f64,
    note_on: bool,
}

impl AmpEnv {
    fn new(sr: f64) -> Self {
        let mut e = Self { sr, time: 0.0, inc: 1000.0 / sr, decay_ms: 0.0, decay_coeff: 1.0, release_coeff: 1.0, sustain: 0.0, level: 0.0, note_on: false };
        e.set_decay(1230.0);
        e.set_release(1.0);
        e
    }

    fn coeff(&self, ms: f64) -> f64 {
        if ms > 0.0 {
            1.0 - (-1.0 / (self.sr * 0.001 * ms)).exp()
        } else {
            1.0
        }
    }

    fn set_decay(&mut self, ms: f64) {
        self.decay_ms = ms.max(0.0);
        self.decay_coeff = self.coeff(ms);
    }

    fn set_release(&mut self, ms: f64) {
        self.release_coeff = self.coeff(ms);
    }

    /// Open303 always starts from the current level.
    fn start(&mut self) {
        self.time = 0.0;
        self.note_on = true;
    }

    fn release(&mut self) {
        self.note_on = false;
        // Skip ahead to the release phase.
        self.time = self.decay_ms + self.inc;
    }

    #[inline]
    fn tick(&mut self) -> f64 {
        if self.time <= 0.0 {
            // Zero attack: jump to the peak.
            self.level = 1.0;
            self.time += self.inc;
        } else if self.time <= self.decay_ms {
            self.level += self.decay_coeff * (self.sustain - self.level);
            self.time += self.inc;
        } else if self.note_on {
            self.level += self.decay_coeff * (self.sustain - self.level);
        } else {
            self.level -= self.release_coeff * self.level;
            self.time += self.inc;
        }
        self.level
    }
}

/// rosic::TeeBeeFilter in TB_303 mode (after mystran and kunn): four
/// coupled one-pole stages with a high-pass in the feedback path, which
/// takes resonance away at low cutoffs like the 303 does.
#[derive(Clone, Copy)]
struct TeeBee {
    y1: f64,
    y2: f64,
    y3: f64,
    y4: f64,
    b0: f64,
    k: f64,
    g: f64,
    cutoff: f64,
    /// Resonance mapped to feel more musical.
    r: f64,
    two_pi_over_sr: f64,
    feedback_hp: OnePole,
}

impl TeeBee {
    fn new(sr: f64) -> Self {
        let mut f = Self { y1: 0.0, y2: 0.0, y3: 0.0, y4: 0.0, b0: 0.0, k: 0.0, g: 1.0, cutoff: 1000.0, r: 0.0, two_pi_over_sr: 2.0 * PI / sr, feedback_hp: OnePole::default() };
        f.feedback_hp.set_highpass(150.0, sr);
        f.update();
        f
    }

    /// `res` from 0 to 1.
    fn set_resonance(&mut self, res: f64) {
        let r = (1.0 - (-3.0 * res).exp()) / (1.0 - (-3.0f64).exp());
        if r != self.r {
            self.r = r;
            self.update();
        }
    }

    #[inline]
    fn set_cutoff(&mut self, fc: f64) {
        if fc != self.cutoff {
            // Open303's absolute floor and ceiling.
            self.cutoff = fc.clamp(200.0, 20000.0);
            self.update();
        }
    }

    #[inline]
    fn update(&mut self) {
        let wc = self.two_pi_over_sr * self.cutoff;
        let fx = wc * FRAC_1_SQRT_2 / (2.0 * PI);
        self.b0 = (0.00045522346 + 6.1922189 * fx) / (1.0 + 12.358354 * fx + 4.4156345 * (fx * fx));
        let k = fx * (fx * (fx * (fx * (fx * (fx + 7198.6997) - 5837.7917) - 476.47308) + 614.95611) + 213.87126) + 16.998792;
        let g = (k / 17.0 - 1.0) * self.r + 1.0;
        self.g = g * (1.0 + self.r);
        self.k = k * self.r;
    }

    fn reset(&mut self) {
        self.feedback_hp.reset();
        self.y1 = 0.0;
        self.y2 = 0.0;
        self.y3 = 0.0;
        self.y4 = 0.0;
    }

    #[inline]
    fn tick(&mut self, x: f64) -> f64 {
        let y0 = x - self.feedback_hp.tick(self.k * self.y4);
        self.y1 += 2.0 * self.b0 * (y0 - self.y1 + self.y2);
        self.y2 += self.b0 * (self.y1 - 2.0 * self.y2 + self.y3);
        self.y3 += self.b0 * (self.y2 - 2.0 * self.y3 + self.y4);
        self.y4 += self.b0 * (self.y3 - 2.0 * self.y4);
        2.0 * self.g * self.y4
    }
}

/// rosic::EllipticQuarterBandFilter: 12th order elliptic low-pass at a
/// quarter of the (oversampled) rate, direct form II.
#[derive(Clone, Copy, Default)]
struct QuarterBand {
    w: [f64; 12],
}

impl QuarterBand {
    const A: [f64; 12] = [
        -9.1891604652189471,
        40.177553696870497,
        -110.11636661771178,
        210.18506612078195,
        -293.84744771903240,
        308.16345558359234,
        -244.06786780384243,
        144.81877911392738,
        -62.770692151724198,
        18.867762095902137,
        -3.5327094230551848,
        0.31183189275203149,
    ];
    const B0: f64 = 0.00013671732099945628;
    const B: [f64; 12] = [
        -0.00055538501265606384,
        0.0013681887636296387,
        -0.0022158566490711852,
        0.0028320091007278322,
        -0.0029776933151090413,
        0.0030283628243514991,
        -0.0029776933151090413,
        0.0028320091007278331,
        -0.0022158566490711861,
        0.0013681887636296393,
        -0.00055538501265606384,
        0.00013671732099945636,
    ];

    fn reset(&mut self) {
        self.w = [0.0; 12];
    }

    #[inline]
    fn tick(&mut self, x: f64) -> f64 {
        let w = &self.w;
        let mut fb = 0.0;
        let mut ff = 0.0;
        for i in 0..12 {
            fb += Self::A[i] * w[i];
            ff += Self::B[i] * w[i];
        }
        let tmp = x + TINY - fb;
        let y = Self::B0 * tmp + ff;
        self.w.copy_within(0..11, 1);
        self.w[0] = tmp;
        y
    }
}

// ---------------------------------------------------------------------------
// The voice

pub struct BasslineState {
    sr: f64,
    tables: &'static WaveTables,
    /// Read position in the wavetables, 0..TABLE_LEN.
    phase: f64,
    /// Held keys, oldest first.
    held: [u8; STACK],
    held_len: usize,
    /// The note the oscillator heads for (kept after release for the tail).
    note: f64,
    /// Open303's oscFreq: the target of the slide.
    osc_freq: f64,
    /// The last frequency the oscillator accepted.
    freq: f64,
    slew: Leaky,
    main_env: DecayEnv,
    rc1: Leaky,
    rc2: Leaky,
    amp_env: AmpEnv,
    declick: Biquad,
    hp1: OnePole,
    hp2: OnePole,
    allpass: OnePole,
    notch: Biquad,
    filter: TeeBee,
    aaf: QuarterBand,
    accent_gain: f64,
    accented: bool,
    idle: bool,
    // Block-rate settings.
    tuning: f64,
    blend: f64,
    cutoff: f64,
    env_scaler: f64,
    env_offset: f64,
    amp_scaler: f64,
    sq_offset: usize,
    drive_gain: f64,
    drive_makeup: f64,
    /// For the display: the filter's cutoff and the amp envelope now.
    vis_cutoff: f64,
    vis_amp: f64,
}

impl BasslineState {
    pub fn new(sr: f32) -> Self {
        let sr = (sr as f64).max(1000.0);
        let os = sr * OVERSAMPLING as f64;
        let mut s = Self {
            sr,
            tables: tables(),
            phase: 0.0,
            held: [0; STACK],
            held_len: 0,
            note: 69.0,
            osc_freq: 440.0,
            freq: 440.0,
            slew: Leaky::default(),
            main_env: DecayEnv::new(200.0, sr),
            rc1: Leaky::default(),
            rc2: Leaky::default(),
            amp_env: AmpEnv::new(sr),
            declick: Biquad::default(),
            hp1: OnePole::default(),
            hp2: OnePole::default(),
            allpass: OnePole::default(),
            notch: Biquad::default(),
            filter: TeeBee::new(os),
            aaf: QuarterBand::default(),
            accent_gain: 0.0,
            accented: false,
            idle: true,
            tuning: 0.0,
            blend: 0.0,
            cutoff: 1000.0,
            env_scaler: 1.0,
            env_offset: 0.0,
            amp_scaler: 1.0,
            sq_offset: 0,
            drive_gain: 1.0,
            drive_makeup: 1.0,
            vis_cutoff: 500.0,
            vis_amp: 0.0,
        };
        // Open303's fixed tweakables.
        s.declick.set_lowpass(200.0, FRAC_1_SQRT_2, sr);
        s.allpass.set_allpass(14.008, sr);
        s.notch.set_band_reject(7.5164, 4.7, sr);
        s.update(&crate::params::defaults(SPECS));
        s
    }

    /// False once the voice has gone quiet after its release.
    pub fn is_active(&self) -> bool {
        !self.idle
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        if p.len() < SPECS.len() {
            return;
        }
        // Velocity zero is a note-off, as in MIDI (and Open303).
        if vel <= 0.0 || vel.is_nan() {
            self.note_off(p, note);
            return;
        }
        let accent = (vel.min(1.0) * 127.0).round() >= p[ACCENT_AT].round();
        if self.held_len == 0 {
            self.trigger(p, note, accent);
        } else {
            self.slide(p, note, accent);
        }
        self.remove(note);
        if self.held_len < STACK {
            self.held[self.held_len] = note;
            self.held_len += 1;
        }
        self.idle = false;
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        if p.len() < SPECS.len() {
            return;
        }
        self.remove(note);
        if self.held_len == 0 {
            self.amp_env.release();
        } else {
            // Slide back to the newest key still held, keeping the accent.
            self.note = self.held[self.held_len - 1] as f64;
            self.osc_freq = note_to_freq(self.note, p[TUNING] as f64);
        }
    }

    pub fn all_off(&mut self) {
        self.held_len = 0;
        self.amp_env.release();
    }

    /// What the display shows: [0] the filter's cutoff now (Hz), [1] the
    /// amp envelope, [2] the pitch (MIDI note, gliding), [3] accent, [4]
    /// sliding, [5] keys held, [6] sounding.
    pub fn visual(&self, out: &mut [f32; crate::engine::INST_VALS]) {
        out[0] = self.vis_cutoff as f32;
        out[1] = if self.idle { 0.0 } else { self.vis_amp as f32 };
        out[2] = (69.0 + 12.0 * (self.freq / 440.0).log2() - self.tuning / 100.0) as f32;
        out[3] = self.accented as u8 as f32;
        out[4] = ((self.osc_freq / self.freq.max(1.0)).log2().abs() > 0.004) as u8 as f32;
        out[5] = self.held_len as f32;
        out[6] = !self.idle as u8 as f32;
    }

    /// Adds the voice into `l` and `r` (mono to both).
    pub fn render(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() || self.idle {
            return;
        }
        self.update(p);
        for (a, b) in l.iter_mut().zip(r.iter_mut()) {
            if self.idle {
                break;
            }
            let y = self.tick() as f32;
            *a += y;
            *b += y;
        }
    }

    fn remove(&mut self, note: u8) {
        let mut n = 0;
        for i in 0..self.held_len {
            if self.held[i] != note {
                self.held[n] = self.held[i];
                n += 1;
            }
        }
        self.held_len = n;
    }

    /// Open303's triggerNote: a new note with fresh envelopes.
    fn trigger(&mut self, p: &[f32], note: u8, accent: bool) {
        // Only reset the oscillator and filters when silent, to avoid clicks.
        if self.idle {
            self.phase = 0.0;
            self.filter.reset();
            self.hp1.reset();
            self.hp2.reset();
            self.allpass.reset();
            self.notch.reset();
            self.aaf.reset();
            self.declick.reset();
        }
        self.set_accent(p, accent);
        self.note = note as f64;
        self.osc_freq = note_to_freq(self.note, p[TUNING] as f64);
        self.slew.y1 = self.osc_freq;
        self.main_env.trigger();
        self.amp_env.start();
        self.idle = false;
    }

    /// Open303's slideToNote: only the pitch target and the accent change.
    fn slide(&mut self, p: &[f32], note: u8, accent: bool) {
        self.note = note as f64;
        self.osc_freq = note_to_freq(self.note, p[TUNING] as f64);
        self.set_accent(p, accent);
    }

    fn set_accent(&mut self, p: &[f32], accent: bool) {
        self.accented = accent;
        if accent {
            self.accent_gain = p[ACCENT].clamp(0.0, 1.0) as f64;
            self.main_env.set_tau(p[ACCENT_DECAY] as f64, self.sr);
            self.amp_env.set_release(ACCENT_AMP_RELEASE);
        } else {
            self.accent_gain = 0.0;
            self.main_env.set_tau(p[DECAY] as f64, self.sr);
            self.amp_env.set_release(p[AMP_RELEASE] as f64);
        }
    }

    /// Picks up the knobs that act immediately in Open303. Decay, accent and
    /// their times are read when a note starts or slides, as in Open303.
    fn update(&mut self, p: &[f32]) {
        let v = |i: usize| p[i] as f64;
        let sr = self.sr;
        let os = sr * OVERSAMPLING as f64;
        self.tuning = v(TUNING);
        self.osc_freq = note_to_freq(self.note, self.tuning);
        self.blend = v(WAVEFORM).clamp(0.0, 1.0);
        self.cutoff = v(CUTOFF).max(1.0);
        (self.env_scaler, self.env_offset) = env_mod_mapping(self.cutoff, v(ENV_MOD).clamp(0.0, 1.0));
        self.filter.set_resonance(v(RESONANCE).clamp(0.0, 1.0));
        self.filter.feedback_hp.set_highpass(v(FEEDBACK_HPF), os);
        self.hp1.set_highpass(v(PRE_HPF), os);
        self.hp2.set_highpass(v(POST_HPF), sr);
        self.amp_scaler = db_to_amp(v(VOLUME));
        // Open303's setSlideTime: the slew time constant is a fifth of it.
        self.slew.set_tau(0.2 * v(SLIDE_TIME).max(0.0), sr);
        self.rc1.set_tau(v(NORMAL_ATTACK).max(0.0), sr);
        self.rc2.set_tau(v(ACCENT_ATTACK).max(0.0), sr);
        self.amp_env.set_decay(v(AMP_DECAY));
        self.amp_env.sustain = if v(AMP_SUSTAIN) <= -60.0 { 0.0 } else { db_to_amp(v(AMP_SUSTAIN)) };
        self.amp_env.set_release(if self.accented { ACCENT_AMP_RELEASE } else { v(AMP_RELEASE) });
        let shift = ((TABLE_LEN as f64 * v(SQUARE_PHASE).clamp(0.0, 360.0) / 360.0 + 0.5).floor() as usize) % TABLE_LEN;
        self.sq_offset = (TABLE_LEN - shift) % TABLE_LEN;
        // Drive: up to 30 dB into tanh, with partial make-up so the level
        // stays in the same ballpark.
        let d = v(DRIVE).clamp(0.0, 1.0);
        self.drive_gain = if d > 0.0 { db_to_amp(30.0 * d) } else { 0.0 };
        self.drive_makeup = self.drive_gain.max(1.0).powf(-0.3);
    }

    /// One saw/square sample (rosic::BlendOscillator) and a phase step.
    #[inline]
    fn osc(&mut self, inc: f64, level: usize) -> f64 {
        let t = self.tables;
        while self.phase >= TABLE_LEN as f64 {
            self.phase -= TABLE_LEN as f64;
        }
        let i = self.phase as usize;
        let frac = self.phase - i as f64;
        let base = level * STRIDE;
        let saw = &t.saw[base..base + STRIDE];
        let sq = &t.square[base..base + STRIDE];
        // The square's phase shift is a circular shift of its table.
        let j = (i + self.sq_offset) & (TABLE_LEN - 1);
        let out1 = (1.0 - self.blend) * ((1.0 - frac) * saw[i] as f64 + frac * saw[i + 1] as f64);
        let out2 = self.blend * ((1.0 - frac) * sq[j] as f64 + frac * sq[j + 1] as f64);
        self.phase += inc;
        // Open303 halves the square to match the saw's loudness.
        out1 + 0.5 * out2
    }

    /// Open303::getSample.
    #[inline]
    fn tick(&mut self) -> f64 {
        let inst = self.slew.tick(self.osc_freq);
        if inst > 0.0 && inst < 20000.0 {
            self.freq = inst;
        }
        let inc = TABLE_LEN as f64 * self.freq / (self.sr * OVERSAMPLING as f64);
        // Two levels above the increment's octave: harmonics stay below a
        // quarter of the oversampled rate.
        let level = (exponent(inc) + 2).clamp(0, NUM_TABLES as i32 - 1) as usize;

        // Cutoff from the main envelope, through the normal and accent RCs.
        let main = self.main_env.tick();
        let t1 = self.rc1.tick(main);
        let t2 = self.rc2.tick(if self.accent_gain > 0.0 { main } else { 0.0 });
        let t1 = self.env_scaler * (t1 - self.env_offset);
        let t2 = self.accent_gain * t2;
        self.vis_cutoff = self.cutoff * (t1 + t2).exp2();
        self.filter.set_cutoff(self.vis_cutoff);

        let mut amp = self.amp_env.tick();
        if self.amp_env.note_on {
            amp += 0.45 * main + self.accent_gain * 4.0 * main;
        }
        let amp = self.declick.tick(amp);
        self.vis_amp = amp;

        let mut x = 0.0;
        for _ in 0..OVERSAMPLING {
            let mut s = -self.osc(inc, level);
            s = self.hp1.tick(s);
            s = self.filter.tick(s);
            if self.drive_gain > 0.0 {
                // Krabton's Drive, band-limited by the oversampling.
                s = (self.drive_gain * s).tanh() * self.drive_makeup;
            }
            x = self.aaf.tick(s);
        }
        // These run at the host rate, as in Open303.
        x = self.allpass.tick(x);
        x = self.hp2.tick(x);
        x = self.notch.tick(x);
        let out = x * amp * self.amp_scaler;

        if !self.amp_env.note_on && self.amp_env.level < IDLE_LEVEL && amp.abs() < IDLE_LEVEL {
            self.idle = true;
        }
        out
    }
}

/// Open303's measured mapping from cutoff and env mod (0..1) to the scale
/// and offset of the normalized envelope, in octaves.
fn env_mod_mapping(cutoff: f64, env_mod: f64) -> (f64, f64) {
    const C0: f64 = 313.8152786059267; // lowest nominal cutoff
    const C1: f64 = 2394.411986817546; // highest nominal cutoff
    const OFFSET_F: f64 = 0.048292930943553;
    const OFFSET_C: f64 = 0.294391201442418;
    const LOW_F: f64 = 3.773996325111173;
    const LOW_C: f64 = 0.736965594166206;
    const HIGH_F: f64 = 4.194548788411135;
    const HIGH_C: f64 = 0.864344900642434;
    let c = (cutoff / C0).ln() / (C1 / C0).ln();
    let low = LOW_F * env_mod + LOW_C;
    let high = HIGH_F * env_mod + HIGH_C;
    ((1.0 - c) * low + c * high, OFFSET_F * c + OFFSET_C)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn render(st: &mut BasslineState, p: &[f32], secs: f32) -> Vec<f32> {
        let n = (secs * SR) as usize;
        let mut l = vec![0.0; n];
        let mut r = vec![0.0; n];
        for (cl, cr) in l.chunks_mut(64).zip(r.chunks_mut(64)) {
            st.render(p, cl, cr);
        }
        assert_eq!(l, r, "mono to both sides");
        l
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    /// High-frequency weight: the level of the first difference relative to
    /// the signal.
    fn brightness(x: &[f32]) -> f32 {
        let d: Vec<f32> = x.windows(2).map(|w| w[1] - w[0]).collect();
        rms(&d) / rms(x).max(1e-12)
    }

    /// Pitch by normalized autocorrelation with parabolic refinement.
    fn pitch_hz(x: &[f32], sr: f32, lo: f32, hi: f32) -> f32 {
        let (min_lag, max_lag) = ((sr / hi) as usize, (sr / lo) as usize);
        let n = x.len() - max_lag - 2;
        let corr = |lag: usize| {
            let (mut xy, mut xx, mut yy) = (0.0f64, 0.0f64, 0.0f64);
            for i in 0..n {
                let (a, b) = (x[i] as f64, x[i + lag] as f64);
                xy += a * b;
                xx += a * a;
                yy += b * b;
            }
            xy / (xx * yy).sqrt().max(1e-20)
        };
        let s: Vec<f64> = (min_lag..=max_lag + 1).map(corr).collect();
        // The first peak close to the best one: multiples of the period
        // correlate just as well.
        let best = s.iter().cloned().fold(f64::MIN, f64::max);
        let b = (1..s.len() - 1).find(|&i| s[i] >= 0.97 * best && s[i] >= s[i - 1] && s[i] >= s[i + 1]).unwrap();
        let shift = 0.5 * (s[b - 1] - s[b + 1]) / (s[b - 1] - 2.0 * s[b] + s[b + 1]);
        sr / ((min_lag + b) as f64 + shift) as f32
    }

    #[test]
    fn every_preset_sounds_and_stops() {
        for name in preset_names() {
            let p = preset(name);
            assert_eq!(p.len(), SPECS.len());
            let mut st = BasslineState::new(SR);
            st.note_on(&p, 36, 0.7);
            let mut out = render(&mut st, &p, 0.25);
            // An accented slide on top.
            st.note_on(&p, 43, 1.0);
            out.extend(render(&mut st, &p, 0.25));
            st.note_off(&p, 43);
            st.note_off(&p, 36);
            let level = rms(&out);
            assert!(level > 0.01, "{name} is silent ({level})");
            let peak = out.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(out.iter().all(|v| v.is_finite()) && peak < 1.5, "{name} peaks at {peak}");
            let _ = render(&mut st, &p, 3.0);
            assert!(!st.is_active(), "{name} keeps playing");
            assert!(render(&mut st, &p, 0.1).iter().all(|&v| v == 0.0));
        }
    }

    #[test]
    fn plays_a2_at_110_hz() {
        for (sr, wave) in [(48000.0f32, 0.0f32), (48000.0, 1.0), (44100.0, 0.85)] {
            let mut p = preset("Classic");
            p[WAVEFORM] = wave;
            let mut st = BasslineState::new(sr);
            st.note_on(&p, 45, 0.5);
            let n = (0.5 * sr) as usize;
            let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
            st.render(&p, &mut l, &mut r);
            let f = pitch_hz(&l[(0.05 * sr) as usize..], sr, 50.0, 400.0);
            assert!((f - 110.0).abs() < 0.5, "{sr} Hz, wave {wave}: {f} Hz");
        }
    }

    #[test]
    fn tuning_shifts_the_pitch() {
        let mut p = preset("Classic");
        p[TUNING] = 1200.0;
        let mut st = BasslineState::new(SR);
        st.note_on(&p, 45, 0.5);
        let x = render(&mut st, &p, 0.4);
        let f = pitch_hz(&x[2400..], SR, 100.0, 800.0);
        assert!((f - 220.0).abs() < 1.0, "{f} Hz");
    }

    #[test]
    fn legato_slides_without_a_new_attack() {
        let mut p = preset("Classic");
        // A slow slide (60 ms time constant) to see it move.
        p[SLIDE_TIME] = 300.0;
        let mut st = BasslineState::new(SR);
        st.note_on(&p, 45, 0.5);
        let _ = render(&mut st, &p, 0.3);
        let amp_before = st.amp_env.level;
        let env_before = st.main_env.y;
        st.note_on(&p, 57, 0.5);
        // Track the oscillator frequency block by block.
        let mut freqs = Vec::new();
        let mut out = Vec::new();
        for _ in 0..(0.4 * SR / 64.0) as usize {
            out.extend(render(&mut st, &p, 64.0 / SR));
            freqs.push(st.freq);
        }
        assert!(freqs[0] > 110.0 && freqs[0] < 120.0, "jumped: {}", freqs[0]);
        assert!(freqs.windows(2).all(|w| w[1] >= w[0]), "slide is not monotonic");
        let at_40ms = freqs[(0.04 * SR / 64.0) as usize];
        assert!(at_40ms > 130.0 && at_40ms < 210.0, "{at_40ms} Hz after 40 ms");
        assert!((freqs.last().unwrap() - 220.0).abs() < 0.5);
        // The envelopes kept running: no new attack.
        assert!(st.amp_env.level < amp_before, "amp envelope restarted");
        assert!(st.main_env.y < env_before, "filter envelope restarted");
        // The audio follows: in between early on, an octave up at the end.
        let early = pitch_hz(&out[..(0.05 * SR) as usize], SR, 80.0, 400.0);
        assert!(early > 115.0 && early < 210.0, "{early} Hz early in the slide");
        let late = pitch_hz(&out[out.len() - (0.1 * SR) as usize..], SR, 80.0, 400.0);
        assert!((late - 220.0).abs() < 1.0, "{late} Hz after the slide");

        // Releasing the newest key slides back to the one still held.
        st.note_off(&p, 57);
        let back = render(&mut st, &p, 0.4);
        assert!(st.is_active() && st.amp_env.note_on, "released while a key is held");
        let f = pitch_hz(&back[back.len() - (0.1 * SR) as usize..], SR, 50.0, 400.0);
        assert!((f - 110.0).abs() < 0.5, "slid back to {f} Hz");
        st.note_off(&p, 45);
        assert!(!st.amp_env.note_on);
    }

    #[test]
    fn accent_is_louder_and_brighter() {
        let p = preset("Classic");
        let play = |vel: f32| {
            let mut st = BasslineState::new(SR);
            st.note_on(&p, 40, vel);
            (render(&mut st, &p, 0.2), st.accent_gain)
        };
        let (normal, g_normal) = play(99.0 / 127.0);
        let (accented, g_accent) = play(100.0 / 127.0);
        assert_eq!(g_normal, 0.0, "velocity 99 is not accented");
        assert!(g_accent > 0.0, "velocity 100 is accented");
        let (a, n) = (rms(&accented), rms(&normal));
        assert!(a > 1.3 * n, "accent {a} vs normal {n}");
        let w = (0.1 * SR) as usize;
        let (ba, bn) = (brightness(&accented[..w]), brightness(&normal[..w]));
        assert!(ba > bn, "accent brightness {ba} vs normal {bn}");
    }

    #[test]
    fn higher_cutoff_is_brighter() {
        let mut p = preset("Classic");
        p[ENV_MOD] = 0.0;
        let play = |cutoff: f32| {
            let mut q = p.clone();
            q[CUTOFF] = cutoff;
            let mut st = BasslineState::new(SR);
            st.note_on(&q, 40, 0.5);
            brightness(&render(&mut st, &q, 0.3)[2400..])
        };
        let (dark, mid, bright) = (play(314.0), play(1000.0), play(2394.0));
        assert!(mid > 1.2 * dark && bright > 1.2 * mid, "{dark} {mid} {bright}");
    }

    #[test]
    fn full_resonance_stays_bounded() {
        for (cutoff, drive) in [(314.0, 0.0), (1000.0, 0.0), (2394.0, 0.0), (800.0, 1.0)] {
            let mut p = preset("Classic");
            p[RESONANCE] = 1.0;
            p[ENV_MOD] = 1.0;
            p[CUTOFF] = cutoff;
            p[DRIVE] = drive;
            p[AMP_SUSTAIN] = 0.0;
            p[ACCENT] = 1.0;
            let mut st = BasslineState::new(SR);
            st.note_on(&p, 33, 1.0);
            let x = render(&mut st, &p, 5.0);
            let peak = x.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(x.iter().all(|v| v.is_finite()) && peak < 4.0, "cutoff {cutoff}: peak {peak}");
            let s = SR as usize;
            let (second, last) = (rms(&x[s..2 * s]), rms(&x[4 * s..]));
            assert!(last < 1.5 * second + 1e-3, "cutoff {cutoff}: grows from {second} to {last}");
        }
    }

    #[test]
    fn mip_levels_are_band_limited() {
        let t = tables();
        let bin = |x: &[f32], k: usize| {
            let (mut re, mut im) = (0.0f64, 0.0f64);
            for (i, &v) in x[..TABLE_LEN].iter().enumerate() {
                let w = 2.0 * PI * (k * i) as f64 / TABLE_LEN as f64;
                re += v as f64 * w.cos();
                im += v as f64 * w.sin();
            }
            (re * re + im * im).sqrt() / TABLE_LEN as f64
        };
        for table in [&t.saw, &t.square] {
            for level in 1..=9 {
                let x = &table[level * STRIDE..(level + 1) * STRIDE];
                let keep = TABLE_LEN >> (level + 1);
                assert!(bin(x, 1) > 0.1, "level {level} lost its fundamental");
                assert!(bin(x, keep) < 1e-6 && bin(x, keep + 1) < 1e-6, "level {level} leaks");
                assert_eq!(x[TABLE_LEN], x[0], "guard sample");
            }
        }
    }

    #[test]
    fn short_parameter_lists_are_ignored() {
        let mut st = BasslineState::new(SR);
        st.note_on(&[0.5; 3], 45, 1.0);
        let (mut l, mut r) = (vec![0.0; 64], vec![0.0; 64]);
        st.render(&[0.5; 3], &mut l, &mut r);
        assert!(!st.is_active() && l.iter().all(|&v| v == 0.0));
    }

    #[test]
    #[ignore]
    fn report_levels() {
        for name in preset_names() {
            let p = preset(name);
            let mut st = BasslineState::new(SR);
            let mut out = Vec::new();
            for (k, &(note, vel)) in [(36u8, 0.7f32), (48, 1.0), (43, 0.7), (36, 1.0)].iter().enumerate() {
                st.note_on(&p, note, vel);
                out.extend(render(&mut st, &p, 0.12));
                if k % 2 == 1 {
                    st.note_off(&p, note);
                }
                st.note_off(&p, note);
                out.extend(render(&mut st, &p, 0.03));
            }
            let peak = out.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            let t = std::time::Instant::now();
            st.note_on(&p, 36, 0.8);
            let _ = render(&mut st, &p, 1.0);
            eprintln!("{name:12} rms {:.3} peak {peak:.3}, 1 s in {:.1} ms", rms(&out), t.elapsed().as_secs_f32() * 1000.0);
        }
    }
}
