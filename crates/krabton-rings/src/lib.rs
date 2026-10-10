//! A Rust port of the DSP code of the Mutable Instruments Rings resonator,
//! by Emilie Gillet (MIT license, see the headers of each module and
//! <https://github.com/pichenettes/eurorack>).
//!
//! Rings renders floating-point audio at 48 kHz in blocks of 24 samples. A
//! [`part::Part`] plays one, two or four voices of a resonator (modal,
//! sympathetic strings, inharmonic string, FM voice, quantized sympathetic
//! strings, string and reverb), excited by the audio input or by an
//! internal exciter (a filtered pulse, a noise burst or an envelope) on
//! each strum. The hidden easter egg, [`string_synth::StringSynthPart`], is
//! a string synth with a few effects.
//!
//! [`Rings`] wraps both for a host: notes strum and set the pitch, an
//! optional mono input excites the resonator, and audio at any host rate is
//! resampled to and from 48 kHz.

#![allow(clippy::too_many_arguments, clippy::needless_range_loop)]

pub mod dsp;
pub mod filter;
pub mod fm_voice;
pub mod fx;
pub mod limiter;
pub mod note_filter;
pub mod onset_detector;
pub mod part;
pub mod plucker;
pub mod resample;
pub mod resonator;
pub mod resources;
pub mod string;
pub mod string_synth;

use dsp::{BLOCK_SIZE, SAMPLE_RATE};
use onset_detector::OnsetDetector;
use part::{Part, Patch, PerformanceState, NUM_CHORDS};
use resample::BlockAdapter;
use string_synth::StringSynthPart;

/// The resonator models, in the order of the module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    Modal,
    SympatheticString,
    InharmonicString,
    FmVoice,
    SympatheticStringQuantized,
    StringAndReverb,
}

/// The effects of the easter egg, in the order of the module (each sits on
/// the model button position of the same index).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EggFx {
    Formant,
    Chorus,
    Reverb,
    Formant2,
    Ensemble,
    Reverb2,
}

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub model: Model,
    /// 1, 2 or 4 (3, the module's hidden setting, works too; others are
    /// clamped to 1..4).
    pub polyphony: u8,
    /// Plays the "Disastrous Peace" string synth instead of the resonator.
    pub easter_egg: bool,
    pub egg_fx: EggFx,
    /// 0..1, like the knobs.
    pub structure: f32,
    pub brightness: f32,
    pub damping: f32,
    pub position: f32,
    /// The chord of the quantized sympathetic strings and of the easter
    /// egg, 0..1 like a knob (11 chords). `None`: from `structure`, as on
    /// the module.
    pub chord: Option<f32>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            model: Model::Modal,
            polyphony: 1,
            easter_egg: false,
            egg_fx: EggFx::Formant,
            structure: 0.5,
            brightness: 0.5,
            damping: 0.5,
            position: 0.5,
            chord: None,
        }
    }
}

/// Notes waiting for their block (one strum per block).
const NOTE_QUEUE: usize = 32;
/// The pitch before the first note: C3.
const DEFAULT_NOTE: f32 = 48.0;
/// The module's tonic with the FREQUENCY knob fully down; notes are played
/// as V/Oct above it.
const TONIC: f32 = 12.0;
/// 48 kHz blocks of output below this level count as quiet.
const QUIET_LEVEL: f32 = 2e-5;
/// Quiet blocks after which [`Rings::is_silent`] says so (half a second).
const QUIET_BLOCKS: u32 = (SAMPLE_RATE as u32 / BLOCK_SIZE as u32) / 2;
/// Without notes for this many blocks (a second), onsets in the audio input
/// strum, as on the module with nothing patched in STRUM and V/OCT.
const ONSET_STRUM_AFTER: u32 = SAMPLE_RATE as u32 / BLOCK_SIZE as u32;
/// The module's noise gate on the audio input.
const NOISE_GATE_THRESHOLD: f32 = 0.00003;

fn knob(x: f32, max: f32) -> f32 {
    if x.is_nan() {
        0.0
    } else {
        x.clamp(0.0, max)
    }
}

/// The control state between blocks (the module's CV scaler and strummer).
struct Controls {
    queue: [(f32, f32); NOTE_QUEUE],
    queue_head: usize,
    queue_len: usize,
    note: f32,
    chord: i32,
    in_level: f32,
    blocks_since_note: u32,
    onset_inhibit: u32,
    quiet_blocks: u32,
    gated: [f32; BLOCK_SIZE],
}

/// Rings, playable from a host.
pub struct Rings {
    part: Part,
    egg: StringSynthPart,
    onset_detector: OnsetDetector,
    adapter: BlockAdapter,
    controls: Controls,
}

impl Rings {
    /// A Rings running at `host_sample_rate` (8 kHz to 768 kHz).
    pub fn new(host_sample_rate: f32) -> Self {
        let rate = if host_sample_rate.is_finite() { host_sample_rate.clamp(8000.0, 768000.0) } else { SAMPLE_RATE };
        let mut onset_detector = OnsetDetector::default();
        let control_rate = SAMPLE_RATE / BLOCK_SIZE as f32;
        onset_detector.init(8.0 / SAMPLE_RATE, 160.0 / SAMPLE_RATE, 1600.0 / SAMPLE_RATE, control_rate, 0.01);
        Self {
            part: Part::new(),
            egg: StringSynthPart::new(),
            onset_detector,
            adapter: BlockAdapter::new(rate),
            controls: Controls {
                queue: [(0.0, 0.0); NOTE_QUEUE],
                queue_head: 0,
                queue_len: 0,
                note: DEFAULT_NOTE,
                chord: 0,
                in_level: 0.0,
                blocks_since_note: ONSET_STRUM_AFTER,
                onset_inhibit: 0,
                quiet_blocks: QUIET_BLOCKS,
                gated: [0.0; BLOCK_SIZE],
            },
        }
    }

