//! The DJ mixer: two decks (tracks with a Turntable on them) and the set, all
//! the other tracks together, each on a channel with a trim, a three-band
//! isolator that kills a band altogether (Linkwitz-Riley crossovers, and with
//! every band at its middle not even those), a filter (low-pass to the left,
//! high-pass to the right), a fader and a side of the crossfader. A channel
//! with Cue goes to the headphones too: with *Split* the music is in the left
//! ear and the cue in the right, for a computer with one stereo output.

use super::{FilterMode, Svf};
use serde::{Deserialize, Serialize};

pub const SET: usize = 2;
pub const SIDES: [&str; 3] = ["A", "Thru", "B"];
/// So low a band is gone.
pub const KILL_DB: f32 = -26.0;
pub const EQ_MAX_DB: f32 = 6.0;

const LOW_HZ: f32 = 250.0;
const HIGH_HZ: f32 = 2500.0;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DjChannel {
    pub trim_db: f32,
    pub high_db: f32,
    pub mid_db: f32,
    pub low_db: f32,
    /// -1 low-pass all the way, 0 open, 1 high-pass all the way.
    pub filter: f32,
    pub fader_db: f32,
    /// Its side of the crossfader (see `SIDES`).
    pub side: u8,
    pub cue: bool,
}

impl Default for DjChannel {
    fn default() -> Self {
        Self { trim_db: 0.0, high_db: 0.0, mid_db: 0.0, low_db: 0.0, filter: 0.0, fader_db: 0.0, side: 1, cue: false }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DjMix {
    pub on: bool,
    /// The tracks that are decks A and B.
    pub decks: [Option<u64>; 2],
    /// Deck A, deck B, the set.
    pub channels: [DjChannel; 3],
    /// -1 all A, 1 all B.
    pub crossfader: f32,
    /// The music in the left ear, the cue in the right.
    pub split_cue: bool,
}

impl Default for DjMix {
    fn default() -> Self {
        let mut channels = [DjChannel::default(); 3];
        channels[0].side = 0;
        channels[1].side = 2;
        Self { on: false, decks: [None, None], channels, crossfader: 0.0, split_cue: false }
    }
}

impl DjMix {
    /// Which deck a track is, while the mixer is on.
    pub fn deck_of(&self, track: u64) -> Option<usize> {
        if !self.on {
            return None;
        }
        self.decks.iter().position(|d| *d == Some(track))
    }
}

/// What the crossfader lets through of a side: equal power.
pub fn side_gain(side: u8, crossfader: f32) -> f32 {
    let a = (crossfader.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
    match side {
        0 => a.cos(),
        2 => a.sin(),
        _ => 1.0,
    }
}

/// A band's gain: nothing left at the bottom of its knob.
pub fn band_gain(db: f32) -> f32 {
    if db <= KILL_DB + 0.01 {
        0.0
    } else {
        10f32.powf(db / 20.0)
    }
}

/// How far to move a record (seconds) so that its beats fall `phase` (0..1) of
/// a beat on: the short way.
pub fn phase_jump(pos: f64, rec_bpm: f64, rec_beat: f64, phase: f64) -> f64 {
    if rec_bpm <= 0.0 {
        return 0.0;
    }
    let beat = 60.0 / rec_bpm;
    let now = ((pos - rec_beat) / beat).rem_euclid(1.0);
    let d = (phase - now + 0.5).rem_euclid(1.0) - 0.5;
    d * beat
}

/// Where a record is in its beat (0..1).
pub fn phase_of(pos: f64, rec_bpm: f64, rec_beat: f64) -> f64 {
    if rec_bpm <= 0.0 {
        return 0.0;
    }
    ((pos - rec_beat) * rec_bpm / 60.0).rem_euclid(1.0)
}

/// A key on the wheel DJs mix by: its number (1..12) and whether it is minor (A) or major (B).
pub fn wheel(root: u8, minor: bool) -> (u8, bool) {
    let major_root = if minor { (root + 3) % 12 } else { root % 12 };
    let fifths = (major_root as u32 * 7) % 12;
    (((7 + fifths) % 12 + 1) as u8, minor)
}

/// A key's name on that wheel (8A).
pub fn wheel_name(root: u8, minor: bool) -> String {
    let (n, m) = wheel(root, minor);
    format!("{n}{}", if m { "A" } else { "B" })
}

/// Whether two keys mix: the same, a step round the wheel, or its other half.
pub fn keys_fit(a: (u8, bool), b: (u8, bool)) -> bool {
    let (na, ma) = wheel(a.0, a.1);
    let (nb, mb) = wheel(b.0, b.1);
    if ma != mb {
        return na == nb;
    }
    let d = (na as i32 - nb as i32).rem_euclid(12);
    d == 0 || d == 1 || d == 11
}

/// The three bands of one side: fourth-order crossovers, whose bands add up to
/// the same level all over; the low band goes through the upper crossover's
/// all-pass, so that it stays in phase with the other two.
#[derive(Default)]
struct Bands {
    low: [Svf; 2],
    rest: [Svf; 2],
    mid: [Svf; 2],
    high: [Svf; 2],
    align_low: [Svf; 2],
    align_high: [Svf; 2],
}

impl Bands {
    fn set(&mut self, sr: f32) {
        let q = std::f32::consts::FRAC_1_SQRT_2;
        for f in self.low.iter_mut().chain(self.rest.iter_mut()) {
            f.set_q(LOW_HZ, q, sr);
        }
        for f in self.mid.iter_mut().chain(self.high.iter_mut()).chain(self.align_low.iter_mut()).chain(self.align_high.iter_mut()) {
            f.set_q(HIGH_HZ, q, sr);
        }
    }

    fn split(&mut self, x: f32) -> [f32; 3] {
        use FilterMode::{High, Low};
        let twice = |f: &mut [Svf; 2], x: f32, mode: FilterMode| {
            let y = f[0].process(x, mode);
            f[1].process(y, mode)
        };
        let lo = twice(&mut self.low, x, Low);
        let lo = twice(&mut self.align_low, lo, Low) + twice(&mut self.align_high, lo, High);
        let rest = twice(&mut self.rest, x, High);
        [lo, twice(&mut self.mid, rest, Low), twice(&mut self.high, rest, High)]
    }
}

/// A channel at work.
pub struct Strip {
    split: [Bands; 2],
    pass: [Svf; 2],
    bands: [f32; 3],
    /// How much of the sound goes through the crossovers.
    shaped: f32,
    gain: f32,
    wet: f32,
    fresh: bool,
}

impl Default for Strip {
    fn default() -> Self {
        Self::new()
    }
}

impl Strip {
    pub fn new() -> Self {
        Self { split: Default::default(), pass: [Svf::new(), Svf::new()], bands: [1.0; 3], shaped: 0.0, gain: 1.0, wet: 0.0, fresh: true }
    }

    /// Runs a block through the channel. What it adds to the cue comes before
    /// the fader. Returns its peak after the fader.
    pub fn process(&mut self, ch: &DjChannel, crossfader: f32, l: &mut [f32], r: &mut [f32], sr: f32, cue: Option<(&mut [f32], &mut [f32])>) -> f32 {
        let n = l.len().min(r.len());
        if n == 0 {
            return 0.0;
        }
        for b in self.split.iter_mut() {
            b.set(sr);
        }
        let amount = ch.filter.clamp(-1.0, 1.0);
        let to_low = amount < 0.0;
        let cut = if to_low { 20000.0 * 0.0075f32.powf(-amount) } else { 20.0 * 400f32.powf(amount) };
        for f in self.pass.iter_mut() {
            f.set_q(cut.min(sr * 0.45), 0.9, sr);
        }
        let mode = if to_low { FilterMode::Low } else { FilterMode::High };
        // A small turn from the middle comes in gently.
        let wet_to = ((amount.abs() - 0.02) / 0.1).clamp(0.0, 1.0);
        let bands_to = [band_gain(ch.low_db), band_gain(ch.mid_db), band_gain(ch.high_db)];
        // All in the middle the crossovers are left out, phase and all.
        let shaped_to = if bands_to == [1.0; 3] { 0.0 } else { 1.0 };
        let trim = 10f32.powf(ch.trim_db / 20.0);
        let fader = if ch.fader_db <= -60.0 { 0.0 } else { 10f32.powf(ch.fader_db / 20.0) };
        let gain_to = fader * side_gain(ch.side, crossfader);
        if self.fresh {
            (self.bands, self.gain, self.wet, self.shaped) = (bands_to, gain_to, wet_to, shaped_to);
            self.fresh = false;
        }
        let step = 1.0 / n as f32;
        let mut cue = cue;
        let mut peak = 0.0f32;
        for i in 0..n {
            let t = (i + 1) as f32 * step;
            let g = [
                self.bands[0] + (bands_to[0] - self.bands[0]) * t,
                self.bands[1] + (bands_to[1] - self.bands[1]) * t,
                self.bands[2] + (bands_to[2] - self.bands[2]) * t,
            ];
            let shaped = self.shaped + (shaped_to - self.shaped) * t;
            let wet = self.wet + (wet_to - self.wet) * t;
            let gain = self.gain + (gain_to - self.gain) * t;
            for (c, x) in [&mut *l, &mut *r].into_iter().enumerate() {
                let dry = x[i];
                let y = if shaped > 0.0 {
                    let b = self.split[c].split(dry);
                    dry + (b[0] * g[0] + b[1] * g[1] + b[2] * g[2] - dry) * shaped
                } else {
                    dry
                };
                let y = y * trim;
                let filtered = self.pass[c].process(y, mode);
                let y = y + (filtered - y) * wet;
                if let Some((cl, cr)) = cue.as_mut() {
                    if c == 0 {
                        cl[i] += y;
                    } else {
                        cr[i] += y;
                    }
                }
                let y = y * gain;
                peak = peak.max(y.abs());
                x[i] = y;
            }
        }
        (self.bands, self.gain, self.wet, self.shaped) = (bands_to, gain_to, wet_to, shaped_to);
        peak
    }
}

/// The music in the left ear and the cue in the right.
pub fn split_for_headphones(l: &mut [f32], r: &mut [f32], cue_l: &[f32], cue_r: &[f32]) {
    for i in 0..l.len().min(r.len()).min(cue_l.len()).min(cue_r.len()) {
        let music = 0.5 * (l[i] + r[i]);
        l[i] = music;
        r[i] = (0.5 * (cue_l[i] + cue_r[i])).clamp(-1.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn tone(hz: f32, n: usize) -> Vec<f32> {
        (0..n).map(|i| (std::f32::consts::TAU * hz * i as f32 / SR).sin() * 0.5).collect()
    }

    fn level_after(ch: &DjChannel, hz: f32) -> f32 {
        let mut s = Strip::new();
        let (mut l, mut r) = (tone(hz, 48000), tone(hz, 48000));
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            s.process(ch, 0.0, a, b, SR, None);
        }
        // Its strength, not its peak: the samples of a high tone miss the top.
        let tail = &l[24000..];
        (tail.iter().map(|v| v * v).sum::<f32>() / tail.len() as f32).sqrt() * std::f32::consts::SQRT_2 / 0.5
    }

    #[test]
    fn all_open_it_is_the_sound_itself() {
        let ch = DjChannel::default();
        let x = tone(330.0, 4096);
        let (mut l, mut r) = (x.clone(), x.clone());
        Strip::new().process(&ch, 0.0, &mut l, &mut r, SR, None);
        for (a, b) in l.iter().zip(&x) {
            assert!((a - b).abs() < 1e-4, "{a} {b}");
        }
    }

    #[test]
    fn a_killed_band_is_gone_and_the_others_stay() {
        let kill_low = DjChannel { low_db: KILL_DB, ..DjChannel::default() };
        assert!(level_after(&kill_low, 50.0) < 0.1, "{}", level_after(&kill_low, 50.0));
        assert!(level_after(&kill_low, 8000.0) > 0.8);
        let kill_high = DjChannel { high_db: KILL_DB, ..DjChannel::default() };
        assert!(level_after(&kill_high, 12000.0) < 0.1, "{}", level_after(&kill_high, 12000.0));
        assert!(level_after(&kill_high, 80.0) > 0.8);
        let kill_mid = DjChannel { mid_db: KILL_DB, ..DjChannel::default() };
        assert!(level_after(&kill_mid, 800.0) < 0.1, "{}", level_after(&kill_mid, 800.0));
        assert!(level_after(&kill_mid, 50.0) > 0.8);
        assert!(level_after(&kill_mid, 12000.0) > 0.8);
    }

    #[test]
    fn the_bands_add_up_to_the_same_level_all_over() {
        // A hair off the middle, so the crossovers are in.
        let ch = DjChannel { low_db: 0.01, ..DjChannel::default() };
        for hz in [40.0, 150.0, 250.0, 700.0, 2500.0, 6000.0, 14000.0] {
            let level = level_after(&ch, hz);
            assert!((level - 1.0).abs() < 0.03, "{hz} Hz: {level}");
        }
    }

    #[test]
    fn the_filter_goes_both_ways() {
        let low = DjChannel { filter: -0.9, ..DjChannel::default() };
        assert!(level_after(&low, 6000.0) < 0.1);
        assert!(level_after(&low, 60.0) > 0.7);
        let high = DjChannel { filter: 0.9, ..DjChannel::default() };
        assert!(level_after(&high, 60.0) < 0.1);
        assert!(level_after(&high, 9000.0) > 0.7);
    }

    #[test]
    fn the_crossfader_keeps_the_power_and_thru_stays() {
        assert_eq!(side_gain(0, -1.0), 1.0);
        assert!(side_gain(0, 1.0).abs() < 1e-6);
        assert!(side_gain(2, -1.0).abs() < 1e-6);
        assert!((side_gain(2, 1.0) - 1.0).abs() < 1e-6);
        for x in [-0.7f32, -0.2, 0.0, 0.4, 0.9] {
            let p = side_gain(0, x).powi(2) + side_gain(2, x).powi(2);
            assert!((p - 1.0).abs() < 1e-5);
            assert_eq!(side_gain(1, x), 1.0);
        }
    }

    #[test]
    fn the_cue_hears_it_before_the_fader_and_split_puts_it_in_the_right_ear() {
        let ch = DjChannel { fader_db: -80.0, ..DjChannel::default() };
        let (mut l, mut r) = (tone(440.0, 1024), tone(440.0, 1024));
        let (mut cl, mut cr) = (vec![0.0f32; 1024], vec![0.0f32; 1024]);
        let peak = Strip::new().process(&ch, 0.0, &mut l, &mut r, SR, Some((&mut cl, &mut cr)));
        assert_eq!(peak, 0.0, "the fader is down");
        assert!(cl.iter().any(|v| v.abs() > 0.4), "the cue still hears it");
        let (mut ml, mut mr) = (vec![0.2f32; 1024], vec![0.4f32; 1024]);
        split_for_headphones(&mut ml, &mut mr, &cl, &cr);
        assert!((ml[10] - 0.3).abs() < 1e-6, "music in the left");
        assert_eq!(mr[100], (0.5 * (cl[100] + cr[100])).clamp(-1.0, 1.0));
    }

    #[test]
    fn phase_goes_the_short_way_onto_the_other_beat() {
        // 120 bpm: half a second a beat. A tenth of a beat late, onto the beat: back 50 ms.
        assert!((phase_jump(10.05, 120.0, 0.0, 0.0) + 0.05).abs() < 1e-9);
        assert!((phase_jump(10.45, 120.0, 0.0, 0.0) - 0.05).abs() < 1e-9, "forward when that is nearer");
        assert!(phase_jump(10.0, 120.0, 0.0, 0.0).abs() < 1e-9);
        assert_eq!(phase_jump(10.0, 0.0, 0.0, 0.5), 0.0, "no beat known, no jump");
        // The first downbeat counts.
        assert!((phase_of(0.35, 120.0, 0.1) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn keys_sit_on_the_wheel() {
        assert_eq!(wheel_name(9, true), "8A", "A minor");
        assert_eq!(wheel_name(0, false), "8B", "C major");
        assert_eq!(wheel_name(7, false), "9B", "G major");
        assert_eq!(wheel_name(4, true), "9A", "E minor");
        assert_eq!(wheel_name(5, false), "7B", "F major");
        assert!(keys_fit((9, true), (9, true)), "the same");
        assert!(keys_fit((9, true), (0, false)), "its relative major");
        assert!(keys_fit((9, true), (4, true)), "a step on");
        assert!(keys_fit((9, true), (2, true)), "a step back");
        assert!(!keys_fit((9, true), (3, true)), "the far side");
        assert!(!keys_fit((9, true), (7, false)), "another number and letter");
    }
}
