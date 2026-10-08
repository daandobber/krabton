//! A decaying excitation pulse (`braids/excitation.h`, Copyright 2012
//! Emilie Gillet, MIT).

#[derive(Clone, Copy, Default)]
pub struct Excitation {
    delay: u32,
    decay: u32,
    counter: i32,
    state: i32,
    level: i32,
}

impl Excitation {
    pub fn init(&mut self) {
        self.delay = 0;
        self.decay = 4093;
        self.counter = 0;
        self.state = 0;
    }

    pub fn set_delay(&mut self, delay: u16) {
        self.delay = delay as u32;
    }

    pub fn set_decay(&mut self, decay: u16) {
        self.decay = decay as u32;
    }

    pub fn trigger(&mut self, level: i32) {
        self.level = level;
        self.counter = self.delay as i32 + 1;
    }

    pub fn done(&self) -> bool {
        self.counter == 0
    }

    #[inline]
    pub fn process(&mut self) -> i32 {
        self.state = self.state.wrapping_mul(self.decay as i32) >> 12;
        if self.counter > 0 {
            self.counter -= 1;
            if self.counter == 0 {
                self.state += if self.level < 0 { -self.level } else { self.level };
            }
        }
        if self.level < 0 {
            -self.state
        } else {
            self.state
        }
    }
}
