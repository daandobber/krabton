//! Second-order filters that follow their analog prototypes up to the
//! Nyquist frequency.
//!
//! The usual bilinear designs squeeze the response near Nyquist ("cramping"):
//! a wide bell at 12 kHz loses its upper half at 44.1 kHz. Here the poles
//! come from the analog poles by impulse invariance and the numerator is
//! fitted so the digital magnitude equals the analog one at DC, at the
//! filter frequency and at Nyquist (after M. Vicanek, "Matched Second Order
//! Digital Filters", 2016). All design math is in f64; processing uses a
//! transposed direct form II in f64, which stays quiet at low frequencies.

use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coefs {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a1: f64,
    pub a2: f64,
}

impl Default for Coefs {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Coefs {
    pub const IDENTITY: Coefs = Coefs { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0 };

    /// Magnitude at `w` radians per sample.
    pub fn mag(&self, w: f64) -> f64 {
        let (c1, s1) = (w.cos(), w.sin());
        let (c2, s2) = ((2.0 * w).cos(), (2.0 * w).sin());
        let (nr, ni) = (self.b0 + self.b1 * c1 + self.b2 * c2, -(self.b1 * s1 + self.b2 * s2));
        let (dr, di) = (1.0 + self.a1 * c1 + self.a2 * c2, -(self.a1 * s1 + self.a2 * s2));
        ((nr * nr + ni * ni) / (dr * dr + di * di).max(1e-300)).sqrt()
    }

    pub fn mag_db(&self, w: f64) -> f64 {
        20.0 * self.mag(w).max(1e-12).log10()
    }

    fn scaled(self, k: f64) -> Self {
        Coefs { b0: self.b0 * k, b1: self.b1 * k, b2: self.b2 * k, ..self }
    }
}

/// Filter memory (transposed direct form II).
#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    z1: f64,
    z2: f64,
}

impl State {
    #[inline]
    pub fn run(&mut self, c: &Coefs, x: f64) -> f64 {
        let y = c.b0 * x + self.z1;
        self.z1 = c.b1 * x - c.a1 * y + self.z2;
        self.z2 = c.b2 * x - c.a2 * y;
        y
    }

    pub fn reset(&mut self) {
        *self = State::default();
    }
}

/// Denominator from the analog poles of s² + 2ζω s + ω² (ω in radians per
/// sample), by impulse invariance.
fn poles(w: f64, zeta: f64) -> (f64, f64) {
    let w = w.min(0.97 * PI);
    let e = (-zeta * w).exp();
    let a1 = if zeta <= 1.0 { -2.0 * e * ((1.0 - zeta * zeta).sqrt() * w).cos() } else { -2.0 * e * ((zeta * zeta - 1.0).sqrt() * w).cosh() };
    (a1, e * e)
}

/// (φ0, φ1, φ2) at `w`: |H|² of a biquad is linear in these.
fn phis(w: f64) -> (f64, f64, f64) {
    let s = (w * 0.5).sin().powi(2);
    (1.0 - s, s, 4.0 * (1.0 - s) * s)
}

/// The numerator that makes the digital magnitude² equal `h2` (the analog
/// magnitude², as a function of radians per sample) at DC, at `wm` and at
/// Nyquist, over the denominator (a1, a2).
fn matched(a1: f64, a2: f64, h2: impl Fn(f64) -> f64, wm: f64) -> Coefs {
    let (ad0, ad1, ad2) = ((1.0 + a1 + a2).powi(2), (1.0 - a1 + a2).powi(2), -4.0 * a2);
    let bd0 = h2(0.0) * ad0;
    let bd1 = h2(PI) * ad1;
    let (p0, p1, p2) = phis(wm);
    let d2 = ad0 * p0 + ad1 * p1 + ad2 * p2;
    let bd2 = (h2(wm) * d2 - bd0 * p0 - bd1 * p1) / p2;
    let (w, v) = (bd0.max(0.0).sqrt(), bd1.max(0.0).sqrt());
    let b1 = 0.5 * (w - v);
    let s = 0.5 * (w + v);
    let p = -0.25 * bd2;
    let disc = (s * s - 4.0 * p).max(0.0).sqrt();
    Coefs { b0: 0.5 * (s + disc), b1, b2: 0.5 * (s - disc), a1, a2 }
}

