//! Standard MIDI files: reading their notes (each track's, in beats) and
//! writing a clip's notes as one.

use crate::project::Note;

/// What a MIDI file holds that Krabton uses.
#[derive(Clone, Debug, Default)]
pub struct MidiSong {
    pub tracks: Vec<MidiSongTrack>,
    /// The first tempo in it (beats per minute).
    pub bpm: Option<f64>,
    /// Beats per bar from its first time signature.
    pub beats_per_bar: Option<u32>,
}

/// One track (or, in a single-track file, one channel) of a MIDI file.
#[derive(Clone, Debug, Default)]
pub struct MidiSongTrack {
    pub name: String,
    pub notes: Vec<Note>,
    /// Its notes are on channel 10, the drums.
    pub drums: bool,
    /// Its first program change: the General MIDI instrument it asks for.
    pub program: Option<u8>,
}

impl MidiSong {
    /// Where its last note ends (beats).
    pub fn length(&self) -> f64 {
        self.tracks.iter().flat_map(|t| t.notes.iter().map(|n| n.start + n.length)).fold(0.0, f64::max)
    }

    /// How long a clip of it is: whole bars of `beats_per_bar`, at least one.
    pub fn clip_length(&self, beats_per_bar: f64) -> f64 {
        let bpb = beats_per_bar.max(1.0);
        (self.length() / bpb - 1e-9).ceil().max(1.0) * bpb
    }

    /// Every track's notes together, in order of time.
    pub fn all_notes(&self) -> Vec<Note> {
        let mut v: Vec<Note> = self.tracks.iter().flat_map(|t| t.notes.iter().cloned()).collect();
        v.sort_by(|a, b| a.start.total_cmp(&b.start).then(a.pitch.cmp(&b.pitch)));
        v
    }
}

struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn u8(&mut self) -> Result<u8, String> {
        let v = *self.b.get(self.at).ok_or("the file ends too soon")?;
        self.at += 1;
        Ok(v)
    }

    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.b.len()).ok_or("the file ends too soon")?;
        let s = &self.b[self.at..end];
        self.at = end;
        Ok(s)
    }

    fn u16(&mut self) -> Result<u16, String> {
        let s = self.take(2)?;
        Ok(u16::from_be_bytes([s[0], s[1]]))
    }

    fn u32(&mut self) -> Result<u32, String> {
        let s = self.take(4)?;
        Ok(u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
    }

    /// A variable-length number (seven bits a byte, the top bit: more follow).
    fn var(&mut self) -> Result<u32, String> {
        let mut v = 0u32;
        for _ in 0..4 {
            let c = self.u8()?;
            v = (v << 7) | (c & 0x7f) as u32;
            if c & 0x80 == 0 {
                return Ok(v);
            }
        }
        Err("a number in the file is too long".into())
    }
}

/// Reads a Standard MIDI File (format 0 or 1, ticks per quarter note).
pub fn read(bytes: &[u8]) -> Result<MidiSong, String> {
    let mut r = Reader { b: bytes, at: 0 };
    if r.take(4)? != b"MThd" {
        return Err("not a MIDI file".into());
    }
    let hlen = r.u32()? as usize;
    let format = r.u16()?;
    let ntracks = r.u16()?;
    let division = r.u16()?;
    r.take(hlen.saturating_sub(6))?;
    if division & 0x8000 != 0 {
        return Err("MIDI files timed in frames (SMPTE) are not read".into());
    }
    if format > 1 {
        return Err("MIDI files of format 2 (separate songs) are not read".into());
    }
    let tpq = division.max(1) as f64;
    let mut song = MidiSong::default();
    for _ in 0..ntracks {
        // Skip chunks that are not tracks.
        let len = loop {
            if r.at >= bytes.len() {
                return Ok(finish(song));
            }
            let is_track = r.take(4)? == b"MTrk";
            let len = r.u32()? as usize;
            if is_track {
                break len;
            }
            r.take(len.min(bytes.len() - r.at))?;
        };
        // A track cut short is read as far as it goes.
        let data = r.take(len.min(bytes.len() - r.at))?;
        song.tracks.extend(read_track(data, tpq, &mut song.bpm, &mut song.beats_per_bar)?);
    }
    Ok(finish(song))
}

/// Drops empty tracks.
fn finish(mut song: MidiSong) -> MidiSong {
    song.tracks.retain(|t| !t.notes.is_empty());
    for t in &mut song.tracks {
        t.notes.sort_by(|a, b| a.start.total_cmp(&b.start).then(a.pitch.cmp(&b.pitch)));
    }
    song
}

