//! Native monome support through serialosc: discovers grids and arcs,
//! connects to them, reads keys/encoders and drives varibright LEDs.
//!
//! Grids that serialosc does not see (newer and DIY grids that are plain USB
//! serial devices) are opened directly on their serial port (see `serial`).
//!
//! serialosc speaks OSC over UDP on localhost. Krabton asks it for devices
//! (`/serialosc/list`), points each device at its own UDP port
//! (`/sys/port`, `/sys/host`, `/sys/prefix`) and then receives
//! `/krabton/grid/key x y s` and `/krabton/enc/delta n d`, and sends
//! `/krabton/grid/led/level/map` and `/krabton/ring/map`.

pub mod osc;
pub mod serial;

use osc::{decode, OscArg, OscMessage};
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

/// serialosc's default discovery port.
pub const SERIALOSC_PORT: u16 = 12002;
const GRID_PREFIX: &str = "/krabton";
const ARC_PREFIX: &str = "/krabton_arc";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceInfo {
    pub id: String,
    pub kind: String,
    pub port: u16,
}

impl DeviceInfo {
    pub fn is_arc(&self) -> bool {
        self.kind.to_lowercase().contains("arc")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// The device list changed.
    Devices,
    GridConnected { id: String, width: u32, height: u32 },
    ArcConnected { id: String },
    Disconnected { id: String },
    Key { x: u32, y: u32, down: bool },
    EncDelta { n: u32, delta: i32 },
    EncKey { n: u32, down: bool },
}

struct Link {
    device: DeviceInfo,
    addr: SocketAddr,
    /// Last LED levels sent, to only send changed 8x8 quads.
    sent: Vec<u8>,
    last_full: Instant,
}

pub struct Monome {
    sock: UdpSocket,
    serialosc: SocketAddr,
    pub devices: Vec<DeviceInfo>,
    grid: Option<Link>,
    arc: Option<Link>,
    pub grid_size: (u32, u32),
    last_discover: Option<Instant>,
    /// A grid on a serial port, when serialosc has none.
    serial: Option<serial::SerialGrid>,
    serial_enabled: bool,
    serial_scan: Option<Instant>,
    serial_sent: Vec<u8>,
    serial_full: Instant,
    /// How far the grid is turned (0, 90, 180 or 270 degrees).
    rotation: u32,
}

/// A key at (`x`, `y`) on a `w` x `h` device, as seen with the device turned
/// `rotation` degrees.
pub fn rotate(rotation: u32, w: u32, h: u32, x: u32, y: u32) -> (u32, u32) {
    match rotation {
        90 => (h - 1 - y, x),
        180 => (w - 1 - x, h - 1 - y),
        270 => (y, w - 1 - x),
        _ => (x, y),
    }
}

/// The opposite of `rotate`: where (`x`, `y`) of the turned grid sits on the
/// `w` x `h` device.
pub fn unrotate(rotation: u32, w: u32, h: u32, x: u32, y: u32) -> (u32, u32) {
    match rotation {
        90 => (y, h - 1 - x),
        180 => (w - 1 - x, h - 1 - y),
        270 => (w - 1 - y, x),
        _ => (x, y),
    }
}

impl Monome {
    pub fn new() -> io::Result<Self> {
        Self::with_serialosc_port(SERIALOSC_PORT)
    }

    pub fn with_serialosc_port(port: u16) -> io::Result<Self> {
        let sock = UdpSocket::bind("127.0.0.1:0")?;
        sock.set_nonblocking(true)?;
        Ok(Self {
            sock,
            serialosc: SocketAddr::from(([127, 0, 0, 1], port)),
            devices: Vec::new(),
            grid: None,
            arc: None,
            grid_size: (16, 8),
            last_discover: None,
            serial: None,
            serial_enabled: false,
            serial_scan: None,
            serial_sent: Vec::new(),
            serial_full: Instant::now(),
            rotation: 0,
        })
    }