/// Radians per sample, and the frequency to match at besides DC and
/// Nyquist (the filter frequency, kept clear of Nyquist).
fn omega(f: f64, sr: f64) -> (f64, f64) {
    let w0 = 2.0 * PI * f.max(1.0) / sr;
    (w0, w0.min(0.75 * PI))
}

/// Peaking filter, `db` at `f`.
pub fn bell(f: f64, sr: f64, q: f64, db: f64) -> Coefs {
    if db.abs() < 1e-6 {
        return Coefs::IDENTITY;
    }
    let (w0, wm) = omega(f, sr);
    let a = 10f64.powf(db / 40.0);
    let q = q.max(0.01);
    let (a1, a2) = poles(w0, 1.0 / (2.0 * a * q));
    let h2 = |w: f64| {
        let x = w / w0;
        let u = (1.0 - x * x).powi(2);
        (u + (a * x / q).powi(2)) / (u + (x / (a * q)).powi(2))
    };
    matched(a1, a2, h2, wm)
}

/// Low shelf: `db` below `f`.
pub fn low_shelf(f: f64, sr: f64, q: f64, db: f64) -> Coefs {
    if db.abs() < 1e-6 {
        return Coefs::IDENTITY;
    }
    let (w0, wm) = omega(f, sr);
    let a = 10f64.powf(db / 40.0);
    let q = q.max(0.01);
    let (a1, a2) = poles(w0 / a.sqrt(), 1.0 / (2.0 * q));
    let h2 = |w: f64| {
        let x = w / w0;
        let k = a.sqrt() * x / q;
        a * a * ((a - x * x).powi(2) + k * k) / ((1.0 - a * x * x).powi(2) + k * k)
    };
    matched(a1, a2, h2, wm)
}

/// High shelf: `db` above `f`.
pub fn high_shelf(f: f64, sr: f64, q: f64, db: f64) -> Coefs {
    if db.abs() < 1e-6 {
        return Coefs::IDENTITY;
    }
    let (w0, wm) = omega(f, sr);
    let a = 10f64.powf(db / 40.0);
    let q = q.max(0.01);
    let (a1, a2) = poles(w0 * a.sqrt(), 1.0 / (2.0 * q));
    let h2 = |w: f64| {
        let x = w / w0;
        let k = a.sqrt() * x / q;
        a * a * ((1.0 - a * x * x).powi(2) + k * k) / ((a - x * x).powi(2) + k * k)
    };
    matched(a1, a2, h2, wm)
}

/// Tilt around `f`: -db/2 at the bottom, +db/2 at the top.
pub fn tilt(f: f64, sr: f64, q: f64, db: f64) -> Coefs {
    high_shelf(f, sr, q, db).scaled(10f64.powf(-db / 40.0))
}

/// Resonant low-pass; `q` = 0.7071 is Butterworth.
pub fn lowpass(f: f64, sr: f64, q: f64) -> Coefs {
    let (w0, wm) = omega(f, sr);
    let q = q.max(0.01);
    let (a1, a2) = poles(w0, 1.0 / (2.0 * q));
    let h2 = |w: f64| {
        let x = w / w0;
        1.0 / ((1.0 - x * x).powi(2) + (x / q).powi(2))
    };
    matched(a1, a2, h2, wm)
}

/// Resonant high-pass.
pub fn highpass(f: f64, sr: f64, q: f64) -> Coefs {
    let (w0, wm) = omega(f, sr);
    let q = q.max(0.01);
    let (a1, a2) = poles(w0, 1.0 / (2.0 * q));
    let h2 = |w: f64| {
        let x = w / w0;
        x.powi(4) / ((1.0 - x * x).powi(2) + (x / q).powi(2))
    };
    matched(a1, a2, h2, wm)
}

/// Band-pass with unity gain at `f`.
pub fn bandpass(f: f64, sr: f64, q: f64) -> Coefs {
    let (w0, wm) = omega(f, sr);
    let q = q.max(0.01);
    let (a1, a2) = poles(w0, 1.0 / (2.0 * q));
    let h2 = |w: f64| {
        let x = w / w0;
        (x / q).powi(2) / ((1.0 - x * x).powi(2) + (x / q).powi(2))
    };
    matched(a1, a2, h2, wm)
}

