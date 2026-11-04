//! The Ableton Move's own screen. It is 128 by 64 black-and-white pixels behind
//! a USB interface of its own ("Ableton Move Display"): a bulk transfer of a
//! 16-byte header, then one of 1024 bytes, page after page of eight rows, each
//! byte a column with its lowest bit on top. Found by trying, on a real Move.
//! Nothing is sent to it while Live has it, or while the Move is left alone.

use crate::move_ctl::Screen;
use nusb::transfer::{Bulk, Out};
use nusb::MaybeFuture;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const WIDTH: usize = 128;
pub const HEIGHT: usize = 64;
const BYTES: usize = WIDTH * HEIGHT / 8;
const VENDOR: u16 = 0x2982;
const PRODUCT: u16 = 0x1958;
const INTERFACE: u8 = 0;
const ENDPOINT: u8 = 0x01;
const HEADER: [u8; 16] = [0xff, 0xcc, 0xaa, 0x88, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

/// A picture for the screen.
#[derive(Clone, PartialEq)]
pub struct Bitmap(pub [u8; BYTES]);

impl Bitmap {
    pub fn blank() -> Self {
        Self([0; BYTES])
    }

    pub fn set(&mut self, x: i32, y: i32) {
        if (0..WIDTH as i32).contains(&x) && (0..HEIGHT as i32).contains(&y) {
            self.0[(y as usize / 8) * WIDTH + x as usize] |= 1 << (y % 8);
        }
    }

    #[cfg(test)]
    pub fn get(&self, x: usize, y: usize) -> bool {
        self.0[(y / 8) * WIDTH + x] & (1 << (y % 8)) != 0
    }

    pub fn fill(&mut self, x: i32, y: i32, w: i32, h: i32) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.set(xx, yy);
            }
        }
    }

    pub fn outline(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.fill(x, y, w, 1);
        self.fill(x, y + h - 1, w, 1);
        self.fill(x, y, 1, h);
        self.fill(x + w - 1, y, 1, h);
    }

    /// Text at a size (1: 6 pixels a letter, 8 high; 2: twice that). Returns its width.
    pub fn text(&mut self, x: i32, y: i32, s: &str, scale: i32) -> i32 {
        let mut at = x;
        for c in s.chars() {
            let glyph = FONT[(c as usize).wrapping_sub(32).min(FONT.len() - 1)];
            for (col, bits) in glyph.iter().enumerate() {
                for row in 0..7 {
                    if bits & (1 << row) != 0 {
                        self.fill(at + col as i32 * scale, y + row * scale, scale, scale);
                    }
                }
            }
            at += 6 * scale;
        }
        at - x
    }
}

/// What fits on a line of the screen.
fn clipped(s: &str, letters: usize) -> String {
    s.chars().take(letters).collect()
}

/// The screen's content: the title big when it fits, the line under it; below
/// that the knob being touched, big, or a little bar for each of the eight.
pub fn render(s: &Screen) -> Bitmap {
    let mut b = Bitmap::blank();
    if s.title.chars().count() <= WIDTH / 12 {
        b.text(0, 0, &s.title, 2);
    } else {
        b.text(0, 3, &clipped(&s.title, WIDTH / 6), 1);
    }
    b.text(0, 17, &clipped(&s.sub, WIDTH / 6), 1);
    match s.hot.and_then(|h| s.params.get(h)) {
        Some((name, value, norm)) => {
            b.text(0, 28, &clipped(name, WIDTH / 6), 1);
            if value.chars().count() <= WIDTH / 12 {
                b.text(0, 38, value, 2);
            } else {
                b.text(0, 42, &clipped(value, WIDTH / 6), 1);
            }
            b.outline(0, 56, WIDTH as i32, 8);
            b.fill(1, 57, (norm.clamp(0.0, 1.0) * (WIDTH as f32 - 2.0)) as i32, 6);
        }
        None => {
            for (i, (_, _, norm)) in s.params.iter().take(8).enumerate() {
                let x = i as i32 * 16;
                b.outline(x + 1, 28, 14, 36);
                let h = (norm.clamp(0.0, 1.0) * 34.0).round() as i32;
                b.fill(x + 2, 63 - h, 12, h);
            }
        }
    }
    b
}