    /// Turns the grid 0, 90, 180 or 270 degrees: keys and LEDs are mapped so
    /// the app always sees it upright. 90 and 270 swap width and height.
    pub fn set_rotation(&mut self, degrees: u32) {
        let r = degrees / 90 % 4 * 90;
        if r != self.rotation {
            self.rotation = r;
            if let Some(l) = &mut self.grid {
                l.sent.clear();
            }
            self.serial_sent.clear();
        }
    }

    pub fn rotation(&self) -> u32 {
        self.rotation
    }

    /// The grid's size as the app sees it (turned).
    pub fn grid_dims(&self) -> (u32, u32) {
        let (w, h) = self.grid_size;
        if self.rotation % 180 == 90 {
            (h, w)
        } else {
            (w, h)
        }
    }

    /// A key from the device, in the app's (turned) coordinates.
    fn key_event(&self, x: u32, y: u32, down: bool) -> Option<Event> {
        let (w, h) = self.grid_size;
        if x >= w || y >= h {
            return None;
        }
        let (x, y) = rotate(self.rotation, w, h, x, y);
        Some(Event::Key { x, y, down })
    }

    fn connected(&self, id: String) -> Event {
        let (width, height) = self.grid_dims();
        Event::GridConnected { id, width, height }
    }

    /// Lets the client open grids on serial ports when serialosc has none.
    /// Off by default, so tests never take over a real grid.
    pub fn set_serial(&mut self, on: bool) {
        self.serial_enabled = on;
        if !on {
            self.serial = None;
        }
    }

    pub fn local_port(&self) -> u16 {
        self.sock.local_addr().map(|a| a.port()).unwrap_or(0)
    }

    fn send(&self, to: SocketAddr, msg: OscMessage) {
        let _ = self.sock.send_to(&msg.encode(), to);
    }

    fn host_args(&self) -> Vec<OscArg> {
        vec![OscArg::Str("127.0.0.1".into()), OscArg::Int(self.local_port() as i32)]
    }

    /// Asks serialosc for its devices and for add/remove notifications.
    pub fn discover(&mut self) {
        self.send(self.serialosc, OscMessage::new("/serialosc/list", self.host_args()));
        self.send(self.serialosc, OscMessage::new("/serialosc/notify", self.host_args()));
        self.last_discover = Some(Instant::now());
    }

    pub fn grid_id(&self) -> Option<&str> {
        self.grid.as_ref().map(|l| l.device.id.as_str()).or_else(|| self.serial.as_ref().filter(|s| s.size.is_some()).map(|s| s.id.as_str()))
    }

    /// The serial port of the grid, when it is not reached through serialosc.
    pub fn grid_port(&self) -> Option<&str> {
        self.serial.as_ref().filter(|_| self.grid.is_none()).map(|s| s.port_name.as_str())
    }

    /// Looks for devices again, on serialosc and on serial ports.
    pub fn rescan(&mut self) {
        self.serial_scan = None;
        self.discover();
    }

    pub fn arc_id(&self) -> Option<&str> {
        self.arc.as_ref().map(|l| l.device.id.as_str())
    }

    /// Routes a device's output to us. Grids and arcs use separate prefixes.
    pub fn connect(&mut self, id: &str) -> bool {
        let Some(dev) = self.devices.iter().find(|d| d.id == id).cloned() else { return false };
        let addr = SocketAddr::from(([127, 0, 0, 1], dev.port));
        let prefix = if dev.is_arc() { ARC_PREFIX } else { GRID_PREFIX };
        self.send(addr, OscMessage::new("/sys/port", vec![OscArg::Int(self.local_port() as i32)]));
        self.send(addr, OscMessage::new("/sys/host", vec![OscArg::Str("127.0.0.1".into())]));
        self.send(addr, OscMessage::new("/sys/prefix", vec![OscArg::Str(prefix.into())]));
        self.send(addr, OscMessage::new("/sys/info", self.host_args()));
        let link = Link { device: dev.clone(), addr, sent: Vec::new(), last_full: Instant::now() };
        if dev.is_arc() {
            self.arc = Some(link);
        } else {
            self.grid = Some(link);
        }
        true
    }