    /// A note (MIDI number, fractional ok) struck with a velocity 0..1
    /// (strum + V/Oct, internal exciter).
    pub fn note_on(&mut self, note: f32, velocity: f32) {
        if !note.is_finite() {
            return;
        }
        let velocity = knob(velocity, 1.0);
        let note = note.clamp(-24.0, 160.0);
        let c = &mut self.controls;
        if c.queue_len == NOTE_QUEUE {
            c.queue_head = (c.queue_head + 1) % NOTE_QUEUE;
            c.queue_len -= 1;
        }
        c.queue[(c.queue_head + c.queue_len) % NOTE_QUEUE] = (note, velocity);
        c.queue_len += 1;
    }

    /// Renders host-rate audio, adding nothing: writes odd/even outputs to
    /// left/right (in place). `input` is an optional mono audio input (same
    /// length); None uses the internal exciter.
    pub fn process(&mut self, p: &Params, input: Option<&[f32]>, left: &mut [f32], right: &mut [f32]) {
        let patch = Patch {
            structure: knob(p.structure, 0.9995),
            brightness: knob(p.brightness, 1.0),
            damping: knob(p.damping, 1.0),
            position: knob(p.position, 1.0),
        };
        let chord_knob = match p.chord {
            Some(c) => knob(c, 1.0),
            None => patch.structure,
        };
        let polyphony = (p.polyphony as usize).clamp(1, part::MAX_POLYPHONY);
        let external = input.is_some();
        let Self { part, egg, onset_detector, adapter, controls } = self;

        adapter.process(input, left, right, |block_in, out, aux| {
            if part.polyphony() != polyphony {
                part.set_polyphony(polyphony);
            }
            if egg.polyphony() != polyphony {
                egg.set_polyphony(polyphony);
            }
            part.set_model(p.model);
            egg.set_fx(p.egg_fx);

            // Hysteresis on chord.
            let chord = chord_knob * (NUM_CHORDS - 1) as f32;
            let hysteresis = if chord - controls.chord as f32 > 0.0 { -0.1 } else { 0.1 };
            controls.chord = ((chord + hysteresis + 0.5) as i32).clamp(0, NUM_CHORDS as i32 - 1);

            // Noise gate on the input (not in the easter egg).
            let input_block: &[f32; BLOCK_SIZE] = if external && !p.easter_egg {
                for (g, &x) in controls.gated.iter_mut().zip(block_in.iter()) {
                    let error = x * x - controls.in_level;
                    controls.in_level += error * if error > 0.0 { 0.1 } else { 0.0001 };
                    let gain = if controls.in_level <= NOISE_GATE_THRESHOLD {
                        (1.0 / NOISE_GATE_THRESHOLD) * controls.in_level
                    } else {
                        1.0
                    };
                    *g = gain * x;
                }
                &controls.gated
            } else {
                block_in
            };

            // Strum: one queued note per block, else an onset in the input.
            let mut ps = PerformanceState {
                strum: false,
                internal_exciter: !external,
                tonic: TONIC,
                note: controls.note - TONIC,
                fm: 0.0,
                chord: controls.chord,
                velocity: 1.0,
            };
            let has_onset = external && !p.easter_egg && onset_detector.process(input_block);
            controls.onset_inhibit = controls.onset_inhibit.saturating_sub(1);
            if controls.queue_len > 0 {
                let (note, velocity) = controls.queue[controls.queue_head];
                controls.queue_head = (controls.queue_head + 1) % NOTE_QUEUE;
                controls.queue_len -= 1;
                controls.note = note;
                controls.blocks_since_note = 0;
                ps.strum = true;
                ps.note = note - TONIC;
                ps.velocity = velocity;
            } else {
                controls.blocks_since_note = controls.blocks_since_note.saturating_add(1);
                if has_onset && controls.blocks_since_note >= ONSET_STRUM_AFTER && controls.onset_inhibit == 0 {
                    ps.strum = true;
                    // The module's strummer inhibits 4 x 10 ms after an onset.
                    controls.onset_inhibit = 80;
                }
            }

            if p.easter_egg {
                egg.process(&ps, &patch, input_block, out, aux);
            } else {
                part.process(&ps, &patch, input_block, out, aux);
            }

            let mut peak = 0.0f32;
            let mut finite = true;
            for (&l, &r) in out.iter().zip(aux.iter()) {
                finite &= l.is_finite() && r.is_finite();
                peak = peak.max(l.abs()).max(r.abs());
            }
            if !finite {
                // Never seen; start over rather than output garbage.
                part.reset();
                egg.reset();
                out.fill(0.0);
                aux.fill(0.0);
                peak = 0.0;
            }
            if peak > QUIET_LEVEL || controls.queue_len > 0 {
                controls.quiet_blocks = 0;
            } else {
                controls.quiet_blocks = controls.quiet_blocks.saturating_add(1);
            }
        });
    }

    /// All voices quiet for a while (half a second below -94 dBFS) and no
    /// note waiting.
    pub fn is_silent(&self) -> bool {
        self.controls.quiet_blocks >= QUIET_BLOCKS && self.controls.queue_len == 0
    }
}

#[cfg(test)]
mod tests;
