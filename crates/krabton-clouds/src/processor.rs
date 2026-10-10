//! The main processing class: recording, the four playback modes, feedback,
//! diffusion, pitch shifting, filtering, reverb and the dry/wet mix
//! (`clouds/dsp/granular_processor.h`, `granular_processor.cc`).
//!
//! Copyright 2014 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::audio_buffer::{AudioBuffer, MuLaw};
use crate::correlator::Correlator;
use crate::dsp::{interpolate, one_pole, semitones_to_ratio, soft_convert, soft_limit, FilterMode, ParameterInterpolator, Random, Svf};
use crate::fx::{Diffuser, PitchShifter, Reverb};
use crate::granular::GranularSamplePlayer;
use crate::looping::LoopingSamplePlayer;
use crate::parameters::Parameters;
use crate::pvoc::PhaseVocoder;
use crate::resources::{LUT_XFADE_IN, LUT_XFADE_OUT};
use crate::src_2x::SampleRateConverter;
use crate::wsola::WsolaSamplePlayer;
use crate::{PlaybackMode, MAX_BLOCK_SIZE};

const DOWNSAMPLING_FACTOR: usize = 2;

/// Sample memory of the module, in bytes: a block of main memory and the
/// 64k core-coupled memory. In stereo each channel gets as much as the
/// small block (the rest of the large one goes to the effects); in mono the
/// large block holds the samples.
const LARGE_BUFFER_SIZE: usize = 118784;
const SMALL_BUFFER_SIZE: usize = 65536 - 128;

pub struct GranularProcessor {
    playback_mode: PlaybackMode,
    previous_playback_mode: Option<PlaybackMode>,
    num_channels: i32,
    low_fidelity: bool,
    reset_buffers: bool,
    freeze_lp: f32,
    dry_wet: f32,

    correlator: Correlator,
    player: GranularSamplePlayer,
    ws_player: WsolaSamplePlayer,
    looper: LoopingSamplePlayer,
    phase_vocoder: PhaseVocoder,

    diffuser: Diffuser,
    reverb: Reverb,
    pitch_shifter: PitchShifter,
    fb_filter: [Svf; 2],
    hp_filter: [Svf; 2],
    lp_filter: [Svf; 2],

    buffer_8: [AudioBuffer<MuLaw>; 2],
    buffer_16: [AudioBuffer<i16>; 2],

    fb: [[f32; 2]; MAX_BLOCK_SIZE],

    pub parameters: Parameters,

    src_down: SampleRateConverter,
    src_up: SampleRateConverter,
    rng: Random,
}

impl GranularProcessor {
    pub fn new() -> Self {
        let mut p = Self {
            playback_mode: PlaybackMode::Granular,
            previous_playback_mode: None,
            num_channels: 2,
            low_fidelity: false,
            reset_buffers: true,
            freeze_lp: 0.0,
            dry_wet: 0.0,
            correlator: Correlator::default(),
            player: GranularSamplePlayer::default(),
            ws_player: WsolaSamplePlayer::default(),
            looper: LoopingSamplePlayer::default(),
            phase_vocoder: PhaseVocoder::default(),
            diffuser: Diffuser::default(),
            reverb: Reverb::default(),
            pitch_shifter: PitchShifter::default(),
            fb_filter: [Svf::default(); 2],
            hp_filter: [Svf::default(); 2],
            lp_filter: [Svf::default(); 2],
            buffer_8: [AudioBuffer::new(LARGE_BUFFER_SIZE), AudioBuffer::new(SMALL_BUFFER_SIZE)],
            buffer_16: [AudioBuffer::new(LARGE_BUFFER_SIZE / 2), AudioBuffer::new(SMALL_BUFFER_SIZE / 2)],
            fb: [[0.0; 2]; MAX_BLOCK_SIZE],
            parameters: Parameters::default(),
            src_down: SampleRateConverter::new(-(DOWNSAMPLING_FACTOR as i32)),
            src_up: SampleRateConverter::new(DOWNSAMPLING_FACTOR as i32),
            rng: Random::default(),
        };
        p.reset_filters();
        p
    }

    fn reset_filters(&mut self) {
        for i in 0..2 {
            self.fb_filter[i].init();
            self.lp_filter[i].init();
            self.hp_filter[i].init();
        }
    }

