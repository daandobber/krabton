//! Brain: the sound that goes in played again out of other sounds. Sounds
//! you give it are cut into short overlapping blocks, each heard (its
//! spectrum in bands, and its MFCCs: the spectrum's shape as an ear has it)
//! and joined to the blocks most like it (its synapses): the brain. What
//! goes in is cut into blocks the same way, and for each the brain finds the
//! block most like it and plays that instead, so a beat comes back as
//! a cappellas, a voice as bubbling mud. *Ratio* blends hearing by spectrum
//! and by MFCC; *Match* hears the level too or only the colour; *Novelty*
//! steers away from blocks used lately and *Boredom* tires of them sooner;
//! *Stickiness* keeps playing on in a sound while that is close enough;
//! *Search* looks through every block (*Basic*), takes the least like it
//! (*Reversed*), or wanders the synapses from the block playing
//! (*Synaptic*: as fast with a big brain as a small one, and different
//! every time round). *Autotune* bends a block towards the pitch it stands
//! in for, *Normalise* plays it as loud as what it stands in for, *Mix*
//! lets what went in through instead. It answers a block late.
//!
//! The brain is made away from the audio (see `build`) and handed over.

use crate::dsp::effects::FxCtx;
use crate::params::{ParamSpec, Unit};
use crate::sample::SampleRef;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub const BLOCK: usize = 0;
pub const RATIO: usize = 1;
pub const MATCH: usize = 2;
pub const NOVELTY: usize = 3;
pub const BOREDOM: usize = 4;
pub const STICKY: usize = 5;
pub const SEARCH: usize = 6;
pub const SYNAPSES: usize = 7;
pub const AUTOTUNE: usize = 8;
pub const NORMALISE: usize = 9;
pub const MIX: usize = 10;
pub const LEVEL: usize = 11;

