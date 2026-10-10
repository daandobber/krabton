//! `BlowHole`: a clarinet with a tonehole and a register vent
//! (`include/BlowHole.h`, `src/BlowHole.cpp`).
//!
//! The clarinet bore is split into three delay lines around a three-port
//! scattering junction for the tonehole (a pole-zero filter) and a register
//! vent filter near the reed, after Scavone and Cook's 1998 tonehole and
//! register-hole models.
//!
//! Copyright (c) 1995--2023 Perry R. Cook and Gary P. Scavone (STK license,
//! see the crate documentation). Rust port for Krabton.
//!
//! Krabton change: the default of Tonehole State (11) in [`crate::MODELS`]
//! is 0, closed, so notes sound at their pitch; STK's constructor starts
//! with the tonehole open, which raises every note by about 4%.

use crate::ugens::*;
use crate::{max_delay_samples, release_amplitude, Model};

pub struct BlowHole {
    sr: f64,
    delays: [DelayL; 3],
    reed_table: ReedTable,
    filter: OneZero,
    tonehole: PoleZero,
    vent: PoleZero,
    envelope: Envelope,
    noise: Noise,
    vibrato: SineWave,
    scatter: f64,
    th_coeff: f64,
    rh_gain: f64,
    output_gain: f64,
    noise_gain: f64,
    vibrato_gain: f64,
    active: bool,
}

impl BlowHole {
    pub fn new(sr: f64) -> Self {
        let mut delays = [
            DelayL::new(0.0, 4095.max((5.0 * sr / 22050.0) as usize + 2)),
            DelayL::new(0.0, max_delay_samples(sr, 0.5)),
            DelayL::new(0.0, 4095.max((4.0 * sr / 22050.0) as usize + 2)),
        ];
        // Delays 0 and 2 are fixed: reed to register vent, tonehole to bell.
        delays[0].set_delay(5.0 * sr / 22050.0);
        delays[2].set_delay(4.0 * sr / 22050.0);

        // Bore and tonehole radii (meters) and the scattering coefficient.
        let rb: f64 = 0.0075;
        let rth: f64 = 0.003;
        let scatter = -rth.powi(2) / (rth.powi(2) + 2.0 * rb.powi(2));

        // Tonehole: effective length 1.4 times the radius.
        let te = 1.4 * rth;
        let th_coeff = (te * 2.0 * sr - 347.23) / (te * 2.0 * sr + 347.23);
        let mut tonehole = PoleZero::default();
        tonehole.a1 = -th_coeff;
        tonehole.b0 = th_coeff;
        tonehole.b1 = -1.0;

        // Register vent: radius 1.5 mm, closed at first.
        let r_rh: f64 = 0.0015;
        let te = 1.4 * r_rh;
        let xi = 0.0;
        let zeta = 347.23 + 2.0 * PI * rb.powi(2) * xi / 1.1769;
        let psi = 2.0 * PI * rb.powi(2) * te / (PI * r_rh.powi(2));
        let rh_coeff = (zeta - 2.0 * sr * psi) / (zeta + 2.0 * sr * psi);
        let rh_gain = -347.23 / (zeta + 2.0 * sr * psi);
        let mut vent = PoleZero::default();
        vent.a1 = rh_coeff;
        vent.b0 = 1.0;
        vent.b1 = 1.0;
        vent.gain = 0.0;

        let mut vibrato = SineWave::new(sr);
        vibrato.set_frequency(5.735);
        let mut b = Self {
            sr,
            delays,
            reed_table: ReedTable { offset: 0.7, slope: -0.3 },
            filter: OneZero::new(-1.0),
            tonehole,
            vent,
            envelope: Envelope::default(),
            noise: Noise::new(0xB701),
            vibrato,
            scatter,
            th_coeff,
            rh_gain,
            output_gain: 1.0,
            noise_gain: 0.2,
            vibrato_gain: 0.01,
            active: false,
        };
        b.set_frequency(220.0);
        b.clear();
        b
    }

    fn set_frequency(&mut self, frequency: f64) {
        // Total delay is the bore round trip minus the fixed sections and
        // about 3.5 samples for the filters.
        let mut delay = (self.sr / frequency) * 0.5 - 3.5;
        delay -= self.delays[0].delay() + self.delays[2].delay();
        self.delays[1].set_delay(delay);
    }

    /// `setVent`: 0 is closed, 1 is fully open.
    fn set_vent(&mut self, value: f64) {
        self.vent.gain = value.clamp(0.0, 1.0) * self.rh_gain;
    }

    /// `setTonehole`: 0 is closed, 1 is fully open.
    fn set_tonehole(&mut self, value: f64) {
        let coeff = if value <= 0.0 {
            0.9995
        } else if value >= 1.0 {
            self.th_coeff
        } else {
            value * (self.th_coeff - 0.9995) + 0.9995
        };
        self.tonehole.a1 = -coeff;
        self.tonehole.b0 = coeff;
    }
}

impl Model for BlowHole {
    fn note_on(&mut self, frequency: f64, amplitude: f64) {
        self.set_frequency(frequency);
        let r = rate(amplitude * 0.005, self.sr);
        if r > 0.0 {
            self.envelope.set_rate(r);
            self.envelope.set_target(0.55 + amplitude * 0.30);
        }
        self.output_gain = amplitude + 0.001;
        self.active = true;
    }

    fn note_off(&mut self, amplitude: f64) {
        self.envelope.set_rate(rate(release_amplitude(amplitude) * 0.01, self.sr));
        self.envelope.set_target(0.0);
        self.active = false;
    }

    fn control_change(&mut self, number: u8, value: f64) {
        let n = value * ONE_OVER_128;
        match number {
            2 => self.reed_table.slope = -0.44 + 0.26 * n,
            4 => self.noise_gain = n * 0.4,
            11 => self.set_tonehole(n),
            1 => self.set_vent(n),
            128 if self.active => self.envelope.set_value(n),
            _ => {}
        }
    }

    fn tick(&mut self) -> f64 {
        let mut breath_pressure = self.envelope.tick();
        breath_pressure += breath_pressure * self.noise_gain * self.noise.tick();
        breath_pressure += breath_pressure * self.vibrato_gain * self.vibrato.tick();

        // Calculate the differential pressure = reflected - mouthpiece pressures.
        let pressure_diff = self.delays[0].last_out() - breath_pressure;

        // Do two-port junction scattering for the register vent.
        let mut pa = breath_pressure + pressure_diff * self.reed_table.tick(pressure_diff);
        let mut pb = self.delays[1].last_out();
        self.vent.tick(pa + pb);

        let out = self.delays[0].tick(self.vent.last_out() + pb) * self.output_gain;

        // Do three-port junction scattering (under the tonehole).
        pa += self.vent.last_out();
        pb = self.delays[2].last_out();
        let pth = self.tonehole.last_out();
        let temp = self.scatter * (pa + pb - 2.0 * pth);

        self.delays[2].tick(self.filter.tick(pa + temp) * -0.95);
        self.delays[1].tick(pb + temp);
        self.tonehole.tick(pa + pb - pth + temp);
        out
    }

    fn clear(&mut self) {
        for d in &mut self.delays {
            d.clear();
        }
        self.filter.clear();
        self.tonehole.clear();
        self.vent.clear();
    }
}
