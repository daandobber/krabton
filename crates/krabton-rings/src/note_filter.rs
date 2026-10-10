//! Smoothing of the V/Oct note (`rings/dsp/note_filter.h`): follows sharp
//! changes at once, median-filters and lags small ones, and keeps a slightly
//! delayed copy for the voice a strum leaves.
//!
//! Copyright 2015 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::filter::DelayLine;

/// Median filter order.
const N: usize = 4;

#[derive(Clone, Debug)]
pub struct NoteFilter {
    previous_values: [f32; N],
    note: f32,
    stable_note: f32,
    delayed_stable_note: DelayLine<16>,
    coefficient: f32,
    stable_coefficient: f32,
    fast_coefficient: f32,
    slow_coefficient: f32,
    lag_coefficient: f32,
}

impl Default for NoteFilter {
    fn default() -> Self {
        Self {
            previous_values: [0.0; N],
            note: 0.0,
            stable_note: 0.0,
            delayed_stable_note: DelayLine::default(),
            coefficient: 0.0,
            stable_coefficient: 0.0,
            fast_coefficient: 0.0,
            slow_coefficient: 0.0,
            lag_coefficient: 0.0,
        }
    }
}

impl NoteFilter {
    pub fn init(
        &mut self,
        sample_rate: f32,
        time_constant_fast_edge: f32,
        time_constant_steady_part: f32,
        edge_recovery_time: f32,
        edge_avoidance_delay: f32,
    ) {
        self.fast_coefficient = 1.0 / (time_constant_fast_edge * sample_rate);
        self.slow_coefficient = 1.0 / (time_constant_steady_part * sample_rate);
        self.lag_coefficient = 1.0 / (edge_recovery_time * sample_rate);

        self.delayed_stable_note.reset();
        self.delayed_stable_note.set_delay(15.min((edge_avoidance_delay * sample_rate) as usize));

        self.note = 69.0;
        self.stable_note = 69.0;
        self.coefficient = self.fast_coefficient;
        self.stable_coefficient = self.slow_coefficient;
        self.previous_values = [self.note; N];
    }

    /// Port addition, called right after a strum has read
    /// [`stable_note`](Self::stable_note): the module's trigger debounce
    /// keeps strums at least 10 ms apart, so the delayed note has caught up
    /// with the strummed note when the next strum reads it. Notes from a
    /// host can come one block apart (chords); filling the delay with the
    /// strummed note at once keeps each voice at its own pitch then.
    pub fn settle(&mut self) {
        self.delayed_stable_note.fill(self.note);
    }

    pub fn process(&mut self, note: f32, strum: bool) -> f32 {
        // If there is a sharp change, follow it instantly.
        if (note - self.note).abs() > 0.4 || strum {
            self.note = note;
            self.stable_note = note;
            self.coefficient = self.fast_coefficient;
            self.stable_coefficient = self.slow_coefficient;
            self.previous_values = [note; N];
        } else {
            // Median filtering of the raw value.
            self.previous_values.rotate_left(1);
            self.previous_values[N - 1] = note;
            let mut sorted = self.previous_values;
            sorted.sort_by(|a, b| a.total_cmp(b));
            let median = 0.5 * (sorted[(N - 1) / 2] + sorted[N / 2]);

            // Adaptive lag processor.
            self.note += self.coefficient * (median - self.note);
            self.stable_note += self.stable_coefficient * (self.note - self.stable_note);

            self.coefficient += self.lag_coefficient * (self.slow_coefficient - self.coefficient);
            self.stable_coefficient += self.lag_coefficient * (self.lag_coefficient - self.stable_coefficient);

            self.delayed_stable_note.write(self.stable_note);
        }
        self.note
    }

    pub fn note(&self) -> f32 {
        self.note
    }

    pub fn stable_note(&self) -> f32 {
        self.delayed_stable_note.read()
    }
}
