//! `VoicForm`: a four-formant singing voice (`include/VoicForm.h`,
//! `src/VoicForm.cpp`), with its `SingWave` glottal source
//! (`include/SingWave.h`, `src/SingWave.cpp`).
//!
//! A looped glottal pulse (`impuls20.raw`) with vibrato, random pitch
//! wander and a pitch glide, plus enveloped noise, goes through a spectral
//! tilt filter and four sweepable formant filters set from the phoneme
//! table (see [`crate::phonemes`]).
//!
//! Copyright (c) 1995--2023 Perry R. Cook and Gary P. Scavone (STK license,
//! see the crate documentation). Rust port for Krabton.
//!
//! Krabton changes: the voice only sounds between `note_on` and `note_off`
//! (in STK, selecting a phoneme starts the voice even without a note). The
//! voiced and noise gains of the selected phoneme (or the Voiced/Unvoiced
//! mix, control 2) are remembered and used at `note_on`: the voiced part
//! gets `amplitude` times the phoneme's voice gain (STK: `amplitude`), and
//! the noise part the phoneme's noise gain. Formant radii, sweep and
//! envelope rates and the tilt pole are given for 44.1 kHz and converted.

use crate::phonemes;
use crate::rawwaves;
use crate::ugens::*;
use crate::Model;

/// `SingWave`: a looped wave with vibrato, random wander and pitch glide.
struct SingWave {
    wave: WaveLoop,
    modulator: Modulate,
    envelope: Envelope,
    pitch_envelope: Envelope,
    rate: f64,
    sweep_rate: f64,
    sr: f64,
}

impl SingWave {
    fn new(sr: f64) -> Self {
        let mut modulator = Modulate::new(sr, 0x5196);
        modulator.set_vibrato_rate(6.0);
        modulator.set_vibrato_gain(0.04);
        modulator.set_random_gain(0.005);
        let mut s = Self {
            wave: WaveLoop::new(rawwaves::impuls20(), sr),
            modulator,
            envelope: Envelope::default(),
            pitch_envelope: Envelope::default(),
            rate: 1.0,
            sweep_rate: rate(0.001, sr),
            sr,
        };
        s.set_frequency(75.0);
        s.pitch_envelope.set_rate(1.0);
        s.tick();
        s.tick();
        s.pitch_envelope.set_rate(s.sweep_rate * s.rate);
        s
    }

    fn set_frequency(&mut self, frequency: f64) {
        let previous = self.rate;
        self.rate = self.wave.size() * frequency / self.sr;
        let temp = (previous - self.rate).abs();
        self.pitch_envelope.set_target(self.rate);
        self.pitch_envelope.set_rate(self.sweep_rate * temp);
    }

    #[inline]
    fn tick(&mut self) -> f64 {
        let mut new_rate = self.pitch_envelope.tick();
        new_rate += new_rate * self.modulator.tick();
        self.wave.set_rate(new_rate);
        self.wave.tick() * self.envelope.tick()
    }
}

pub struct VoicForm {
    sr: f64,
    voiced: SingWave,
    noise: Noise,
    noise_env: Envelope,
    filters: [FormSwep; 4],
    onepole: OnePole,
    onezero: OneZero,
    voiced_gain: f64,
    unvoiced_gain: f64,
    active: bool,
}

impl VoicForm {
    pub fn new(sr: f64) -> Self {
        let mut voiced = SingWave::new(sr);
        voiced.envelope.set_rate(rate(0.001, sr));
        voiced.envelope.set_target(0.0);
        let mut filters: [FormSwep; 4] = Default::default();
        for f in &mut filters {
            f.set_sweep_rate(rate(0.001, sr));
        }
        let mut noise_env = Envelope::default();
        noise_env.set_rate(rate(0.001, sr));
        noise_env.set_target(0.0);
        let mut v = Self {
            sr,
            voiced,
            noise: Noise::new(0x7015),
            noise_env,
            filters,
            onepole: OnePole::new(radius(0.9, sr)),
            onezero: OneZero::new(-0.9),
            voiced_gain: 1.0,
            unvoiced_gain: 0.0,
            active: false,
        };
        v.set_phoneme(0, 1.0);
        v.clear();
        v
    }

    /// Sets the formant targets of phoneme `index`, with its formant
    /// frequencies scaled by `scale`, and its voiced and noise gains.
    fn set_phoneme(&mut self, index: usize, scale: f64) {
        let i = index % 32;
        for (k, f) in self.filters.iter_mut().enumerate() {
            let [freq, r, db] = phonemes::PARAMETERS[i][k];
            f.set_targets(scale * freq, radius(r, self.sr), 10f64.powf(db / 20.0), self.sr);
        }
        self.set_voiced(phonemes::GAINS[i][0]);
        self.set_unvoiced(phonemes::GAINS[i][1]);
    }

    fn set_voiced(&mut self, gain: f64) {
        self.voiced_gain = gain;
        if self.active {
            self.voiced.envelope.set_target(gain);
        }
    }

    fn set_unvoiced(&mut self, gain: f64) {
        self.unvoiced_gain = gain;
        if self.active {
            self.noise_env.set_target(gain);
        }
    }

    fn set_tilt(&mut self, amount: f64) {
        self.onepole.set_pole(radius(0.97 - amount * 0.2, self.sr));
    }
}

impl Model for VoicForm {
    fn note_on(&mut self, frequency: f64, amplitude: f64) {
        self.voiced.set_frequency(frequency);
        self.voiced.envelope.set_target(amplitude * self.voiced_gain);
        self.noise_env.set_target(self.unvoiced_gain);
        self.set_tilt(amplitude);
        self.active = true;
    }

    fn note_off(&mut self, _amplitude: f64) {
        self.voiced.envelope.key_off();
        self.noise_env.set_target(0.0);
        self.active = false;
    }

    fn control_change(&mut self, number: u8, value: f64) {
        let n = value * ONE_OVER_128;
        match number {
            // Voiced/unvoiced mix.
            2 => {
                self.set_voiced(1.0 - n);
                self.set_unvoiced(0.01 * n);
            }
            // Vowel/phoneme selection, in four ranges of formant scaling.
            4 => {
                let i = value as usize;
                let (index, scale) = match i {
                    0..=31 => (i, 0.9),
                    32..=63 => (i - 32, 1.0),
                    64..=95 => (i - 64, 1.1),
                    96..=127 => (i - 96, 1.2),
                    _ => (0, 1.4),
                };
                self.set_phoneme(index, scale);
            }
            11 => self.voiced.modulator.set_vibrato_rate(n * 12.0),
            1 => self.voiced.modulator.set_vibrato_gain(n * 0.2),
            // Loudness (spectral tilt), on a sounding note.
            128 if self.active => {
                self.voiced.envelope.set_target(n);
                self.set_tilt(n);
            }
            _ => {}
        }
    }

    fn set_preset(&mut self, index: usize) {
        self.set_phoneme(index, 1.0);
    }

    fn tick(&mut self) -> f64 {
        let mut temp = self.onepole.tick(self.onezero.tick(self.voiced.tick()));
        temp += self.noise_env.tick() * self.noise.tick();
        let mut y = 0.0;
        for f in &mut self.filters {
            y += f.tick(temp, self.sr);
        }
        y
    }

    fn clear(&mut self) {
        self.onezero.clear();
        self.onepole.clear();
        for f in &mut self.filters {
            f.clear();
        }
    }
}
