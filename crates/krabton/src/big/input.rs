//! Big Picture's input: gamepads (through gilrs: Xbox, PlayStation, the
//! Steam Deck's own controls, generic pads), the keyboard (arrows, Enter,
//! Escape: for devices whose buttons send keys) and the screen, turned into
//! the same few actions. Directions repeat while held, the triggers and the
//! right stick turn the focused value.

use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Nav {
    Up,
    Down,
    Left,
    Right,
    /// Choose, play, launch (South: A on an Xbox pad, cross on a PlayStation one).
    A,
    /// Back (East).
    B,
    X,
    Y,
    /// The shoulder buttons: the pages.
    PrevPage,
    NextPage,
    /// Play and stop.
    Start,
    /// The menu.
    Select,
    /// The right stick pressed: record.
    Record,
    /// The left stick pressed.
    LeftStick,
}

impl Nav {
    pub fn is_direction(self) -> bool {
        matches!(self, Nav::Up | Nav::Down | Nav::Left | Nav::Right)
    }
}

/// What came in this frame.
#[derive(Clone, Debug, Default)]
pub struct Frame {
    /// Presses, and repeats of held directions.
    pub pressed: Vec<Nav>,
    pub released: Vec<Nav>,
    /// How hard to turn the focused value (-1..1): the triggers and the right stick.
    pub turn: f32,
    /// The triggers alone (right minus left, -1..1).
    pub trig: f32,
    /// The right stick (x right, y up), dead zone taken out.
    pub stick: (f32, f32),
    /// The right stick's up and down alone, for turning (-1..1).
    pub stick_turn: f32,
    /// The left stick (x right, y up), dead zone taken out.
    pub lstick: (f32, f32),
    /// Each trigger alone (0..1).
    pub lt: f32,
    pub rt: f32,
    /// Seconds since the last frame.
    pub dt: f32,
}

impl Frame {
    pub fn has(&self, n: Nav) -> bool {
        self.pressed.contains(&n)
    }

    /// Takes a press out (so later handlers do not see it again).
    pub fn take(&mut self, n: Nav) -> bool {
        match self.pressed.iter().position(|p| *p == n) {
            Some(i) => {
                self.pressed.remove(i);
                true
            }
            None => false,
        }
    }
}

const REPEAT_DELAY: f32 = 0.35;
const REPEAT_EVERY: f32 = 0.075;

/// A held direction and when it repeats next.
#[derive(Clone, Copy, Debug)]
struct Held {
    nav: Nav,
    next: f32,
}

pub struct Pads {
    gilrs: Option<gilrs::Gilrs>,
    /// Directions held on a pad (D-pad or the left stick), for repeats.
    held: Vec<Held>,
    stick: Option<Nav>,
    lt: f32,
    rt: f32,
    rx: f32,
    ry: f32,
    last: Instant,
    clock: f32,
    /// The connected pad's name, for the screen.
    pub name: Option<String>,
    /// The left stick moves like the D-pad (off while it plays notes).
    pub lstick_nav: bool,
}

impl Default for Pads {
    fn default() -> Self {
        Self::new()
    }
}

impl Pads {
    pub fn new() -> Self {
        // Test windows never take the real controllers.
        let gilrs = if std::env::var_os("KRABTON_TEST_WINDOW").is_some() { None } else { gilrs::Gilrs::new().ok() };
        let mut p = Self { gilrs, held: Vec::new(), stick: None, lt: 0.0, rt: 0.0, rx: 0.0, ry: 0.0, last: Instant::now(), clock: 0.0, name: None, lstick_nav: true };
        p.refresh_name();
        p
    }

    fn refresh_name(&mut self) {
        self.name = self.gilrs.as_ref().and_then(|g| g.gamepads().find(|(_, pad)| pad.is_connected()).map(|(_, pad)| pad.name().to_string()));
    }

    fn button(b: gilrs::Button) -> Option<Nav> {
        use gilrs::Button as B;
        Some(match b {
            B::South => Nav::A,
            B::East => Nav::B,
            B::West => Nav::X,
            B::North => Nav::Y,
            B::LeftTrigger => Nav::PrevPage,
            B::RightTrigger => Nav::NextPage,
            B::Start => Nav::Start,
            B::Select | B::Mode => Nav::Select,
            B::RightThumb => Nav::Record,
            B::LeftThumb => Nav::LeftStick,
            B::DPadUp => Nav::Up,
            B::DPadDown => Nav::Down,
            B::DPadLeft => Nav::Left,
            B::DPadRight => Nav::Right,
            _ => return None,
        })
    }

    fn press(&mut self, nav: Nav, out: &mut Frame) {
        out.pressed.push(nav);
        if nav.is_direction() {
            self.held.retain(|h| h.nav != nav);
            self.held.push(Held { nav, next: self.clock + REPEAT_DELAY });
        }
    }

    fn release(&mut self, nav: Nav, out: &mut Frame) {
        out.released.push(nav);
        self.held.retain(|h| h.nav != nav);
    }