/// Block lengths (samples).
pub const BLOCKS: [usize; 4] = [512, 1024, 2048, 4096];
pub const BLOCK_NAMES: &[&str] = &["512", "1024", "2048", "4096"];
pub const MATCHES: &[&str] = &["Full", "Colour"];
pub const SEARCHES: &[&str] = &["Basic", "Reversed", "Synaptic"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Block", "Brain", BLOCK_NAMES, 2),
    ParamSpec::lin("Ratio", "Brain", 0.0, 1.0, 0.5, Unit::Percent),
    ParamSpec::choice("Match", "Brain", MATCHES, 0),
    ParamSpec::lin("Novelty", "Search", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Boredom", "Search", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Stickiness", "Search", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Search", "Search", SEARCHES, 2),
    ParamSpec::lin("Synapses", "Search", 2.0, 64.0, 16.0, Unit::None),
    ParamSpec::lin("Autotune", "Play", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Normalise", "Play", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Mix", "Play", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Level", "Play", -24.0, 12.0, 0.0, Unit::Db),
];

/// Bands of a block's spectrum, MFCCs, synapses a block keeps.
pub const BANDS: usize = 48;
pub const MFCCS: usize = 13;
const MELS: usize = 26;
pub const MAX_SYNAPSES: usize = 64;
/// The most blocks a brain keeps, and those each is compared with to find its synapses.
pub const MAX_BLOCKS: usize = 16384;
const SYN_CANDIDATES: usize = 2048;

/// What a block sounds like.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Heard {
    /// Its spectrum in bands (log), and the same less its mean (colour only).
    pub bands: [f32; BANDS],
    pub shape: [f32; BANDS],
    pub mfcc: [f32; MFCCS],
    pub rms: f32,
    /// Its pitch (Hz; 0: none heard).
    pub f0: f32,
}

impl Default for Heard {
    fn default() -> Self {
        Self { bands: [0.0; BANDS], shape: [0.0; BANDS], mfcc: [0.0; MFCCS], rms: 0.0, f0: 0.0 }
    }
}

/// How unlike two blocks are: by spectrum and MFCC as `ratio` blends them, with their level or without.
pub fn distance(a: &Heard, b: &Heard, ratio: f32, colour_only: bool) -> f32 {
    let mut df = 0.0;
    if ratio < 1.0 {
        let (x, y) = if colour_only { (&a.shape, &b.shape) } else { (&a.bands, &b.bands) };
        for k in 0..BANDS {
            let d = x[k] - y[k];
            df += d * d;
        }
        df /= BANDS as f32;
    }
    let mut dm = 0.0;
    if ratio > 0.0 {
        let from = usize::from(colour_only);
        for k in from..MFCCS {
            let d = a.mfcc[k] - b.mfcc[k];
            dm += d * d;
        }
        dm /= (MFCCS - from) as f32;
    }
    (1.0 - ratio) * df + ratio * dm
}

/// Hears blocks of one length: its window, its FFT, its bands and mel filters.
pub struct Ear {
    n: usize,
    sr: f32,
    window: Vec<f32>,
    re: Vec<f32>,
    im: Vec<f32>,
    /// Each band's bins (from, to).
    bands: Vec<(usize, usize)>,
    /// Each mel band's bins and weights.
    mels: Vec<Vec<(usize, f32)>>,
}

impl Ear {
    pub fn new(n: usize, sr: f32) -> Self {
        let n = n.next_power_of_two().max(64);
        let window = (0..n).map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / n as f32).cos()).collect();
        let half = n / 2;
        let bin = |hz: f32| ((hz * n as f32 / sr).round() as usize).clamp(1, half - 1);
        let edge = |k: usize| 40.0 * (16000.0f32.min(sr * 0.45) / 40.0).powf(k as f32 / BANDS as f32);
        let bands = (0..BANDS).map(|k| (bin(edge(k)), bin(edge(k + 1)).max(bin(edge(k)) + 1))).collect();
        let mel = |hz: f32| 2595.0 * (1.0 + hz / 700.0).log10();
        let hz = |m: f32| 700.0 * (10f32.powf(m / 2595.0) - 1.0);
        let (lo, hi) = (mel(60.0), mel((sr * 0.45).min(12000.0)));
        let points: Vec<f32> = (0..MELS + 2).map(|k| hz(lo + (hi - lo) * k as f32 / (MELS + 1) as f32)).collect();
        let mels = (0..MELS)
            .map(|m| {
                let (a, c, b) = (points[m], points[m + 1], points[m + 2]);
                (bin(a)..=bin(b))
                    .filter_map(|i| {
                        let f = i as f32 * sr / n as f32;
                        let w = if f < c { (f - a) / (c - a).max(1e-3) } else { (b - f) / (b - c).max(1e-3) };
                        (w > 0.0).then_some((i, w))
                    })
                    .collect()
            })
            .collect();
        Self { n, sr, window, re: vec![0.0; n], im: vec![0.0; n], bands, mels }
    }

    pub fn len(&self) -> usize {
        self.n
    }

    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// The block's window (the same the blocks are played in).
    pub fn window(&self) -> &[f32] {
        &self.window
    }

    /// Hears a block (`n` samples, mono).
    pub fn hear(&mut self, block: &[f32]) -> Heard {
        let n = self.n;
        let mut sum = 0.0;
        for i in 0..n {
            let v = block.get(i).copied().unwrap_or(0.0);
            sum += v * v;
            self.re[i] = v * self.window[i];
            self.im[i] = 0.0;
        }
        fft(&mut self.re, &mut self.im);
        let mag = |re: &[f32], im: &[f32], i: usize| (re[i] * re[i] + im[i] * im[i]).sqrt() * 2.0 / n as f32;
        let mut h = Heard { rms: (sum / n as f32).sqrt(), ..Default::default() };
        for (k, (a, b)) in self.bands.iter().enumerate() {
            let m = (*a..*b).map(|i| mag(&self.re, &self.im, i)).sum::<f32>() / (b - a) as f32;
            h.bands[k] = (m + 1e-6).ln();
        }
        let mean = h.bands.iter().sum::<f32>() / BANDS as f32;
        for k in 0..BANDS {
            h.shape[k] = h.bands[k] - mean;
        }
        // MFCCs: the log mel energies, turned by a DCT.
        let mut logmel = [0.0f32; MELS];
        for (m, filt) in self.mels.iter().enumerate() {
            let e: f32 = filt.iter().map(|&(i, w)| w * mag(&self.re, &self.im, i).powi(2)).sum();
            logmel[m] = (e + 1e-10).ln();
        }
        for (c, out) in h.mfcc.iter_mut().enumerate() {
            let mut s = 0.0;
            for (m, v) in logmel.iter().enumerate() {
                s += v * (std::f32::consts::PI * c as f32 * (m as f32 + 0.5) / MELS as f32).cos();
            }
            *out = s * (2.0 / MELS as f32).sqrt() * 0.5;
        }
        // The pitch: the strongest peak from 50 Hz to 1 kHz, between its bins.
        let (lo, hi) = (((50.0 * n as f32 / self.sr) as usize).max(2), ((1000.0 * n as f32 / self.sr) as usize).min(n / 2 - 2));
        let mut best = (0, 0.0);
        for i in lo..hi {
            let m = mag(&self.re, &self.im, i);
            if m > best.1 {
                best = (i, m);
            }
        }
        if best.1 > 1e-4 && best.0 > lo {
            let (a, b, c) = (mag(&self.re, &self.im, best.0 - 1), best.1, mag(&self.re, &self.im, best.0 + 1));
            let shift = 0.5 * (a - c) / (a - 2.0 * b + c).min(-1e-9);
            h.f0 = (best.0 as f32 + shift) * self.sr / n as f32;
        }
        h
    }
}

