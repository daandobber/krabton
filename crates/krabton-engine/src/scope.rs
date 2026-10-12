//! Lock-free sample taps for waveform views and the Oscilloscope device.
//!
//! The audio thread writes samples into ring buffers of atomics; the UI
//! copies the latest ones out. A read can overlap a write and show a few
//! samples from two different blocks, which is fine for drawing. Nothing
//! here allocates or locks on the audio thread.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Samples kept per device tap (stereo): about 0.7 s at 48 kHz.
pub const SCOPE_LEN: usize = 32768;
/// Device taps that can show audio at the same time (an analyzer device
/// uses two: before and after it).
pub const MAX_SCOPES: usize = 48;
/// Added to a device id for the tap in front of the device.
pub const PRE_KEY: u64 = 1 << 63;
/// Devices that can publish meter values at the same time.
pub const MAX_METER_DEVICES: usize = 128;
/// Values per device in the meter bank.
pub const METER_VALUES: usize = 32;

pub struct ScopeBuffer {
    l: Box<[AtomicU32]>,
    r: Box<[AtomicU32]>,
    written: AtomicU64,
}

impl ScopeBuffer {
    /// `len` must be a power of two. A mono buffer stores (L + R) / 2.
    pub fn new(len: usize, stereo: bool) -> Self {
        debug_assert!(len.is_power_of_two());
        let make = |n: usize| (0..n).map(|_| AtomicU32::new(0)).collect::<Vec<_>>().into_boxed_slice();
        Self { l: make(len), r: make(if stereo { len } else { 0 }), written: AtomicU64::new(0) }
    }

    pub fn len(&self) -> usize {
        self.l.len()
    }

    pub fn is_empty(&self) -> bool {
        self.l.is_empty()
    }

    pub fn is_stereo(&self) -> bool {
        !self.r.is_empty()
    }

    /// Total samples written so far.
    pub fn written(&self) -> u64 {
        self.written.load(Ordering::Acquire)
    }

    /// Audio thread: appends a block.
    pub fn write(&self, l: &[f32], r: &[f32]) {
        let mask = self.l.len() - 1;
        let mut pos = self.written.load(Ordering::Relaxed);
        if self.is_stereo() {
            for (a, b) in l.iter().zip(r) {
                let i = pos as usize & mask;
                self.l[i].store(a.to_bits(), Ordering::Relaxed);
                self.r[i].store(b.to_bits(), Ordering::Relaxed);
                pos += 1;
            }
        } else {
            for (a, b) in l.iter().zip(r) {
                self.l[pos as usize & mask].store(((a + b) * 0.5).to_bits(), Ordering::Relaxed);
                pos += 1;
            }
        }
        self.written.store(pos, Ordering::Release);
    }

    /// UI: copies the latest `n` samples, oldest first (the right channel
    /// stays empty for a mono buffer). Returns the write count they end at.
    pub fn read(&self, n: usize, l: &mut Vec<f32>, r: &mut Vec<f32>) -> u64 {
        let end = self.written();
        let n = n.min(self.l.len()).min(end as usize);
        let mask = self.l.len() - 1;
        let start = end - n as u64;
        l.clear();
        r.clear();
        l.extend((start..end).map(|p| f32::from_bits(self.l[p as usize & mask].load(Ordering::Relaxed))));
        if self.is_stereo() {
            r.extend((start..end).map(|p| f32::from_bits(self.r[p as usize & mask].load(Ordering::Relaxed))));
        }
        end
    }
}

/// A fixed set of stereo buffers that Oscilloscope devices claim by id.
pub struct ScopeBank {
    owners: Vec<AtomicU64>,
    buffers: Vec<ScopeBuffer>,
}

impl Default for ScopeBank {
    fn default() -> Self {
        Self::new()
    }
}

impl ScopeBank {
    pub fn new() -> Self {
        Self { owners: (0..MAX_SCOPES).map(|_| AtomicU64::new(0)).collect(), buffers: (0..MAX_SCOPES).map(|_| ScopeBuffer::new(SCOPE_LEN, true)).collect() }
    }

