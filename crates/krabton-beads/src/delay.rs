//! The delay mode (`beads/dsp/delay_clock.cc`, `delay_controller.cc`,
//! `multitap_delay.h`, `tap.h`): with SIZE fully clockwise Beads becomes a
//! delay. Its time comes from taps on SEED or a clock (DENSITY then picks
//! a multiple or division), or from DENSITY itself; TIME adds echoes at
//! multiples of the time; SHAPE shapes each echo; PITCH shifts them; with
//! FREEZE on it slices the recording in a rhythm instead.
//!
//! Copyright 2017 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::delay_line::DelayLine;
use crate::random::{RandomWalkModulation, RandomizedParameter};
use crate::trigger::SeedButtonState;
use crate::util::{integral_fractional, interpolate, one_pole, semitones_to_ratio, GateFlags, HysteresisFilter, HysteresisQuantizer2, ParameterInterpolator, Random, SineOscillator, Svf, GATE_FLAG_RISING};

/// Extracts a steady clock from taps or a clock signal.
#[derive(Clone, Copy, Debug, Default)]
pub struct DelayClock {
    interval: [f32; 2],
    average_interval: f32,
    fade: f32,
    fade_increment: f32,
    max_fade_increment: f32,
    min_fade_increment: f32,
    acceptable_jitter: f32,
    jitter_sensitivity: f32,
    schedule_change: bool,
    next_interval: i32,
    max_interval: i32,
    rotate: usize,
}

impl DelayClock {
    pub fn init(&mut self, sample_rate: f32) {
        self.average_interval = sample_rate * 0.25;
        self.interval = [self.average_interval; 2];
        self.fade = 1.0;
        self.min_fade_increment = 1.0 / (sample_rate * 0.25);
        self.max_fade_increment = 1.0 / (sample_rate * 0.02);
        self.fade_increment = self.max_fade_increment;
        self.acceptable_jitter = sample_rate * 0.001;
        self.jitter_sensitivity = 1.0;
        self.schedule_change = false;
        self.next_interval = 0;
        self.max_interval = (sample_rate * 4.0) as i32;
        self.rotate = 0;
    }

    pub fn update(&mut self, interval: f32, lp_coefficient: f32) {
        one_pole(&mut self.interval[0], interval, lp_coefficient);
        self.interval[1] = self.interval[0];
        self.fade = 1.0;
        self.rotate = 0;
    }

    pub fn interval(&self, i: usize) -> f32 {
        self.interval[(i + self.rotate) % 2]
    }

    pub fn gain(&self, i: usize) -> f32 {
        if (i + self.rotate) % 2 == 1 {
            self.fade
        } else {
            1.0 - self.fade
        }
    }

    pub fn retrig(&self) -> bool {
        self.fade == 0.0 && self.next_interval == 0
    }

    pub fn new_interval_index(&self) -> usize {
        (self.rotate + 1) % 2
    }

