//! Physical-model instruments from the Synthesis ToolKit in C++ (STK) by
//! Perry R. Cook and Gary P. Scavone, ported to Rust for Krabton.
//!
//! The monophonic waveguide, modal and formant voices of STK 5.0
//! (<https://github.com/thestk/stk>): Flute, Clarinet, Saxofony, Brass,
//! BlowBotl, BlowHole, Recorder, Whistle, Bowed, Plucked, Mandolin, Sitar,
//! StifKarp, ModalBar, BandedWG and VoicForm, with the unit generators they
//! use ([`ugens`]) and the rawwaves they play ([`rawwaves`]). Algorithms,
//! tables and control mappings follow the C++ code; each module names the
//! files it was ported from and the (few) places where it differs.
//!
//! Use [`MODELS`] to list the instruments and [`create`] to make a voice:
//! controls take STK's `controlChange` numbers and 0..128 values.
//!
//! Things that differ from STK everywhere:
//!
//! - Each voice runs at its own sample rate (there is no global
//!   `Stk::sampleRate`). Constants STK gives per sample (envelope rates,
//!   decay radii, the Whistle physics step) are taken as meant for STK's
//!   default 44.1 kHz and converted, so a voice sounds the same at 48 or
//!   96 kHz. Filters whose coefficients STK hard-codes (the violin body,
//!   the recorder's viscothermal losses) are used as they are.
//! - Delay lines are allocated in [`create`] for notes down to
//!   [`MIN_FREQUENCY`]; note frequencies are clamped to
//!   `MIN_FREQUENCY..=sample_rate / 8` and control values to 0..=128 (STK
//!   ignores out-of-range values). Nothing allocates after `create`.
//! - Breath pressure / volume / bow velocity on control 128 (and Bowed's
//!   100) act only on a sounding note, between `note_on` and `note_off`,
//!   so sending default controls never starts a sound by itself.
//! - `note_off` release rates that STK derives from the note-off amplitude
//!   are kept above the rate for an amplitude of 0.1, so a note-off with
//!   amplitude 0 (common from MIDI hosts) still stops blown and bowed
//!   voices instead of sustaining forever.
//! - Noise comes from a per-voice xorshift generator instead of `rand()`.
//! - If a voice ever produces a non-finite or runaway (> 64) sample, its
//!   state is cleared and it outputs silence.
//!
//! Model-specific changes (Brass lip filter, Recorder internal rate,
//! note-off damping for Mandolin and struck BandedWG, BlowHole default
//! tonehole, sample-rate scaling of levels) are described in each module.
//!
//! # License
//!
//! The Synthesis ToolKit in C++ (STK)
//!
//! Copyright (c) 1995--2023 Perry R. Cook and Gary P. Scavone
//!
//! Permission is hereby granted, free of charge, to any person obtaining
//! a copy of this software and associated documentation files (the
//! "Software"), to deal in the Software without restriction, including
//! without limitation the rights to use, copy, modify, merge, publish,
//! distribute, sublicense, and/or sell copies of the Software, and to
//! permit persons to whom the Software is furnished to do so, subject to
//! the following conditions:
//!
//! The above copyright notice and this permission notice shall be
//! included in all copies or substantial portions of the Software.
//!
//! Any person wishing to distribute modifications to the Software is
//! asked to send the modifications to the original developer so that they
//! can be incorporated into the canonical version.  This is, however, not
//! a binding provision of this license.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
//! EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
//! MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
//! IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR
//! ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF
//! CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
//! WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#![allow(clippy::excessive_precision)]

pub mod bandedwg;
pub mod blowbotl;
pub mod blowhole;
pub mod bowed;
pub mod brass;
pub mod clarinet;
pub mod flute;
pub mod mandolin;
pub mod modalbar;
pub mod phonemes;
pub mod plucked;
pub mod rawwaves;
pub mod recorder;
pub mod saxofony;
pub mod sitar;
pub mod stifkarp;
pub mod ugens;
pub mod voicform;
pub mod whistle;