    /// Reads everything that arrived. Also auto-connects the first grid and
    /// arc and re-asks serialosc for devices every few seconds.
    pub fn poll(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        let mut buf = [0u8; 2048];
        let mut msgs = Vec::new();
        for _ in 0..4096 {
            match self.sock.recv_from(&mut buf) {
                Ok((n, _)) => msgs.extend(decode(&buf[..n])),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                // Windows: an earlier send hit a closed port. Keep reading.
                Err(_) => continue,
            }
        }
        for m in msgs {
            self.handle(m, &mut events);
        }
        if self.grid.is_none() {
            if let Some(d) = self.devices.iter().find(|d| !d.is_arc()).cloned() {
                if self.connect(&d.id) {
                    events.push(self.connected(d.id));
                }
            }
        }
        if self.arc.is_none() {
            if let Some(d) = self.devices.iter().find(|d| d.is_arc()).cloned() {
                if self.connect(&d.id) {
                    events.push(Event::ArcConnected { id: d.id });
                }
            }
        }
        let stale = self.last_discover.map(|t| t.elapsed() > Duration::from_secs(4)).unwrap_or(true);
        if stale && (self.grid.is_none() || self.arc.is_none()) {
            self.discover();
        }
        self.poll_serial(&mut events);
        events
    }

    /// A grid on a serial port: found every few seconds while serialosc has
    /// none, read every poll.
    fn poll_serial(&mut self, events: &mut Vec<Event>) {
        if !self.serial_enabled || self.grid.is_some() {
            return;
        }
        if self.serial.is_none() {
            let due = self.serial_scan.map(|t| t.elapsed() > Duration::from_secs(3)).unwrap_or(true);
            if !due {
                return;
            }
            self.serial_scan = Some(Instant::now());
            for (port, number) in serial::SerialGrid::find() {
                if let Ok(g) = serial::SerialGrid::open(&port, &number) {
                    self.serial = Some(g);
                    self.serial_sent.clear();
                    break;
                }
            }
        }
        let Some(g) = &mut self.serial else { return };
        let had_size = g.size.is_some();
        match g.poll() {
            Ok(msgs) => {
                let size = g.size;
                let id = g.id.clone();
                if let (false, Some((w, h))) = (had_size, size) {
                    self.grid_size = (w.max(1), h.max(1));
                    self.serial_sent.clear();
                    events.push(self.connected(id));
                }
                for m in msgs {
                    if let serial::SerialMsg::Key { x, y, down } = m {
                        events.extend(self.key_event(x, y, down));
                    }
                }
            }
            Err(_) => {
                let id = g.id.clone();
                self.serial = None;
                events.push(Event::Disconnected { id });
            }
        }
    }

    /// LEDs of a serial grid: changed 8x8 quads, a full refresh now and then.
    fn serial_leds(&mut self, levels: &[u8], w: u32, h: u32) {
        let Some(g) = &mut self.serial else { return };
        if g.size.is_none() {
            return;
        }
        let total = (w * h) as usize;
        let full = self.serial_sent.len() != total || self.serial_full.elapsed() > Duration::from_secs(2);
        if full {
            self.serial_full = Instant::now();
        }
        let mut out = Vec::new();
        for qy in (0..h).step_by(8) {
            for qx in (0..w).step_by(8) {
                let mut quad = [0u8; 64];
                let mut changed = full;
                for y in 0..8 {
                    for x in 0..8 {
                        let (gx, gy) = (qx + x, qy + y);
                        let level = if gx < w && gy < h { levels[(gy * w + gx) as usize].min(15) } else { 0 };
                        if !full && gx < w && gy < h && self.serial_sent.get((gy * w + gx) as usize) != Some(&level) {
                            changed = true;
                        }
                        quad[(y * 8 + x) as usize] = level;
                    }
                }
                if changed {
                    out.extend_from_slice(&serial::level_map(qx, qy, &quad));
                }
            }
        }
        self.serial_sent = levels[..total].to_vec();
        if !out.is_empty() && g.send(&out).is_err() {
            self.serial = None;
        }
    }

