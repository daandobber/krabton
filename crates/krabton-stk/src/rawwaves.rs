//! The STK rawwaves the instruments play: excitation and body samples.
//!
//! The files in `crates/krabton-stk/rawwaves/` are copied unchanged from
//! STK's `rawwaves/` directory: headerless 16-bit big-endian mono at
//! 22050 Hz. They are decoded once, normalized to a peak of 1.0 like
//! `FileWvIn` does by default, and shared by all voices.
//!
//! - `marmstk1.raw`: the marimba stick strike exciting `ModalBar`.
//! - `mand1.raw` .. `mand12.raw`: mandolin body impulse responses for twelve
//!   microphone positions, exciting `Mandolin`.
//! - `impuls20.raw`: one glottal pulse period, looped by `VoicForm`.
//!
//! The Synthesis ToolKit in C++, Copyright (c) 1995--2023 Perry R. Cook and
//! Gary P. Scavone (STK license, see the crate documentation).

use std::sync::OnceLock;

static MARMSTK1: &[u8] = include_bytes!("../rawwaves/marmstk1.raw");
static IMPULS20: &[u8] = include_bytes!("../rawwaves/impuls20.raw");
static MAND: [&[u8]; 12] = [
    include_bytes!("../rawwaves/mand1.raw"),
    include_bytes!("../rawwaves/mand2.raw"),
    include_bytes!("../rawwaves/mand3.raw"),
    include_bytes!("../rawwaves/mand4.raw"),
    include_bytes!("../rawwaves/mand5.raw"),
    include_bytes!("../rawwaves/mand6.raw"),
    include_bytes!("../rawwaves/mand7.raw"),
    include_bytes!("../rawwaves/mand8.raw"),
    include_bytes!("../rawwaves/mand9.raw"),
    include_bytes!("../rawwaves/mand10.raw"),
    include_bytes!("../rawwaves/mand11.raw"),
    include_bytes!("../rawwaves/mand12.raw"),
];

/// Decodes 16-bit big-endian samples and normalizes them to a peak of one.
/// With `looped`, the first sample is repeated at the end (`FileLoop`).
fn decode(bytes: &[u8], looped: bool) -> Vec<f64> {
    let mut data: Vec<f64> =
        bytes.chunks_exact(2).map(|c| i16::from_be_bytes([c[0], c[1]]) as f64 / 32768.0).collect();
    if data.is_empty() {
        data.push(0.0);
    }
    let peak = data.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    if peak > 0.0 {
        data.iter_mut().for_each(|v| *v /= peak);
    }
    if looped {
        data.push(data[0]);
    }
    data
}

/// The marimba strike, `marmstk1.raw` (256 samples).
pub fn marmstk1() -> &'static [f64] {
    static DATA: OnceLock<Vec<f64>> = OnceLock::new();
    DATA.get_or_init(|| decode(MARMSTK1, false))
}

/// One glottal period for looping, `impuls20.raw` (256 samples plus the
/// wrap-around sample).
pub fn impuls20() -> &'static [f64] {
    static DATA: OnceLock<Vec<f64>> = OnceLock::new();
    DATA.get_or_init(|| decode(IMPULS20, true))
}

/// The twelve mandolin body responses, `mand1.raw` .. `mand12.raw`
/// (1024 samples each).
pub fn mandolin_bodies() -> &'static [Vec<f64>; 12] {
    static DATA: OnceLock<[Vec<f64>; 12]> = OnceLock::new();
    DATA.get_or_init(|| std::array::from_fn(|i| decode(MAND[i], false)))
}