#[cfg(test)]
mod tests;

/// One `controlChange` number an instrument responds to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlInfo {
    /// STK control number; values range over 0..=128.
    pub number: u8,
    /// The name STK's documentation uses.
    pub name: &'static str,
    /// The value [`create`] applies (STK's own default where it has one).
    pub default: f32,
}

/// Description of one instrument model.
#[derive(Clone, Copy, Debug)]
pub struct ModelInfo {
    /// Stable identifier for [`create`] (the STK class name, lowercase).
    pub id: &'static str,
    pub name: &'static str,
    /// "Winds", "Strings", "Mallets" or "Voice".
    pub family: &'static str,
    pub description: &'static str,
    pub controls: &'static [ControlInfo],
    /// Sub-presets selectable with [`Instrument::set_preset`] (and an STK
    /// control); empty if none.
    pub presets: &'static [&'static str],
}

/// A monophonic STK voice.
pub trait Instrument: Send {
    /// Starts a note; `amplitude` is 0..1.
    fn note_on(&mut self, frequency: f32, amplitude: f32);
    /// Releases the note; `amplitude` is 0..1 (STK uses it as the release
    /// or damping speed).
    fn note_off(&mut self, amplitude: f32);
    /// STK `controlChange`: `value` is 0..128.
    fn control_change(&mut self, number: u8, value: f32);
    /// Selects a sub-preset (ModalBar, BandedWG, VoicForm phonemes).
    fn set_preset(&mut self, _index: usize) {}
    /// Computes one output sample.
    fn tick(&mut self) -> f32;
}

/// Lowest note frequency the delay lines are allocated for.
pub const MIN_FREQUENCY: f64 = 20.0;

/// Delay line length for `factor` periods of [`MIN_FREQUENCY`] at `sr`.
pub(crate) fn max_delay_samples(sr: f64, factor: f64) -> usize {
    (factor * sr / MIN_FREQUENCY).ceil() as usize + 4
}

/// The note-off amplitude used where STK derives a release or damping
/// speed from it: at least 0.1, so a release always ends.
pub(crate) fn release_amplitude(amplitude: f64) -> f64 {
    amplitude.clamp(0.1, 1.0)
}

/// The instrument interface the models implement, in `f64` with already
/// sanitized arguments (see [`Voice`]).
pub(crate) trait Model: Send {
    fn note_on(&mut self, frequency: f64, amplitude: f64);
    fn note_off(&mut self, amplitude: f64);
    fn control_change(&mut self, number: u8, value: f64);
    fn set_preset(&mut self, _index: usize) {}
    fn tick(&mut self) -> f64;
    /// Silences the voice by clearing its delay lines and filters.
    fn clear(&mut self);
}

/// Wraps a [`Model`]: clamps arguments and guards the output.
pub(crate) struct Voice {
    pub(crate) model: Box<dyn Model>,
    pub(crate) max_frequency: f64,
}

impl Instrument for Voice {
    fn note_on(&mut self, frequency: f32, amplitude: f32) {
        if !frequency.is_finite() {
            return;
        }
        let f = (frequency as f64).clamp(MIN_FREQUENCY, self.max_frequency);
        let a = if amplitude.is_finite() { (amplitude as f64).clamp(0.0, 1.0) } else { 0.0 };
        self.model.note_on(f, a);
    }

    fn note_off(&mut self, amplitude: f32) {
        let a = if amplitude.is_finite() { (amplitude as f64).clamp(0.0, 1.0) } else { 0.0 };
        self.model.note_off(a);
    }

    fn control_change(&mut self, number: u8, value: f32) {
        if !value.is_finite() {
            return;
        }
        let v = if number == 101 {
            // Bowed's frequency control takes Hz.
            (value as f64).clamp(MIN_FREQUENCY, self.max_frequency)
        } else {
            (value as f64).clamp(0.0, 128.0)
        };
        self.model.control_change(number, v);
    }