    fn slot_of(&self, id: u64) -> Option<usize> {
        self.owners.iter().position(|o| o.load(Ordering::Acquire) == id)
    }

    /// Audio thread: the slot of device `id`, claiming a free one if needed.
    pub fn claim(&self, id: u64) -> Option<usize> {
        if id == 0 {
            return None;
        }
        self.slot_of(id).or_else(|| self.owners.iter().position(|o| o.compare_exchange(0, id, Ordering::AcqRel, Ordering::Relaxed).is_ok()))
    }

    pub fn owner(&self, slot: usize) -> u64 {
        self.owners.get(slot).map(|o| o.load(Ordering::Acquire)).unwrap_or(0)
    }

    pub fn buffer(&self, slot: usize) -> &ScopeBuffer {
        &self.buffers[slot]
    }

    /// UI: the buffer that device `id` writes to, if it has one.
    pub fn find(&self, id: u64) -> Option<&ScopeBuffer> {
        self.slot_of(id).map(|s| &self.buffers[s])
    }

    /// UI: frees the slots of devices that no longer exist.
    pub fn release_unused(&self, alive: impl Fn(u64) -> bool) {
        for o in &self.owners {
            let id = o.load(Ordering::Acquire);
            if id != 0 && !alive(id) {
                let _ = o.compare_exchange(id, 0, Ordering::AcqRel, Ordering::Relaxed);
            }
        }
    }
}

/// Numbers devices publish for their displays (gain reduction, loudness,
/// dynamic EQ gains), by device id.
pub struct MeterBank {
    owners: Vec<AtomicU64>,
    values: Vec<AtomicU32>,
}

impl Default for MeterBank {
    fn default() -> Self {
        Self::new()
    }
}

impl MeterBank {
    pub fn new() -> Self {
        Self { owners: (0..MAX_METER_DEVICES).map(|_| AtomicU64::new(0)).collect(), values: (0..MAX_METER_DEVICES * METER_VALUES).map(|_| AtomicU32::new(0)).collect() }
    }

    fn claim(&self, id: u64) -> Option<usize> {
        if id == 0 {
            return None;
        }
        self.owners
            .iter()
            .position(|o| o.load(Ordering::Acquire) == id)
            .or_else(|| self.owners.iter().position(|o| o.compare_exchange(0, id, Ordering::AcqRel, Ordering::Relaxed).is_ok()))
    }

    /// UI: the latest values of device `id`.
    pub fn read(&self, id: u64) -> Option<[f32; METER_VALUES]> {
        let slot = self.owners.iter().position(|o| o.load(Ordering::Acquire) == id)?;
        let mut out = [0.0; METER_VALUES];
        for (k, v) in out.iter_mut().enumerate() {
            *v = f32::from_bits(self.values[slot * METER_VALUES + k].load(Ordering::Relaxed));
        }
        Some(out)
    }

    /// UI: frees the slots of devices that no longer exist.
    pub fn release_unused(&self, alive: impl Fn(u64) -> bool) {
        for o in &self.owners {
            let id = o.load(Ordering::Acquire);
            if id != 0 && !alive(id) {
                let _ = o.compare_exchange(id, 0, Ordering::AcqRel, Ordering::Relaxed);
            }
        }
    }
}

/// Values a device fills in while processing, published after each block.
pub struct MeterTap {
    pub values: [f32; METER_VALUES],
    slot: Option<usize>,
}

impl Default for MeterTap {
    fn default() -> Self {
        Self { values: [0.0; METER_VALUES], slot: None }
    }
}

impl MeterTap {
    pub fn publish(&mut self, bank: &MeterBank, id: u64) {
        let slot = match self.slot {
            Some(s) if bank.owners[s].load(Ordering::Acquire) == id => s,
            _ => match bank.claim(id) {
                Some(s) => {
                    self.slot = Some(s);
                    s
                }
                None => return,
            },
        };
        for (k, v) in self.values.iter().enumerate() {
            bank.values[slot * METER_VALUES + k].store(v.to_bits(), Ordering::Relaxed);
        }
    }
}

