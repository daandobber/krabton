//! Rust ports of a selection of Airwindows effects by Chris Johnson, for
//! Krabton.
//!
//! Each effect is a faithful port of the `processReplacing` path of the
//! LinuxVST sources at <https://github.com/airwindows/airwindows>
//! (`plugins/LinuxVST/src/<Name>/`): the same math with `f64` internal state,
//! the same 0..1 parameters (A, B, C, ...) with the same mappings and
//! defaults, the same `overallscale = sampleRate / 44100` handling, and the
//! same floating-point noise floor and 32-bit floating-point dither (the
//! `fpdL`/`fpdR` xorshift generators).
//!
//! Use [`EFFECTS`] to list what is available and [`create`] to make an
//! instance. Parameters are read once per `process` call, like the originals
//! read them once per host block.
//!
//! Original code: Copyright (c) 2018 Chris Johnson (airwindows), MIT license:
//!
//! > Permission is hereby granted, free of charge, to any person obtaining a
//! > copy of this software and associated documentation files (the
//! > "Software"), to deal in the Software without restriction, including
//! > without limitation the rights to use, copy, modify, merge, publish,
//! > distribute, sublicense, and/or sell copies of the Software, and to permit
//! > persons to whom the Software is furnished to do so, subject to the
//! > following conditions:
//! >
//! > The above copyright notice and this permission notice shall be included
//! > in all copies or substantial portions of the Software.
//! >
//! > THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
//! > OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
//! > MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN
//! > NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
//! > DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
//! > OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE
//! > USE OR OTHER DEALINGS IN THE SOFTWARE.
//!
//! The Rust port is part of Krabton and also MIT licensed.

#![allow(clippy::too_many_arguments, clippy::needless_range_loop, clippy::excessive_precision)]

mod dsp;

mod air3;
mod buttercomp2;
mod capacitor2;
mod chamber;
mod density2;
mod derez2;
mod galactic;
mod ironoxide5;
mod mackity;
mod pressure4;
mod purestdrive;
mod tapedelay2;
mod totape6;
mod tube2;
mod verbity2;
mod pear2;
mod isolator3;
mod hull2;
mod baxandall2;
mod cabs;
mod distance2;
mod coils2;
mod fracture2;
mod ensemble;
mod infinity2;

/// One parameter. Values are 0..1, as in Airwindows.
#[derive(Clone, Copy, Debug)]
pub struct ParamInfo {
    pub name: &'static str,
    pub default: f32,
}

/// Static description of one effect.
#[derive(Clone, Copy, Debug)]
pub struct EffectInfo {
    /// The Airwindows plug-in name, e.g. "Galactic". Stable; use it to save.
    pub id: &'static str,
    /// What the user sees.
    pub name: &'static str,
    /// One of "Reverb", "Tape", "Saturation", "Dynamics", "EQ", "Lo-Fi", "Delay", "Other".
    pub category: &'static str,
    /// One short sentence in plain English.
    pub description: &'static str,
    /// The parameters in Airwindows order (A, B, C, ...).
    pub params: &'static [ParamInfo],
}

/// A running effect instance.
pub trait Effect: Send {
    /// Processes a stereo block in place. `params` holds one 0..1 value per
    /// [`ParamInfo`]; missing values use the defaults, out-of-range values
    /// are clamped. Works on any block length (if the channels differ in
    /// length, only the common length is processed).
    fn process(&mut self, params: &[f32], left: &mut [f32], right: &mut [f32]);
}

/// All effects, in a stable order.
pub static EFFECTS: &[EffectInfo] = &[
    galactic::INFO,
    verbity2::INFO,
    chamber::INFO,
    totape6::INFO,
    ironoxide5::INFO,
    purestdrive::INFO,
    density2::INFO,
    tube2::INFO,
    mackity::INFO,
    pressure4::INFO,
    buttercomp2::INFO,
    air3::INFO,
    capacitor2::INFO,
    derez2::INFO,
    tapedelay2::INFO,
    pear2::INFO,
    isolator3::INFO,
    hull2::INFO,
    baxandall2::INFO,
    cabs::INFO,
    distance2::INFO,
    coils2::INFO,
    fracture2::INFO,
    ensemble::INFO,
    infinity2::INFO,
];

/// Creates the effect with the given [`EffectInfo::id`], or `None` for an
/// unknown id. All buffers are allocated here; `process` never allocates.
/// The sample rate is clamped to 22.05..192 kHz, the range the originals are
/// written for (outside it some coefficients leave their stable range); a
/// non-finite one counts as 44.1 kHz.
pub fn create(id: &str, sample_rate: f32) -> Option<Box<dyn Effect>> {
    let info = EFFECTS.iter().find(|info| info.id == id)?;
    let (make, _) = constructor(id)?;
    let sr = if sample_rate.is_finite() { (sample_rate as f64).clamp(MIN_SAMPLE_RATE, MAX_SAMPLE_RATE) } else { 44100.0 };
    Some(make(info, sr))
}

