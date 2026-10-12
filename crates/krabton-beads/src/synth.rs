//! The synth Beads plays when nothing comes in (`beads/dsp/synth.cc`,
//! `wavetable_voice.h`): a wavetable oscillator (the dry signal) and grains
//! of the same wavetables (the wet one). FEEDBACK picks one of twelve banks
//! of sixteen waves, TIME the wave in the bank, PITCH the pitch (from C3);
//! the quality setting lowers the bit depth for grit.
//!
//! Copyright 2017 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::fx::Reverb;
use crate::grain::{interpolate_wave_hermite, NUM_WAVES_PER_TABLE, TABLE_SIZE, WAVETABLE_ROOT_NOTE};
use crate::player::WavetableGranularPlayer;
use crate::processor::MixParameters;
use crate::random::{ModulatedParameter, Parameters, RandomWalkModulation};
use crate::trigger::{SeedButtonState, TriggerGenerator, TriggerOutput};
use crate::util::{integral_fractional, one_pole, semitones_to_ratio, GateFlags, HysteresisQuantizer2, Limiter, ParameterInterpolator, Random};

pub const NUM_BANKS: usize = 12;

/// The integrated waves: 12 banks of 16 waves of 256 points (and 4 for
/// interpolation), from the module's resources.
pub fn waves() -> Vec<i16> {
    include_bytes!("waves.bin").chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect()
}

/// The oscillator, playing on forever.
pub struct WavetableVoice {
    frequency: f32,
    phase: f32,
    root_note: f32,
    lp: f32,
    lp_2: f32,
    previous: f32,
    wave_lp: f32,
    bank: usize,
    next_bank: usize,
    xfade_counter: i32,
    xfade_duration: i32,
    position_random_walk: RandomWalkModulation,
}

impl WavetableVoice {
    pub fn new(sample_rate: f32) -> Self {
        let root_note = WAVETABLE_ROOT_NOTE / sample_rate;
        let mut position_random_walk = RandomWalkModulation::default();
        position_random_walk.init(1.01 / sample_rate);
        Self { frequency: root_note, phase: 0.0, root_note, lp: 0.0, lp_2: 0.0, previous: 0.0, wave_lp: 0.0, bank: 0, next_bank: 0, xfade_counter: 0, xfade_duration: (sample_rate * 0.025) as i32, position_random_walk }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(&mut self, waves: &[i16], bank: usize, bit_shift: u32, freeze: &[bool], p: &Parameters, out: &mut [[f32; 2]], rng: &mut Random) {
        let size = out.len();
        let scale_up = (1u32 << bit_shift) as f32;
        let scale_down = 1.0 / scale_up;
        let gain = scale_down * 2.0 / self.xfade_duration as f32;
        if bank != self.bank && self.xfade_counter == 0 {
            self.next_bank = bank;
            self.xfade_counter = self.xfade_duration;
        }
        let target = (semitones_to_ratio(p.pitch.base) * self.root_note).min(0.25);
        let mut f_state = self.frequency;
        {
            let mut fm = ParameterInterpolator::new(&mut f_state, target, size);
            for (i, o) in out.iter_mut().enumerate() {
                let position = p.time.value_with(Some(&mut self.position_random_walk), if freeze[i] { 0 } else { 1 }, 1.0, true, rng);
                one_pole(&mut self.wave_lp, position, 0.025);
                let wave = self.wave_lp * (NUM_WAVES_PER_TABLE - 1) as f32;
                let frequency = fm.next();
                self.phase += frequency;
                if self.phase >= 1.0 {
                    self.phase -= 1.0;
                }
                if self.xfade_counter != 0 {
                    self.xfade_counter -= 1;
                    if self.xfade_counter == self.xfade_duration / 2 {
                        self.bank = self.next_bank;
                    }
                }
                let (wi, wf) = integral_fractional(wave);
                let wi = (wi.max(0) as usize).min(NUM_WAVES_PER_TABLE - 2);
                let start = (wi + self.bank * NUM_WAVES_PER_TABLE) * (TABLE_SIZE + 4);
                let (pi, pf) = integral_fractional(self.phase * TABLE_SIZE as f32);
                let pi = pi.clamp(0, TABLE_SIZE as i32 - 1) as usize;
                let x0 = interpolate_wave_hermite(&waves[start..], pi, pf);
                let x1 = interpolate_wave_hermite(&waves[start + TABLE_SIZE + 4..], pi, pf);
                let sample = x0 + (x1 - x0) * wf;
                let scale = 1.0 / (frequency * 131072.0);
                let cutoff = (TABLE_SIZE as f32 * frequency).min(1.0);
                one_pole(&mut self.lp, (sample - self.previous) * scale, cutoff);
                let lp = self.lp;
                one_pole(&mut self.lp_2, lp, cutoff);
                self.previous = sample;
                let xfade_gain = (self.xfade_counter - self.xfade_duration / 2).abs() as f32;
                let s = (((self.lp_2 * scale_up) as i32) as f32 * xfade_gain * gain).clamp(-1.0, 1.0);
                o[0] += s;
                o[1] += s;
            }
        }
        self.frequency = f_state;
    }
}

pub struct Synth {
    trigger: TriggerGenerator,
    dry: f32,
    wet: f32,
    reverb: Reverb,
    pre_reverb_limiter: Limiter,
    output_limiter: Limiter,
    player: WavetableGranularPlayer,
    voice: WavetableVoice,
    bank_quantizer: HysteresisQuantizer2,
    waves: Vec<i16>,
    triggers: Vec<TriggerOutput>,
    pub rng: Random,
}

/// The synth runs at the codec's own rate.
pub const SYNTH_RATE: f32 = 48000.0;

impl Synth {
    pub fn new() -> Self {
        let sr = SYNTH_RATE;
        let mut pre = Limiter::default();
        pre.init(1.0 / (sr * 4.0), 0.9);
        let mut post = Limiter::default();
        post.init(1.0 / (sr * 6.0), 0.9);
        let mut bank_quantizer = HysteresisQuantizer2::default();
        bank_quantizer.init(NUM_BANKS as i32, 0.048, false);
        Self {
            trigger: TriggerGenerator::new(sr),
            dry: 0.0,
            wet: 0.0,
            reverb: Reverb::new(sr),
            pre_reverb_limiter: pre,
            output_limiter: post,
            player: WavetableGranularPlayer::new(sr),
            voice: WavetableVoice::new(sr),
            bank_quantizer,
            waves: waves(),
            triggers: vec![TriggerOutput::default(); 64],
            rng: Random::default(),
        }
    }