/// What a device with a display sends to the UI: its input (optional), its
/// output and meter values (optional).
#[derive(Default)]
pub struct DeviceTaps {
    pub pre: Option<ScopeTap>,
    pub post: ScopeTap,
    pub meters: Option<MeterTap>,
}

impl DeviceTaps {
    /// Taps before and after the device, with meters.
    pub fn full() -> Self {
        Self { pre: Some(ScopeTap::default()), post: ScopeTap::default(), meters: Some(MeterTap::default()) }
    }

    /// Audio thread, before the device runs.
    pub fn before(&mut self, scopes: &ScopeBank, id: u64, l: &[f32], r: &[f32]) {
        if let Some(t) = &mut self.pre {
            t.write(scopes, id | PRE_KEY, l, r);
        }
    }

    /// Audio thread, after the device ran.
    pub fn after(&mut self, scopes: &ScopeBank, meters: &MeterBank, id: u64, l: &[f32], r: &[f32]) {
        self.post.write(scopes, id, l, r);
        if let Some(m) = &mut self.meters {
            m.publish(meters, id);
        }
    }
}

/// Runtime state of an Oscilloscope device: which slot it writes to.
#[derive(Default)]
pub struct ScopeTap {
    slot: Option<usize>,
}

impl ScopeTap {
    pub fn write(&mut self, bank: &ScopeBank, id: u64, l: &[f32], r: &[f32]) {
        let slot = match self.slot {
            Some(s) if bank.owner(s) == id => s,
            _ => {
                self.slot = bank.claim(id);
                match self.slot {
                    Some(s) => s,
                    None => return,
                }
            }
        };
        bank.buffer(slot).write(l, r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_return_the_latest_samples_in_order() {
        let b = ScopeBuffer::new(16, true);
        let l: Vec<f32> = (0..40).map(|i| i as f32).collect();
        let r: Vec<f32> = l.iter().map(|x| -x).collect();
        b.write(&l[..25], &r[..25]);
        b.write(&l[25..], &r[25..]);
        let (mut a, mut c) = (Vec::new(), Vec::new());
        assert_eq!(b.read(8, &mut a, &mut c), 40);
        assert_eq!(a, (32..40).map(|i| i as f32).collect::<Vec<_>>());
        assert_eq!(c, (32..40).map(|i| -(i as f32)).collect::<Vec<_>>());
        // Asking for more than the buffer holds gives the whole buffer.
        b.read(100, &mut a, &mut c);
        assert_eq!(a.len(), 16);
        assert_eq!(a[0], 24.0);
        // A mono buffer keeps the mid signal.
        let m = ScopeBuffer::new(8, false);
        m.write(&[1.0, 2.0], &[3.0, 0.0]);
        m.read(2, &mut a, &mut c);
        assert_eq!(a, vec![2.0, 1.0]);
        assert!(c.is_empty());
    }

    #[test]
    fn devices_claim_and_release_slots() {
        let bank = ScopeBank::new();
        let mut taps: Vec<ScopeTap> = (0..MAX_SCOPES + 1).map(|_| ScopeTap::default()).collect();
        for (i, t) in taps.iter_mut().enumerate() {
            t.write(&bank, 100 + i as u64, &[0.5], &[0.5]);
        }
        assert!(bank.find(100).is_some());
        assert!(bank.find(100 + MAX_SCOPES as u64).is_none(), "only MAX_SCOPES slots");
        // Device 100 is deleted; the extra device gets its slot.
        bank.release_unused(|id| id != 100);
        let meters = MeterBank::new();
        let mut m = MeterTap::default();
        m.values[3] = -6.5;
        m.publish(&meters, 42);
        assert_eq!(meters.read(42).unwrap()[3], -6.5);
        meters.release_unused(|_| false);
        assert!(meters.read(42).is_none());
        taps[MAX_SCOPES].write(&bank, 100 + MAX_SCOPES as u64, &[0.25], &[0.25]);
        assert!(bank.find(100).is_none());
        assert!(bank.find(100 + MAX_SCOPES as u64).is_some());
    }
}
