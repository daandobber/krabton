//! Grids straight over their serial port, for when serialosc does not see
//! them (newer grids are USB serial devices without the FTDI chip that
//! older serialosc versions look for).
//!
//! The protocol is monome's "mext" serial protocol, as in libmonome: one
//! header byte (subsystem << 4 | command) followed by a fixed payload. The
//! grid sends `0x20 x y` / `0x21 x y` for key up and down, answers `0x01`
//! with its id (`0x01` + 32 bytes) and `0x05` with its size (`0x03 x y`);
//! LEDs are set an 8x8 quad at a time with `0x1A x y` and 32 bytes of 4-bit
//! levels. Some DIY grids print a line of text when the port opens, so the
//! first moments are thrown away before asking anything.

use serialport::{SerialPort, SerialPortType, UsbPortInfo};
use std::io::{Read, Write};
use std::time::{Duration, Instant};

/// Whether a USB serial port looks like a monome device: its product or
/// maker says so, or its serial number is monome's `m` and digits.
pub fn looks_like_monome(info: &UsbPortInfo) -> bool {
    let says = |s: &Option<String>| s.as_deref().is_some_and(|s| s.to_lowercase().contains("monome"));
    let serial = info.serial_number.as_deref().unwrap_or("");
    let monome_serial = serial.len() > 3 && serial.to_lowercase().starts_with('m') && serial[1..].chars().all(|c| c.is_ascii_digit());
    says(&info.product) || says(&info.manufacturer) || (monome_serial && (info.vid == 0x0403 || info.vid == 0x16C0 || info.vid == 0x0483))
}

/// What a grid sends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SerialMsg {
    Key { x: u32, y: u32, down: bool },
    Size { width: u32, height: u32 },
    Id(String),
    /// A reply we do not use.
    Other(u8),
}

/// Payload length after each header byte the grid can send.
fn payload_len(header: u8) -> Option<usize> {
    Some(match header {
        0x00 => 2,  // query: subsystem, count
        0x01 => 32, // id
        0x02 => 2,  // grid offset
        0x03 => 2,  // grid size
        0x04 => 2,  // address
        0x0F => 8,  // firmware version
        0x20 | 0x21 => 2,
        0x50 => 2,  // encoder delta
        0x51 | 0x52 => 1,
        _ => return None,
    })
}

/// Takes the complete messages off the front of `buf`.
pub fn parse(buf: &mut Vec<u8>) -> Vec<SerialMsg> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < buf.len() {
        let header = buf[i];
        let Some(len) = payload_len(header) else {
            // Not something we know: skip the byte and resync.
            i += 1;
            continue;
        };
        if i + 1 + len > buf.len() {
            break;
        }
        let p = &buf[i + 1..i + 1 + len];
        out.push(match header {
            0x20 | 0x21 => SerialMsg::Key { x: p[0] as u32, y: p[1] as u32, down: header == 0x21 },
            0x03 => SerialMsg::Size { width: p[0] as u32, height: p[1] as u32 },
            0x01 => SerialMsg::Id(String::from_utf8_lossy(p).trim_end_matches(['\0', ' ']).to_string()),
            h => SerialMsg::Other(h),
        });
        i += 1 + len;
    }
    buf.drain(..i);
    out
}

/// The message setting the 8x8 quad at (`x`, `y`) to `levels` (0..15, row
/// by row), two levels per byte, the first in the high nibble.
pub fn level_map(x: u32, y: u32, levels: &[u8; 64]) -> [u8; 35] {
    let mut m = [0u8; 35];
    m[0] = 0x1A;
    m[1] = x as u8;
    m[2] = y as u8;
    for i in 0..32 {
        m[3 + i] = (levels[2 * i].min(15) << 4) | levels[2 * i + 1].min(15);
    }
    m
}

/// A grid opened on its serial port.
pub struct SerialGrid {
    port: Box<dyn SerialPort>,
    pub port_name: String,
    /// The serial number, which tells grids apart.
    pub id: String,
    /// What the grid calls itself.
    pub name: String,
    pub size: Option<(u32, u32)>,
    buf: Vec<u8>,
    opened: Instant,
    asked: bool,
}

/// Time to let a grid finish whatever it says when the port opens.
const SETTLE: Duration = Duration::from_millis(250);