    pub fn set_playback_mode(&mut self, playback_mode: PlaybackMode) {
        self.playback_mode = playback_mode;
    }

    pub fn playback_mode(&self) -> PlaybackMode {
        self.playback_mode
    }

    /// Bit 0: mono; bit 1: 8-bit mu-law at 16 kHz.
    pub fn set_quality(&mut self, quality: u8) {
        self.set_num_channels(if quality & 1 != 0 { 1 } else { 2 });
        self.set_low_fidelity(quality >> 1 != 0);
    }

    fn set_num_channels(&mut self, num_channels: i32) {
        self.reset_buffers = self.reset_buffers || self.num_channels != num_channels;
        self.num_channels = num_channels;
    }

    fn set_low_fidelity(&mut self, low_fidelity: bool) {
        self.reset_buffers = self.reset_buffers || low_fidelity != self.low_fidelity;
        self.low_fidelity = low_fidelity;
    }

    pub fn quality(&self) -> u8 {
        let mut quality = 0;
        if self.num_channels == 1 {
            quality |= 1;
        }
        if self.low_fidelity {
            quality |= 2;
        }
        quality
    }

    fn sample_rate(&self) -> f32 {
        32000.0 / if self.low_fidelity { DOWNSAMPLING_FACTOR as f32 } else { 1.0 }
    }

    fn process_granular(&mut self, input: &[[f32; 2]], output: &mut [[f32; 2]]) {
        let freeze = self.parameters.freeze;
        // With the exception of the spectral mode, all modes require the
        // incoming audio signal to be written to the recording buffer.
        if self.playback_mode != PlaybackMode::Spectral {
            for i in 0..self.num_channels as usize {
                if self.low_fidelity {
                    self.buffer_8[i].write_fade(input, i, !freeze);
                } else {
                    self.buffer_16[i].write_fade(input, i, !freeze);
                }
            }
        }

        let p = &mut self.parameters;
        match self.playback_mode {
            PlaybackMode::Granular => {
                // In Granular mode, DENSITY is a meta parameter.
                p.granular.use_deterministic_seed = p.density < 0.5;
                p.granular.overlap = if p.density >= 0.53 {
                    (p.density - 0.53) * 2.12
                } else if p.density <= 0.47 {
                    (0.47 - p.density) * 2.12
                } else {
                    0.0
                };
                // And TEXTURE too.
                p.granular.window_shape = if p.texture < 0.75 { p.texture * 1.333 } else { 1.0 };
                if self.low_fidelity {
                    self.player.play(&self.buffer_8, p, output, &mut self.rng);
                } else {
                    self.player.play(&self.buffer_16, p, output, &mut self.rng);
                }
            }
            PlaybackMode::Stretch => {
                if self.low_fidelity {
                    self.ws_player.play(&self.buffer_8, p, &mut self.correlator, output);
                } else {
                    self.ws_player.play(&self.buffer_16, p, &mut self.correlator, output);
                }
            }
            PlaybackMode::LoopingDelay => {
                if self.low_fidelity {
                    self.looper.play(&self.buffer_8, p, output);
                } else {
                    self.looper.play(&self.buffer_16, p, output);
                }
            }
            PlaybackMode::Spectral => {
                p.spectral.quantization = p.texture;
                p.spectral.refresh_rate = 0.01 + 0.99 * p.density;
                let warp = p.size - 0.5;
                p.spectral.warp = 4.0 * warp * warp * warp + 0.5;

                let mut randomization = p.density - 0.5;
                randomization *= randomization * 4.2;
                randomization -= 0.05;
                p.spectral.phase_randomization = randomization.clamp(0.0, 1.0);
                self.phase_vocoder.process(input, output);

                if self.num_channels == 1 {
                    for frame in output.iter_mut() {
                        frame[1] = frame[0];
                    }
                }
            }
        }
    }