/// An FFT in place (a power of two long).
fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let a = -std::f32::consts::TAU / len as f32;
        let (wr, wi) = (a.cos(), a.sin());
        for start in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0f32, 0.0f32);
            for k in 0..len / 2 {
                let (i, j) = (start + k, start + k + len / 2);
                let (tr, ti) = (re[j] * cr - im[j] * ci, re[j] * ci + im[j] * cr);
                re[j] = re[i] - tr;
                im[j] = im[i] - ti;
                re[i] += tr;
                im[i] += ti;
                (cr, ci) = (cr * wr - ci * wi, cr * wi + ci * wr);
            }
        }
        len <<= 1;
    }
}

/// A sound in the brain, at the engine's rate.
pub struct BrainSound {
    pub name: String,
    pub left: Vec<f32>,
    pub right: Vec<f32>,
}

/// A block of the brain: which sound, from where, what it sounds like, where it sits on the face (0..1 both ways).
#[derive(Clone, Copy, Debug)]
pub struct BrainBlock {
    pub sound: u16,
    pub start: u32,
    pub heard: Heard,
    pub x: f32,
    pub y: f32,
}

/// The brain: its sounds, cut in blocks, each with its synapses (the
/// blocks most like it, nearest first).
pub struct BrainData {
    pub block: usize,
    pub sr: f32,
    pub sounds: Vec<BrainSound>,
    pub blocks: Vec<BrainBlock>,
    pub synapses: Vec<u32>,
    /// What it was made from (to see when it must be made again).
    pub key: u64,
}

impl BrainData {
    pub fn synapses(&self, i: usize) -> &[u32] {
        &self.synapses[i * MAX_SYNAPSES..(i + 1) * MAX_SYNAPSES]
    }

    /// The block that follows block `i` in its sound (half a block on), if there is one.
    pub fn next(&self, i: usize) -> Option<usize> {
        let b = self.blocks.get(i)?;
        let n = self.blocks.get(i + 1)?;
        (n.sound == b.sound && n.start as usize == b.start as usize + self.block / 2).then_some(i + 1)
    }
}