    pub fn process(&mut self, tick: bool, tap: bool) {
        self.next_interval += 1;
        self.fade = (self.fade + self.fade_increment).min(1.0);
        if tick || tap {
            if self.next_interval < self.max_interval {
                let interval = self.next_interval as f32;
                let big_change = (interval - self.interval[1]).abs() > self.acceptable_jitter;
                let small_change = (self.average_interval - self.interval[1]).abs() > self.jitter_sensitivity * self.acceptable_jitter;
                if big_change {
                    self.jitter_sensitivity = 1.0;
                    self.average_interval = interval;
                } else {
                    one_pole(&mut self.average_interval, interval, 0.1);
                    self.jitter_sensitivity = (self.jitter_sensitivity * 0.95).max(0.05);
                }
                self.schedule_change = self.schedule_change || tap || big_change || small_change;
            }
            self.next_interval = 0;
        }
        let can_change = tap || self.fade >= 1.0;
        if self.schedule_change && can_change {
            self.rotate += 1;
            self.interval[0] = self.interval[1];
            self.interval[1] = self.average_interval;
            self.fade = 0.0;
            self.fade_increment = (2.0 / self.average_interval).clamp(self.min_fade_increment, self.max_fade_increment);
            self.schedule_change = false;
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DelayTap {
    pub delay: i32,
    pub gain: f32,
}

pub const NUM_INTERVALS: usize = 2;
pub const NUM_RATIOS: usize = 2;
pub const NUM_HEADS: usize = NUM_INTERVALS * NUM_RATIOS;
pub const NUM_TAPS: usize = NUM_HEADS * 2;

#[derive(Clone, Copy, Debug, Default)]
pub struct DelayOutput {
    pub tap: [DelayTap; NUM_TAPS],
    pub gate: bool,
}

#[inline]
fn triangle(phase: f32, pw: f32) -> f32 {
    let slope_up = 1.0 / pw;
    let slope_down = 1.0 / (1.0 - pw);
    if phase < pw {
        phase * slope_up
    } else {
        1.0 - (phase - pw) * slope_down
    }
}

const PW: [f32; 6] = [0.0, 0.001, 0.01, 0.5, 0.75, 0.999];

/// A head playing one slice of a frozen recording.
#[derive(Clone, Copy, Debug, Default)]
struct PlayHead {
    start: i32,
    remaining: i32,
    mid_point: i32,
    phase: f32,
    phase_increment: f32,
    gain: f32,
}

impl PlayHead {
    fn init(&mut self) {
        *self = Self::default();
    }

    fn done(&self) -> bool {
        self.remaining == 0
    }

    fn half_way(&self) -> bool {
        self.remaining != 0 && self.remaining == self.mid_point
    }

    fn start(&mut self, start: i32, duration: i32, gain: f32) {
        self.start = start;
        self.remaining = duration;
        self.mid_point = duration >> 1;
        self.phase = 0.0;
        self.phase_increment = 1.0 / duration.max(1) as f32;
        self.gain = gain;
    }

    fn render(&mut self, tap: &mut DelayTap, shape: f32, slave: bool) {
        if self.done() {
            tap.delay = 0;
            tap.gain = 0.0;
            return;
        }
        let envelope = if slave {
            let e = [0.0, 0.0, 0.0, triangle(self.phase, 0.5), 0.0, 0.0];
            interpolate(&e, shape, 5.0)
        } else if shape < 0.2 {
            let square = (100.0 * triangle(self.phase, 0.5)).min(1.0);
            let tri = triangle(self.phase, PW[1]);
            square + (tri - square) * shape * 5.0
        } else {
            triangle(self.phase, interpolate(&PW, shape, 5.0))
        };
        tap.delay = (self.start + self.remaining) << 10;
        tap.gain = self.gain * envelope;
        self.remaining -= 1;
        self.phase += self.phase_increment;
    }
}

/// The envelope of an echo.
#[derive(Clone, Copy, Debug, Default)]
struct TapEnvelope {
    phase: f32,
    phase_increment: f32,
}

impl TapEnvelope {
    fn init(&mut self) {
        *self = Self::default();
    }

    fn render(&mut self, shape: f32) -> f32 {
        let envelope = if shape < 0.2 {
            let tri = triangle(self.phase, PW[1]);
            1.0 + (tri - 1.0) * shape * 5.0
        } else {
            triangle(self.phase, interpolate(&PW, shape, 5.0))
        };
        self.phase += self.phase_increment;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        envelope
    }
}

const MIN_DELAY: i32 = 64 + 4;
const NUM_DELAY_RATIOS: usize = 20;
const RATIO: [f32; NUM_DELAY_RATIOS + 2] = [
    1.0 / 16.0,
    1.0 / 16.0,
    1.0 / 8.0,
    1.0 / 8.0,
    1.0 / 4.0,
    1.0 / 4.0,
    1.0 / 2.0,
    1.0 / 2.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0 / 2.0,
    1.0 / 3.0,
    1.0 / 4.0,
    1.0 / 6.0,
    1.0 / 8.0,
    1.0 / 10.0,
    1.0 / 12.0,
    1.0 / 16.0,
    1.0 / 16.0,
    1.0 / 16.0,
];

pub struct DelayController {
    max_delay: i32,
    clock: DelayClock,
    play_head: [PlayHead; NUM_HEADS],
    slave_play_head: [PlayHead; NUM_HEADS],
    numerator_quantizer: [HysteresisQuantizer2; NUM_HEADS],
    tap_envelope: [TapEnvelope; NUM_TAPS],
    position_modulation: RandomWalkModulation,
    shape_modulation: RandomWalkModulation,
    tap_modulation: SineOscillator,
    density_filter: HysteresisFilter,
    fm_filter: HysteresisFilter,
    slice_index_filter: HysteresisFilter,
    multiplier_lp: f32,
    shape: f32,
    linear_fm: f32,
    fm: f32,
    density: f32,
    freeze: bool,
    seed_button_state: SeedButtonState,
}

impl DelayController {
    pub fn new(sample_rate: f32, max_delay: i32) -> Self {
        let mut d = Self {
            max_delay,
            clock: DelayClock::default(),
            play_head: [PlayHead::default(); NUM_HEADS],
            slave_play_head: [PlayHead::default(); NUM_HEADS],
            numerator_quantizer: [HysteresisQuantizer2::default(); NUM_HEADS],
            tap_envelope: [TapEnvelope::default(); NUM_TAPS],
            position_modulation: RandomWalkModulation::default(),
            shape_modulation: RandomWalkModulation::default(),
            tap_modulation: SineOscillator::default(),
            density_filter: HysteresisFilter::default(),
            fm_filter: HysteresisFilter::default(),
            slice_index_filter: HysteresisFilter::default(),
            multiplier_lp: 0.0,
            shape: 0.0,
            linear_fm: 0.0,
            fm: 1.0,
            density: 0.0,
            freeze: false,
            seed_button_state: SeedButtonState::Off,
        };
        d.init(sample_rate, max_delay);
        d
    }

    pub fn init(&mut self, sample_rate: f32, max_delay: i32) {
        self.max_delay = max_delay;
        self.clock.init(sample_rate);
        for i in 0..NUM_HEADS {
            self.numerator_quantizer[i].init(2, 0.05, false);
            self.play_head[i].init();
            self.slave_play_head[i].init();
        }
        for t in self.tap_envelope.iter_mut() {
            t.init();
        }
        self.position_modulation.init(0.1 / sample_rate);
        self.shape_modulation.init(0.1 / sample_rate);
        self.tap_modulation.init(0.17 / sample_rate);
        const K: f32 = 0.0025;
        self.density_filter.init(K);
        self.slice_index_filter.init(K);
        self.fm_filter.init(K);
        self.multiplier_lp = 0.0;
        self.shape = 0.0;
        self.linear_fm = 0.0;
        self.fm = 1.0;
        self.density = 0.0;
        self.freeze = false;
        self.seed_button_state = SeedButtonState::Off;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        button: SeedButtonState,
        external: Option<&[GateFlags]>,
        freeze: &[bool],
        density: f32,
        mut linear_fm: f32,
        mut exponential_fm: f32,
        position: &RandomizedParameter,
        shape: &RandomizedParameter,
        out: &mut [DelayOutput],
        rng: &mut Random,
    ) {
        let synchronized = !(button == SeedButtonState::Locked && external.is_none());
        let size = out.len();
        if synchronized {
            linear_fm += exponential_fm;
            exponential_fm = 0.0;
        }
        one_pole(&mut self.linear_fm, linear_fm, 0.05);
        let density = (density + self.linear_fm).clamp(0.0, 1.0);
        let slice_modulation = 1.005 * position.value_with(Some(&mut self.position_modulation), size, 1.0, true, rng);
        let shape_target = shape.value_with(Some(&mut self.shape_modulation), size, 1.0, true, rng) * 0.995;
        let fm_target = semitones_to_ratio(-self.fm_filter.process(exponential_fm) * 60.0);
        let density_target = self.density_filter.process(density);
        let mut shape_state = self.shape;
        let mut fm_state = self.fm;
        let mut density_state = self.density;
        let mut tap = button == SeedButtonState::On && self.seed_button_state == SeedButtonState::Off;
        self.seed_button_state = button;
        {
            let mut modulated_shape = ParameterInterpolator::new(&mut shape_state, shape_target, size);
            let mut modulated_fm = ParameterInterpolator::new(&mut fm_state, fm_target, size);
            let mut modulated_density = ParameterInterpolator::new(&mut density_state, density_target, size);
            for (n, o) in out.iter_mut().enumerate() {
                let mut tap_ratio = [0.0f32; NUM_RATIOS];
                let mut tap_gain = [0.0f32; NUM_RATIOS];
                let density = modulated_density.next();
                let fm = modulated_fm.next();
                if synchronized {
                    // The interval between taps on the button or pulses on SEED.
                    let tick = external.is_some_and(|e| e[n] & GATE_FLAG_RISING != 0);
                    self.clock.process(tick, tap);
                    tap = false;
                    // DENSITY picks a ratio of it.
                    let (ri, mut rf) = integral_fractional(density * NUM_DELAY_RATIOS as f32);
                    let ri = ri.clamp(0, NUM_DELAY_RATIOS as i32) as usize;
                    rf -= 0.5;
                    rf *= if density < 0.5 { 4.0 } else { 1.5 };
                    rf += 0.5;
                    let rf = rf.clamp(0.0, 1.0);
                    let flip = ri & 1;
                    tap_ratio[flip] = RATIO[ri];
                    tap_ratio[1 - flip] = RATIO[ri + 1];
                    tap_gain[flip] = 1.0 - rf;
                    tap_gain[1 - flip] = rf;
                    tap_ratio[0] *= fm;
                    tap_ratio[1] *= fm;
                } else {
                    let new_interval = (MIN_DELAY as f32).max(fm * self.max_delay as f32 * semitones_to_ratio(-(density - 0.5).abs() * 216.0));
                    let lp = (10.0 / new_interval).clamp(0.005, 0.7);
                    self.clock.update(new_interval, lp);
                    tap_ratio = [1.0, 0.77];
                    let mut fade_to_odd = ((density - 0.5) * 25.0).clamp(0.0, 1.0);
                    fade_to_odd *= 0.25 + 0.75 * self.tap_modulation.render();
                    tap_gain = [1.0 - fade_to_odd * 0.2, fade_to_odd * 0.8];
                }
                let shape = modulated_shape.next();
                if freeze[n] {
                    // Frozen: slices of the recording, in rhythm.
                    if !self.freeze {
                        for i in 0..NUM_HEADS {
                            self.play_head[i].init();
                            self.slave_play_head[i].init();
                        }
                    }
                    if self.clock.retrig() {
                        let start = self.clock.new_interval_index() * NUM_RATIOS;
                        for i in start..start + NUM_RATIOS {
                            self.play_head[i].init();
                            self.slave_play_head[i].init();
                        }
                    }
                    let slice_index_smooth = self.slice_index_filter.process(slice_modulation);
                    for i in 0..NUM_HEADS {
                        let new_head = if self.play_head[i].done() {
                            Some(false)
                        } else if self.play_head[i].half_way() {
                            Some(true)
                        } else {
                            None
                        };
                        let interval = i / NUM_RATIOS;
                        let ratio = i % NUM_RATIOS;
                        if let Some(slave) = new_head {
                            let slice_duration = ((self.clock.interval(interval) * tap_ratio[ratio]) as i32).max(1);
                            let num_slices = (self.max_delay / slice_duration).max(1);
                            if self.numerator_quantizer[i].num_steps() != num_slices {
                                self.numerator_quantizer[i].init(num_slices, 0.05 / 5.0 * num_slices as f32, false);
                            }
                            let slice_index = self.numerator_quantizer[i].process(slice_index_smooth);
                            let gain = tap_gain[ratio] * self.clock.gain(interval);
                            let head = if slave { &mut self.slave_play_head[i] } else { &mut self.play_head[i] };
                            head.start(slice_duration * slice_index + MIN_DELAY, slice_duration, gain);
                        }
                        self.play_head[i].render(&mut o.tap[2 * i], shape, false);
                        self.slave_play_head[i].render(&mut o.tap[2 * i + 1], shape, true);
                    }
                    o.gate = self.play_head[self.clock.new_interval_index() * NUM_RATIOS].phase < 0.5;
                } else {
                    if self.freeze {
                        for t in self.tap_envelope.iter_mut() {
                            t.init();
                        }
                    }
                    if self.clock.retrig() {
                        let start = self.clock.new_interval_index() * NUM_HEADS;
                        for t in self.tap_envelope[start..start + NUM_HEADS].iter_mut() {
                            t.init();
                        }
                    }
                    one_pole(&mut self.multiplier_lp, slice_modulation, 0.01);
                    let multiplier = (self.multiplier_lp * 15.0 + 1.0).clamp(1.0, 16.0);
                    let (mi, mf) = integral_fractional(multiplier);
                    let mf = (0.5 + 8.0 * (mf - 0.5)).clamp(0.0, 1.0);
                    let mut loudest = 0.0f32;
                    let mut loudest_index = 0;
                    for t in 0..NUM_TAPS {
                        let interval = t / NUM_HEADS;
                        let ratio = (t / NUM_INTERVALS) % NUM_RATIOS;
                        let tp = t & 1;
                        let i = (2 * ratio + if mi & 1 == 1 { tp } else { 1 - tp }) + interval * NUM_HEADS;
                        let delay_time = (self.clock.interval(interval) * tap_ratio[ratio] * (mi + tp as i32) as f32).clamp(MIN_DELAY as f32, self.max_delay as f32);
                        self.tap_envelope[i].phase_increment = 1.0 / delay_time;
                        o.tap[i].delay = (delay_time * 1024.0) as i32;
                        let ratio_fade = if tp == 1 { mf } else { 1.0 - mf };
                        let base_gain = tap_gain[ratio] * ratio_fade * self.clock.gain(interval);
                        o.tap[i].gain = base_gain * self.tap_envelope[i].render(shape);
                        if base_gain > loudest {
                            loudest = base_gain;
                            loudest_index = i;
                        }
                    }
                    o.gate = self.tap_envelope[loudest_index].phase < 0.5;
                }
                self.freeze = freeze[n];
            }
        }
        self.shape = shape_state;
        self.fm = fm_state;
        self.density = density_state;
    }
}

/// A delay tap that can shift pitch: two heads sweeping a window,
/// crossfaded (`WINDOW` 1: a plain tap).
#[derive(Clone, Copy, Debug, Default)]
pub struct Tap<const WINDOW: i32> {
    phase: i32,
}

impl<const WINDOW: i32> Tap<WINDOW> {
    #[inline]
    pub fn render(&mut self, line: &DelayLine, delay: i32, gain: f32, speed: f32, out: &mut [f32; 2]) {
        if delay == 0 {
            return;
        }
        if WINDOW == 1 {
            let s = line.read_hermite_fixed(delay);
            out[0] += gain * s[0];
            out[1] += gain * s[1];
            return;
        }
        let full = 1024 * WINDOW;
        let half = full >> 1;
        self.phase += (1024.0 * (speed - 1.0)) as i32;
        if self.phase >= full {
            self.phase -= full;
        } else if self.phase < 0 {
            self.phase += full;
        }
        let delay = (full + 1).max(delay + half);
        let phase = self.phase as f32 / full as f32;
        let tri = if phase < 0.5 { 2.0 * phase } else { 2.0 - 2.0 * phase };
        let h1 = line.read_hermite_fixed(-self.phase + delay);
        let h2 = line.read_hermite_fixed(-((self.phase + half) % full) + delay);
        out[0] += (h2[0] + (h1[0] - h2[0]) * tri) * gain;
        out[1] += (h2[1] + (h1[1] - h2[1]) * tri) * gain;
    }
}

/// The eight echoes, pitch-shifted when PITCH is off noon, through a
/// low-pass or high-pass set by the SIZE attenuverter.
pub struct MultitapDelay {
    tap: [Tap<2048>; NUM_TAPS],
    no_latency_tap: [Tap<1>; NUM_TAPS],
    filter: [Svf; 2],
}

impl MultitapDelay {
    pub fn new() -> Self {
        let mut filter = [Svf::default(); 2];
        for f in filter.iter_mut() {
            f.init();
        }
        Self { tap: [Tap::default(); NUM_TAPS], no_latency_tap: [Tap::default(); NUM_TAPS], filter }
    }

    pub fn reset(&mut self) {
        self.tap = [Tap::default(); NUM_TAPS];
        self.no_latency_tap = [Tap::default(); NUM_TAPS];
    }

    #[inline]
    pub fn render(&mut self, line: &DelayLine, taps: &DelayOutput, speed: f32, bandwidth: f32, out: &mut [f32; 2]) {
        // Crossfade between the plain and the pitch-shifted echoes.
        let gain = (100.0 * ((speed - 1.0).abs() - 0.01)).clamp(0.0, 1.0);
        let mut mix = [0.0f32; 2];
        for i in 0..NUM_TAPS {
            self.tap[i].render(line, taps.tap[i].delay, taps.tap[i].gain * gain, speed, &mut mix);
            self.no_latency_tap[i].render(line, taps.tap[i].delay, taps.tap[i].gain * (1.0 - gain), speed, &mut mix);
        }
        let amount = (2.0 * bandwidth - 1.0).abs();
        // Low-pass: -7 to -2 octaves; high-pass: -8 to -3.
        let octave = if bandwidth < 0.5 { -7.0 + bandwidth * 10.0 } else { -8.0 + (bandwidth - 0.5) * 10.0 };
        let f = 0.49 * semitones_to_ratio(12.0 * octave);
        let q = 1.0 + 16.0 * amount * f;
        let wet_sqrt = amount * (2.0 - amount);
        let wet_amount = wet_sqrt * (2.0 - wet_sqrt);
        self.filter[0].set_f_q_fast(f, q);
        let f0 = self.filter[0];
        self.filter[1].set(&f0);
        let wet = if bandwidth < 0.5 { [self.filter[0].low_pass(mix[0]), self.filter[1].low_pass(mix[1])] } else { [self.filter[0].high_pass(mix[0]), self.filter[1].high_pass(mix[1])] };
        out[0] += mix[0] + (wet[0] - mix[0]) * wet_amount;
        out[1] += mix[1] + (wet[1] - mix[1]) * wet_amount;
    }
}

impl Default for MultitapDelay {
    fn default() -> Self {
        Self::new()
    }
}