    /// Processes a block of up to 32 codec frames.
    pub fn process(&mut self, input: &[[i16; 2]], output: &mut [[i16; 2]]) {
        let size = input.len().min(output.len()).min(MAX_BLOCK_SIZE);
        if self.reset_buffers || self.previous_playback_mode != Some(self.playback_mode) {
            for frame in output[..size].iter_mut() {
                *frame = [0; 2];
            }
            return;
        }

        // Convert input buffers to float, and mixdown for mono processing.
        let mut in_ = [[0.0f32; 2]; MAX_BLOCK_SIZE];
        for (x, frame) in in_.iter_mut().zip(&input[..size]) {
            *x = [frame[0] as f32 / 32768.0, frame[1] as f32 / 32768.0];
        }
        if self.num_channels == 1 {
            for x in in_[..size].iter_mut() {
                let m = (x[0] + x[1]) * 0.5;
                *x = [m, m];
            }
        }

        // Apply feedback, with high-pass filtering to prevent build-ups at
        // very low frequencies (causing large DC swings).
        one_pole(&mut self.freeze_lp, if self.parameters.freeze { 1.0 } else { 0.0 }, 0.0005);
        let feedback = self.parameters.feedback;
        let cutoff = (20.0 + 100.0 * feedback * feedback) / self.sample_rate();
        self.fb_filter[0].set_f_q_fast(cutoff, 1.0);
        let f0 = self.fb_filter[0];
        self.fb_filter[1].set(&f0);
        self.fb_filter[0].process_frames(FilterMode::HighPass, &mut self.fb[..size], 0);
        self.fb_filter[1].process_frames(FilterMode::HighPass, &mut self.fb[..size], 1);
        let fb_gain = feedback * (1.0 - self.freeze_lp);
        for (x, fb) in in_[..size].iter_mut().zip(&self.fb[..size]) {
            for c in 0..2 {
                x[c] += fb_gain * (soft_limit(fb_gain * 1.4 * fb[c] + x[c]) - x[c]);
            }
        }

        let mut out = [[0.0f32; 2]; MAX_BLOCK_SIZE];
        if self.low_fidelity {
            let downsampled_size = size / DOWNSAMPLING_FACTOR;
            let mut in_downsampled = [[0.0f32; 2]; MAX_BLOCK_SIZE / DOWNSAMPLING_FACTOR];
            let mut out_downsampled = [[0.0f32; 2]; MAX_BLOCK_SIZE / DOWNSAMPLING_FACTOR];
            self.src_down.process(&in_[..downsampled_size * DOWNSAMPLING_FACTOR], &mut in_downsampled[..downsampled_size]);
            self.process_granular(&in_downsampled[..downsampled_size], &mut out_downsampled[..downsampled_size]);
            self.src_up.process(&out_downsampled[..downsampled_size], &mut out[..downsampled_size * DOWNSAMPLING_FACTOR]);
        } else {
            self.process_granular(&in_[..size], &mut out[..size]);
        }
        let out = &mut out[..size];

        // Diffusion and pitch-shifting post-processings.
        let p = self.parameters;
        if self.playback_mode != PlaybackMode::Spectral {
            let texture = p.texture;
            let diffusion = if self.playback_mode == PlaybackMode::Granular {
                if texture > 0.75 {
                    (texture - 0.75) * 4.0
                } else {
                    0.0
                }
            } else {
                p.density
            };
            self.diffuser.set_amount(diffusion);
            self.diffuser.process(out);
        }

        if self.playback_mode == PlaybackMode::LoopingDelay && (!p.freeze || self.looper.synchronized()) {
            self.pitch_shifter.set_ratio(semitones_to_ratio(p.pitch));
            self.pitch_shifter.set_size(p.size);
            self.pitch_shifter.process(out);
        }

        // Apply filters.
        if self.playback_mode == PlaybackMode::LoopingDelay || self.playback_mode == PlaybackMode::Stretch {
            let cutoff = p.texture;
            let lp_cutoff =
                (0.5 * semitones_to_ratio((if cutoff < 0.5 { cutoff - 0.5 } else { 0.0 }) * 216.0)).clamp(0.0, 0.499);
            let hp_cutoff =
                (0.25 * semitones_to_ratio((if cutoff < 0.5 { -0.5 } else { cutoff - 1.0 }) * 216.0)).clamp(0.0, 0.499);
            let lpq = 1.0 + 3.0 * (1.0 - feedback) * (0.5 - lp_cutoff);
            self.lp_filter[0].set_f_q_fast(lp_cutoff, lpq);
            self.lp_filter[0].process_frames(FilterMode::LowPass, out, 0);
            let f = self.lp_filter[0];
            self.lp_filter[1].set(&f);
            self.lp_filter[1].process_frames(FilterMode::LowPass, out, 1);

            self.hp_filter[0].set_f_q_fast(hp_cutoff, 1.0);
            self.hp_filter[0].process_frames(FilterMode::HighPass, out, 0);
            let f = self.hp_filter[0];
            self.hp_filter[1].set(&f);
            self.hp_filter[1].process_frames(FilterMode::HighPass, out, 1);
        }

        // This is what is fed back. Reverb is not fed back.
        self.fb[..size].copy_from_slice(out);

        // Apply reverb.
        let mut reverb_amount = p.reverb * 0.95;
        reverb_amount += feedback * (2.0 - feedback) * self.freeze_lp;
        let reverb_amount = reverb_amount.clamp(0.0, 1.0);

        self.reverb.set_amount(reverb_amount * 0.54);
        self.reverb.set_diffusion(0.7);
        self.reverb.set_time(0.35 + 0.63 * reverb_amount);
        self.reverb.set_input_gain(0.2);
        self.reverb.set_lp(0.6 + 0.37 * feedback);
        self.reverb.process(out);

        const POST_GAIN: f32 = 1.2;
        let mut dry_wet_mod = ParameterInterpolator::new(&mut self.dry_wet, p.dry_wet, size);
        for ((o, x), frame) in output[..size].iter_mut().zip(out.iter()).zip(&input[..size]) {
            let dry_wet = dry_wet_mod.next();
            let fade_in = interpolate(&LUT_XFADE_IN, dry_wet, 16.0);
            let fade_out = interpolate(&LUT_XFADE_OUT, dry_wet, 16.0);
            let mut l = frame[0] as f32 / 32768.0 * fade_out;
            let mut r = frame[1] as f32 / 32768.0 * fade_out;
            l += x[0] * POST_GAIN * fade_in;
            r += x[1] * POST_GAIN * fade_in;
            *o = [soft_convert(l), soft_convert(r)];
        }
    }