/// Notch at `f`.
pub fn notch(f: f64, sr: f64, q: f64) -> Coefs {
    let (w0, wm) = omega(f, sr);
    let q = q.max(0.01);
    let (a1, a2) = poles(w0, 1.0 / (2.0 * q));
    let _ = wm;
    // Zeros exactly on the unit circle at the notch, unity gain at DC.
    let c = -2.0 * w0.min(PI).cos();
    let k = (1.0 + a1 + a2) / (2.0 + c);
    Coefs { b0: k, b1: k * c, b2: k, a1, a2 }
}

/// First-order low-pass (6 dB/oct), as a biquad with b2 = a2 = 0.
pub fn lowpass1(f: f64, sr: f64) -> Coefs {
    let w0 = 2.0 * PI * f.max(1.0) / sr;
    let p = (-w0.min(0.97 * PI)).exp();
    let x = PI / w0;
    let nyq = (1.0 / (1.0 + x * x)).sqrt();
    let (sum, diff) = (1.0 - p, (1.0 + p) * nyq);
    Coefs { b0: 0.5 * (sum + diff), b1: 0.5 * (sum - diff), b2: 0.0, a1: -p, a2: 0.0 }
}

/// First-order high-pass (6 dB/oct).
pub fn highpass1(f: f64, sr: f64) -> Coefs {
    let w0 = 2.0 * PI * f.max(1.0) / sr;
    let p = (-w0.min(0.97 * PI)).exp();
    let x = PI / w0;
    let nyq = (x * x / (1.0 + x * x)).sqrt();
    let b = 0.5 * (1.0 + p) * nyq;
    Coefs { b0: b, b1: -b, b2: 0.0, a1: -p, a2: 0.0 }
}

/// Up to eight sections making one filter.
#[derive(Clone, Copy, Debug)]
pub struct Cascade {
    pub sections: [Coefs; 8],
    pub n: usize,
}

impl Cascade {
    pub const NONE: Cascade = Cascade { sections: [Coefs::IDENTITY; 8], n: 0 };

    pub fn one(c: Coefs) -> Self {
        let mut s = Self::NONE;
        s.sections[0] = c;
        s.n = 1;
        s
    }

    pub fn mag_db(&self, w: f64) -> f64 {
        self.sections[..self.n].iter().map(|c| c.mag_db(w)).sum()
    }

    fn push(&mut self, c: Coefs) {
        if self.n < 8 {
            self.sections[self.n] = c;
            self.n += 1;
        }
    }
}