/// One track chunk: its notes per channel (a channel each becomes a track
/// when one chunk holds several, as format 0 files do).
fn read_track(data: &[u8], tpq: f64, bpm: &mut Option<f64>, bpb: &mut Option<u32>) -> Result<Vec<MidiSongTrack>, String> {
    let mut r = Reader { b: data, at: 0 };
    let mut tick: u64 = 0;
    let mut running: u8 = 0;
    let mut name = String::new();
    // Per channel: its notes, and the notes held (pitch -> start tick, velocity).
    let mut notes: Vec<Vec<Note>> = vec![Vec::new(); 16];
    let mut held: Vec<std::collections::HashMap<u8, (u64, u8)>> = vec![Default::default(); 16];
    let mut programs = [None; 16];
    let beat = |t: u64| t as f64 / tpq;
    while r.at < data.len() {
        tick += r.var()? as u64;
        let mut status = r.u8()?;
        if status < 0x80 {
            // Running status: the byte is the first data byte.
            if running == 0 {
                return Err("a MIDI event without its status".into());
            }
            r.at -= 1;
            status = running;
        }
        match status {
            0xff => {
                let kind = r.u8()?;
                let len = r.var()? as usize;
                let body = r.take(len)?;
                match kind {
                    0x03 if name.is_empty() => name = String::from_utf8_lossy(body).trim().to_string(),
                    0x51 if body.len() == 3 && bpm.is_none() => {
                        let us = ((body[0] as u32) << 16) | ((body[1] as u32) << 8) | body[2] as u32;
                        if us > 0 {
                            *bpm = Some(60_000_000.0 / us as f64);
                        }
                    }
                    0x58 if !body.is_empty() && bpb.is_none() => *bpb = Some(body[0].max(1) as u32),
                    0x2f => break,
                    _ => {}
                }
            }
            0xf0 | 0xf7 => {
                let len = r.var()? as usize;
                r.take(len)?;
            }
            _ => {
                running = status;
                let ch = (status & 0x0f) as usize;
                match status & 0xf0 {
                    0x80 | 0x90 => {
                        let pitch = r.u8()? & 0x7f;
                        let vel = r.u8()? & 0x7f;
                        // A note-off, or a note-on at velocity 0, ends the note held.
                        if let Some((start, v)) = held[ch].remove(&pitch) {
                            let len = (beat(tick) - beat(start)).max(1.0 / 64.0);
                            notes[ch].push(Note::new(pitch, beat(start), len, v as f32 / 127.0));
                        }
                        if status & 0xf0 == 0x90 && vel > 0 {
                            held[ch].insert(pitch, (tick, vel));
                        }
                    }
                    0xa0 | 0xb0 | 0xe0 => {
                        r.take(2)?;
                    }
                    0xc0 => {
                        let p = r.u8()? & 0x7f;
                        programs[ch].get_or_insert(p);
                    }
                    0xd0 => {
                        r.take(1)?;
                    }
                    _ => return Err(format!("an unknown MIDI event ({status:#x})")),
                }
            }
        }
    }
    // Notes still held at the end end there.
    for (ch, h) in held.into_iter().enumerate() {
        for (pitch, (start, v)) in h {
            notes[ch].push(Note::new(pitch, beat(start), (beat(tick) - beat(start)).max(0.25), v as f32 / 127.0));
        }
    }
    let used: Vec<usize> = (0..16).filter(|&c| !notes[c].is_empty()).collect();
    Ok(used
        .iter()
        .map(|&c| {
            let label = match (name.is_empty(), used.len() > 1) {
                (false, false) => name.clone(),
                (false, true) => format!("{name} {}", c + 1),
                (true, _) if c == 9 => "Drums".to_string(),
                (true, _) => format!("Channel {}", c + 1),
            };
            MidiSongTrack { name: label, notes: std::mem::take(&mut notes[c]), drums: c == 9, program: programs[c] }
        })
        .collect())
}

/// Writes notes as a single-track Standard MIDI File (format 0, 480 ticks
/// a beat), named `name`, at `bpm`, `length` beats long.
pub fn write(notes: &[Note], length: f64, name: &str, bpm: f64) -> Vec<u8> {
    const TPQ: f64 = 480.0;
    let tick = |b: f64| (b.max(0.0) * TPQ).round() as u64;
    // (tick, order, bytes): note-offs before note-ons at the same tick.
    let mut events: Vec<(u64, u8, Vec<u8>)> = Vec::new();
    let mut meta = vec![0xff, 0x03];
    push_var(&mut meta, name.len() as u32);
    meta.extend_from_slice(name.as_bytes());
    events.push((0, 0, meta));
    let us = (60_000_000.0 / bpm.max(1.0)).round() as u32;
    events.push((0, 0, vec![0xff, 0x51, 0x03, (us >> 16) as u8, (us >> 8) as u8, us as u8]));
    for n in notes {
        let vel = (n.velocity * 127.0).round().clamp(1.0, 127.0) as u8;
        events.push((tick(n.start), 2, vec![0x90, n.pitch & 0x7f, vel]));
        events.push((tick(n.start + n.length).max(tick(n.start) + 1), 1, vec![0x80, n.pitch & 0x7f, 0]));
    }
    events.push((tick(length).max(events.iter().map(|e| e.0).max().unwrap_or(0)), 3, vec![0xff, 0x2f, 0x00]));
    events.sort_by_key(|e| (e.0, e.1));
    let mut track = Vec::new();
    let mut last = 0u64;
    for (t, _, bytes) in events {
        push_var(&mut track, (t - last) as u32);
        track.extend_from_slice(&bytes);
        last = t;
    }
    let mut out = Vec::with_capacity(track.len() + 22);
    out.extend_from_slice(b"MThd");
    out.extend_from_slice(&6u32.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&(TPQ as u16).to_be_bytes());
    out.extend_from_slice(b"MTrk");
    out.extend_from_slice(&(track.len() as u32).to_be_bytes());
    out.extend_from_slice(&track);
    out
}

