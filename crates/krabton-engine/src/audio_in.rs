//! The audio interface input, handed from the input callback to the engine
//! through a lock-free single-producer single-consumer ring.
//!
//! Input and output may run on different clocks (a USB microphone and the
//! speakers, say). The engine reads exactly as many frames as it renders;
//! when the input runs late it gets silence for the missing frames, and when
//! the input piles up it skips ahead to a small, fixed latency.

use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};

/// Input channels Krabton keeps (Ext. In 1 to 8).
pub const MAX_INPUTS: usize = 8;
/// Frames the engine stays behind the input once it is running.
const TARGET_LATENCY: u64 = 768;
/// Beyond this many waiting frames the engine skips ahead.
const MAX_LATENCY: u64 = 4096;

pub struct InputRing {
    data: Box<[AtomicU32]>,
    mask: u64,
    channels: AtomicUsize,
    written: AtomicU64,
    read: AtomicU64,
    /// Frames dropped or padded because of clock drift.
    pub glitches: AtomicU64,
}

impl InputRing {
    /// `frames` is rounded up to a power of two; `channels` is what the
    /// device delivers (only the first [`MAX_INPUTS`] are kept).
    pub fn new(frames: usize, channels: usize) -> Self {
        let frames = frames.next_power_of_two().max(1024);
        Self {
            data: (0..frames * MAX_INPUTS).map(|_| AtomicU32::new(0)).collect(),
            mask: frames as u64 - 1,
            channels: AtomicUsize::new(channels.clamp(1, MAX_INPUTS)),
            written: AtomicU64::new(0),
            read: AtomicU64::new(0),
            glitches: AtomicU64::new(0),
        }
    }

    pub fn channels(&self) -> usize {
        self.channels.load(Ordering::Relaxed)
    }

    /// Input callback: appends interleaved frames of `ch` channels. Frames
    /// that do not fit are dropped.
    pub fn push(&self, interleaved: &[f32], ch: usize) {
        let ch = ch.max(1);
        let keep = ch.min(MAX_INPUTS);
        let mut w = self.written.load(Ordering::Relaxed);
        let r = self.read.load(Ordering::Acquire);
        let room = (self.mask + 1).saturating_sub(w - r);
        let frames = (interleaved.len() / ch) as u64;
        if frames > room {
            self.glitches.fetch_add(frames - room, Ordering::Relaxed);
        }
        for f in interleaved.chunks_exact(ch).take(room as usize) {
            let base = ((w & self.mask) as usize) * MAX_INPUTS;
            for (c, &v) in f.iter().take(keep).enumerate() {
                self.data[base + c].store(v.to_bits(), Ordering::Relaxed);
            }
            w += 1;
        }
        self.written.store(w, Ordering::Release);
    }

    /// Engine: takes `n` frames, channel-major (`out[c * stride + i]`).
    /// Missing frames are silent. Returns the frames that were real input.
    pub fn pop(&self, n: usize, out: &mut [f32], stride: usize) -> usize {
        let w = self.written.load(Ordering::Acquire);
        let mut r = self.read.load(Ordering::Relaxed);
        if w - r > MAX_LATENCY {
            self.glitches.fetch_add(w - r - TARGET_LATENCY, Ordering::Relaxed);
            r = w - TARGET_LATENCY;
        }
        let avail = ((w - r) as usize).min(n);
        if avail < n && r > 0 {
            self.glitches.fetch_add((n - avail) as u64, Ordering::Relaxed);
        }
        let ch = self.channels();
        for c in 0..MAX_INPUTS {
            let dst = &mut out[c * stride..c * stride + n];
            if c >= ch {
                dst.fill(0.0);
                continue;
            }
            for (i, d) in dst.iter_mut().enumerate() {
                *d = if i < avail { f32::from_bits(self.data[(((r + i as u64) & self.mask) as usize) * MAX_INPUTS + c].load(Ordering::Relaxed)) } else { 0.0 };
            }
        }
        self.read.store(r + avail as u64, Ordering::Release);
        avail
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_come_out_in_order_per_channel() {
        let ring = InputRing::new(1024, 2);
        let x: Vec<f32> = (0..200).flat_map(|i| [i as f32, -(i as f32)]).collect();
        ring.push(&x, 2);
        let mut out = vec![9.0; MAX_INPUTS * 64];
        assert_eq!(ring.pop(64, &mut out, 64), 64);
        assert_eq!(out[0], 0.0);
        assert_eq!(out[63], 63.0);
        assert_eq!(out[64 + 5], -5.0);
        assert!(out[2 * 64..].iter().all(|&v| v == 0.0), "unused channels are silent");
        assert_eq!(ring.pop(64, &mut out, 64), 64);
        assert_eq!(out[0], 64.0);
    }

    #[test]
    fn late_input_pads_and_piled_up_input_skips_ahead() {
        let ring = InputRing::new(16384, 1);
        ring.push(&[0.5; 10], 1);
        let mut out = vec![0.0; MAX_INPUTS * 32];
        assert_eq!(ring.pop(32, &mut out, 32), 10);
        assert_eq!(out[9], 0.5);
        assert_eq!(out[10], 0.0);
        // Far too much waiting: only the latest TARGET_LATENCY frames stay.
        let lots: Vec<f32> = (0..10000).map(|i| i as f32).collect();
        ring.push(&lots, 1);
        ring.pop(32, &mut out, 32);
        assert_eq!(out[0], (10000 - TARGET_LATENCY) as f32);
    }
}
