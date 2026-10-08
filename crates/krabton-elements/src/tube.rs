//! Simple waveguide tube for the blow exciter (`elements/dsp/tube.h`,
//! `tube.cc`).
//!
//! Copyright 2014 Emilie Gillet, MIT license. Rust port for Krabton.

use crate::dsp::integral_fractional;

pub const TUBE_DELAY_SIZE: usize = 2048;

#[derive(Clone, Debug)]
pub struct Tube {
    delay_ptr: usize,
    zero_state: f32,
    pole_state: f32,
    delay_line: Box<[f32]>,
}

impl Default for Tube {
    fn default() -> Self {
        Self { delay_ptr: 0, zero_state: 0.0, pole_state: 0.0, delay_line: vec![0.0; TUBE_DELAY_SIZE].into_boxed_slice() }
    }
}

impl Tube {
    pub fn reset(&mut self) {
        self.zero_state = 0.0;
        self.pole_state = 0.0;
        self.delay_ptr = 0;
    }

    pub fn process(&mut self, frequency: f32, envelope: f32, damping: f32, timbre: f32, input_output: &mut [f32], gain: f32) {
        let mut delay = 1.0 / frequency;
        while delay >= TUBE_DELAY_SIZE as f32 {
            delay *= 0.5;
        }
        let (delay_integral, delay_fractional) = integral_fractional(delay);
        let delay_integral = delay_integral.max(0) as usize;
        let envelope = envelope.min(1.0);
        let damping = 3.6 - damping * 1.8;
        let lpf_coefficient = (frequency * (1.0 + timbre * timbre * 256.0)).min(0.995);
        let mut d = self.delay_ptr;
        for io in input_output.iter_mut() {
            let breath = *io * damping + 0.8;
            let a = self.delay_line[(d + delay_integral) % TUBE_DELAY_SIZE];
            let b = self.delay_line[(d + delay_integral + 1) % TUBE_DELAY_SIZE];
            let input = a + (b - a) * delay_fractional;
            let pressure_delta = -0.95 * (input * envelope + self.zero_state) - breath;
            self.zero_state = input;
            let reed = pressure_delta * -0.2 + 0.8;
            let out = (pressure_delta * reed + breath).clamp(-5.0, 5.0);
            self.delay_line[d] = out * 0.5;
            d = if d == 0 { TUBE_DELAY_SIZE - 1 } else { d - 1 };
            self.pole_state += lpf_coefficient * (out - self.pole_state);
            *io += gain * envelope * self.pole_state;
        }
        self.delay_ptr = d;
    }
}
