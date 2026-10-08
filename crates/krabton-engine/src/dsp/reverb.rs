//! Freeverb-style stereo reverb with predelay and a low cut on the input.

use super::OnePole;

const COMBS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASSES: [usize; 4] = [556, 441, 341, 225];
const SPREAD: usize = 23;
const MAX_PREDELAY_S: f32 = 0.25;

#[inline]
fn undenorm(x: f32) -> f32 {
    if x.abs() < 1e-18 {
        0.0
    } else {
        x
    }
}

struct Comb {
    buf: Vec<f32>,
    idx: usize,
    store: f32,
}

impl Comb {
    fn new(len: usize) -> Self {
        Self { buf: vec![0.0; len.max(1)], idx: 0, store: 0.0 }
    }

    #[inline]
    fn tick(&mut self, x: f32, fb: f32, damp: f32) -> f32 {
        let out = self.buf[self.idx];
        self.store = undenorm(out * (1.0 - damp) + self.store * damp);
        self.buf[self.idx] = x + self.store * fb;
        self.idx += 1;
        if self.idx >= self.buf.len() {
            self.idx = 0;
        }
        out
    }
}

struct Allpass {
    buf: Vec<f32>,
    idx: usize,
}

impl Allpass {
    fn new(len: usize) -> Self {
        Self { buf: vec![0.0; len.max(1)], idx: 0 }
    }

    #[inline]
    fn tick(&mut self, x: f32) -> f32 {
        let b = self.buf[self.idx];
        self.buf[self.idx] = undenorm(x + b * 0.5);
        self.idx += 1;
        if self.idx >= self.buf.len() {
            self.idx = 0;
        }
        b - x
    }
}

pub struct Reverb {
    combs: [Vec<Comb>; 2],
    aps: [Vec<Allpass>; 2],
    pre: [Vec<f32>; 2],
    pre_idx: usize,
    hp: [OnePole; 2],
    sr: f32,
}

impl Reverb {
    pub fn new(sr: f32) -> Self {
        let scale = sr / 44100.0;
        let mk_combs = |extra: usize| COMBS.iter().map(|&c| Comb::new(((c + extra) as f32 * scale) as usize)).collect::<Vec<_>>();
        let mk_aps = |extra: usize| ALLPASSES.iter().map(|&c| Allpass::new(((c + extra) as f32 * scale) as usize)).collect::<Vec<_>>();
        let pre_len = (MAX_PREDELAY_S * sr) as usize + 2;
        Self {
            combs: [mk_combs(0), mk_combs(SPREAD)],
            aps: [mk_aps(0), mk_aps(SPREAD)],
            pre: [vec![0.0; pre_len], vec![0.0; pre_len]],
            pre_idx: 0,
            hp: [OnePole::default(); 2],
            sr,
        }
    }

    /// size, damping, width, predelay (ms), low cut (Hz), mix
    pub fn process(&mut self, size: f32, damping: f32, width: f32, predelay_ms: f32, low_cut: f32, mix: f32, l: &mut [f32], r: &mut [f32]) {
        let fb = 0.7 + size.clamp(0.0, 1.0) * 0.28;
        let damp = damping.clamp(0.0, 1.0) * 0.4;
        let wet1 = width / 2.0 + 0.5;
        let wet2 = (1.0 - width) / 2.0;
        let pre_len = self.pre[0].len();
        let delay = ((predelay_ms / 1000.0 * self.sr) as usize).min(pre_len - 1);
        for h in &mut self.hp {
            h.set(low_cut, self.sr);
        }
        let wet = mix.clamp(0.0, 1.0);
        let dry = 1.0 - wet;
        for i in 0..l.len().min(r.len()) {
            let (xl, xr) = (l[i], r[i]);
            self.pre[0][self.pre_idx] = xl;
            self.pre[1][self.pre_idx] = xr;
            let read = (self.pre_idx + pre_len - delay) % pre_len;
            let dl = self.hp[0].hp(self.pre[0][read]);
            let dr = self.hp[1].hp(self.pre[1][read]);
            self.pre_idx = (self.pre_idx + 1) % pre_len;
            let input = (dl + dr) * 0.015;
            let mut out = [0.0f32; 2];
            for ch in 0..2 {
                let mut acc = 0.0;
                for c in self.combs[ch].iter_mut() {
                    acc += c.tick(input, fb, damp);
                }
                for a in self.aps[ch].iter_mut() {
                    acc = a.tick(acc);
                }
                out[ch] = acc;
            }
            let yl = out[0] * wet1 + out[1] * wet2;
            let yr = out[1] * wet1 + out[0] * wet2;
            l[i] = xl * dry + yl * wet * 3.0;
            r[i] = xr * dry + yr * wet * 3.0;
        }
    }
}
