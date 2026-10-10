//! `Recorder`: a recorder / flute physical model (`include/Recorder.h`,
//! `src/Recorder.cpp`), by Mathias Bredholt (McGill University), formatted
//! for STK by Gary Scavone in 2023.
//!
//! Based on "Sound production in recorderlike instruments. II. A simulation
//! model" by M.P. Verge, A. Hirschberg and R. Causse (JASA 1997): a jet
//! with a delay and a bandpass jet filter drives the mouth, which feeds a
//! pipe made of two delay lines with viscothermal loss and radiation
//! filters, plus filtered turbulence noise.
//!
//! Copyright (c) 1995--2023 Perry R. Cook and Gary P. Scavone (STK license,
//! see the crate documentation). Rust port for Krabton.
//!
//! Krabton changes: Softness (2) at 0 is clamped to a tiny positive value,
//! because STK divides by it (0/0 gives NaN there); and STK's radiation
//! filter becomes unstable above about 62 kHz, so at higher host rates the
//! model runs at the host rate divided by 2 (or more, to stay at or below
//! 50 kHz) with linear interpolation.
//!
//! Like in STK, the pitch does not reliably follow the note: the pipe is
//! tuned an octave below the note (two delay lines of `sr / f - 7`
//! samples) and, with the fixed breath pressure of `note_on`, the jet
//! locks onto an upper pipe mode or an edge tone (about 1.3 kHz at
//! 220 Hz). Only some pressure and note combinations sound the note.

use crate::ugens::*;
use crate::{max_delay_samples, release_amplitude, Model};

const RHO: f64 = 1.2041; // density of air
const C0: f64 = 343.21; // speed of sound in air
const LC: f64 = 0.02; // length of flue canal
const H_EXIT: f64 = 0.001; // height of flue exit
const H_PIPE: f64 = 0.02; // pipe diameter
const W: f64 = 4.0 * H_EXIT; // width of mouth
const SP: f64 = H_PIPE * H_PIPE; // cross-section of pipe
const SM: f64 = W * H_PIPE; // cross-section of mouth
const DIN: f64 = 0.0030; // end correction
const DOUT: f64 = 0.0063; // end correction
const DM: f64 = DIN + DOUT; // end correction of mouth
const DD: f64 = 0.0035; // acoustic distance between Q1 and Q2
const B_JET: f64 = 0.4 * H_EXIT; // jet width
const B2: f64 = SP / (RHO * C0);

/// Highest rate the model runs at; higher host rates run it at a fraction
/// of the rate with linear interpolation.
const MAX_INTERNAL_RATE: f64 = 50000.0;

pub struct Recorder {
    /// The internal sample rate: the host rate divided by `sub`.
    sr: f64,
    /// Host samples per internal sample, and the interpolation state.
    sub: usize,
    phase: usize,
    y_prev: f64,
    y_cur: f64,
    pin_delay: DelayL,
    pout_delay: DelayL,
    jet_delay: DelayL,
    radiation_filter: Iir,
    visco_in_filter: Iir,
    visco_out_filter: Iir,
    jet_filter: Iir,
    turb: Noise,
    turb_filter: Iir,
    vibrato: SineWave,
    adsr: Adsr,
    max_pressure: f64,
    vibrato_gain: f64,
    noise_gain: f64,
    output_gain: f64,
    psi: f64,
    pout: f64,
    poutm1: f64,
    poutm2: f64,
    pin: f64,
    pinm1: f64,
    pinm2: f64,
    b1: f64,
    b3: f64,
    b4: f64,
    uj: f64,
    qj: f64,
    qjm1: f64,
    qjm2: f64,
    q1: f64,
    qp: f64,
    pm: f64,
    active: bool,
}

