//! The state variable filter of Braids (`braids/svf.h`, Copyright 2012
//! Emilie Gillet, MIT).

use crate::dsp::{clip, interpolate824_u16};
use crate::resources::{LUT_SVF_CUTOFF, LUT_SVF_DAMP};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SvfMode {
    Lp,
    Bp,
    Hp,
}

#[derive(Clone, Copy)]
pub struct Svf {
    dirty: bool,
    frequency: i16,
    resonance: i16,
    punch: i32,
    f: i32,
    damp: i32,
    lp: i32,
    bp: i32,
    mode: SvfMode,
}

impl Default for Svf {
    fn default() -> Self {
        let mut s = Self { dirty: true, frequency: 0, resonance: 0, punch: 0, f: 0, damp: 0, lp: 0, bp: 0, mode: SvfMode::Bp };
        s.init();
        s
    }
}

impl Svf {
    pub fn init(&mut self) {
        self.lp = 0;
        self.bp = 0;
        self.frequency = 33 << 7;
        self.resonance = 16384;
        self.dirty = true;
        self.punch = 0;
        self.mode = SvfMode::Bp;
    }

    pub fn set_frequency(&mut self, frequency: i16) {
        self.dirty = self.dirty || self.frequency != frequency;
        self.frequency = frequency;
    }

    pub fn set_resonance(&mut self, resonance: i16) {
        self.resonance = resonance;
        self.dirty = true;
    }

    pub fn set_punch(&mut self, punch: u16) {
        self.punch = ((punch as u32 * punch as u32) >> 24) as i32;
    }

    pub fn set_mode(&mut self, mode: SvfMode) {
        self.mode = mode;
    }

    #[inline]
    pub fn process(&mut self, input: i32) -> i32 {
        if self.dirty {
            self.f = interpolate824_u16(&LUT_SVF_CUTOFF, (self.frequency as i32 as u32) << 17) as i32;
            self.damp = interpolate824_u16(&LUT_SVF_DAMP, (self.resonance as i32 as u32) << 17) as i32;
            self.dirty = false;
        }
        let mut f = self.f;
        let mut damp = self.damp;
        if self.punch != 0 {
            let punch_signal = if self.lp > 4096 { self.lp } else { 2048 };
            f += ((punch_signal >> 4) * self.punch) >> 9;
            damp += (punch_signal - 2048) >> 3;
        }
        let notch = input - (self.bp * damp >> 15);
        self.lp += f * self.bp >> 15;
        self.lp = clip(self.lp);
        let hp = notch - self.lp;
        self.bp += f * hp >> 15;
        self.bp = clip(self.bp);
        match self.mode {
            SvfMode::Bp => self.bp,
            SvfMode::Hp => hp,
            SvfMode::Lp => self.lp,
        }
    }
}