const MIN_SAMPLE_RATE: f64 = 22050.0;
const MAX_SAMPLE_RATE: f64 = 192000.0;

type Constructor = fn(&'static EffectInfo, f64) -> Box<dyn Effect>;

/// The constructor for an id, and how many parameters its kernel reads.
fn constructor(id: &str) -> Option<(Constructor, usize)> {
    fn entry<K: Kernel>() -> (Constructor, usize) {
        (wrap::<K>, K::NUM_PARAMS)
    }
    Some(match id {
        "Galactic" => entry::<galactic::Galactic>(),
        "Verbity2" => entry::<verbity2::Verbity2>(),
        "Chamber" => entry::<chamber::Chamber>(),
        "ToTape6" => entry::<totape6::ToTape6>(),
        "IronOxide5" => entry::<ironoxide5::IronOxide5>(),
        "PurestDrive" => entry::<purestdrive::PurestDrive>(),
        "Density2" => entry::<density2::Density2>(),
        "Tube2" => entry::<tube2::Tube2>(),
        "Mackity" => entry::<mackity::Mackity>(),
        "Pressure4" => entry::<pressure4::Pressure4>(),
        "ButterComp2" => entry::<buttercomp2::ButterComp2>(),
        "Air3" => entry::<air3::Air3>(),
        "Capacitor2" => entry::<capacitor2::Capacitor2>(),
        "DeRez2" => entry::<derez2::DeRez2>(),
        "TapeDelay2" => entry::<tapedelay2::TapeDelay2>(),
        "Pear2" => entry::<pear2::Pear2>(),
        "Isolator3" => entry::<isolator3::Isolator3>(),
        "Hull2" => entry::<hull2::Hull2>(),
        "Baxandall2" => entry::<baxandall2::Baxandall2>(),
        "Cabs" => entry::<cabs::Cabs>(),
        "Distance2" => entry::<distance2::Distance2>(),
        "Coils2" => entry::<coils2::Coils2>(),
        "Fracture2" => entry::<fracture2::Fracture2>(),
        "Ensemble" => entry::<ensemble::Ensemble>(),
        "Infinity2" => entry::<infinity2::Infinity2>(),
        _ => return None,
    })
}

/// Most parameters any effect has.
const MAX_PARAMS: usize = 8;

/// The per-effect DSP. `process` gets exactly `NUM_PARAMS` values, already
/// clamped to 0..1, and two channels of equal length with finite samples.
pub(crate) trait Kernel: Send + 'static {
    const NUM_PARAMS: usize;
    fn new(sample_rate: f64) -> Self
    where
        Self: Sized;
    /// Back to the startup state (keeps the dither generators). No allocation.
    fn reset(&mut self);
    fn process(&mut self, p: &[f64], left: &mut [f32], right: &mut [f32]);
}

fn wrap<K: Kernel>(info: &'static EffectInfo, sample_rate: f64) -> Box<dyn Effect> {
    debug_assert_eq!(info.params.len(), K::NUM_PARAMS);
    debug_assert!(K::NUM_PARAMS <= MAX_PARAMS);
    Box::new(Guarded { kernel: K::new(sample_rate), info })
}

/// Resolves parameters and keeps the output finite whatever comes in.
struct Guarded<K: Kernel> {
    kernel: K,
    info: &'static EffectInfo,
}

impl<K: Kernel> Effect for Guarded<K> {
    fn process(&mut self, params: &[f32], left: &mut [f32], right: &mut [f32]) {
        let mut p = [0.0f64; MAX_PARAMS];
        let count = K::NUM_PARAMS.min(MAX_PARAMS);
        for (i, slot) in p.iter_mut().enumerate().take(count) {
            let default = self.info.params.get(i).map_or(0.0, |info| info.default);
            let v = params.get(i).copied().unwrap_or(default);
            *slot = if v.is_finite() { v.clamp(0.0, 1.0) as f64 } else { default as f64 };
        }
        let n = left.len().min(right.len());
        let (left, right) = (&mut left[..n], &mut right[..n]);
        for x in left.iter_mut().chain(right.iter_mut()) {
            if !x.is_finite() {
                *x = 0.0;
            }
        }
        self.kernel.process(&p[..count], left, right);
        if left.iter().chain(right.iter()).any(|x| !x.is_finite()) {
            left.fill(0.0);
            right.fill(0.0);
            self.kernel.reset();
            #[cfg(test)]
            tests::GUARD_TRIPS.with(|c| c.set(c.get() + 1));
        }
    }
}

#[cfg(test)]
mod tests;