fn push_var(out: &mut Vec<u8>, mut v: u32) {
    let mut bytes = vec![(v & 0x7f) as u8];
    v >>= 7;
    while v > 0 {
        bytes.push(((v & 0x7f) as u8) | 0x80);
        v >>= 7;
    }
    bytes.reverse();
    out.extend_from_slice(&bytes);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_written_are_read_back() {
        let notes = vec![Note::new(60, 0.0, 1.0, 0.8), Note::new(64, 0.5, 0.25, 0.5), Note::new(67, 3.0, 1.0, 1.0)];
        let bytes = write(&notes, 4.0, "Chords", 100.0);
        let song = read(&bytes).expect("reads");
        assert_eq!(song.tracks.len(), 1);
        assert_eq!(song.tracks[0].name, "Chords");
        assert!((song.bpm.unwrap() - 100.0).abs() < 0.01);
        let back = &song.tracks[0].notes;
        assert_eq!(back.len(), 3);
        for (a, b) in notes.iter().zip(back) {
            assert_eq!(a.pitch, b.pitch);
            assert!((a.start - b.start).abs() < 1e-3 && (a.length - b.length).abs() < 1e-3);
            assert!((a.velocity - b.velocity).abs() < 0.01);
        }
    }

    #[test]
    fn a_format_0_file_splits_into_its_channels_with_running_status() {
        // Hand-made: 96 ticks a beat; a C on channel 1 and a kick on 10,
        // the second note-on in running status, a note-on at velocity 0 as its off.
        let mut trk = vec![0x00, 0x90, 60, 100, 0x00, 62, 90, 0x60, 0x80, 60, 0, 0x00, 0x90, 62, 0, 0x00, 0x99, 36, 120, 0x30, 0x89, 36, 0, 0x00, 0xff, 0x2f, 0x00];
        let mut f = b"MThd".to_vec();
        f.extend_from_slice(&6u32.to_be_bytes());
        f.extend_from_slice(&[0, 0, 0, 1, 0, 96]);
        f.extend_from_slice(b"MTrk");
        f.extend_from_slice(&(trk.len() as u32).to_be_bytes());
        f.append(&mut trk);
        let song = read(&f).expect("reads");
        assert_eq!(song.tracks.len(), 2);
        assert_eq!(song.tracks[0].notes.len(), 2, "two notes on channel 1");
        assert!((song.tracks[0].notes[0].length - 1.0).abs() < 1e-9);
        assert!(song.tracks[1].drums);
        assert!((song.tracks[1].notes[0].start - 1.0).abs() < 1e-9 && (song.tracks[1].notes[0].length - 0.5).abs() < 1e-9);
    }

    #[test]
    fn programs_are_kept_and_clips_last_whole_bars() {
        // A program change to 33 (fingered bass), then a note of five beats.
        let mut trk = vec![0x00, 0xc0, 33, 0x00, 0x90, 40, 100, 0x83, 0x60, 0x80, 40, 0, 0x00, 0xff, 0x2f, 0x00];
        let mut f = b"MThd".to_vec();
        f.extend_from_slice(&6u32.to_be_bytes());
        f.extend_from_slice(&[0, 0, 0, 1, 0, 96]);
        f.extend_from_slice(b"MTrk");
        f.extend_from_slice(&(trk.len() as u32).to_be_bytes());
        f.append(&mut trk);
        let song = read(&f).expect("reads");
        assert_eq!(song.tracks[0].program, Some(33));
        assert!((song.length() - 5.0).abs() < 1e-9);
        assert_eq!(song.clip_length(4.0), 8.0);
        assert_eq!(song.clip_length(3.0), 6.0);
        let empty = MidiSong::default();
        assert_eq!(empty.clip_length(4.0), 4.0);
    }

    #[test]
    fn rubbish_is_refused() {
        assert!(read(b"RIFF....").is_err());
        assert!(read(b"MThd").is_err());
    }
}