/// What a brain is made from: its sounds (by their buffers) and the block length.
pub fn key_of(sounds: &[SampleRef], block: usize, sr: f32) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |v: u64| {
        h ^= v;
        h = h.wrapping_mul(0x0100_0000_01b3);
    };
    for s in sounds {
        mix(Arc::as_ptr(&s.0) as usize as u64);
    }
    mix(block as u64);
    mix(sr.to_bits() as u64);
    h
}

/// Makes a brain of `sounds` in blocks of `block` at `sr` (None when `cancel` is set on the way).
pub fn build(sounds: &[SampleRef], block: usize, sr: f32, cancel: &AtomicBool) -> Option<BrainData> {
    let mut ear = Ear::new(block, sr);
    let block = ear.len();
    let hop = block / 2;
    let mut out = BrainData { block, sr, sounds: Vec::new(), blocks: Vec::new(), synapses: Vec::new(), key: key_of(sounds, block, sr) };
    let mut mono = vec![0.0; block];
    for (si, s) in sounds.iter().enumerate().take(u16::MAX as usize) {
        // At the engine's rate.
        let ratio = s.sample_rate as f32 / sr;
        let n = (s.frames() as f32 / ratio) as usize;
        let at = |ch: &[f32], i: usize| {
            let x = i as f32 * ratio;
            let (k, f) = (x as usize, x.fract());
            let a = ch.get(k).copied().unwrap_or(0.0);
            let b = ch.get(k + 1).copied().unwrap_or(a);
            a + (b - a) * f
        };
        let right = if s.right.is_empty() { &s.left } else { &s.right };
        let sound = BrainSound { name: s.name.clone(), left: (0..n).map(|i| at(&s.left, i)).collect(), right: (0..n).map(|i| at(right, i)).collect() };
        let mut start = 0;
        while start + block <= sound.left.len().max(block) && out.blocks.len() < MAX_BLOCKS {
            if cancel.load(Ordering::Relaxed) {
                return None;
            }
            for (i, m) in mono.iter_mut().enumerate() {
                *m = 0.5 * (sound.left.get(start + i).copied().unwrap_or(0.0) + sound.right.get(start + i).copied().unwrap_or(0.0));
            }
            let heard = ear.hear(&mono);
            out.blocks.push(BrainBlock { sound: si as u16, start: start as u32, heard, x: 0.0, y: 0.0 });
            start += hop;
            if sound.left.len() < block {
                break;
            }
        }
        out.sounds.push(sound);
    }
    // Where each sits on the face: across by its brightness, up by its level
    // (by their order among all, so they spread over it).
    let n = out.blocks.len();
    if n > 0 {
        let place = |key: &dyn Fn(&Heard) -> f32| {
            let mut order: Vec<usize> = (0..n).collect();
            order.sort_by(|&a, &b| key(&out.blocks[a].heard).total_cmp(&key(&out.blocks[b].heard)));
            let mut at = vec![0.0f32; n];
            for (rank, &i) in order.iter().enumerate() {
                at[i] = rank as f32 / (n.max(2) - 1) as f32;
            }
            at
        };
        let xs = place(&|h: &Heard| -h.mfcc[1]);
        let ys = place(&|h: &Heard| h.rms);
        for (i, b) in out.blocks.iter_mut().enumerate() {
            b.x = xs[i];
            b.y = 1.0 - ys[i];
        }
    }
    // The synapses: the blocks most like each (among all, or some by chance in a big brain).
    out.synapses = vec![0; n * MAX_SYNAPSES];
    let mut cand: Vec<(f32, u32)> = Vec::with_capacity(SYN_CANDIDATES.min(n));
    let mut rng = 0x1234_5678u32;
    for i in 0..n {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        cand.clear();
        let a = &out.blocks[i].heard;
        let consider = |j: usize, cand: &mut Vec<(f32, u32)>| {
            if j != i {
                cand.push((distance(a, &out.blocks[j].heard, 0.5, false), j as u32));
            }
        };
        if n <= SYN_CANDIDATES {
            for j in 0..n {
                consider(j, &mut cand);
            }
        } else {
            for _ in 0..SYN_CANDIDATES {
                rng ^= rng << 13;
                rng ^= rng >> 17;
                rng ^= rng << 5;
                consider(rng as usize % n, &mut cand);
            }
        }
        let k = MAX_SYNAPSES.min(cand.len());
        if k > 0 {
            cand.select_nth_unstable_by(k - 1, |x, y| x.0.total_cmp(&y.0));
            cand[..k].sort_by(|x, y| x.0.total_cmp(&y.0));
        }
        for s in 0..MAX_SYNAPSES {
            out.synapses[i * MAX_SYNAPSES + s] = cand.get(s % k.max(1)).map_or(i as u32, |c| c.1);
        }
    }
    Some(out)
}