impl SerialGrid {
    /// The serial ports that look like monome grids.
    pub fn find() -> Vec<(String, String)> {
        serialport::available_ports()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|p| match &p.port_type {
                SerialPortType::UsbPort(info) if looks_like_monome(info) => Some((p.port_name.clone(), info.serial_number.clone().unwrap_or_default())),
                _ => None,
            })
            .collect()
    }

    /// Opens the port; the grid is asked for its id and size once it has
    /// settled (see `poll`).
    pub fn open(port_name: &str, serial: &str) -> std::io::Result<Self> {
        let port = serialport::new(port_name, 115_200).timeout(Duration::from_millis(1)).open().map_err(std::io::Error::other)?;
        Ok(Self { port, port_name: port_name.to_string(), id: serial.to_lowercase(), name: String::new(), size: None, buf: Vec::with_capacity(256), opened: Instant::now(), asked: false })
    }

    /// Reads what arrived. An error means the grid is gone.
    pub fn poll(&mut self) -> std::io::Result<Vec<SerialMsg>> {
        let mut chunk = [0u8; 512];
        loop {
            let waiting = self.port.bytes_to_read().map_err(std::io::Error::other)? as usize;
            if waiting == 0 {
                break;
            }
            match self.port.read(&mut chunk[..waiting.min(512)]) {
                Ok(0) => break,
                Ok(n) => self.buf.extend_from_slice(&chunk[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
                Err(e) => return Err(e),
            }
        }
        if !self.asked {
            // Drop the greeting, then ask for the id and the size.
            self.buf.clear();
            if self.opened.elapsed() < SETTLE {
                return Ok(Vec::new());
            }
            let _ = self.port.clear(serialport::ClearBuffer::Input);
            self.port.write_all(&[0x01, 0x05])?;
            self.asked = true;
            return Ok(Vec::new());
        }
        let msgs = parse(&mut self.buf);
        for m in &msgs {
            match m {
                SerialMsg::Size { width, height } => self.size = Some((*width, *height)),
                SerialMsg::Id(name) => self.name = name.clone(),
                _ => {}
            }
        }
        Ok(msgs)
    }

    pub fn send(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.port.write_all(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_size_and_id_are_parsed_across_reads() {
        let mut buf = vec![0x21, 3, 4, 0x20, 3];
        assert_eq!(parse(&mut buf), vec![SerialMsg::Key { x: 3, y: 4, down: true }]);
        assert_eq!(buf, vec![0x20, 3], "an incomplete message waits for the rest");
        buf.extend_from_slice(&[4, 0x03, 16, 8]);
        assert_eq!(parse(&mut buf), vec![SerialMsg::Key { x: 3, y: 4, down: false }, SerialMsg::Size { width: 16, height: 8 }]);
        let mut id = vec![0x01];
        id.extend_from_slice(b"m1234567");
        id.resize(33, 0);
        assert_eq!(parse(&mut id), vec![SerialMsg::Id("m1234567".into())]);
        // Unknown bytes are skipped.
        let mut junk = vec![0xEE, 0x21, 0, 0];
        assert_eq!(parse(&mut junk), vec![SerialMsg::Key { x: 0, y: 0, down: true }]);
    }

    #[test]
    fn level_maps_pack_two_leds_per_byte() {
        let mut levels = [0u8; 64];
        levels[0] = 15;
        levels[1] = 3;
        levels[63] = 20; // clipped to 15
        let m = level_map(8, 0, &levels);
        assert_eq!(&m[..4], &[0x1A, 8, 0, 0xF3]);
        assert_eq!(m[34], 0x0F);
    }

    #[test]
    fn monome_ports_are_recognized() {
        let port = |vid, pid, serial: &str, product: Option<&str>| UsbPortInfo { vid, pid, serial_number: Some(serial.into()), manufacturer: None, product: product.map(|s| s.into()) };
        assert!(looks_like_monome(&port(0x16C0, 0x0483, "m1234567", None)));
        assert!(looks_like_monome(&port(0x0403, 0x6001, "m1000042", None)));
        assert!(looks_like_monome(&port(0x1234, 0x5678, "abc", Some("monome grid"))));
        assert!(!looks_like_monome(&port(0x16C0, 0x0483, "12345", Some("Teensyduino Serial"))));
    }
}