    fn handle(&mut self, m: OscMessage, events: &mut Vec<Event>) {
        let addr = m.addr.as_str();
        match addr {
            "/serialosc/device" | "/serialosc/add" => {
                if let (Some(id), Some(kind), Some(port)) = (m.str(0), m.str(1), m.int(2)) {
                    if !self.devices.iter().any(|d| d.id == id) {
                        self.devices.push(DeviceInfo { id: id.into(), kind: kind.into(), port: port as u16 });
                        events.push(Event::Devices);
                    }
                }
                if addr == "/serialosc/add" {
                    self.send(self.serialosc, OscMessage::new("/serialosc/notify", self.host_args()));
                }
            }
            "/serialosc/remove" => {
                if let Some(id) = m.str(0) {
                    let id = id.to_string();
                    self.devices.retain(|d| d.id != id);
                    if self.grid.as_ref().map(|l| l.device.id == id).unwrap_or(false) {
                        self.grid = None;
                        events.push(Event::Disconnected { id: id.clone() });
                    }
                    if self.arc.as_ref().map(|l| l.device.id == id).unwrap_or(false) {
                        self.arc = None;
                        events.push(Event::Disconnected { id: id.clone() });
                    }
                    events.push(Event::Devices);
                }
                self.send(self.serialosc, OscMessage::new("/serialosc/notify", self.host_args()));
            }
            "/sys/size" => {
                if let (Some(w), Some(h)) = (m.int(0), m.int(1)) {
                    self.grid_size = (w.max(1) as u32, h.max(1) as u32);
                    if let Some(l) = &mut self.grid {
                        l.sent.clear();
                        let id = l.device.id.clone();
                        events.push(self.connected(id));
                    }
                }
            }
            _ => {
                if let Some(rest) = addr.strip_prefix(GRID_PREFIX) {
                    if rest == "/grid/key" {
                        if let (Some(x), Some(y), Some(s)) = (m.int(0), m.int(1), m.int(2)) {
                            events.extend(self.key_event(x.max(0) as u32, y.max(0) as u32, s != 0));
                        }
                    }
                }
                if let Some(rest) = addr.strip_prefix(ARC_PREFIX) {
                    match rest {
                        "/enc/delta" => {
                            if let (Some(n), Some(d)) = (m.int(0), m.int(1)) {
                                events.push(Event::EncDelta { n: n.max(0) as u32, delta: d });
                            }
                        }
                        "/enc/key" => {
                            if let (Some(n), Some(s)) = (m.int(0), m.int(1)) {
                                events.push(Event::EncKey { n: n.max(0) as u32, down: s != 0 });
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Sends grid LED levels (0..15, row-major, `w * h`, as the app sees the
    /// grid). Only changed 8x8 quads are sent, plus a full refresh every two
    /// seconds.
    pub fn grid_leds(&mut self, levels: &[u8], w: u32, h: u32) {
        let total = (w * h) as usize;
        if levels.len() < total || total == 0 {
            return;
        }
        if self.rotation != 0 {
            // Back to the device's own layout.
            let (dw, dh) = if self.rotation % 180 == 90 { (h, w) } else { (w, h) };
            let mut turned = vec![0u8; total];
            for y in 0..h {
                for x in 0..w {
                    let (dx, dy) = unrotate(self.rotation, dw, dh, x, y);
                    turned[(dy * dw + dx) as usize] = levels[(y * w + x) as usize];
                }
            }
            return self.device_leds(&turned, dw, dh);
        }
        self.device_leds(levels, w, h);
    }

    /// LED levels in the device's own layout.
    fn device_leds(&mut self, levels: &[u8], w: u32, h: u32) {
        let total = (w * h) as usize;
        if self.grid.is_none() {
            self.serial_leds(levels, w, h);
            return;
        }
        let Some(link) = &mut self.grid else { return };
        let full = link.sent.len() != total || link.last_full.elapsed() > Duration::from_secs(2);
        if full {
            link.last_full = Instant::now();
        }
        let addr = link.addr;
        let mut msgs = Vec::new();
        for qy in (0..h).step_by(8) {
            for qx in (0..w).step_by(8) {
                let mut quad = Vec::with_capacity(66);
                quad.push(OscArg::Int(qx as i32));
                quad.push(OscArg::Int(qy as i32));
                let mut changed = full;
                for y in qy..qy + 8 {
                    for x in qx..qx + 8 {
                        let level = if x < w && y < h { levels[(y * w + x) as usize].min(15) } else { 0 };
                        if !full && x < w && y < h && link.sent.get((y * w + x) as usize) != Some(&level) {
                            changed = true;
                        }
                        quad.push(OscArg::Int(level as i32));
                    }
                }
                if changed {
                    msgs.push(OscMessage::new(format!("{GRID_PREFIX}/grid/led/level/map"), quad));
                }
            }
        }
        link.sent = levels[..total].to_vec();
        for m in msgs {
            self.send(addr, m);
        }
    }

    /// Sets the 64 LEDs (0..15) of arc ring `n`.
    pub fn ring_leds(&mut self, n: u32, levels: &[u8; 64]) {
        let Some(link) = &mut self.arc else { return };
        let start = n as usize * 64;
        if link.sent.len() < start + 64 {
            link.sent.resize(start + 64, 255);
        }
        if link.sent[start..start + 64] == levels[..] {
            return;
        }
        link.sent[start..start + 64].copy_from_slice(levels);
        let addr = link.addr;
        let mut args = vec![OscArg::Int(n as i32)];
        args.extend(levels.iter().map(|&l| OscArg::Int(l.min(15) as i32)));
        self.send(addr, OscMessage::new(format!("{ARC_PREFIX}/ring/map"), args));
    }

    /// Turns every LED off (e.g. when quitting).
    pub fn all_off(&mut self) {
        if let Some(g) = &mut self.serial {
            // Level "all" to 0.
            let _ = g.send(&[0x19, 0]);
        }
        if let Some(l) = &self.grid {
            self.send(l.addr, OscMessage::new(format!("{GRID_PREFIX}/grid/led/level/all"), vec![OscArg::Int(0)]));
        }
        if let Some(l) = &self.arc {
            for n in 0..4 {
                self.send(l.addr, OscMessage::new(format!("{ARC_PREFIX}/ring/all"), vec![OscArg::Int(n), OscArg::Int(0)]));
            }
        }
    }
}

impl Drop for Monome {
    fn drop(&mut self) {
        self.all_off();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recv(sock: &UdpSocket) -> Vec<(OscMessage, SocketAddr)> {
        let mut out = Vec::new();
        let mut buf = [0u8; 2048];
        let deadline = Instant::now() + Duration::from_millis(500);
        loop {
            match sock.recv_from(&mut buf) {
                Ok((n, from)) => out.extend(decode(&buf[..n]).into_iter().map(|m| (m, from))),
                Err(_) => {
                    if !out.is_empty() || Instant::now() > deadline {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
        }
        out
    }

    /// Keeps reading until `count` messages at `addr` came in (they can
    /// arrive in separate packets, one later than the other), or two seconds
    /// passed; then a moment more, so one too many is seen too.
    fn recv_until(sock: &UdpSocket, addr: &str, count: usize) -> Vec<(OscMessage, SocketAddr)> {
        let mut out = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(2);
        let seen = |out: &Vec<(OscMessage, SocketAddr)>| out.iter().filter(|(m, _)| m.addr == addr).count();
        while Instant::now() < deadline && seen(&out) < count {
            out.extend(recv(sock));
        }
        std::thread::sleep(Duration::from_millis(30));
        out.extend(recv(sock));
        out
    }

    /// Polls until `done` holds for what came in (a busy machine can be slow
    /// to deliver), or two seconds passed.
    fn poll_until(m: &mut Monome, done: impl Fn(&[Event]) -> bool) -> Vec<Event> {
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut events = Vec::new();
        loop {
            events.extend(m.poll());
            if done(&events) || Instant::now() > deadline {
                return events;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// A fake serialosc + fake grid on localhost, talking real OSC/UDP.
    #[test]
    fn discovers_connects_reads_keys_and_sends_leds() {
        let serialosc = UdpSocket::bind("127.0.0.1:0").unwrap();
        serialosc.set_nonblocking(true).unwrap();
        let grid = UdpSocket::bind("127.0.0.1:0").unwrap();
        grid.set_nonblocking(true).unwrap();
        let grid_port = grid.local_addr().unwrap().port();

        let mut m = Monome::with_serialosc_port(serialosc.local_addr().unwrap().port()).unwrap();
        m.discover();
        let asks = recv(&serialosc);
        let list = asks.iter().find(|(msg, _)| msg.addr == "/serialosc/list").expect("list request");
        assert_eq!(list.0.str(0), Some("127.0.0.1"));
        let client = SocketAddr::from(([127, 0, 0, 1], list.0.int(1).unwrap() as u16));

        // serialosc answers with one grid.
        let dev = OscMessage::new("/serialosc/device", vec![OscArg::Str("m1000042".into()), OscArg::Str("monome 128".into()), OscArg::Int(grid_port as i32)]);
        serialosc.send_to(&dev.encode(), client).unwrap();
        let events = poll_until(&mut m, |e| e.iter().any(|e| matches!(e, Event::GridConnected { .. })));
        assert!(events.contains(&Event::Devices));
        assert!(events.iter().any(|e| matches!(e, Event::GridConnected { .. })));
        assert_eq!(m.grid_id(), Some("m1000042"));

        // The grid is told where to send and which prefix to use.
        let setup = recv(&grid);
        let addrs: Vec<&str> = setup.iter().map(|(msg, _)| msg.addr.as_str()).collect();
        assert!(addrs.contains(&"/sys/port") && addrs.contains(&"/sys/host") && addrs.contains(&"/sys/prefix"));

        // The grid reports its size and a key press.
        grid.send_to(&OscMessage::new("/sys/size", vec![OscArg::Int(16), OscArg::Int(8)]).encode(), client).unwrap();
        grid.send_to(&OscMessage::new("/krabton/grid/key", vec![OscArg::Int(3), OscArg::Int(2), OscArg::Int(1)]).encode(), client).unwrap();
        let events = poll_until(&mut m, |e| e.contains(&Event::Key { x: 3, y: 2, down: true }));
        assert!(events.contains(&Event::Key { x: 3, y: 2, down: true }), "{events:?}");
        assert_eq!(m.grid_size, (16, 8));

        // LEDs: two quads on a 16x8 grid, then only the changed quad.
        let mut levels = vec![0u8; 128];
        levels[5] = 15;
        m.grid_leds(&levels, 16, 8);
        let maps = recv_until(&grid, "/krabton/grid/led/level/map", 2);
        let maps: Vec<&OscMessage> = maps.iter().map(|(msg, _)| msg).filter(|msg| msg.addr == "/krabton/grid/led/level/map").collect();
        assert_eq!(maps.len(), 2);
        assert_eq!(maps[0].args.len(), 66);
        assert_eq!(maps[0].int(2 + 5), Some(15));
        levels[12] = 9;
        m.grid_leds(&levels, 16, 8);
        let maps = recv_until(&grid, "/krabton/grid/led/level/map", 1);
        let maps: Vec<&OscMessage> = maps.iter().map(|(msg, _)| msg).filter(|msg| msg.addr == "/krabton/grid/led/level/map").collect();
        assert_eq!(maps.len(), 1);
        assert_eq!(maps[0].int(0), Some(8), "only the right quad changed");

        // Unplugging.
        serialosc.send_to(&OscMessage::new("/serialosc/remove", vec![OscArg::Str("m1000042".into()), OscArg::Str("monome 128".into()), OscArg::Int(grid_port as i32)]).encode(), client).unwrap();
        let events = poll_until(&mut m, |e| e.contains(&Event::Disconnected { id: "m1000042".into() }));
        assert!(events.contains(&Event::Disconnected { id: "m1000042".into() }));
        assert!(m.grid_id().is_none());
    }

    #[test]
    fn rotation_maps_keys_and_leds_both_ways() {
        for r in [0, 90, 180, 270] {
            let (w, h) = (16, 8);
            let (tw, th) = if r % 180 == 90 { (h, w) } else { (w, h) };
            let mut seen = std::collections::HashSet::new();
            for y in 0..h {
                for x in 0..w {
                    let (tx, ty) = rotate(r, w, h, x, y);
                    assert!(tx < tw && ty < th, "{r}: ({x},{y}) -> ({tx},{ty})");
                    assert!(seen.insert((tx, ty)));
                    assert_eq!(unrotate(r, w, h, tx, ty), (x, y));
                }
            }
        }
        assert_eq!(rotate(180, 16, 8, 0, 0), (15, 7));
    }

    #[test]
    fn a_grid_turned_upside_down() {
        let serialosc = UdpSocket::bind("127.0.0.1:0").unwrap();
        let grid = UdpSocket::bind("127.0.0.1:0").unwrap();
        grid.set_nonblocking(true).unwrap();
        let mut m = Monome::with_serialosc_port(serialosc.local_addr().unwrap().port()).unwrap();
        m.set_rotation(180);
        let client = SocketAddr::from(([127, 0, 0, 1], m.local_port()));
        let dev = OscMessage::new("/serialosc/device", vec![OscArg::Str("m1000042".into()), OscArg::Str("monome 128".into()), OscArg::Int(grid.local_addr().unwrap().port() as i32)]);
        serialosc.send_to(&dev.encode(), client).unwrap();
        std::thread::sleep(Duration::from_millis(30));
        m.poll();
        recv(&grid);
        grid.send_to(&OscMessage::new("/sys/size", vec![OscArg::Int(16), OscArg::Int(8)]).encode(), client).unwrap();
        grid.send_to(&OscMessage::new("/krabton/grid/key", vec![OscArg::Int(0), OscArg::Int(0), OscArg::Int(1)]).encode(), client).unwrap();
        std::thread::sleep(Duration::from_millis(30));
        let events = m.poll();
        assert!(events.contains(&Event::Key { x: 15, y: 7, down: true }), "{events:?}");
        // The app's top-left LED is the device's bottom-right one.
        let mut levels = vec![0u8; 128];
        levels[0] = 15;
        m.grid_leds(&levels, 16, 8);
        let maps = recv_until(&grid, "/krabton/grid/led/level/map", 2);
        let right = maps.iter().map(|(msg, _)| msg).find(|msg| msg.addr == "/krabton/grid/led/level/map" && msg.int(0) == Some(8)).expect("right quad");
        assert_eq!(right.int(2 + 63), Some(15));
        // Turned a quarter, the grid is 8 wide and 16 high.
        m.set_rotation(90);
        assert_eq!(m.grid_dims(), (8, 16));
    }

    #[test]
    fn arc_encoders() {
        let serialosc = UdpSocket::bind("127.0.0.1:0").unwrap();
        let arc = UdpSocket::bind("127.0.0.1:0").unwrap();
        arc.set_nonblocking(true).unwrap();
        let mut m = Monome::with_serialosc_port(serialosc.local_addr().unwrap().port()).unwrap();
        let client = SocketAddr::from(([127, 0, 0, 1], m.local_port()));
        let dev = OscMessage::new("/serialosc/device", vec![OscArg::Str("m0000007".into()), OscArg::Str("monome arc 4".into()), OscArg::Int(arc.local_addr().unwrap().port() as i32)]);
        serialosc.send_to(&dev.encode(), client).unwrap();
        let events = poll_until(&mut m, |e| e.iter().any(|e| matches!(e, Event::GridConnected { .. })));
        assert!(events.contains(&Event::ArcConnected { id: "m0000007".into() }));
        arc.send_to(&OscMessage::new("/krabton_arc/enc/delta", vec![OscArg::Int(2), OscArg::Int(-3)]).encode(), client).unwrap();
        std::thread::sleep(Duration::from_millis(30));
        assert!(m.poll().contains(&Event::EncDelta { n: 2, delta: -3 }));
        let mut ring = [0u8; 64];
        ring[..10].fill(12);
        m.ring_leds(1, &ring);
        let got = recv_until(&arc, "/krabton_arc/ring/map", 1);
        let map = got.iter().find(|(msg, _)| msg.addr == "/krabton_arc/ring/map").expect("ring map");
        assert_eq!(map.0.int(0), Some(1));
        assert_eq!(map.0.args.len(), 65);
    }
}
