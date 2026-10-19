//! Krabton: changes to every voice of a channel on top of what the bank's
//! preset sets. At their defaults they change nothing.

/// How a channel shapes its voices.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shape {
    /// The volume envelope's times, relative: -1 a sixteenth, 0 as the
    /// preset has them, +1 sixteen times as long and up to 4 s more (so a
    /// preset with none gets some).
    pub attack: f32,
    pub decay: f32,
    pub release: f32,
    /// Its sustain, relative: -1 none, 0 as the preset, +1 all of it.
    pub sustain: f32,
    /// The filter's cutoff moved (cents) and its resonance raised (dB).
    pub cutoff: f32,
    pub resonance: f32,
    /// A filter envelope of its own: how far it moves the cutoff at its
    /// peak (cents), its attack and decay (seconds) and its sustain (0..1).
    pub env_amount: f32,
    pub env_attack: f32,
    pub env_decay: f32,
    pub env_sustain: f32,
    /// How much darker the softest note is than the hardest (cents).
    pub vel_cutoff: f32,
    /// The vibrato LFO: its rate (times the preset's) and its delay added
    /// (seconds); its depth is the mod wheel's.
    pub vib_rate: f32,
    pub vib_delay: f32,
    /// The modulation LFO: tremolo (dB), filter wobble (cents), its rate
    /// (times the preset's).
    pub tremolo: f32,
    pub wobble: f32,
    pub mod_rate: f32,
    /// Fine tuning (cents).
    pub fine: f32,
    /// The width of the preset's panning: 0 all in the middle, 1 as it is, 2 twice as wide.
    pub width: f32,
    /// Where the sample starts: 0 at its start, 1 at its loop (or near its end).
    pub start: f32,
}

impl Default for Shape {
    fn default() -> Self {
        Self {
            attack: 0.0,
            decay: 0.0,
            release: 0.0,
            sustain: 0.0,
            cutoff: 0.0,
            resonance: 0.0,
            env_amount: 0.0,
            env_attack: 0.005,
            env_decay: 0.6,
            env_sustain: 0.0,
            vel_cutoff: 0.0,
            vib_rate: 1.0,
            vib_delay: 0.0,
            tremolo: 0.0,
            wobble: 0.0,
            mod_rate: 1.0,
            fine: 0.0,
            width: 1.0,
            start: 0.0,
        }
    }
}

impl Shape {
    /// An envelope time (seconds) changed by `v` (-1..1).
    pub(crate) fn time(t: f32, v: f32) -> f32 {
        let v = v.clamp(-1.0, 1.0);
        (t.max(0.0) * 2f32.powf(4.0 * v) + v.max(0.0).powi(2) * 4.0).max(0.0)
    }

    /// A sustain attenuation (dB) changed by `v` (-1..1).
    pub(crate) fn sustain_db(att: f32, v: f32) -> f32 {
        let v = v.clamp(-1.0, 1.0);
        let att = att.max(0.0);
        if v >= 0.0 {
            att * (1.0 - v)
        } else {
            att + (100.0 - att).max(0.0) * -v
        }
    }

    /// Krabton's filter envelope (0..1) `t` seconds into a note; let go at
    /// `released` (seconds after, and from what level) it falls in 0.3 s.
    pub(crate) fn env(&self, t: f32, released: Option<(f32, f32)>) -> f32 {
        if let Some((since, from)) = released {
            return from * (-since * 4.6 / 0.3).exp();
        }
        let a = self.env_attack.max(1e-4);
        if t < a {
            t / a
        } else {
            let s = self.env_sustain.clamp(0.0, 1.0);
            s + (1.0 - s) * (-(t - a) * 4.6 / self.env_decay.max(1e-3)).exp()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn at_rest_nothing_changes() {
        assert_eq!(Shape::time(0.25, 0.0), 0.25);
        assert_eq!(Shape::sustain_db(12.0, 0.0), 12.0);
        // The ends: a sixteenth, sixteen times and four seconds more.
        assert!((Shape::time(1.6, -1.0) - 0.1).abs() < 1e-6);
        assert!((Shape::time(0.0, 1.0) - 4.0).abs() < 1e-6);
        assert_eq!(Shape::sustain_db(30.0, 1.0), 0.0);
        assert_eq!(Shape::sustain_db(30.0, -1.0), 100.0);
    }

    #[test]
    fn the_filter_envelope_rises_falls_and_lets_go() {
        let s = Shape { env_attack: 0.1, env_decay: 1.0, env_sustain: 0.25, ..Shape::default() };
        assert!((s.env(0.05, None) - 0.5).abs() < 1e-6);
        assert!((s.env(0.1, None) - 1.0).abs() < 1e-6);
        assert!((s.env(5.0, None) - 0.25).abs() < 1e-3);
        assert!(s.env(0.0, Some((1.0, 0.5))) < 0.001);
    }
}