    fn set_preset(&mut self, index: usize) {
        self.model.set_preset(index);
    }

    fn tick(&mut self) -> f32 {
        let y = self.model.tick();
        if y.is_finite() && y.abs() <= 64.0 {
            y as f32
        } else {
            self.model.clear();
            0.0
        }
    }
}

const fn c(number: u8, name: &'static str, default: f32) -> ControlInfo {
    ControlInfo { number, name, default }
}

/// Control value for a normalized STK parameter value (value / 128).
const fn v(normalized: f32) -> f32 {
    normalized * 128.0
}

/// All instruments, in display order.
pub static MODELS: &[ModelInfo] = &[
    ModelInfo {
        id: "flute",
        name: "Flute",
        family: "Winds",
        description: "Jet-driven bore waveguide flute with breath noise and vibrato (Cook, after Karjalainen, Smith and Waryznyk).",
        controls: &[
            c(2, "Jet Delay", v(0.5)),
            c(4, "Noise Gain", v(0.15 / 0.4)),
            c(11, "Vibrato Frequency", v(5.925 / 12.0)),
            c(1, "Vibrato Gain", v(0.05 / 0.4)),
            c(128, "Breath Pressure", v(0.8)),
        ],
        presets: &[],
    },
    ModelInfo {
        id: "clarinet",
        name: "Clarinet",
        family: "Winds",
        description: "Reed-table clarinet: a single cylindrical bore waveguide with breath noise and vibrato.",
        controls: &[
            c(2, "Reed Stiffness", v(0.14 / 0.26)),
            c(4, "Noise Gain", v(0.5)),
            c(11, "Vibrato Frequency", v(5.735 / 12.0)),
            c(1, "Vibrato Gain", v(0.2)),
            c(128, "Breath Pressure", v(0.79)),
        ],
        presets: &[],
    },
    ModelInfo {
        id: "saxofony",
        name: "Saxofony",
        family: "Winds",
        description: "Faux conical-bore reed instrument: a reed exciting a string-like waveguide split at the blow position.",
        controls: &[
            c(2, "Reed Stiffness", v(0.5)),
            c(26, "Reed Aperture", v(0.5)),
            c(4, "Noise Gain", v(0.5)),
            c(11, "Blow Position", v(0.2)),
            c(29, "Vibrato Frequency", v(5.735 / 12.0)),
            c(1, "Vibrato Gain", v(0.2)),
            c(128, "Breath Pressure", v(0.79)),
        ],
        presets: &[],
    },
    ModelInfo {
        id: "brass",
        name: "Brass",
        family: "Winds",
        description: "Lip-reed brass: a resonant lip model driving a bore with slide, after Cook's TBone.",
        controls: &[
            c(2, "Lip Tension", 63.0),
            c(4, "Slide Length", v(0.5)),
            c(11, "Vibrato Frequency", v(6.137 / 12.0)),
            c(1, "Vibrato Gain", 0.0),
            c(128, "Volume", v(1.0)),
        ],
        presets: &[],
    },
    ModelInfo {
        id: "blowbotl",
        name: "Blown Bottle",
        family: "Winds",
        description: "Blowing across a bottle: a Helmholtz resonator driven by a jet and breath noise.",
        controls: &[
            c(4, "Noise Gain", v(20.0 / 30.0)),
            c(11, "Vibrato Frequency", v(5.925 / 12.0)),
            c(1, "Vibrato Gain", 0.0),
            c(128, "Volume", v(0.8)),
        ],
        presets: &[],
    },
    ModelInfo {
        id: "blowhole",
        name: "Blow Hole",
        family: "Winds",
        description: "Clarinet with a tonehole and a register vent you can open and close (Scavone and Cook).",
        controls: &[
            c(2, "Reed Stiffness", v(0.14 / 0.26)),
            c(4, "Noise Gain", v(0.5)),
            c(11, "Tonehole State", 0.0),
            c(1, "Register State", 0.0),
            c(128, "Breath Pressure", v(0.79)),
        ],
        presets: &[],
    },
    ModelInfo {
        id: "recorder",
        name: "Recorder",
        family: "Winds",
        description: "Recorder after Verge, Hirschberg and Causse (Bredholt): jet, labium and pipe simulation with turbulence. Experimental: the pitch often jumps to upper pipe modes or edge tones.",
        controls: &[
            c(2, "Softness", v(0.5)),
            c(4, "Noise Gain", v(0.2)),
            c(16, "Noise Cutoff", v(0.25)),
            c(11, "Vibrato Frequency", v(4.0 / 12.0)),
            c(1, "Vibrato Gain", 0.0),
            c(128, "Breath Pressure", v(1.0)),
        ],
        presets: &[],
    },
    ModelInfo {
        id: "whistle",
        name: "Whistle",
        family: "Winds",
        description: "Referee's whistle: a pea bouncing in a can modulates a sine and noise; sounds two octaves above the note.",
        controls: &[
            c(4, "Noise Gain", v(0.5)),
            c(11, "Fipple Modulation Frequency", v(0.5)),
            c(1, "Fipple Modulation Gain", v(0.5)),
            c(2, "Blowing Frequency Modulation", v(0.5)),
            c(128, "Volume", v(0.8)),
        ],
        presets: &[],
    },
    ModelInfo {
        id: "bowed",
        name: "Bowed",
        family: "Strings",
        description: "Bowed string: bow friction table between neck and bridge waveguides, with a violin body filter.",
        controls: &[
            c(2, "Bow Pressure", v(0.5)),
            c(4, "Bow Position", v(0.127236)),
            c(11, "Vibrato Frequency", v(6.12723 / 12.0)),
            c(1, "Vibrato Gain", 0.0),
            c(100, "Bow Velocity", v(0.9)),
            c(128, "Volume", v(0.9)),
        ],
        presets: &[],
    },
    ModelInfo {
        id: "plucked",
        name: "Plucked",
        family: "Strings",
        description: "The basic Karplus-Strong plucked string.",
        controls: &[],
        presets: &[],
    },
    ModelInfo {
        id: "mandolin",
        name: "Mandolin",
        family: "Strings",
        description: "Two detuned commuted-synthesis strings excited by recorded mandolin body responses.",
        controls: &[
            c(2, "Body Size", v(0.5)),
            c(4, "Pluck Position", v(0.4)),
            c(11, "String Sustain", v(0.025 / 0.03)),
            c(1, "String Detuning", v(0.05)),
            c(128, "Microphone Position", 0.0),
        ],
        presets: &[],
    },
    ModelInfo {
        id: "sitar",
        name: "Sitar",
        family: "Strings",
        description: "Karplus-Strong string with a gliding, buzzing delay length and noise excitation.",
        controls: &[],
        presets: &[],
    },
    ModelInfo {
        id: "stifkarp",
        name: "Stiff String",
        family: "Strings",
        description: "Plucked stiff string: Karplus-Strong with allpass dispersion and a pickup-position comb.",
        controls: &[
            c(4, "Pickup Position", v(0.4)),
            c(11, "String Sustain", v(0.025 / 0.03)),
            c(1, "String Stretch", v(0.001)),
        ],
        presets: &[],
    },
    ModelInfo {
        id: "modalbar",
        name: "Modal Bar",
        family: "Mallets",
        description: "Struck bars: a stick strike exciting four resonant modes, with nine presets.",
        controls: &[
            c(16, "Modal Presets", 0.0),
            c(2, "Stick Hardness", v(0.429688)),
            c(4, "Stick Position", v(0.445312)),
            c(1, "Vibrato Gain", 0.0),
            c(11, "Vibrato Frequency", v(0.5)),
            c(8, "Direct Stick Mix", v(0.09375)),
            c(128, "Volume", v(1.0)),
        ],
        presets: modalbar::PRESET_NAMES,
    },
    ModelInfo {
        id: "bandedwg",
        name: "Banded Waveguide",
        family: "Mallets",
        description: "Banded waveguides (Essl): struck or bowed bars, glass harmonica and Tibetan bowl.",
        controls: &[
            c(16, "Instrument Presets", 0.0),
            c(2, "Bow Pressure", 0.0),
            c(4, "Bow Motion", 0.0),
            c(11, "Vibrato Frequency", 0.0),
            c(1, "Gain", v(0.99)),
            c(64, "Set Striking", 0.0),
            c(128, "Bow Velocity", v(0.9)),
        ],
        presets: bandedwg::PRESET_NAMES,
    },
    ModelInfo {
        id: "voicform",
        name: "Voice Formant",
        family: "Voice",
        description: "Singing voice: a glottal pulse with vibrato through four formant filters, 32 phonemes.",
        controls: &[
            c(2, "Voiced/Unvoiced Mix", 0.0),
            c(4, "Vowel/Phoneme Selection", 32.0),
            c(11, "Vibrato Frequency", v(0.5)),
            c(1, "Vibrato Gain", v(0.2)),
            c(128, "Loudness (Spectral Tilt)", v(0.8)),
        ],
        presets: &phonemes::NAMES,
    },
];

