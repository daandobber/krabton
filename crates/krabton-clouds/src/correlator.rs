//! Finds the best alignment between two signals for WSOLA. Correlation is
//! computed by XOR-ing the sign bits of the samples, 32 at a time
//! (`clouds/dsp/correlator.h`, `clouds/dsp/correlator.cc`).
//!
//! Copyright 2014 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::wsola::MAX_WSOLA_SIZE;

/// Words of sign bits in each of the three blocks of the module's memory.
pub const CORRELATOR_BLOCK_SIZE: usize = (MAX_WSOLA_SIZE as usize / 32) + 2;

pub struct Correlator {
    source: [u32; CORRELATOR_BLOCK_SIZE],
    destination: [u32; CORRELATOR_BLOCK_SIZE * 2],
    offset: i32,
    increment: i32,
    size: i32,
    candidate: i32,
    best_score: u32,
    best_match: i32,
    done: bool,
}

impl Default for Correlator {
    fn default() -> Self {
        Self {
            source: [0; CORRELATOR_BLOCK_SIZE],
            destination: [0; CORRELATOR_BLOCK_SIZE * 2],
            offset: 0,
            increment: 0,
            size: 0,
            candidate: 0,
            best_score: 0,
            best_match: 0,
            done: true,
        }
    }
}

impl Correlator {
    pub fn init(&mut self) {
        self.offset = 0;
        self.best_match = 0;
        self.done = true;
    }

    #[inline]
    pub fn best_match(&self) -> i32 {
        self.offset.wrapping_add(self.best_match.wrapping_mul(self.increment >> 4) >> 12)
    }

    pub fn source_mut(&mut self) -> &mut [u32] {
        &mut self.source
    }

    pub fn destination_mut(&mut self) -> &mut [u32] {
        &mut self.destination
    }

    pub fn start_search(&mut self, size: i32, offset: i32, increment: i32) {
        self.offset = offset;
        self.increment = increment;
        self.best_score = 0;
        self.best_match = 0;
        self.candidate = 0;
        self.size = size;
        self.done = false;
    }

    #[inline]
    pub fn done(&self) -> bool {
        self.done
    }

    pub fn evaluate_some_candidates(&mut self) {
        let mut num_candidates = (self.size.max(0) >> 2) + 16;
        while num_candidates > 0 {
            self.evaluate_next_candidate();
            num_candidates -= 1;
        }
    }

    /// Runs the search to the end (the module spreads it over the idle time
    /// between audio blocks).
    pub fn evaluate_all_candidates(&mut self) {
        while !self.done {
            self.evaluate_next_candidate();
        }
    }

    pub fn evaluate_next_candidate(&mut self) {
        if self.done {
            return;
        }
        let num_words = (self.size.max(0) >> 5) as usize;
        let offset_words = (self.candidate >> 5) as usize;
        let offset_bits = (self.candidate & 0x1f) as u32;
        let word = |i: usize| self.destination.get(offset_words + i).copied().unwrap_or(0);
        let mut xcorr = 0u32;
        for i in 0..num_words {
            let source_bits = self.source.get(i).copied().unwrap_or(0);
            // A shift by 32 gives 0 on the module's ARM core.
            let destination_bits = word(i).checked_shl(offset_bits).unwrap_or(0)
                | word(i + 1).checked_shr(32 - offset_bits).unwrap_or(0);
            xcorr += (!(source_bits ^ destination_bits)).count_ones();
        }
        if xcorr > self.best_score {
            self.best_match = self.candidate;
            self.best_score = xcorr;
        }
        self.candidate += 1;
        self.done = self.candidate >= self.size;
    }
}
