//! `Whistle`: a police / referee's whistle (`include/Whistle.h`,
//! `src/Whistle.cpp`), by Perry R. Cook.
//!
//! A pea bounces around inside a 2-D can, pushed by the breath and pulled
//! by gravity; its distance to the fipple (a "bumper" sphere) modulates the
//! frequency and gain of a sine plus noise. The whistle is a transposing
//! instrument: it sounds two octaves above the note.
//!
//! Copyright (c) 1995--2023 Perry R. Cook and Gary P. Scavone (STK license,
//! see the crate documentation). Rust port for Krabton.
//!
//! Krabton changes: the voice is silent until the first note (STK starts
//! blowing in the constructor), the physics time step, the envelope rate
//! and the fipple smoothing pole are given for 44.1 kHz and converted to
//! the voice's sample rate, the debugging printout is gone, and the
//! sub-sampling switch (STK's undocumented control 64) is not handled.

use crate::ugens::*;
use crate::{release_amplitude, Model};

const CAN_RADIUS: f64 = 100.0;
const PEA_RADIUS: f64 = 30.0;
const BUMP_RADIUS: f64 = 5.0;
const NORM_CAN_LOSS: f64 = 0.97;
const GRAVITY: f64 = 20.0;
const NORM_TICK_SIZE: f64 = 0.004;
const ENV_RATE: f64 = 0.001;

#[derive(Clone, Copy, Debug, Default)]
struct Vector3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Vector3 {
    fn length(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
}

/// `Sphere`: position, velocity and radius of a ball.
#[derive(Clone, Debug, Default)]
struct Sphere {
    position: Vector3,
    velocity: Vector3,
    radius: f64,
}

impl Sphere {
    /// Distance from `p` to the surface (negative inside).
    fn is_inside(&self, p: &Vector3) -> f64 {
        let d = Vector3 { x: p.x - self.position.x, y: p.y - self.position.y, z: p.z - self.position.z };
        d.length() - self.radius
    }

    fn add_velocity(&mut self, x: f64, y: f64, z: f64) {
        self.velocity.x += x;
        self.velocity.y += y;
        self.velocity.z += z;
    }

    fn tick(&mut self, dt: f64) {
        self.position.x += dt * self.velocity.x;
        self.position.y += dt * self.velocity.y;
        self.position.z += dt * self.velocity.z;
    }
}

pub struct Whistle {
    sr: f64,
    onepole: OnePole,
    noise: Noise,
    envelope: Envelope,
    can: Sphere,
    pea: Sphere,
    bumper: Sphere,
    sine: SineWave,
    base_frequency: f64,
    noise_gain: f64,
    fipple_freq_mod: f64,
    fipple_gain_mod: f64,
    blow_freq_mod: f64,
    tick_size: f64,
    can_loss: f64,
    active: bool,
}

impl Whistle {
    pub fn new(sr: f64) -> Self {
        let mut sine = SineWave::new(sr);
        sine.set_frequency(2800.0);
        let can = Sphere { radius: CAN_RADIUS, ..Sphere::default() };
        let bumper = Sphere {
            radius: BUMP_RADIUS,
            position: Vector3 { x: 0.0, y: CAN_RADIUS - BUMP_RADIUS, z: 0.0 },
            ..Sphere::default()
        };
        let pea = Sphere {
            radius: PEA_RADIUS,
            position: Vector3 { x: 0.0, y: CAN_RADIUS / 2.0, z: 0.0 },
            velocity: Vector3 { x: 35.0, y: 15.0, z: 0.0 },
        };
        let mut envelope = Envelope::default();
        envelope.set_rate(rate(ENV_RATE, sr));
        Self {
            sr,
            onepole: OnePole::new(radius(0.95, sr)),
            noise: Noise::new(0x3415),
            envelope,
            can,
            pea,
            bumper,
            sine,
            base_frequency: 2000.0,
            noise_gain: 0.125,
            fipple_freq_mod: 0.5,
            fipple_gain_mod: 0.5,
            blow_freq_mod: 0.25,
            tick_size: rate(NORM_TICK_SIZE, sr),
            can_loss: NORM_CAN_LOSS,
            active: false,
        }
    }
}

impl Model for Whistle {
    fn note_on(&mut self, frequency: f64, amplitude: f64) {
        // The whistle is a transposing instrument.
        self.base_frequency = frequency * 4.0;
        if amplitude > 0.0 {
            self.envelope.set_rate(rate(ENV_RATE, self.sr));
            self.envelope.set_target(amplitude * 2.0);
        }
        self.active = true;
    }