    /// Work done by the module outside of the audio interrupt: buffer
    /// allocation on mode and quality changes, the spectral frames and the
    /// WSOLA correlation search.
    pub fn prepare(&mut self) {
        let playback_mode_changed = self.previous_playback_mode != Some(self.playback_mode);
        let benign_change = self.previous_playback_mode != Some(PlaybackMode::Spectral)
            && self.playback_mode != PlaybackMode::Spectral
            && self.previous_playback_mode.is_some();

        if !self.reset_buffers && playback_mode_changed && benign_change {
            self.reset_filters();
            self.pitch_shifter.clear();
            self.previous_playback_mode = Some(self.playback_mode);
        }

        if (playback_mode_changed && !benign_change) || self.reset_buffers {
            self.parameters.freeze = false;
        }

        if self.reset_buffers || (playback_mode_changed && !benign_change) {
            let buffer_size = if self.num_channels == 1 {
                // Large buffer: 120k of sample memory.
                [LARGE_BUFFER_SIZE, 0]
            } else {
                // 64k of sample memory per channel.
                [SMALL_BUFFER_SIZE, SMALL_BUFFER_SIZE]
            };

            self.diffuser.init();
            self.reverb.init();
            self.correlator.init();
            self.pitch_shifter.init();

            if self.playback_mode == PlaybackMode::Spectral {
                self.phase_vocoder.init(self.num_channels as usize);
            } else {
                for i in 0..self.num_channels as usize {
                    if self.low_fidelity {
                        self.buffer_8[i].init(buffer_size[i]);
                    } else {
                        self.buffer_16[i].init(buffer_size[i] >> 1);
                    }
                }
                let num_grains = ((if self.num_channels == 1 { 40 } else { 32 }) * (if self.low_fidelity { 23 } else { 16 })) >> 4;
                self.player.init(self.num_channels, num_grains);
                self.ws_player.init(self.num_channels);
                self.looper.init(self.num_channels);
            }
            self.reset_buffers = false;
            self.previous_playback_mode = Some(self.playback_mode);
        }

        if self.playback_mode == PlaybackMode::Spectral {
            self.phase_vocoder.buffer(&self.parameters, &mut self.rng);
        } else if self.playback_mode == PlaybackMode::Stretch {
            if self.low_fidelity {
                self.ws_player.load_correlator(&self.buffer_8, &mut self.correlator);
            } else {
                self.ws_player.load_correlator(&self.buffer_16, &mut self.correlator);
            }
            self.correlator.evaluate_all_candidates();
        }
    }
}
