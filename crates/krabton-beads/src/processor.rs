//! The whole chain (`beads/dsp/processor.cc`, `granular_processor.h`,
//! `sample_rate_conversion_filters.h`): the input with the feedback, the
//! recording, the grains or the delay, dry/wet, the tape's wobble, the
//! reverb and the limiters, in one of four qualities:
//!
//! - Hi-Fi: 48 kHz, 16-bit, about 4 s;
//! - Clouds: 32 kHz, 12-bit, about 8 s ("cold digital");
//! - Sunny Tape: 24 kHz inside a 48 kHz chain, 12-bit, about 10 s;
//! - Scorched Cassette: 24 kHz, 8-bit µ-law, about 16 s, with wow and
//!   flutter.
//!
//! Mono doubles the time.
//!
//! Copyright 2017 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::delay::{DelayController, DelayOutput, MultitapDelay, NUM_TAPS};
use crate::delay_line::{DelayLine, Resolution};
use crate::fx::{Reverb, WowAndFlutter};
use crate::player::DelayGranularPlayer;
use crate::random::{ModulatedParameter, Parameters, RandomWalkModulation};
use crate::trigger::{SeedButtonState, TriggerGenerator, TriggerOutput};
use crate::util::{clip16, linear_to_mu_law, mu_law_to_linear, one_pole, semitones_to_ratio, smooth_step, soft_clip, GateFlags, Limiter, ParameterInterpolator, Random, Svf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Media {
    DirtyTape = 0,
    CleanTape = 1,
    VintageDigital = 2,
    Digital = 3,
}

impl Media {
    pub fn from_index(i: usize) -> Self {
        match i {
            0 => Media::DirtyTape,
            1 => Media::CleanTape,
            2 => Media::VintageDigital,
            _ => Media::Digital,
        }
    }

    /// The rate the codec runs at.
    pub fn sample_rate(self) -> f32 {
        [24000.0, 48000.0, 32000.0, 48000.0][self as usize]
    }

    /// How much slower the recording runs than the codec.
    pub fn ratio(self) -> usize {
        [1, 2, 1, 1][self as usize]
    }

    fn resolution(self) -> Resolution {
        match self {
            Media::DirtyTape => Resolution::Bit8,
            Media::CleanTape | Media::VintageDigital => Resolution::Bit12,
            Media::Digital => Resolution::Bit16,
        }
    }

    /// Stereo frames of recording: 768 KB of memory at the resolution.
    fn max_delay(self) -> usize {
        const BITS: usize = 768 * 1000 * 8;
        BITS / ([8, 12, 12, 16][self as usize] * 2)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MixParameters {
    pub feedback: f32,
    pub dry_wet: f32,
    pub reverb_amount: f32,
}

/// Half-band filters for the 2x rate change of the Sunny Tape setting
/// (remez, 48 taps, symmetric: the first half).
const SRC_UP_2_48: [f32; 24] = [
    -4.287457573e-04,
    -8.226487157e-04,
    7.100052364e-04,
    2.247177610e-03,
    4.027580292e-04,
    -4.271779932e-03,
    -3.893072135e-03,
    4.843915649e-03,
    1.019934854e-02,
    -9.694387912e-04,
    -1.727659358e-02,
    -1.027548927e-02,
    2.001194882e-02,
    2.929003471e-02,
    -1.083867877e-02,
    -5.164495731e-02,
    -1.821391107e-02,
    6.671043028e-02,
    7.404344683e-02,
    -5.604828388e-02,
    -1.683376729e-01,
    -2.385313356e-02,
    3.910061488e-01,
    7.674091912e-01,
];
const SRC_DOWN_2_48: [f32; 24] = [
    -2.143728787e-04,
    -4.113243578e-04,
    3.550026182e-04,
    1.123588805e-03,
    2.013790146e-04,
    -2.135889966e-03,
    -1.946536068e-03,
    2.421957824e-03,
    5.099674271e-03,
    -4.847193956e-04,
    -8.638296788e-03,
    -5.137744633e-03,
    1.000597441e-02,
    1.464501735e-02,
    -5.419339385e-03,
    -2.582247866e-02,
    -9.106955536e-03,
    3.335521514e-02,
    3.702172341e-02,
    -2.802414194e-02,
    -8.416883646e-02,
    -1.192656678e-02,
    1.955030744e-01,
    3.837045956e-01,
];

#[inline]
fn tap(h: &[f32; 24], j: usize) -> f32 {
    if j < 24 {
        h[j]
    } else {
        h[47 - j]
    }
}

/// Halves the rate: one output for every two inputs.
#[derive(Clone, Copy)]
struct SrcDown {
    x: [f32; 48],
    pos: usize,
}

impl SrcDown {
    fn new() -> Self {
        Self { x: [0.0; 48], pos: 0 }
    }

    fn process(&mut self, input: &[f32], out: &mut [f32]) {
        for (k, o) in out.iter_mut().enumerate() {
            for i in 0..2 {
                self.pos = (self.pos + 47) % 48;
                self.x[self.pos] = input[2 * k + i];
            }
            let mut y = 0.0;
            for j in 0..48 {
                y += self.x[(self.pos + j) % 48] * tap(&SRC_DOWN_2_48, j);
            }
            *o = y;
        }
    }
}

/// Doubles the rate: two outputs for every input.
#[derive(Clone, Copy)]
struct SrcUp {
    x: [f32; 24],
    pos: usize,
}

impl SrcUp {
    fn new() -> Self {
        Self { x: [0.0; 24], pos: 0 }
    }

    fn process(&mut self, input: &[f32], out: &mut [f32]) {
        for (n, &v) in input.iter().enumerate() {
            self.pos = (self.pos + 23) % 24;
            self.x[self.pos] = v;
            for k in 0..2 {
                let mut y = 0.0;
                for i in 0..24 {
                    y += self.x[(self.pos + i) % 24] * tap(&SRC_UP_2_48, k + 2 * i);
                }
                out[2 * n + k] = y;
            }
        }
    }
}

/// The recording and its playback, at the recording's own rate.
pub struct GranularProcessor {
    media: Media,
    mono: bool,
    sample_rate: f32,
    line: DelayLine,
    player: DelayGranularPlayer,
    multitap: MultitapDelay,
    feedback: f32,
    speed: f32,
    bandwidth: f32,
    previous: [f32; 2],
    dc_blocker: [Svf; 2],
    pitch_modulation: RandomWalkModulation,
    input_limiter: Limiter,
    src_down: [SrcDown; 2],
    src_up: [SrcUp; 2],
    // Scratch for the rate change.
    d_freeze: Vec<bool>,
    d_triggers: Vec<TriggerOutput>,
    d_taps: Vec<DelayOutput>,
    d_in: Vec<[f32; 2]>,
    d_out: Vec<[f32; 2]>,
    up: Vec<f32>,
    down: Vec<f32>,
}

const MAX_BLOCK: usize = 64;

impl GranularProcessor {
    pub fn new() -> Self {
        let sr = 48000.0;
        let mut g = Self {
            media: Media::Digital,
            mono: false,
            sample_rate: sr,
            line: DelayLine::new(Media::DirtyTape.max_delay() * 2),
            player: DelayGranularPlayer::new(sr),
            multitap: MultitapDelay::new(),
            feedback: 0.0,
            speed: 0.0,
            bandwidth: 0.0,
            previous: [0.0; 2],
            dc_blocker: [Svf::default(); 2],
            pitch_modulation: RandomWalkModulation::default(),
            input_limiter: Limiter::default(),
            src_down: [SrcDown::new(); 2],
            src_up: [SrcUp::new(); 2],
            d_freeze: Vec::with_capacity(MAX_BLOCK),
            d_triggers: Vec::with_capacity(MAX_BLOCK),
            d_taps: Vec::with_capacity(MAX_BLOCK),
            d_in: Vec::with_capacity(MAX_BLOCK),
            d_out: Vec::with_capacity(MAX_BLOCK),
            up: vec![0.0; MAX_BLOCK],
            down: vec![0.0; MAX_BLOCK],
        };
        g.configure(Media::Digital, false);
        g
    }

    /// The recording's rate for a setting.
    fn inner_rate(media: Media) -> f32 {
        media.sample_rate() / media.ratio() as f32
    }

    pub fn configure(&mut self, media: Media, mono: bool) {
        self.media = media;
        self.mono = mono;
        let sr = Self::inner_rate(media);
        self.sample_rate = sr;
        let res = media.resolution();
        let max = media.max_delay();
        if mono {
            self.line.configure(res, 1, max * 2);
        } else {
            self.line.configure(res, 2, max);
        }
        self.player = DelayGranularPlayer::new(sr);
        self.multitap.reset();
        self.feedback = 0.0;
        self.speed = 0.0;
        self.bandwidth = 0.0;
        self.previous = [0.0; 2];
        for f in self.dc_blocker.iter_mut() {
            f.init();
            f.set_f_q_fast(40.0 / sr, 0.5);
        }
        self.src_down = [SrcDown::new(); 2];
        self.src_up = [SrcUp::new(); 2];
        self.pitch_modulation.init(0.2 / sr);
        self.input_limiter.init(1.0 / sr, 0.95);
    }

    /// Recording frames times the rate ratio: the longest delay in codec
    /// samples (stereo).
    pub fn max_delay_times_ratio(&self) -> i32 {
        (self.media.max_delay() * self.media.ratio()) as i32
    }

    pub fn trim(&mut self) {
        self.line.trim();
    }

    pub fn remaining(&self) -> i32 {
        self.line.remaining()
    }

    /// The grains now.
    pub fn grains(&self, out: &mut Vec<crate::grain::GrainView>) {
        let max = self.line.max_delay();
        out.extend(self.player.player.grains.iter().filter_map(|g| g.view(max)));
    }

    /// How much of the recording is filled (0..1), for a display.
    pub fn filled(&self) -> f32 {
        self.line.duration() as f32 / self.line.max_delay().max(1) as f32
    }

    #[allow(clippy::too_many_arguments)]
    pub fn process(&mut self, triggers: Option<&[TriggerOutput]>, taps: Option<&[DelayOutput]>, freeze: &[bool], p: &Parameters, feedback: f32, input: &[[f32; 2]], out: &mut [[f32; 2]], rng: &mut Random) {
        let ratio = self.media.ratio();
        if ratio == 1 {
            self.process_internal(triggers, taps, freeze, p, feedback, input, out, rng);
            return;
        }
        let size = input.len();
        let d_size = size / ratio;
        let inv = 1.0 / ratio as f32;
        self.d_freeze.clear();
        for k in 0..d_size {
            self.d_freeze.push((0..ratio).any(|j| freeze[k * ratio + j]));
        }
        self.d_triggers.clear();
        if let Some(t) = triggers {
            for k in 0..d_size {
                let mut o = TriggerOutput { regularity: t[k * ratio].regularity, ..Default::default() };
                for j in 0..ratio {
                    let src = &t[k * ratio + j];
                    if src.trigger && !o.trigger {
                        o.trigger = true;
                        o.fractional_sample = (src.fractional_sample + j as f32) * inv;
                    }
                    o.gate |= src.gate;
                }
                self.d_triggers.push(o);
            }
        }
        self.d_taps.clear();
        if let Some(t) = taps {
            for k in 0..d_size {
                let mut o = t[k * ratio];
                for tp in o.tap.iter_mut().take(NUM_TAPS) {
                    tp.delay /= ratio as i32;
                }
                self.d_taps.push(o);
            }
        }
        self.d_in.clear();
        self.d_in.resize(d_size, [0.0; 2]);
        for c in 0..2 {
            for i in 0..size {
                self.up[i] = input[i][c];
            }
            let (up, down) = (&self.up[..size], &mut self.down[..d_size]);
            self.src_down[c].process(up, down);
            for i in 0..d_size {
                self.d_in[i][c] = self.down[i];
            }
        }
        self.d_out.clear();
        self.d_out.resize(d_size, [0.0; 2]);
        let d_in = std::mem::take(&mut self.d_in);
        let mut d_out = std::mem::take(&mut self.d_out);
        let d_freeze = std::mem::take(&mut self.d_freeze);
        let d_triggers = std::mem::take(&mut self.d_triggers);
        let d_taps = std::mem::take(&mut self.d_taps);
        self.process_internal(triggers.map(|_| &d_triggers[..]), taps.map(|_| &d_taps[..]), &d_freeze, p, feedback, &d_in, &mut d_out, rng);
        for c in 0..2 {
            for i in 0..d_size {
                self.down[i] = d_out[i][c];
            }
            let (down, up) = (&self.down[..d_size], &mut self.up[..size]);
            self.src_up[c].process(down, up);
            for i in 0..size {
                out[i][c] = self.up[i];
            }
        }
        self.d_in = d_in;
        self.d_out = d_out;
        self.d_freeze = d_freeze;
        self.d_triggers = d_triggers;
        self.d_taps = d_taps;
    }

    #[allow(clippy::too_many_arguments)]
    fn process_internal(&mut self, triggers: Option<&[TriggerOutput]>, taps: Option<&[DelayOutput]>, freeze: &[bool], p: &Parameters, feedback: f32, input: &[[f32; 2]], out: &mut [[f32; 2]], rng: &mut Random) {
        let size = input.len();
        let speed = semitones_to_ratio(p.pitch.value_with(Some(&mut self.pitch_modulation), size, 24.0, false, rng));
        let bandwidth = (0.5 * (1.0 + p.size.attenuverter) + if p.size.has_cv { p.size.cv } else { 0.0 }).clamp(0.0, 1.0);
        let max_feedback_gain = if taps.is_some() { 1.5 } else { 3.0 };
        let (lp_coefficient, soft_limit_gain) = match self.media.resolution() {
            Resolution::Bit8 => (0.75, 1.75),
            Resolution::Bit12 => (0.9, 1.3),
            Resolution::Bit16 => (0.999, 0.0),
        };
        let (mut bw_s, mut fb_s, mut sp_s) = (self.bandwidth, self.feedback, self.speed);
        {
            let mut bw_m = ParameterInterpolator::new(&mut bw_s, bandwidth, size);
            let mut fb_m = ParameterInterpolator::new(&mut fb_s, feedback, size);
            let mut sp_m = ParameterInterpolator::new(&mut sp_s, speed, size);
            for i in 0..size {
                let fb = fb_m.next();
                let x = input[i];
                let mut s = x;
                if soft_limit_gain != 0.0 {
                    let gain = max_feedback_gain / soft_limit_gain;
                    s[0] += self.previous[0] * fb * gain;
                    s[1] += self.previous[1] * fb * gain;
                } else {
                    s[0] += self.previous[0] * fb * fb * max_feedback_gain;
                    s[1] += self.previous[1] * fb * fb * max_feedback_gain;
                }
                let mut one = [s];
                self.input_limiter.process(&mut one);
                s = one[0];
                if soft_limit_gain != 0.0 {
                    for c in 0..2 {
                        let v = soft_clip(s[c] * soft_limit_gain);
                        s[c] = x[c] + fb * (v - x[c]);
                    }
                }
                let mut o = [0.0f32; 2];
                self.line.write_frozen(s, freeze[i]);
                if let Some(t) = triggers {
                    self.player.render(&self.line, &t[i], p, &mut o, rng);
                }
                if let Some(t) = taps {
                    self.multitap.render(&self.line, &t[i], sp_m.next(), bw_m.next(), &mut o);
                }
                for c in 0..2 {
                    let hp = self.dc_blocker[c].high_pass(o[c]);
                    one_pole(&mut self.previous[c], hp, lp_coefficient);
                }
                out[i] = o;
            }
        }
        self.bandwidth = bw_s;
        self.feedback = fb_s;
        self.speed = sp_s;
    }
}

impl Default for GranularProcessor {
    fn default() -> Self {
        Self::new()
    }
}

const DELAY_MODE_TRANSITION_TIME: f32 = 1.5;
const DELAY_MODE_FADE_TIME: f32 = 0.75;

/// Beads processing its input.
pub struct Processor {
    granular: GranularProcessor,
    trigger: TriggerGenerator,
    delay_controller: DelayController,
    media: Media,
    mono: bool,
    delay_mode_transition: f32,
    dry: f32,
    wet: f32,
    wow_and_flutter: WowAndFlutter,
    reverb: Reverb,
    pre_reverb_limiter: Limiter,
    output_limiter: Limiter,
    pub rng: Random,
    triggers: Vec<TriggerOutput>,
    taps: Vec<DelayOutput>,
    dry_signal: Vec<[f32; 2]>,
    gates: Vec<bool>,
}

impl Processor {
    pub fn new() -> Self {
        let mut wow = WowAndFlutter::new(24000.0);
        wow.set_lp(0.95);
        let mut p = Self {
            granular: GranularProcessor::new(),
            trigger: TriggerGenerator::new(48000.0),
            delay_controller: DelayController::new(48000.0, 192000),
            media: Media::Digital,
            mono: false,
            delay_mode_transition: -DELAY_MODE_TRANSITION_TIME,
            dry: 0.0,
            wet: 0.0,
            wow_and_flutter: wow,
            reverb: Reverb::new(48000.0),
            pre_reverb_limiter: Limiter::default(),
            output_limiter: Limiter::default(),
            rng: Random::default(),
            triggers: vec![TriggerOutput::default(); MAX_BLOCK],
            taps: vec![DelayOutput::default(); MAX_BLOCK],
            dry_signal: vec![[0.0; 2]; MAX_BLOCK],
            gates: vec![false; MAX_BLOCK],
        };
        p.configure(Media::Digital, false);
        p
    }

    pub fn media(&self) -> Media {
        self.media
    }

    pub fn mono(&self) -> bool {
        self.mono
    }

    pub fn sample_rate(&self) -> f32 {
        self.media.sample_rate()
    }

    pub fn configure(&mut self, media: Media, mono: bool) {
        self.media = media;
        self.mono = mono;
        self.granular.configure(media, mono);
        let max_delay = self.granular.max_delay_times_ratio() * if mono { 2 } else { 1 };
        let sr = self.sample_rate();
        self.pre_reverb_limiter.init(1.0 / (sr * 4.0), 0.9);
        self.output_limiter.init(1.0 / (sr * 6.0), 0.9);
        self.trigger.init(sr);
        self.delay_controller.init(sr, max_delay);
        self.reverb.reset(sr);
        self.reverb.clear();
        // Start empty, as after MEDIA + FREEZE on the module: TIME then
        // reaches over what has been recorded so far, not over silence.
        self.granular.trim();
    }

    /// MEDIA + FREEZE: the recording's length back to nothing.
    pub fn trim(&mut self) {
        self.granular.trim();
    }

    /// How full the recording is (0..1).
    pub fn filled(&self) -> f32 {
        self.granular.filled()
    }

    pub fn grains(&self, out: &mut Vec<crate::grain::GrainView>) {
        self.granular.grains(out);
    }

    /// In delay mode (SIZE fully clockwise)?
    pub fn delay_mode(&self) -> bool {
        self.delay_mode_transition > 0.0
    }

    /// The grains' gate of the last block (for a light).
    pub fn gate(&self) -> bool {
        self.gates.first().copied().unwrap_or(false)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn process(&mut self, button: SeedButtonState, seed_input: Option<&[GateFlags]>, freeze: &[bool], density: &ModulatedParameter, p: &Parameters, mix: &MixParameters, input: &[[f32; 2]], out: &mut [[f32; 2]]) {
        let size = input.len().min(MAX_BLOCK);
        let sr = self.sample_rate();
        let delay_mode = self.delay_mode_transition > 0.0;
        let threshold = if delay_mode { 0.98 } else { 0.99 };
        let direction = if p.size.base > threshold { 1.0 } else { -1.0 };
        self.delay_mode_transition = (self.delay_mode_transition + direction * size as f32 / sr).clamp(-DELAY_MODE_TRANSITION_TIME, DELAY_MODE_TRANSITION_TIME);
        let expo_fm_amount = (20.0 * ((density.base - 0.5).abs() - 0.225)).clamp(0.0, 1.0);
        let rng = &mut self.rng;
        // Grain triggers, or the delay's taps.
        if delay_mode {
            self.delay_controller.render(button, seed_input, &freeze[..size], density.base, (1.0 - expo_fm_amount) * density.cv, expo_fm_amount * density.cv, &p.time, &p.shape, &mut self.taps[..size], rng);
            for i in 0..size {
                self.gates[i] = self.taps[i].gate;
            }
        } else {
            self.trigger.render(button, seed_input, density.base, (1.0 - expo_fm_amount) * density.cv, expo_fm_amount * density.cv, &mut self.triggers[..size], rng);
            for i in 0..size {
                self.gates[i] = self.triggers[i].gate;
            }
        }
        let (triggers, taps) = if delay_mode { (None, Some(&self.taps[..size])) } else { (Some(&self.triggers[..size]), None) };
        self.granular.process(triggers, taps, &freeze[..size], p, mix.feedback, &input[..size], &mut out[..size], rng);
        // The dry signal goes through the same converter.
        for i in 0..size {
            self.dry_signal[i] = match self.media {
                Media::VintageDigital => [((input[i][0] * 4096.0) as i32) as f32 / 4096.0, ((input[i][1] * 4096.0) as i32) as f32 / 4096.0],
                Media::DirtyTape => [mu_law_to_linear(linear_to_mu_law(clip16((input[i][0] * 32768.0) as i32) as i16)), mu_law_to_linear(linear_to_mu_law(clip16((input[i][1] * 32768.0) as i32) as i16))],
                _ => input[i],
            };
        }
        // Dry/wet, the dry fading while switching to or from delay mode.
        let wet_fade = smooth_step((self.delay_mode_transition.abs() / DELAY_MODE_FADE_TIME).min(1.0));
        let dw = mix.dry_wet;
        let dry = (1.0 - dw) * (1.0 + dw) * 0.75;
        let wet = dw * (2.0 - dw) * 1.5 * wet_fade;
        let (mut dry_s, mut wet_s) = (self.dry, self.wet);
        {
            let mut dry_m = ParameterInterpolator::new(&mut dry_s, dry, size);
            let mut wet_m = ParameterInterpolator::new(&mut wet_s, wet, size);
            for i in 0..size {
                let (d, w) = (dry_m.next(), wet_m.next());
                out[i][0] = d * self.dry_signal[i][0] + w * out[i][0];
                out[i][1] = d * self.dry_signal[i][1] + w * out[i][1];
            }
        }
        self.dry = dry_s;
        self.wet = wet_s;
        if self.media == Media::DirtyTape {
            self.wow_and_flutter.process(&mut out[..size], &mut self.rng);
        }
        self.pre_reverb_limiter.process(&mut out[..size]);
        // Reverb.
        let caveman = self.media != Media::Digital;
        let r = mix.reverb_amount;
        let reverb_time = r * r * r;
        self.reverb.set_mix(r * r * 0.9);
        self.reverb.set_time(if caveman { 0.45 + 0.5 * reverb_time } else { 0.3 + 0.6 * reverb_time });
        self.reverb.set_lp(if caveman {
            0.4
        } else if self.media == Media::VintageDigital {
            0.75
        } else {
            0.6
        });
        self.reverb.process(&mut out[..size]);
        self.output_limiter.process(&mut out[..size]);
    }
}

impl Default for Processor {
    fn default() -> Self {
        Self::new()
    }
}