/// Keeps the Move's screen showing the picture it is given, on a thread of its
/// own, and looks for it again when it goes away.
pub struct ScreenLink {
    picture: Arc<Mutex<Bitmap>>,
    stop: Arc<AtomicBool>,
    pub on: Arc<AtomicBool>,
}

impl ScreenLink {
    pub fn start() -> Self {
        let picture = Arc::new(Mutex::new(Bitmap::blank()));
        let stop = Arc::new(AtomicBool::new(false));
        let on = Arc::new(AtomicBool::new(true));
        let (p, s, o) = (picture.clone(), stop.clone(), on.clone());
        std::thread::Builder::new().name("move-screen".into()).spawn(move || run(p, s, o)).ok();
        Self { picture, stop, on }
    }

    pub fn show(&self, b: Bitmap) {
        if let Ok(mut p) = self.picture.lock() {
            *p = b;
        }
    }
}

impl Drop for ScreenLink {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}

fn open() -> Option<nusb::Endpoint<Bulk, Out>> {
    let info = nusb::list_devices().wait().ok()?.find(|d| d.vendor_id() == VENDOR && d.product_id() == PRODUCT)?;
    let device = info.open().wait().ok()?;
    let interface = device.claim_interface(INTERFACE).wait().ok()?;
    interface.endpoint::<Bulk, Out>(ENDPOINT).ok()
}

fn send(ep: &mut nusb::Endpoint<Bulk, Out>, data: Vec<u8>) -> bool {
    ep.submit(data.into());
    ep.wait_next_complete(Duration::from_millis(500)).is_some_and(|c| c.status.is_ok())
}