/// Controls that are not sent at creation because any value changes the
/// voice's mode (BandedWG's bow motion switches to velocity tracking).
const NOT_APPLIED_AT_CREATE: &[(&str, u8)] = &[("bandedwg", 4)];

fn boxed<M: Model + 'static>(model: M) -> Box<dyn Model> {
    Box::new(model)
}

/// Creates the instrument `id` (see [`MODELS`]) at `sample_rate`, with all
/// its controls set to their defaults. Returns `None` for unknown ids.
pub fn create(id: &str, sample_rate: f32) -> Option<Box<dyn Instrument>> {
    create_voice(id, sample_rate).map(|v| Box::new(v) as Box<dyn Instrument>)
}

/// [`create`] without the box, for the tests.
pub(crate) fn create_voice(id: &str, sample_rate: f32) -> Option<Voice> {
    let sr = if sample_rate.is_finite() { (sample_rate as f64).clamp(8000.0, 768_000.0) } else { 44100.0 };
    let info = MODELS.iter().find(|m| m.id == id)?;
    let model = match id {
        "flute" => boxed(flute::Flute::new(sr)),
        "clarinet" => boxed(clarinet::Clarinet::new(sr)),
        "saxofony" => boxed(saxofony::Saxofony::new(sr)),
        "brass" => boxed(brass::Brass::new(sr)),
        "blowbotl" => boxed(blowbotl::BlowBotl::new(sr)),
        "blowhole" => boxed(blowhole::BlowHole::new(sr)),
        "recorder" => boxed(recorder::Recorder::new(sr)),
        "whistle" => boxed(whistle::Whistle::new(sr)),
        "bowed" => boxed(bowed::Bowed::new(sr)),
        "plucked" => boxed(plucked::Plucked::new(sr)),
        "mandolin" => boxed(mandolin::Mandolin::new(sr)),
        "sitar" => boxed(sitar::Sitar::new(sr)),
        "stifkarp" => boxed(stifkarp::StifKarp::new(sr)),
        "modalbar" => boxed(modalbar::ModalBar::new(sr)),
        "bandedwg" => boxed(bandedwg::BandedWG::new(sr)),
        "voicform" => boxed(voicform::VoicForm::new(sr)),
        _ => return None,
    };
    let mut voice = Voice { model, max_frequency: sr / 8.0 };
    for control in info.controls {
        if !NOT_APPLIED_AT_CREATE.contains(&(id, control.number)) {
            voice.control_change(control.number, control.default);
        }
    }
    Some(voice)
}