/// A Butterworth low- or high-pass of `order` (1 to 16, 6 dB/oct each).
/// `resonance` above 0.7071 raises the Q of the sharpest section.
pub fn butterworth(order: usize, highpass_: bool, f: f64, sr: f64, resonance: f64) -> Cascade {
    let order = order.clamp(1, 16);
    let mut c = Cascade::NONE;
    let mut qs = [0.0f64; 8];
    let mut nq = 0;
    if order % 2 == 1 {
        c.push(if highpass_ { highpass1(f, sr) } else { lowpass1(f, sr) });
        for k in 1..=(order - 1) / 2 {
            qs[nq] = 1.0 / (2.0 * (k as f64 * PI / order as f64).cos());
            nq += 1;
        }
    } else {
        for k in 1..=order / 2 {
            qs[nq] = 1.0 / (2.0 * ((2 * k - 1) as f64 * PI / (2 * order) as f64).cos());
            nq += 1;
        }
    }
    let boost = resonance / std::f64::consts::FRAC_1_SQRT_2;
    let sharpest = (0..nq).max_by(|&a, &b| qs[a].partial_cmp(&qs[b]).unwrap()).unwrap_or(0);
    for (k, &q) in qs[..nq].iter().enumerate() {
        let q = if k == sharpest { q * boost } else { q };
        c.push(if highpass_ { highpass(f, sr, q) } else { lowpass(f, sr, q) });
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analog_bell_db(f: f64, f0: f64, q: f64, db: f64) -> f64 {
        let a = 10f64.powf(db / 40.0);
        let x = f / f0;
        let u = (1.0 - x * x).powi(2);
        10.0 * ((u + (a * x / q).powi(2)) / (u + (x / (a * q)).powi(2))).log10()
    }

    fn w(f: f64, sr: f64) -> f64 {
        2.0 * PI * f / sr
    }

    #[test]
    fn bells_follow_the_analog_curve_up_to_nyquist() {
        let sr = 44100.0;
        for &(f0, q, db) in &[(12000.0, 0.7, 12.0), (8000.0, 2.0, -9.0), (1000.0, 1.0, 6.0), (40.0, 4.0, 10.0), (20.0, 1.0, -12.0), (16000.0, 0.5, 6.0)] {
            let c = bell(f0, sr, q, db);
            for k in 0..200 {
                let f = 10.0 * (2000.0f64).powf(k as f64 / 199.0);
                if f > 0.49 * sr {
                    break;
                }
                let err = c.mag_db(w(f, sr)) - analog_bell_db(f, f0, q, db);
                if std::env::var("BIQUAD_REPORT").is_ok() && err.abs() > 0.2 {
                    eprintln!("bell {f0} Hz Q {q} {db} dB: {err:.2} dB off at {f:.0} Hz");
                }
                assert!(err.abs() < 1.0, "bell {f0} Hz Q {q} {db} dB: {err:.2} dB off at {f:.0} Hz");
            }
        }
    }

    #[test]
    fn shelves_and_tilt_reach_their_gains() {
        let sr = 48000.0;
        let ls = low_shelf(200.0, sr, 0.707, 9.0);
        assert!((ls.mag_db(w(20.0, sr)) - 9.0).abs() < 0.2);
        assert!(ls.mag_db(w(10000.0, sr)).abs() < 0.2);
        let hs = high_shelf(3000.0, sr, 0.707, -6.0);
        assert!((hs.mag_db(w(20000.0, sr)) + 6.0).abs() < 0.3);
        assert!(hs.mag_db(w(50.0, sr)).abs() < 0.1);
        let t = tilt(1000.0, sr, 0.5, 6.0);
        assert!((t.mag_db(w(20.0, sr)) + 3.0).abs() < 0.3);
        assert!((t.mag_db(w(20000.0, sr)) - 3.0).abs() < 0.4);
    }

    #[test]
    fn cuts_have_their_slope() {
        let sr = 48000.0;
        for (order, slope) in [(1usize, 6.0), (2, 12.0), (3, 18.0), (4, 24.0), (8, 48.0), (16, 96.0)] {
            let hp = butterworth(order, true, 1000.0, sr, std::f64::consts::FRAC_1_SQRT_2);
            // -3 dB at the corner, the slope two octaves down.
            assert!((hp.mag_db(w(1000.0, sr)) + 3.01).abs() < 0.3, "order {order} corner {:.2}", hp.mag_db(w(1000.0, sr)));
            let drop = hp.mag_db(w(250.0, sr)) - hp.mag_db(w(125.0, sr));
            assert!((drop - slope).abs() < slope * 0.08, "order {order}: {drop:.1} dB per octave");
            let lp = butterworth(order, false, 1000.0, sr, std::f64::consts::FRAC_1_SQRT_2);
            assert!(lp.mag_db(w(50.0, sr)).abs() < 0.05);
            assert!((lp.mag_db(w(1000.0, sr)) + 3.01).abs() < 0.3);
        }
    }

    #[test]
    fn notch_band_pass_and_processing_are_sane() {
        let sr = 48000.0;
        let n = notch(1000.0, sr, 2.0);
        assert!(n.mag_db(w(1000.0, sr)) < -40.0);
        assert!(n.mag_db(w(100.0, sr)).abs() < 0.2);
        let bp = bandpass(1000.0, sr, 2.0);
        assert!(bp.mag_db(w(1000.0, sr)).abs() < 0.1);
        assert!(bp.mag_db(w(100.0, sr)) < -20.0);
        // Running the filter gives the designed gain.
        let c = bell(1000.0, sr, 1.0, 12.0);
        let mut st = State::default();
        let (mut peak, n) = (0.0f64, 48000);
        for i in 0..n {
            let y = st.run(&c, (2.0 * PI * 1000.0 * i as f64 / sr).sin());
            if i > n / 2 {
                peak = peak.max(y.abs());
            }
        }
        assert!((20.0 * peak.log10() - 12.0).abs() < 0.1);
    }
}