/// Where the Brain stands, for its face (see `Brain::taps`): the block
/// playing, how many there are, how close it came (0..1), how loud what
/// came in is, and the blocks played last (newest first).
pub const SHOWN: usize = 24;

const MAX_BLOCK: usize = 4096;

/// The Brain playing.
pub struct Brain {
    sr: f32,
    brain: Option<Arc<BrainData>>,
    usage: Vec<f32>,
    ear: Ear,
    input: Vec<f32>,
    in_pos: usize,
    since: usize,
    out_l: Vec<f32>,
    out_r: Vec<f32>,
    target: Vec<f32>,
    out_pos: usize,
    block: Vec<f32>,
    now: Option<usize>,
    avg: f32,
    rng: u32,
    picked: [f32; SHOWN],
    closeness: f32,
    level: f32,
    pub taps: crate::scope::DeviceTaps,
}

impl Brain {
    pub fn new(sr: f32) -> Self {
        Self {
            sr,
            brain: None,
            usage: Vec::new(),
            ear: Ear::new(2048, sr),
            input: vec![0.0; MAX_BLOCK],
            in_pos: 0,
            since: 0,
            out_l: vec![0.0; 2 * MAX_BLOCK],
            out_r: vec![0.0; 2 * MAX_BLOCK],
            target: vec![0.0; 2 * MAX_BLOCK],
            out_pos: 0,
            block: vec![0.0; MAX_BLOCK],
            now: None,
            avg: 1.0,
            rng: 0x2545_f491,
            picked: [-1.0; SHOWN],
            closeness: 0.0,
            level: 0.0,
            taps: crate::scope::DeviceTaps { meters: Some(Default::default()), ..Default::default() },
        }
    }

    /// The brain to play from (from the device; the engine hands it over).
    /// The one let go of, to be dropped away from the audio.
    pub fn attach(&mut self, brain: Option<&Arc<BrainData>>) -> Option<Arc<BrainData>> {
        let same = match (&self.brain, brain) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if same {
            return None;
        }
        let old = self.brain.take();
        if let Some(b) = brain {
            if b.block != self.ear.len() {
                self.ear = Ear::new(b.block, self.sr);
            }
            self.usage = vec![0.0; b.blocks.len()];
            self.brain = Some(b.clone());
        }
        self.now = None;
        old
    }