impl Recorder {
    pub fn new(host_sr: f64) -> Self {
        // STK's radiation filter is unstable above about 62 kHz.
        let sub = (host_sr / MAX_INTERNAL_RATE).ceil().max(1.0) as usize;
        let sr = host_sr / sub as f64;
        let n = max_delay_samples(sr, 1.0);
        let mut pin_delay = DelayL::new(0.0, n);
        let mut pout_delay = DelayL::new(0.0, n);
        let mut jet_delay = DelayL::new(0.0, 4095);
        pin_delay.tick(0.0);
        pout_delay.tick(0.0);
        jet_delay.tick(0.0);
        jet_delay.set_delay(200.0);
        let mut vibrato = SineWave::new(sr);
        vibrato.set_frequency(4.0);

        let t = 1.0 / sr;
        let b1 = RHO / (4.0 * PI * C0 * t * t);
        let b3 = DM * SP / (t * SM * C0);
        let b4 = RHO * DOUT / (SM * t);

        // Radiation filter.
        let rp = (SP / PI).sqrt();
        let a = rp * rp / (4.0 * C0 * C0 * t * t);
        let b = 0.82 * rp / (C0 * t);
        let mut radiation_filter = Iir::default();
        radiation_filter.set_coefficients(&[1.0 + a - b, b - 2.0 * a, a], &[a - b - 1.0, b - 2.0 * a, a]);

        // Viscothermal losses (coefficients as designed in STK).
        let b_visco = [0.83820223947141, -0.16888603248373, -0.64759781930259, 0.07424498608506];
        let a_visco = [1.0, -0.33623476246554, -0.71257915055968, 0.14508304017256];
        let mut visco_in_filter = Iir::default();
        visco_in_filter.set_coefficients(&b_visco, &a_visco);
        let visco_out_filter = visco_in_filter.clone();

        let mut r = Self {
            sr,
            sub,
            phase: 0,
            y_prev: 0.0,
            y_cur: 0.0,
            pin_delay,
            pout_delay,
            jet_delay,
            radiation_filter,
            visco_in_filter,
            visco_out_filter,
            jet_filter: Iir::default(),
            turb: Noise::new(0x7EC0),
            turb_filter: Iir::default(),
            vibrato,
            adsr: Adsr::new(sr),
            max_pressure: 0.0,
            vibrato_gain: 0.0,
            noise_gain: 0.2,
            output_gain: 0.0,
            psi: 1.0,
            pout: 0.0,
            poutm1: 0.0,
            poutm2: 0.0,
            pin: 0.0,
            pinm1: 0.0,
            pinm2: 0.0,
            b1,
            b3,
            b4,
            uj: 0.0,
            qj: 0.0,
            qjm1: 0.0,
            qjm2: 0.0,
            q1: 0.0,
            qp: 0.0,
            pm: 0.0,
            active: false,
        };
        r.set_breath_cutoff(500.0);
        r.set_frequency(880.0);
        r
    }

    fn set_frequency(&mut self, frequency: f64) {
        let m = self.sr / frequency - 4.0 - 3.0;
        self.pin_delay.set_delay(m);
        self.pout_delay.set_delay(m);
    }

    fn set_breath_cutoff(&mut self, cutoff: f64) {
        let q = 0.99;
        let r = 2.0 * (PI * cutoff / self.sr).sin();
        let qq = 1.0 - r * q;
        self.turb_filter.set_coefficients(&[r * r], &[1.0, r * r - qq - 1.0, qq]);
    }
}

impl Model for Recorder {
    fn note_on(&mut self, frequency: f64, amplitude: f64) {
        self.set_frequency(frequency);
        let r = rate(amplitude * 0.02, self.sr);
        if r > 0.0 {
            self.adsr.set_attack_rate(r);
            self.max_pressure = 35.0 * (1.1 + amplitude * 0.20);
            self.adsr.key_on();
        }
        self.output_gain = amplitude / 40.0;
        self.active = true;
    }

    fn note_off(&mut self, amplitude: f64) {
        self.adsr.set_release_rate(rate(release_amplitude(amplitude) * 0.02, self.sr));
        self.adsr.key_off();
        self.active = false;
    }

    fn control_change(&mut self, number: u8, value: f64) {
        let n = value * ONE_OVER_128;
        match number {
            2 => self.psi = (2.0 * n).max(1e-3),
            16 => self.set_breath_cutoff(n * 2000.0),
            4 => self.noise_gain = n,
            11 => self.vibrato.set_frequency(n * 12.0),
            1 => self.vibrato_gain = n * 0.4,
            128 if self.active => self.max_pressure = 35.0 * n,
            _ => {}
        }
    }

    fn tick(&mut self) -> f64 {
        if self.phase == 0 {
            self.y_prev = self.y_cur;
            self.y_cur = self.tick_internal();
        }
        self.phase += 1;
        let y = self.y_prev + (self.y_cur - self.y_prev) * self.phase as f64 / self.sub as f64;
        if self.phase == self.sub {
            self.phase = 0;
        }
        y
    }

    fn clear(&mut self) {
        self.clear_internal();
    }
}