    /// This frame's input. The keyboard counts only in Big Picture (`keys`).
    pub fn poll(&mut self, ctx: &egui::Context, keys: bool) -> Frame {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32().min(0.1);
        self.last = now;
        self.clock += dt;
        let mut out = Frame { dt, ..Default::default() };
        let mut events = Vec::new();
        let mut changed_pads = false;
        if let Some(g) = self.gilrs.as_mut() {
            while let Some(gilrs::Event { event, .. }) = g.next_event() {
                events.push(event);
            }
        }
        for event in events {
            use gilrs::EventType as E;
            match event {
                E::ButtonPressed(b, _) => {
                    if let Some(n) = Self::button(b) {
                        self.press(n, &mut out);
                    }
                }
                E::ButtonReleased(b, _) => {
                    if let Some(n) = Self::button(b) {
                        self.release(n, &mut out);
                    }
                }
                E::ButtonChanged(gilrs::Button::LeftTrigger2, v, _) => self.lt = v,
                E::ButtonChanged(gilrs::Button::RightTrigger2, v, _) => self.rt = v,
                E::AxisChanged(gilrs::Axis::RightStickY, v, _) => self.ry = v,
                E::AxisChanged(gilrs::Axis::RightStickX, v, _) => self.rx = v,
                E::AxisChanged(gilrs::Axis::LeftZ, v, _) => self.lt = (v + 1.0) * 0.5,
                E::AxisChanged(gilrs::Axis::RightZ, v, _) => self.rt = (v + 1.0) * 0.5,
                E::AxisChanged(axis @ (gilrs::Axis::LeftStickX | gilrs::Axis::LeftStickY), _, _) => {
                    let _ = axis;
                    if !self.lstick_nav {
                        if let Some(old) = self.stick.take() {
                            self.release(old, &mut out);
                        }
                        continue;
                    }
                    // The left stick as a direction (with some hysteresis).
                    let (x, y) = self.left_stick();
                    let dir = if x.abs() > y.abs() {
                        if x > 0.55 {
                            Some(Nav::Right)
                        } else if x < -0.55 {
                            Some(Nav::Left)
                        } else {
                            None
                        }
                    } else if y > 0.55 {
                        Some(Nav::Up)
                    } else if y < -0.55 {
                        Some(Nav::Down)
                    } else {
                        None
                    };
                    let keep = self.stick.is_some() && x.abs().max(y.abs()) > 0.35;
                    if dir != self.stick && !(keep && dir.is_none()) {
                        if let Some(old) = self.stick.take() {
                            self.release(old, &mut out);
                        }
                        if let Some(d) = dir {
                            self.stick = Some(d);
                            self.press(d, &mut out);
                        }
                    }
                }
                E::Connected | E::Disconnected => changed_pads = true,
                _ => {}
            }
        }
        if changed_pads {
            self.refresh_name();
        }
        // Held directions repeat.
        for h in self.held.iter_mut() {
            if self.clock >= h.next {
                out.pressed.push(h.nav);
                h.next = self.clock + REPEAT_EVERY;
            }
        }
        if keys {
            ctx.input(|i| {
                for e in &i.events {
                    if let egui::Event::Key { key, pressed, repeat, modifiers, .. } = e {
                        let nav = match key {
                            egui::Key::ArrowUp => Some(Nav::Up),
                            egui::Key::ArrowDown => Some(Nav::Down),
                            egui::Key::ArrowLeft => Some(Nav::Left),
                            egui::Key::ArrowRight => Some(Nav::Right),
                            egui::Key::Enter => Some(Nav::A),
                            egui::Key::Escape | egui::Key::Backspace => Some(Nav::B),
                            egui::Key::X => Some(Nav::X),
                            egui::Key::Y => Some(Nav::Y),
                            egui::Key::Q | egui::Key::PageUp => Some(Nav::PrevPage),
                            egui::Key::E | egui::Key::PageDown => Some(Nav::NextPage),
                            egui::Key::Tab if modifiers.shift => Some(Nav::PrevPage),
                            egui::Key::Tab => Some(Nav::NextPage),
                            egui::Key::Space => Some(Nav::Start),
                            egui::Key::M => Some(Nav::Select),
                            egui::Key::R => Some(Nav::Record),
                            egui::Key::H => Some(Nav::LeftStick),
                            _ => None,
                        };
                        let Some(nav) = nav else { continue };
                        if *pressed && (!*repeat || nav.is_direction()) {
                            out.pressed.push(nav);
                        } else if !*pressed {
                            out.released.push(nav);
                        }
                    }
                }
            });
        }
        // Turning: the triggers and the right stick (with a dead zone).
        let stick = if self.ry.abs() > 0.2 { (self.ry - 0.2 * self.ry.signum()) / 0.8 } else { 0.0 };
        let trig = |v: f32| if v > 0.08 { v } else { 0.0 };
        out.turn = (trig(self.rt) - trig(self.lt) + stick).clamp(-1.0, 1.0);
        out.trig = trig(self.rt) - trig(self.lt);
        out.stick_turn = stick;
        let mag = (self.rx * self.rx + self.ry * self.ry).sqrt();
        out.stick = if mag > 0.15 { (self.rx, self.ry) } else { (0.0, 0.0) };
        let (lx, ly) = self.left_stick();
        out.lstick = if (lx * lx + ly * ly).sqrt() > 0.15 { (lx, ly) } else { (0.0, 0.0) };
        out.lt = trig(self.lt);
        out.rt = trig(self.rt);
        if keys {
            // On a keyboard: [ and ] (or - and =) turn.
            ctx.input(|i| {
                if i.key_down(egui::Key::CloseBracket) || i.key_down(egui::Key::Equals) {
                    out.turn = 0.6;
                    out.trig = 1.0;
                    out.rt = 1.0;
                }
                if i.key_down(egui::Key::OpenBracket) || i.key_down(egui::Key::Minus) {
                    out.turn = -0.6;
                    out.trig = -1.0;
                    out.lt = 1.0;
                }
            });
        }
        out
    }

    fn left_stick(&self) -> (f32, f32) {
        let Some(g) = self.gilrs.as_ref() else { return (0.0, 0.0) };
        let mut best = (0.0f32, 0.0f32);
        for (_, pad) in g.gamepads() {
            let x = pad.value(gilrs::Axis::LeftStickX);
            let y = pad.value(gilrs::Axis::LeftStickY);
            if x.abs().max(y.abs()) > best.0.abs().max(best.1.abs()) {
                best = (x, y);
            }
        }
        best
    }

}
