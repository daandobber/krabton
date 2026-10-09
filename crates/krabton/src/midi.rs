//! MIDI keyboard input via midir. Messages are forwarded to the UI thread,
//! which routes them to the armed/selected track.

use crossbeam_channel::{unbounded, Receiver, Sender};
use midir::{Ignore, MidiInput, MidiInputConnection};

#[derive(Clone, Copy, Debug)]
pub enum MidiMsg {
    NoteOn { pitch: u8, velocity: f32, channel: u8 },
    NoteOff { pitch: u8 },
}

pub struct Midi {
    pub ports: Vec<String>,
    pub connected: Option<String>,
    conn: Option<MidiInputConnection<Sender<MidiMsg>>>,
    tx: Sender<MidiMsg>,
    pub rx: Receiver<MidiMsg>,
}

impl Midi {
    pub fn new() -> Self {
        let (tx, rx) = unbounded();
        let mut m = Self { ports: Vec::new(), connected: None, conn: None, tx, rx };
        m.refresh();
        m
    }

    pub fn refresh(&mut self) {
        self.ports = MidiInput::new("krabton-scan")
            .map(|input| input.ports().iter().filter_map(|p| input.port_name(p).ok()).collect())
            .unwrap_or_default();
    }

    pub fn disconnect(&mut self) {
        if let Some(c) = self.conn.take() {
            c.close();
        }
        self.connected = None;
    }

    pub fn connect(&mut self, name: &str) -> Result<(), String> {
        self.disconnect();
        let mut input = MidiInput::new("krabton").map_err(|e| e.to_string())?;
        input.ignore(Ignore::All);
        let ports = input.ports();
        let port = ports
            .iter()
            .find(|p| input.port_name(p).map(|n| n == name).unwrap_or(false))
            .ok_or_else(|| format!("MIDI port '{name}' not found"))?
            .clone();
        let conn = input
            .connect(
                &port,
                "krabton-in",
                |_stamp, msg, tx: &mut Sender<MidiMsg>| {
                    if msg.len() < 3 {
                        return;
                    }
                    let (status, channel, d1, d2) = (msg[0] & 0xf0, msg[0] & 0x0f, msg[1], msg[2]);
                    let m = match status {
                        0x90 if d2 > 0 => MidiMsg::NoteOn { pitch: d1, velocity: d2 as f32 / 127.0, channel },
                        0x90 | 0x80 => MidiMsg::NoteOff { pitch: d1 },
                        _ => return,
                    };
                    let _ = tx.send(m);
                },
                self.tx.clone(),
            )
            .map_err(|e| e.to_string())?;
        self.conn = Some(conn);
        self.connected = Some(name.to_string());
        Ok(())
    }

    /// Connects to the first available port, if any.
    pub fn auto_connect(&mut self) {
        if let Some(first) = self.ports.first().cloned() {
            let _ = self.connect(&first);
        }
    }
}