fn run(picture: Arc<Mutex<Bitmap>>, stop: Arc<AtomicBool>, on: Arc<AtomicBool>) {
    while !stop.load(Ordering::Acquire) {
        if !on.load(Ordering::Acquire) {
            std::thread::sleep(Duration::from_millis(300));
            continue;
        }
        let Some(mut ep) = open() else {
            // Not there, or Live has it: look again in a while.
            std::thread::sleep(Duration::from_secs(2));
            continue;
        };
        while !stop.load(Ordering::Acquire) && on.load(Ordering::Acquire) {
            let frame = picture.lock().map(|p| p.0.to_vec()).unwrap_or_default();
            if frame.len() != BYTES || !send(&mut ep, HEADER.to_vec()) || !send(&mut ep, frame) {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

/// Five columns a letter, lowest bit on top: ASCII from the space to the tilde,
/// and a last one for anything else.
const FONT: [[u8; 5]; 96] = [
    [0x00, 0x00, 0x00, 0x00, 0x00], [0x00, 0x00, 0x5f, 0x00, 0x00], [0x00, 0x07, 0x00, 0x07, 0x00], [0x14, 0x7f, 0x14, 0x7f, 0x14],
    [0x24, 0x2a, 0x7f, 0x2a, 0x12], [0x23, 0x13, 0x08, 0x64, 0x62], [0x36, 0x49, 0x55, 0x22, 0x50], [0x00, 0x05, 0x03, 0x00, 0x00],
    [0x00, 0x1c, 0x22, 0x41, 0x00], [0x00, 0x41, 0x22, 0x1c, 0x00], [0x14, 0x08, 0x3e, 0x08, 0x14], [0x08, 0x08, 0x3e, 0x08, 0x08],
    [0x00, 0x50, 0x30, 0x00, 0x00], [0x08, 0x08, 0x08, 0x08, 0x08], [0x00, 0x60, 0x60, 0x00, 0x00], [0x20, 0x10, 0x08, 0x04, 0x02],
    [0x3e, 0x51, 0x49, 0x45, 0x3e], [0x00, 0x42, 0x7f, 0x40, 0x00], [0x42, 0x61, 0x51, 0x49, 0x46], [0x21, 0x41, 0x45, 0x4b, 0x31],
    [0x18, 0x14, 0x12, 0x7f, 0x10], [0x27, 0x45, 0x45, 0x45, 0x39], [0x3c, 0x4a, 0x49, 0x49, 0x30], [0x01, 0x71, 0x09, 0x05, 0x03],
    [0x36, 0x49, 0x49, 0x49, 0x36], [0x06, 0x49, 0x49, 0x29, 0x1e], [0x00, 0x36, 0x36, 0x00, 0x00], [0x00, 0x56, 0x36, 0x00, 0x00],
    [0x08, 0x14, 0x22, 0x41, 0x00], [0x14, 0x14, 0x14, 0x14, 0x14], [0x00, 0x41, 0x22, 0x14, 0x08], [0x02, 0x01, 0x51, 0x09, 0x06],
    [0x32, 0x49, 0x79, 0x41, 0x3e], [0x7e, 0x11, 0x11, 0x11, 0x7e], [0x7f, 0x49, 0x49, 0x49, 0x36], [0x3e, 0x41, 0x41, 0x41, 0x22],
    [0x7f, 0x41, 0x41, 0x22, 0x1c], [0x7f, 0x49, 0x49, 0x49, 0x41], [0x7f, 0x09, 0x09, 0x09, 0x01], [0x3e, 0x41, 0x49, 0x49, 0x7a],
    [0x7f, 0x08, 0x08, 0x08, 0x7f], [0x00, 0x41, 0x7f, 0x41, 0x00], [0x20, 0x40, 0x41, 0x3f, 0x01], [0x7f, 0x08, 0x14, 0x22, 0x41],
    [0x7f, 0x40, 0x40, 0x40, 0x40], [0x7f, 0x02, 0x0c, 0x02, 0x7f], [0x7f, 0x04, 0x08, 0x10, 0x7f], [0x3e, 0x41, 0x41, 0x41, 0x3e],
    [0x7f, 0x09, 0x09, 0x09, 0x06], [0x3e, 0x41, 0x51, 0x21, 0x5e], [0x7f, 0x09, 0x19, 0x29, 0x46], [0x46, 0x49, 0x49, 0x49, 0x31],
    [0x01, 0x01, 0x7f, 0x01, 0x01], [0x3f, 0x40, 0x40, 0x40, 0x3f], [0x1f, 0x20, 0x40, 0x20, 0x1f], [0x3f, 0x40, 0x38, 0x40, 0x3f],
    [0x63, 0x14, 0x08, 0x14, 0x63], [0x07, 0x08, 0x70, 0x08, 0x07], [0x61, 0x51, 0x49, 0x45, 0x43], [0x00, 0x7f, 0x41, 0x41, 0x00],
    [0x02, 0x04, 0x08, 0x10, 0x20], [0x00, 0x41, 0x41, 0x7f, 0x00], [0x04, 0x02, 0x01, 0x02, 0x04], [0x40, 0x40, 0x40, 0x40, 0x40],
    [0x00, 0x01, 0x02, 0x04, 0x00], [0x20, 0x54, 0x54, 0x54, 0x78], [0x7f, 0x48, 0x44, 0x44, 0x38], [0x38, 0x44, 0x44, 0x44, 0x20],
    [0x38, 0x44, 0x44, 0x48, 0x7f], [0x38, 0x54, 0x54, 0x54, 0x18], [0x08, 0x7e, 0x09, 0x01, 0x02], [0x0c, 0x52, 0x52, 0x52, 0x3e],
    [0x7f, 0x08, 0x04, 0x04, 0x78], [0x00, 0x44, 0x7d, 0x40, 0x00], [0x20, 0x40, 0x44, 0x3d, 0x00], [0x7f, 0x10, 0x28, 0x44, 0x00],
    [0x00, 0x41, 0x7f, 0x40, 0x00], [0x7c, 0x04, 0x18, 0x04, 0x78], [0x7c, 0x08, 0x04, 0x04, 0x78], [0x38, 0x44, 0x44, 0x44, 0x38],
    [0x7c, 0x14, 0x14, 0x14, 0x08], [0x08, 0x14, 0x14, 0x18, 0x7c], [0x7c, 0x08, 0x04, 0x04, 0x08], [0x48, 0x54, 0x54, 0x54, 0x20],
    [0x04, 0x3f, 0x44, 0x40, 0x20], [0x3c, 0x40, 0x40, 0x20, 0x7c], [0x1c, 0x20, 0x40, 0x20, 0x1c], [0x3c, 0x40, 0x30, 0x40, 0x3c],
    [0x44, 0x28, 0x10, 0x28, 0x44], [0x0c, 0x50, 0x50, 0x50, 0x3c], [0x44, 0x64, 0x54, 0x4c, 0x44], [0x00, 0x08, 0x36, 0x41, 0x00],
    [0x00, 0x00, 0x7f, 0x00, 0x00], [0x00, 0x41, 0x36, 0x08, 0x00], [0x10, 0x08, 0x08, 0x10, 0x08], [0x02, 0x01, 0x51, 0x09, 0x06],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_byte_is_a_column_with_its_lowest_bit_on_top() {
        let mut b = Bitmap::blank();
        b.set(0, 0);
        assert_eq!(b.0[0], 1);
        b.set(0, 7);
        assert_eq!(b.0[0], 0x81);
        b.set(3, 9);
        assert_eq!(b.0[128 + 3], 0b10);
        // Outside the screen nothing happens.
        b.set(128, 0);
        b.set(0, 64);
        b.set(-1, -1);
        assert_eq!(b.0.iter().map(|v| v.count_ones()).sum::<u32>(), 3);
    }

    #[test]
    fn letters_land_where_they_should_at_their_size() {
        let mut b = Bitmap::blank();
        // The I: three columns of its top row, and its stem.
        assert_eq!(b.text(0, 0, "I", 1), 6);
        assert!(b.get(1, 0) && b.get(2, 0) && b.get(3, 0) && b.get(2, 3) && !b.get(0, 0));
        let mut big = Bitmap::blank();
        assert_eq!(big.text(0, 0, "I", 2), 12);
        assert!(big.get(4, 0) && big.get(5, 1) && big.get(4, 6));
        // Anything else draws the last letter, not outside the table.
        Bitmap::blank().text(0, 0, "é€\u{1F980}", 1);
    }

    fn lit(b: &Bitmap, x: std::ops::Range<usize>, y: std::ops::Range<usize>) -> usize {
        y.flat_map(|yy| x.clone().map(move |xx| (xx, yy))).filter(|&(xx, yy)| b.get(xx, yy)).count()
    }

    #[test]
    fn the_touched_knob_shows_big_and_the_others_as_bars() {
        let params: Vec<(String, String, f32)> = (0..8).map(|i| (format!("P{i}"), "50 %".to_string(), i as f32 / 7.0)).collect();
        let s = Screen { title: "Munich".into(), sub: "Bass".into(), params: params.clone(), hot: None };
        let b = render(&s);
        // The last bar is full, the first has none.
        assert!(lit(&b, 114, 30..63) == 0 || lit(&b, 114..126, 30..63) > 300);
        assert!(lit(&b, 3..13, 30..62) == 0, "the first knob is at nothing");
        assert!(lit(&b, 115..125, 30..62) > 300, "the last is full");
        // Touched: its value's bar across the bottom.
        let s = Screen { title: "Munich".into(), sub: "Bass".into(), params, hot: Some(7) };
        let b = render(&s);
        assert!(lit(&b, 1..127, 57..63) > 700);
        // A long title goes small and stays on the screen.
        let s = Screen { title: "A Very Long Track Name Indeed".into(), sub: String::new(), params: Vec::new(), hot: None };
        assert!(lit(&render(&s), 0..128, 0..16) > 0);
    }
}