impl Recorder {
    /// One sample of the STK model at the internal rate.
    fn tick_internal(&mut self) -> f64 {
        self.pinm2 = self.pinm1;
        self.pinm1 = self.pin;
        self.pin = self.pin_delay.last_out();

        self.poutm2 = self.poutm1;
        self.poutm1 = self.pout;
        let mut pout_l = self.pout_delay.last_out();

        self.pin = self.visco_in_filter.tick(self.pin);
        pout_l = self.visco_out_filter.tick(pout_l);

        let pf = self.max_pressure
            * self.adsr.tick()
            * (self.vibrato.tick() * self.vibrato_gain + (1.0 - self.vibrato_gain));

        let t = 1.0 / self.sr;

        // Jet velocity and flow.
        let ujm1 = self.uj;
        self.uj = ujm1 + t / (RHO * LC) * (pf - self.pm - 0.5 * RHO * ujm1 * ujm1);
        self.qjm2 = self.qjm1;
        self.qjm1 = self.qj;
        self.qj = H_EXIT * H_PIPE * self.uj;

        // Jet drive: a bandpass jet filter tuned by the steady jet velocity.
        let uj_steady = (2.0 * pf / RHO).sqrt().max(0.1);
        let fc_jet = 0.36 / W * uj_steady;
        let g_jet = 0.002004 * (-0.06046 * uj_steady).exp();
        let r_jet = 0.95 - uj_steady * 0.015;
        let b0_jet = g_jet * (1.0 - r_jet * r_jet) / 2.0;
        self.jet_filter.set_coefficients(
            &[b0_jet, 0.0, -b0_jet],
            &[1.0, -2.0 * r_jet * (2.0 * PI * fc_jet * t).cos(), r_jet * r_jet],
        );
        let eta = self.jet_filter.tick(self.jet_delay.last_out());

        let q1m1 = self.q1;
        self.q1 = B_JET * H_PIPE * self.uj * (1.0 + (eta / (self.psi * B_JET)).tanh());
        let pjd = -RHO * DD / SM * (self.q1 - q1m1) / t;

        // Nonlinear losses at the labium.
        let qp_sign = if self.qp < 0.0 {
            -1.0
        } else if self.qp > 0.0 {
            1.0
        } else {
            0.0
        };
        let pa = -0.5 * RHO * (self.qp / (0.6 * SM)) * (self.qp / (0.6 * SM)) * qp_sign;

        // Turbulence.
        let pt = self.turb_filter.tick(self.noise_gain * self.turb.tick() * 0.5 * RHO * self.uj * self.uj);

        let dp = pjd + pa + pt;

        let b1 = self.b1;
        self.pout = ((self.b3 - b1 * B2 - 1.0) * self.pin
            + (2.0 * b1 * B2 - self.b3) * (self.pinm1 - self.poutm1)
            + b1 * B2 * (self.poutm2 - self.pinm2)
            - b1 * (self.qj - 2.0 * self.qjm1 + self.qjm2)
            + self.b4 * (self.qj - self.qjm1)
            + dp)
            / (1.0 - b1 * B2 + self.b3);

        let qpm1 = self.qp;
        self.qp = SP / (RHO * C0) * (self.pout - self.pin);
        self.pm = self.pout + self.pin - dp + RHO * DIN / SM * (self.qp - qpm1) / t;

        let q1d = self.q1 - 0.5 * B_JET * H_PIPE * self.uj;
        let vac = 2.0 / PI * self.qp / SM - 0.38 * q1d / SM;
        self.jet_delay.tick(vac);
        self.jet_delay.set_delay((W / (0.6 * uj_steady * t)).min(200.0));

        let pin_l = self.radiation_filter.tick(pout_l);
        self.pout_delay.tick(self.pout);
        self.pin_delay.tick(pin_l);

        self.output_gain * (self.pout + self.pin)
    }

    fn clear_internal(&mut self) {
        self.y_prev = 0.0;
        self.y_cur = 0.0;
        self.pin_delay.clear();
        self.pout_delay.clear();
        self.jet_delay.clear();
        self.radiation_filter.clear();
        self.visco_in_filter.clear();
        self.visco_out_filter.clear();
        self.jet_filter.clear();
        self.turb_filter.clear();
        self.pout = 0.0;
        self.poutm1 = 0.0;
        self.poutm2 = 0.0;
        self.pin = 0.0;
        self.pinm1 = 0.0;
        self.pinm2 = 0.0;
        self.uj = 0.0;
        self.qj = 0.0;
        self.qjm1 = 0.0;
        self.qjm2 = 0.0;
        self.q1 = 0.0;
        self.qp = 0.0;
        self.pm = 0.0;
    }
}