    fn note_off(&mut self, amplitude: f64) {
        self.envelope.set_rate(rate(release_amplitude(amplitude) * 0.02, self.sr));
        self.envelope.key_off();
        self.active = false;
    }

    fn control_change(&mut self, number: u8, value: f64) {
        let n = value * ONE_OVER_128;
        match number {
            4 => self.noise_gain = 0.25 * n,
            11 => self.fipple_freq_mod = n,
            1 => self.fipple_gain_mod = n,
            128 if self.active => self.envelope.set_target(n * 2.0),
            2 => self.blow_freq_mod = n * 0.5,
            _ => {}
        }
    }

    fn tick(&mut self) -> f64 {
        let ts = self.tick_size;
        let mut temp = self.bumper.is_inside(&self.pea.position);
        let env_out = self.envelope.tick();

        if temp < BUMP_RADIUS + PEA_RADIUS {
            let tx = env_out * ts * 2000.0 * self.noise.tick();
            let ty = -env_out * ts * 1000.0 * (1.0 + self.noise.tick());
            self.pea.add_velocity(tx, ty, 0.0);
            self.pea.tick(ts);
        }

        let modulation = (-temp * 0.01).exp(); // exponential distance falloff of fipple/pea effect
        temp = self.onepole.tick(modulation); // smooth it a little
        let mut gain = (1.0 - self.fipple_gain_mod * 0.5) + 2.0 * self.fipple_gain_mod * temp;
        gain *= gain; // squared distance/gain
        let mut temp_freq = 1.0 + self.fipple_freq_mod * (0.25 - temp) + self.blow_freq_mod * (env_out - 1.0);
        temp_freq *= self.base_frequency;
        self.sine.set_frequency(temp_freq);

        let p = self.pea.position;
        temp = -self.can.is_inside(&p); // the pea is inside the can, so the distance is negated
        if temp < PEA_RADIUS * 1.25 {
            // Bounce off the can: reflect the radial velocity component.
            let v = self.pea.velocity;
            let phi = -p.y.atan2(p.x);
            let (sinphi, cosphi) = phi.sin_cos();
            let mut temp1 = cosphi * v.x - sinphi * v.y;
            let temp2 = sinphi * v.x + cosphi * v.y;
            temp1 = -temp1;
            let tx = cosphi * temp1 + sinphi * temp2;
            let ty = -sinphi * temp1 + cosphi * temp2;
            self.pea.velocity = Vector3 { x: tx, y: ty, z: 0.0 };
            self.pea.tick(ts);
            self.pea.velocity = Vector3 { x: tx * self.can_loss, y: ty * self.can_loss, z: 0.0 };
            self.pea.tick(ts);
        }

        // The breath swirls the pea around the can (STK reads the position
        // through a pointer, so this sees the bounce above).
        let p = self.pea.position;
        temp = p.length();
        let (tx, ty) = if temp > 0.01 {
            let phi = p.y.atan2(p.x) + 0.3 * temp / CAN_RADIUS;
            let (sinphi, cosphi) = phi.sin_cos();
            (3.0 * temp * cosphi, 3.0 * temp * sinphi)
        } else {
            (0.0, 0.0)
        };
        temp = (0.9 + 0.1 * self.noise.tick()) * env_out * 0.6 * ts;
        self.pea.add_velocity(temp * tx, temp * ty - GRAVITY * ts, 0.0);
        self.pea.tick(ts);

        let temp = env_out * env_out * gain / 2.0;
        let sound_mix = temp * (self.sine.tick() + self.noise_gain * self.noise.tick());
        0.20 * sound_mix
    }

    fn clear(&mut self) {
        self.onepole.clear();
        self.pea.position = Vector3 { x: 0.0, y: CAN_RADIUS / 2.0, z: 0.0 };
        self.pea.velocity = Vector3 { x: 35.0, y: 15.0, z: 0.0 };
    }
}