    fn random(&mut self) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng
    }

    /// The block to play for what came in (`heard`).
    fn search(&mut self, b: &BrainData, heard: &Heard, p: &[f32]) -> (usize, f32) {
        let n = b.blocks.len();
        let ratio = p[RATIO].clamp(0.0, 1.0);
        let colour = p[MATCH] >= 0.5;
        let novelty = p[NOVELTY].clamp(0.0, 1.0) * self.avg * 3.0;
        let mode = p[SEARCH].round() as usize;
        let score = |i: usize, usage: &[f32]| distance(heard, &b.blocks[i].heard, ratio, colour) + novelty * usage.get(i).copied().unwrap_or(0.0);
        // Stickiness: on in the same sound while that is close enough.
        if let Some(next) = self.now.and_then(|c| b.next(c)) {
            let d = distance(heard, &b.blocks[next].heard, ratio, colour);
            if d < self.avg * p[STICKY].clamp(0.0, 1.0) * 2.0 {
                return (next, d);
            }
        }
        let mut best = (0usize, f32::MAX);
        let look = |i: usize, best: &mut (usize, f32), usage: &[f32]| {
            let s = score(i, usage);
            if s < best.1 {
                *best = (i, s);
            }
        };
        match (mode, self.now) {
            // Reversed: the least like it (not silence).
            (1, _) => {
                let mut worst = (0usize, f32::MIN);
                for i in 0..n {
                    if b.blocks[i].heard.rms > 1e-3 {
                        let s = distance(heard, &b.blocks[i].heard, ratio, colour) - novelty * self.usage[i];
                        if s > worst.1 {
                            worst = (i, s);
                        }
                    }
                }
                return (worst.0, worst.1.max(0.0));
            }
            // Synaptic: from the block playing, along its synapses (and on in its sound, and a few by chance to find the way).
            (2, Some(c)) => {
                let k = (p[SYNAPSES].round() as usize).clamp(1, MAX_SYNAPSES);
                for s in 0..k {
                    look(b.synapses(c)[s] as usize, &mut best, &self.usage);
                }
                look(c, &mut best, &self.usage);
                if let Some(next) = b.next(c) {
                    look(next, &mut best, &self.usage);
                }
                for _ in 0..4 {
                    let r = self.random() as usize % n;
                    look(r, &mut best, &self.usage);
                }
            }
            _ => {
                for i in 0..n {
                    look(i, &mut best, &self.usage);
                }
            }
        }
        best
    }

    /// Half a block came in: what it is heard as, and the block played for it.
    fn hop(&mut self, p: &[f32]) {
        let n = self.ear.len();
        for i in 0..n {
            self.block[i] = self.input[(self.in_pos + MAX_BLOCK - n + i) % MAX_BLOCK];
        }
        let heard = self.ear.hear(&self.block[..n]);
        self.level = heard.rms;
        let ring = 2 * MAX_BLOCK;
        // What came in, laid back together (for Mix, and when there is no brain).
        for i in 0..n {
            self.target[(self.out_pos + i) % ring] += self.block[i] * self.ear.window[i];
        }
        let Some(b) = self.brain.clone() else { return };
        if b.blocks.is_empty() {
            return;
        }
        let (pick, d) = self.search(&b, &heard, p);
        self.avg = self.avg * 0.95 + d.max(1e-4) * 0.05;
        self.closeness = 1.0 / (1.0 + d / self.avg.max(1e-6));
        // Used: less new for a while (the more bored, the more).
        let decay = 0.995 - 0.03 * p[BOREDOM].clamp(0.0, 1.0);
        for u in self.usage.iter_mut() {
            *u *= decay;
        }
        if let Some(u) = self.usage.get_mut(pick) {
            *u += 1.0 + 3.0 * p[BOREDOM].clamp(0.0, 1.0);
        }
        self.now = Some(pick);
        self.picked.copy_within(0..SHOWN - 1, 1);
        self.picked[0] = pick as f32;
        let blk = &b.blocks[pick];
        // Bent towards the pitch it stands in for, and as loud as it, as far as asked.
        let mut rate = 1.0f32;
        if heard.f0 > 0.0 && blk.heard.f0 > 0.0 {
            rate = (heard.f0 / blk.heard.f0).clamp(0.5, 2.0).powf(p[AUTOTUNE].clamp(0.0, 1.0));
        }
        let follow = (heard.rms / blk.heard.rms.max(1e-5)).clamp(0.0, 8.0);
        let gain = 1.0 + (follow - 1.0) * p[NORMALISE].clamp(0.0, 1.0);
        let s = &b.sounds[blk.sound as usize];
        let start = blk.start as f32;
        for i in 0..n {
            let x = start + i as f32 * rate;
            let (k, f) = (x as usize, x.fract());
            let at = |ch: &[f32]| {
                let a = ch.get(k).copied().unwrap_or(0.0);
                a + (ch.get(k + 1).copied().unwrap_or(0.0) - a) * f
            };
            let w = self.ear.window[i] * gain;
            let o = (self.out_pos + i) % ring;
            self.out_l[o] += at(&s.left) * w;
            self.out_r[o] += at(&s.right) * w;
        }
    }

    pub fn process(&mut self, p: &[f32], l: &mut [f32], r: &mut [f32], _ctx: &FxCtx) {
        let n = self.ear.len();
        let hop = n / 2;
        let ring = 2 * MAX_BLOCK;
        let mix = p[MIX].clamp(0.0, 1.0);
        let level = 10f32.powf(p[LEVEL] / 20.0);
        let has_brain = self.brain.as_ref().is_some_and(|b| !b.blocks.is_empty());
        for (a, b) in l.iter_mut().zip(r.iter_mut()) {
            self.input[self.in_pos] = 0.5 * (*a + *b);
            self.in_pos = (self.in_pos + 1) % MAX_BLOCK;
            self.since += 1;
            if self.since >= hop {
                self.since = 0;
                self.hop(p);
            }
            let o = self.out_pos;
            let t = self.target[o];
            let (wl, wr) = (self.out_l[o], self.out_r[o]);
            self.target[o] = 0.0;
            self.out_l[o] = 0.0;
            self.out_r[o] = 0.0;
            self.out_pos = (o + 1) % ring;
            if has_brain {
                *a = (wl * mix + t * (1.0 - mix)) * level;
                *b = (wr * mix + t * (1.0 - mix)) * level;
            } else {
                *a = t * level;
                *b = t * level;
            }
        }
        if let Some(m) = self.taps.meters.as_mut() {
            m.values[0] = self.now.map_or(-1.0, |i| i as f32);
            m.values[1] = self.brain.as_ref().map_or(0.0, |b| b.blocks.len() as f32);
            m.values[2] = self.closeness;
            m.values[3] = self.level;
            m.values[4..4 + SHOWN].copy_from_slice(&self.picked);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample::SampleData;

    const SR: f32 = 48000.0;

    fn tone(hz: f32, secs: f32, amp: f32) -> Vec<f32> {
        (0..(secs * SR) as usize).map(|i| (std::f32::consts::TAU * hz * i as f32 / SR).sin() * amp).collect()
    }

    fn sound(name: &str, left: Vec<f32>) -> SampleRef {
        SampleRef::new(SampleData::from_buffers(name, None, SR as u32, left.clone(), left))
    }

    fn ctx() -> FxCtx {
        FxCtx { sr: SR, bpm: 120.0, beat: 0.0, playing: true, scale: Default::default(), note: None }
    }

    #[test]
    fn a_block_is_heard_by_its_pitch_and_colour() {
        let mut ear = Ear::new(2048, SR);
        let low = ear.hear(&tone(110.0, 0.05, 0.5));
        let high = ear.hear(&tone(880.0, 0.05, 0.5));
        assert!((low.f0 - 110.0).abs() < 4.0, "{}", low.f0);
        assert!((high.f0 - 880.0).abs() < 8.0, "{}", high.f0);
        let quiet_low = ear.hear(&tone(110.0, 0.05, 0.05));
        assert!(distance(&low, &quiet_low, 0.5, true) < distance(&low, &high, 0.5, true) * 0.2, "colour only: the level does not count");
        assert!(distance(&low, &quiet_low, 0.5, false) > distance(&low, &quiet_low, 0.5, true), "with the level it does");
    }

    #[test]
    fn a_brain_is_made_of_blocks_joined_to_those_like_them() {
        let s = vec![sound("low", tone(110.0, 0.5, 0.5)), sound("high", tone(880.0, 0.5, 0.5))];
        let b = build(&s, 1024, SR, &AtomicBool::new(false)).unwrap();
        assert_eq!(b.block, 1024);
        let per = (0.5 * SR) as usize / 512 - 1;
        assert_eq!(b.blocks.len(), 2 * per);
        // A low block's nearest are low blocks.
        let syn = b.synapses(3);
        assert!(syn[..8].iter().all(|&j| b.blocks[j as usize].sound == 0), "{:?}", &syn[..8]);
        assert_eq!(b.next(0), Some(1));
        assert_eq!(b.next(per - 1), None, "the last of a sound goes nowhere");
        assert!(build(&s, 1024, SR, &AtomicBool::new(true)).is_none(), "cancelled");
        assert_ne!(key_of(&s, 1024, SR), key_of(&s, 2048, SR));
    }

    #[test]
    fn what_goes_in_comes_out_as_the_brains_nearest_sound() {
        // A brain of a low and a high tone; a low tone in: the low tone out, a block late.
        let s = vec![sound("low", tone(110.0, 1.0, 0.5)), sound("high", tone(880.0, 1.0, 0.5))];
        let b = Arc::new(build(&s, 2048, SR, &AtomicBool::new(false)).unwrap());
        for mode in [0.0, 2.0] {
            let mut fx = Brain::new(SR);
            assert!(fx.attach(Some(&b)).is_none());
            let mut p = crate::params::defaults(SPECS);
            p[SEARCH] = mode;
            let input = tone(110.0, 0.5, 0.3);
            let (mut l, mut r) = (input.clone(), input.clone());
            for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
                fx.process(&p, a, b, &ctx());
            }
            let m = fx.taps.meters.as_ref().unwrap();
            let playing = m.values[0] as usize;
            assert_eq!(b.blocks[playing].sound, 0, "search {mode}: the low tone");
            // The output is the low tone (its pitch heard), not silence.
            let tail = &l[l.len() - 4096..];
            let heard = Ear::new(4096, SR).hear(tail);
            assert!((heard.f0 - 110.0).abs() < 6.0 && heard.rms > 0.05, "{} {}", heard.f0, heard.rms);
        }
    }

    #[test]
    fn without_a_brain_what_goes_in_comes_through_and_novelty_moves_on() {
        let mut fx = Brain::new(SR);
        let p = crate::params::defaults(SPECS);
        let input = tone(220.0, 0.3, 0.3);
        let (mut l, mut r) = (input.clone(), input.clone());
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            fx.process(&p, a, b, &ctx());
        }
        // A block late, the same sound.
        let late = 2048;
        let err: f32 = (8000..12000).map(|i| (l[i] - input[i - late]).abs()).fold(0.0, f32::max);
        assert!(err < 0.02, "through, a block late: {err}");
        // Novelty: the same input does not pick the same block every time.
        let s = vec![sound("a", tone(110.0, 1.0, 0.5))];
        let b = Arc::new(build(&s, 1024, SR, &AtomicBool::new(false)).unwrap());
        let mut fx = Brain::new(SR);
        fx.attach(Some(&b));
        let mut p = crate::params::defaults(SPECS);
        (p[NOVELTY], p[SEARCH]) = (1.0, 0.0);
        let input = tone(110.0, 1.0, 0.5);
        let (mut l, mut r) = (input.clone(), input.clone());
        let mut seen = std::collections::HashSet::new();
        for (a, b) in l.chunks_mut(512).zip(r.chunks_mut(512)) {
            fx.process(&p, a, b, &ctx());
            seen.insert(fx.taps.meters.as_ref().unwrap().values[0] as i64);
        }
        assert!(seen.len() > 10, "novelty spreads the blocks: {}", seen.len());
    }
}