    pub fn grains(&self, out: &mut Vec<crate::grain::GrainView>) {
        let one_hertz = 1.0 / SYNTH_RATE;
        out.extend(self.player.player.grains.iter().filter_map(|g| g.view(one_hertz)));
    }

    /// The bank FEEDBACK picks now.
    pub fn bank(&self) -> usize {
        self.voice.bank
    }

    /// `quality` 0..3 as the media setting: 4 bits, 16 then 6, 7 bits, 16.
    #[allow(clippy::too_many_arguments)]
    pub fn render(&mut self, button: SeedButtonState, seed_input: Option<&[GateFlags]>, freeze: &[bool], quality: usize, density: &ModulatedParameter, parameters: &Parameters, mix: &MixParameters, out: &mut [[f32; 2]]) {
        let size = out.len().min(64);
        let mut p = *parameters;
        p.pitch.base += if p.pitch.has_cv { p.pitch.cv } else { 0.0 };
        p.pitch.has_cv = false;
        let expo_fm_amount = (20.0 * ((density.base - 0.5).abs() - 0.225)).clamp(0.0, 1.0);
        let rng = &mut self.rng;
        self.trigger.render(button, seed_input, density.base, (1.0 - expo_fm_amount) * density.cv, expo_fm_amount * density.cv, &mut self.triggers[..size], rng);
        let bank = self.bank_quantizer.process(mix.feedback).clamp(0, NUM_BANKS as i32 - 1) as usize;
        const CONTINUOUS_SHIFT: [u32; 4] = [4, 16, 7, 16];
        const GRAIN_SHIFT: [u32; 4] = [4, 6, 7, 16];
        let q = quality.min(3);
        for o in out[..size].iter_mut() {
            *o = [0.0; 2];
        }
        self.voice.render(&self.waves, bank, CONTINUOUS_SHIFT[q], &freeze[..size], &p, &mut out[..size], rng);
        // Dry (the oscillator) and wet (the grains).
        let dw = mix.dry_wet;
        let dry = (1.0 - dw) * (1.0 + dw) * 0.5;
        let wet = dw * (2.0 - dw);
        let (mut dry_s, mut wet_s) = (self.dry, self.wet);
        {
            let mut dry_m = ParameterInterpolator::new(&mut dry_s, dry, size);
            let mut wet_m = ParameterInterpolator::new(&mut wet_s, wet, size);
            for i in 0..size {
                let mut w = [0.0f32; 2];
                self.player.render(&self.waves, bank, GRAIN_SHIFT[q], &self.triggers[i], freeze[i], &p, &mut w, rng);
                let (d, wg) = (dry_m.next(), wet_m.next());
                out[i][0] = d * out[i][0] + w[0] * wg;
                out[i][1] = d * out[i][1] + w[1] * wg;
            }
        }
        self.dry = dry_s;
        self.wet = wet_s;
        self.pre_reverb_limiter.process(&mut out[..size]);
        let r = mix.reverb_amount;
        let reverb_time = r * r * r;
        self.reverb.set_mix(r * r * 0.8);
        self.reverb.set_time(0.3 + 0.6 * reverb_time);
        self.reverb.set_lp(0.6);
        self.reverb.process(&mut out[..size]);
        self.output_limiter.process(&mut out[..size]);
    }
}

impl Default for Synth {
    fn default() -> Self {
        Self::new()
    }
}
