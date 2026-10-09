//! Finds grids on serial ports and asks them for their id and size, the way
//! Krabton does when serialosc does not see a grid (read-only: no LEDs).
//!
//! ```sh
//! cargo run -p krabton-monome --example serial_probe
//! ```

use krabton_monome::serial::SerialGrid;
use std::time::{Duration, Instant};

fn main() {
    for p in serialport::available_ports().unwrap_or_default() {
        println!("{} {:?}", p.port_name, p.port_type);
    }
    for (port, serial) in SerialGrid::find() {
        println!("grid on {port} ({serial})");
        let mut g = match SerialGrid::open(&port, &serial) {
            Ok(g) => g,
            Err(e) => {
                println!("  cannot open: {e}");
                continue;
            }
        };
        let end = Instant::now() + Duration::from_millis(800);
        while Instant::now() < end {
            match g.poll() {
                Ok(msgs) => msgs.iter().for_each(|m| println!("  {m:?}")),
                Err(e) => println!("  error: {e}"),
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        println!("  id {} size {:?}", g.id, g.size);
    }
}
