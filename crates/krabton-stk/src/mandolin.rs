//! `Mandolin`: a two-string mandolin (`include/Mandolin.h`,
//! `src/Mandolin.cpp`) and its `Twang` strings (`include/Twang.h`,
//! `src/Twang.cpp`).
//!
//! Two slightly detuned commuted-synthesis strings (allpass-interpolated
//! Karplus-Strong loops with a pluck-position comb filter) are excited by
//! one of twelve recorded mandolin body impulse responses (`mand1.raw` ..
//! `mand12.raw`), selected as the "microphone position", after Smith and
//! Jaffe's commuted synthesis.
//!
//! Copyright (c) 1995--2023 Perry R. Cook and Gary P. Scavone (STK license,
//! see the crate documentation). Rust port for Krabton.
//!
//! Krabton changes: no pluck sounds before the first note (in STK the body
//! sample starts playing when the instrument is constructed), and
//! `note_off` damps the strings like `StifKarp` does, with a loop gain of
//! (1 - amplitude) / 2 until the next note; STK's Mandolin ignores
//! note-offs and rings out.

use crate::rawwaves;
use crate::ugens::*;
use crate::{max_delay_samples, release_amplitude, Model};

/// `Twang`: an enhanced Karplus-Strong string with a pluck-position comb.
pub struct Twang {
    sr: f64,
    delay_line: DelayA,
    comb_delay: DelayL,
    loop_filter: Fir2,
    frequency: f64,
    loop_gain: f64,
    pluck_position: f64,
}

impl Twang {
    pub fn new(sr: f64, max_delay: usize) -> Self {
        let mut t = Self {
            sr,
            delay_line: DelayA::new(0.5, max_delay),
            comb_delay: DelayL::new(0.0, max_delay),
            loop_filter: Fir2::new(0.5, 0.5),
            frequency: 220.0,
            loop_gain: 0.995,
            pluck_position: 0.4,
        };
        t.set_frequency(220.0);
        t
    }

    pub fn set_frequency(&mut self, frequency: f64) {
        self.frequency = frequency;
        // Delay = length - filter delay.
        let delay = self.sr / frequency - self.loop_filter.phase_delay(frequency, self.sr);
        self.delay_line.set_delay(delay);
        self.set_loop_gain(self.loop_gain);
        // Set the pluck position, which puts zeroes at position * length.
        self.comb_delay.set_delay(0.5 * self.pluck_position * delay);
    }

    pub fn set_loop_gain(&mut self, loop_gain: f64) {
        if !(0.0..1.0).contains(&loop_gain) {
            return;
        }
        self.loop_gain = loop_gain;
        self.loop_filter.gain = self.filter_gain(loop_gain);
    }

    fn filter_gain(&self, loop_gain: f64) -> f64 {
        let gain = loop_gain + self.frequency * 0.000005;
        if gain >= 1.0 {
            0.99999
        } else {
            gain
        }
    }

    /// Damps the string until the next `set_frequency` / `set_loop_gain`.
    pub fn damp(&mut self, loop_gain: f64) {
        self.loop_filter.gain = self.loop_filter.gain.min(self.filter_gain(loop_gain));
    }

    pub fn set_pluck_position(&mut self, position: f64) {
        if (0.0..=1.0).contains(&position) {
            self.pluck_position = position;
        }
    }

    #[inline]
    pub fn tick(&mut self, input: f64) -> f64 {
        let x = input + self.loop_filter.tick(self.delay_line.last_out());
        let mut y = self.delay_line.tick(x);
        y -= self.comb_delay.tick(y);
        y * 0.5
    }

    pub fn clear(&mut self) {
        self.delay_line.clear();
        self.comb_delay.clear();
        self.loop_filter.clear();
    }
}

pub struct Mandolin {
    sr: f64,
    strings: [Twang; 2],
    soundfiles: [WvIn; 12],
    mic: usize,
    detuning: f64,
    frequency: f64,
    pluck_amplitude: f64,
}

impl Mandolin {
    pub fn new(sr: f64) -> Self {
        // The second string can be detuned down by 10%.
        let n = max_delay_samples(sr, 1.15);
        let bodies = rawwaves::mandolin_bodies();
        let mut m = Self {
            sr,
            strings: [Twang::new(sr, n), Twang::new(sr, n)],
            soundfiles: std::array::from_fn(|i| WvIn::new(&bodies[i], sr)),
            mic: 0,
            detuning: 0.995,
            frequency: 220.0,
            pluck_amplitude: 0.5,
        };
        m.set_frequency(220.0);
        m.set_pluck_position(0.4);
        m
    }

    fn set_frequency(&mut self, frequency: f64) {
        self.frequency = frequency;
        self.strings[0].set_frequency(frequency);
        self.strings[1].set_frequency(frequency * self.detuning);
    }

    fn set_pluck_position(&mut self, position: f64) {
        self.strings[0].set_pluck_position(position);
        self.strings[1].set_pluck_position(position);
    }

    fn set_detune(&mut self, detune: f64) {
        if detune <= 0.0 {
            return;
        }
        self.detuning = detune;
        self.strings[1].set_frequency(self.frequency * self.detuning);
    }

    fn set_body_size(&mut self, size: f64) {
        // Scale the commuted body response by its sample rate (22050).
        let r = size * 22050.0 / self.sr;
        for s in &mut self.soundfiles {
            s.set_rate(r);
        }
    }
}

impl Model for Mandolin {
    fn note_on(&mut self, frequency: f64, amplitude: f64) {
        self.set_frequency(frequency);
        self.soundfiles[self.mic].reset();
        self.pluck_amplitude = amplitude;
    }

    fn note_off(&mut self, amplitude: f64) {
        let g = (1.0 - release_amplitude(amplitude)) * 0.5;
        self.strings[0].damp(g);
        self.strings[1].damp(g);
    }

    fn control_change(&mut self, number: u8, value: f64) {
        let n = value * ONE_OVER_128;
        match number {
            2 => self.set_body_size(n * 2.0),
            4 => self.set_pluck_position(n),
            11 => {
                self.strings[0].set_loop_gain(0.97 + n * 0.03);
                self.strings[1].set_loop_gain(0.97 + n * 0.03);
            }
            1 => self.set_detune(1.0 - n * 0.1),
            128 => self.mic = ((n * 11.0) as usize).min(11),
            _ => {}
        }
    }

    fn tick(&mut self) -> f64 {
        let s = &mut self.soundfiles[self.mic];
        let temp = if !s.is_finished() { s.tick() * self.pluck_amplitude } else { 0.0 };
        let mut y = self.strings[0].tick(temp);
        y += self.strings[1].tick(temp);
        y * 0.2
    }

    fn clear(&mut self) {
        self.strings[0].clear();
        self.strings[1].clear();
    }
}
