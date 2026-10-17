//! The grand piano's samples: the Salamander Grand Piano V3 by Alexander
//! Holm, a Yamaha C5 recorded at 48 kHz in 16 velocity layers (Creative
//! Commons Attribution 3.0). Krabton downloads the FLAC files once and
//! decodes them into one file of 16-bit stereo frames, `samples.bin`, with
//! an `index.json`. That file is memory-mapped: the 880 MB stay on disk and
//! the system keeps what is played in its cache. A helper thread reads ahead
//! of every note that starts, so the audio thread finds it in memory.

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The keys the notes were recorded on (minor thirds from A0), with the
/// keys each one plays.
pub const REGIONS: [(u8, u8, u8); 30] = [
    (21, 21, 22),
    (24, 23, 25),
    (27, 26, 28),
    (30, 29, 31),
    (33, 32, 34),
    (36, 35, 37),
    (39, 38, 40),
    (42, 41, 43),
    (45, 44, 46),
    (48, 47, 49),
    (51, 50, 52),
    (54, 53, 55),
    (57, 56, 58),
    (60, 59, 61),
    (63, 62, 64),
    (66, 65, 67),
    (69, 68, 70),
    (72, 71, 73),
    (75, 74, 76),
    (78, 77, 79),
    (81, 80, 82),
    (84, 83, 85),
    (87, 86, 88),
    (90, 89, 91),
    (93, 92, 94),
    (96, 95, 97),
    (99, 98, 100),
    (102, 101, 103),
    (105, 104, 106),
    (108, 107, 108),
];
/// The lowest MIDI velocity of each of the 16 layers.
pub const LAYER_LOVEL: [u8; 16] = [1, 27, 35, 37, 44, 47, 51, 57, 65, 73, 81, 89, 97, 105, 113, 121];
/// Corrections (cents) of the "Retuned" version by Markus Fiedler.
pub const RETUNE: [i8; 30] = [10, 13, 11, -3, -9, -9, -11, -7, -4, 0, -6, -3, -3, -6, -3, 0, -4, -8, -8, -5, -7, -8, -12, -13, -12, -17, -17, -27, -38, -38];

/// Where the hammer hits in every sample (frames at 48 kHz), by layer and
/// region, from the SFZ edition: before it the key travels down and its
/// felt rubs, a quiet scraping that the player skips by default.
pub const OFFSETS: [[u16; 30]; 16] = [
    [481, 1388, 1059, 971, 356, 682, 711, 995, 894, 918, 883, 698, 714, 695, 662, 515, 609, 569, 638, 610, 599, 1071, 633, 674, 760, 556, 528, 665, 1058, 650],
    [866, 2717, 1096, 940, 752, 812, 545, 863, 554, 874, 714, 656, 607, 708, 558, 581, 625, 677, 689, 586, 629, 1572, 778, 636, 685, 763, 687, 492, 723, 674],
    [1148, 1288, 1123, 1162, 479, 779, 686, 1000, 675, 1009, 785, 647, 596, 678, 620, 658, 576, 606, 577, 659, 642, 1364, 636, 693, 646, 700, 665, 635, 690, 606],
    [1026, 2272, 1369, 1357, 462, 945, 595, 808, 577, 693, 816, 818, 546, 709, 731, 525, 663, 587, 719, 692, 1122, 777, 594, 662, 619, 548, 694, 679, 665, 655],
    [1118, 1994, 1109, 1240, 509, 738, 465, 831, 723, 915, 719, 1016, 745, 562, 784, 587, 685, 595, 730, 636, 783, 624, 595, 712, 573, 524, 573, 639, 684, 659],
    [1678, 1928, 1344, 1233, 757, 677, 566, 921, 803, 971, 746, 867, 661, 786, 670, 669, 645, 676, 745, 664, 1065, 758, 604, 647, 668, 709, 461, 721, 632, 663],
    [866, 1825, 1233, 1246, 774, 813, 554, 709, 748, 895, 765, 767, 586, 798, 695, 687, 549, 668, 667, 690, 403, 598, 596, 633, 719, 595, 576, 686, 654, 608],
    [1206, 1771, 1268, 1281, 673, 761, 670, 618, 757, 796, 781, 814, 705, 723, 687, 725, 612, 498, 754, 712, 532, 780, 672, 669, 632, 608, 750, 727, 694, 708],
    [1191, 1794, 1237, 1390, 453, 751, 618, 832, 799, 935, 806, 815, 690, 754, 616, 732, 669, 677, 744, 733, 395, 524, 572, 625, 657, 631, 644, 569, 717, 579],
    [1788, 1382, 1152, 1008, 797, 811, 663, 813, 675, 775, 624, 658, 660, 822, 670, 571, 593, 702, 682, 680, 593, 542, 528, 773, 549, 616, 770, 591, 690, 632],
    [1231, 2351, 1221, 1315, 738, 813, 665, 712, 784, 857, 733, 748, 696, 671, 702, 619, 627, 815, 729, 754, 422, 644, 591, 567, 670, 616, 781, 838, 682, 616],
    [1127, 2135, 1151, 1248, 717, 648, 719, 812, 757, 856, 720, 743, 727, 691, 573, 654, 537, 814, 686, 709, 636, 614, 663, 632, 592, 655, 598, 650, 707, 579],
    [1861, 2402, 1320, 1187, 615, 716, 787, 931, 761, 849, 785, 752, 800, 783, 628, 474, 651, 600, 648, 644, 913, 635, 586, 573, 632, 722, 553, 599, 601, 661],
    [1361, 2418, 1264, 1217, 800, 630, 818, 1045, 838, 1041, 853, 820, 802, 662, 657, 570, 730, 732, 648, 767, 729, 524, 646, 601, 737, 597, 613, 634, 697, 596],
    [870, 2168, 1287, 1439, 743, 661, 793, 1067, 731, 908, 801, 638, 689, 796, 644, 643, 592, 720, 707, 695, 394, 636, 594, 578, 636, 572, 524, 534, 688, 679],
    [1754, 2163, 1318, 1134, 699, 820, 788, 975, 917, 935, 870, 879, 839, 727, 690, 602, 664, 720, 707, 693, 1522, 626, 621, 620, 635, 554, 524, 771, 688, 716],
];

/// The folder the library lives in once installed.
pub const FOLDER: &str = "Salamander Grand Piano";
pub const CREDIT: &str = "Salamander Grand Piano V3 by Alexander Holm, CC BY 3.0";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Entry {
    pub name: String,
    /// First frame in `samples.bin`.
    pub offset: u64,
    pub frames: u32,
}

#[derive(Serialize, Deserialize)]
struct Index {
    version: u32,
    sample_rate: u32,
    samples: Vec<Entry>,
}

const MISSING: u32 = u32::MAX;

pub struct PianoLibrary {
    map: memmap2::Mmap,
    pub sample_rate: u32,
    entries: Vec<Entry>,
    /// Sample of each region and layer.
    pub notes: [[u32; 16]; 30],
    /// Key-release (hammer and damper) noise of each key, A0 to C8.
    pub rel: [u32; 88],
    /// String resonance on release by region: loud, soft and always.
    pub harm: [[u32; 30]; 3],
    pub pedal_down: [u32; 2],
    pub pedal_up: [u32; 2],
    prefetch: crossbeam_channel::Sender<u32>,
    pub dir: PathBuf,
}

/// The position (0..30) of a sample note name like "D#3".
fn region_of(name: &str) -> Option<usize> {
    let (letter, octave) = name.split_at(name.len().checked_sub(1)?);
    let octave: i32 = octave.parse().ok()?;
    let pc = match letter {
        "C" => 0,
        "D#" => 3,
        "F#" => 6,
        "A" => 9,
        _ => return None,
    };
    let key = (octave + 1) * 12 + pc;
    REGIONS.iter().position(|r| r.0 as i32 == key)
}

impl PianoLibrary {
    /// Opens an installed library (`index.json` and `samples.bin` in `dir`).
    pub fn open(dir: &Path) -> Result<Arc<Self>, String> {
        let index: Index = serde_json::from_slice(&std::fs::read(dir.join("index.json")).map_err(|e| format!("{}: {e}", dir.display()))?).map_err(|e| e.to_string())?;
        let file = File::open(dir.join("samples.bin")).map_err(|e| e.to_string())?;
        // SAFETY: the file is Krabton's own and is not changed while mapped.
        let map = unsafe { memmap2::Mmap::map(&file) }.map_err(|e| e.to_string())?;
        let mut lib = Self {
            map,
            sample_rate: index.sample_rate,
            entries: index.samples,
            notes: [[MISSING; 16]; 30],
            rel: [MISSING; 88],
            harm: [[MISSING; 30]; 3],
            pedal_down: [MISSING; 2],
            pedal_up: [MISSING; 2],
            prefetch: crossbeam_channel::bounded(1).0,
            dir: dir.to_path_buf(),
        };
        let total = lib.map.len() as u64 / 4;
        for (i, e) in lib.entries.iter().enumerate() {
            if e.offset + e.frames as u64 > total {
                return Err(format!("{}: {} lies outside samples.bin", dir.display(), e.name));
            }
            let i = i as u32;
            let n = e.name.as_str();
            if let Some(rest) = n.strip_prefix("rel") {
                if let Ok(k) = rest.parse::<usize>() {
                    if (1..=88).contains(&k) {
                        lib.rel[k - 1] = i;
                    }
                }
            } else if let Some(rest) = n.strip_prefix("harmV3") {
                if let Some(r) = region_of(rest) {
                    lib.harm[2][r] = i;
                }
            } else if let Some(rest) = n.strip_prefix("harmL") {
                if let Some(r) = region_of(rest) {
                    lib.harm[0][r] = i;
                }
            } else if let Some(rest) = n.strip_prefix("harmS") {
                if let Some(r) = region_of(rest) {
                    lib.harm[1][r] = i;
                }
            } else if let Some(rest) = n.strip_prefix("pedalD") {
                if let Ok(k) = rest.parse::<usize>() {
                    lib.pedal_down[(k.max(1) - 1).min(1)] = i;
                }
            } else if let Some(rest) = n.strip_prefix("pedalU") {
                if let Ok(k) = rest.parse::<usize>() {
                    lib.pedal_up[(k.max(1) - 1).min(1)] = i;
                }
            } else if let Some((note, layer)) = n.split_once('v') {
                if let (Some(r), Ok(l)) = (region_of(note), layer.parse::<usize>()) {
                    if (1..=16).contains(&l) {
                        lib.notes[r][l - 1] = i;
                    }
                }
            }
        }
        // A layer that is missing borrows its neighbour.
        for r in 0..30 {
            for l in 0..16 {
                if lib.notes[r][l] == MISSING {
                    lib.notes[r][l] = (0..16).map(|d| [l.saturating_sub(d), (l + d).min(15)]).flatten().map(|k| lib.notes[r][k]).find(|&s| s != MISSING).unwrap_or(MISSING);
                }
            }
        }
        if lib.notes.iter().flatten().any(|&s| s == MISSING) {
            return Err(format!("{}: notes are missing", dir.display()));
        }
        // The read-ahead helper maps the file again and pages in what notes will need.
        let (tx, rx) = crossbeam_channel::bounded::<u32>(1024);
        lib.prefetch = tx;
        let spans: Vec<(u64, u32, bool)> = lib.entries.iter().map(|e| (e.offset, e.frames, e.name.contains('v') && !e.name.starts_with("harm"))).collect();
        let path = dir.join("samples.bin");
        std::thread::Builder::new()
            .name("piano read-ahead".into())
            .spawn(move || {
                let Ok(file) = File::open(&path) else { return };
                // SAFETY: as above.
                let Ok(map) = (unsafe { memmap2::Mmap::map(&file) }) else { return };
                let touch = |from: u64, frames: u64| {
                    let (a, b) = ((from * 4) as usize, ((from + frames) * 4).min(map.len() as u64) as usize);
                    let mut sum = 0u8;
                    let mut i = a;
                    while i < b {
                        // SAFETY: inside the mapping.
                        sum = sum.wrapping_add(unsafe { std::ptr::read_volatile(map.as_ptr().add(i)) });
                        i += 4096;
                    }
                    std::hint::black_box(sum);
                };
                // First the start of every note (and all of the short noises), then on request.
                for &(off, frames, note) in &spans {
                    touch(off, if note { (frames as u64).min(48000 / 2) } else { frames as u64 });
                }
                while let Ok(s) = rx.recv() {
                    if let Some(&(off, frames, _)) = spans.get(s as usize) {
                        touch(off, frames as u64);
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Arc::new(lib))
    }

    /// The interleaved stereo frames of sample `s`.
    pub fn frames(&self, s: u32) -> &[i16] {
        let Some(e) = self.entries.get(s as usize) else { return &[] };
        let bytes = &self.map[(e.offset * 4) as usize..((e.offset + e.frames as u64) * 4) as usize];
        // SAFETY: the map is page aligned and every sample starts on a frame (4 bytes).
        unsafe { std::slice::from_raw_parts(bytes.as_ptr() as *const i16, bytes.len() / 2) }
    }

    pub fn exists(&self, s: u32) -> bool {
        (s as usize) < self.entries.len()
    }

    /// Asks the helper to read the rest of sample `s` into memory (never waits).
    pub fn prefetch(&self, s: u32) {
        let _ = self.prefetch.try_send(s);
    }
}

static SHARED: Mutex<Option<Arc<PianoLibrary>>> = Mutex::new(None);

/// The library new engines (and offline renders) play from.
pub fn shared() -> Option<Arc<PianoLibrary>> {
    SHARED.lock().ok().and_then(|g| g.clone())
}

pub fn set_shared(lib: Option<Arc<PianoLibrary>>) {
    if let Ok(mut g) = SHARED.lock() {
        *g = lib;
    }
}

/// Whether `dir` holds an installed library.
pub fn installed(dir: &Path) -> bool {
    dir.join("index.json").is_file() && dir.join("samples.bin").is_file()
}

/// Decodes the FLAC files in `flac_dir` into `out_dir` (`samples.bin` and
/// `index.json`): 24-bit to 16-bit with a little dither, tails below -72 dB
/// cut off with a short fade. `progress(done, total)` is called per file.
pub fn build(flac_dir: &Path, out_dir: &Path, progress: &dyn Fn(usize, usize)) -> Result<(), String> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(flac_dir)
        .map_err(|e| format!("{}: {e}", flac_dir.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("flac")))
        .collect();
    files.sort();
    if files.is_empty() {
        return Err("no FLAC files to build the piano from".into());
    }
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let tmp = out_dir.join("samples.bin.part");
    let mut out = BufWriter::new(File::create(&tmp).map_err(|e| e.to_string())?);
    let mut entries = Vec::with_capacity(files.len());
    let mut offset = 0u64;
    let mut rate = 48000;
    let mut rng = 0x9E37_79B9u32;
    let mut dither = move || {
        rng ^= rng << 13;
        rng ^= rng >> 17;
        rng ^= rng << 5;
        (rng >> 8) as f32 / 16_777_216.0
    };
    for (n, path) in files.iter().enumerate() {
        progress(n, files.len());
        let mut reader = claxon::FlacReader::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let info = reader.streaminfo();
        rate = info.sample_rate;
        let channels = info.channels.max(1) as usize;
        let scale = 32768.0 / (1u64 << (info.bits_per_sample.max(1) - 1)) as f32;
        let raw: Vec<i32> = reader.samples().collect::<Result<_, _>>().map_err(|e| format!("{}: {e}", path.display()))?;
        let frames = raw.len() / channels;
        let at = |i: usize, c: usize| raw[i * channels + c.min(channels - 1)] as f32 * scale;
        // Where it has died away to -72 dB of its peak.
        let peak = (0..frames).map(|i| at(i, 0).abs().max(at(i, 1).abs())).fold(0.0f32, f32::max).max(1.0);
        let floor = peak * 10f32.powf(-72.0 / 20.0);
        let last = (0..frames).rev().find(|&i| at(i, 0).abs().max(at(i, 1).abs()) > floor).unwrap_or(0);
        let keep = (last + rate as usize / 4).min(frames);
        let fade = (rate as usize / 20).min(keep);
        let mut bytes = Vec::with_capacity(keep * 4);
        for i in 0..keep {
            let g = if i + fade >= keep && keep < frames { (keep - i) as f32 / fade as f32 } else { 1.0 };
            for c in 0..2 {
                let v = (at(i, c) * g + dither() - dither()).round().clamp(-32768.0, 32767.0) as i16;
                bytes.extend_from_slice(&v.to_le_bytes());
            }
        }
        out.write_all(&bytes).map_err(|e| e.to_string())?;
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        entries.push(Entry { name, offset, frames: keep as u32 });
        offset += keep as u64;
    }
    progress(files.len(), files.len());
    out.flush().map_err(|e| e.to_string())?;
    drop(out);
    let index = Index { version: 1, sample_rate: rate, samples: entries };
    std::fs::write(out_dir.join("index.json"), serde_json::to_vec(&index).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, out_dir.join("samples.bin")).map_err(|e| e.to_string())?;
    Ok(())
}

/// Writes a small library of sine "notes" for tests (every region and
/// layer, louder for higher layers), with release and pedal noises.
#[cfg(test)]
pub fn write_test_library(dir: &Path, rate: u32) {
    std::fs::create_dir_all(dir).unwrap();
    let mut out = Vec::new();
    let mut entries = Vec::new();
    let mut offset = 0u64;
    let mut push = |name: String, freq: f32, amp: f32, secs: f32, out: &mut Vec<u8>| {
        let n = (secs * rate as f32) as usize;
        for i in 0..n {
            let env = (-(i as f32) / (rate as f32 * secs * 0.4)).exp();
            let v = ((std::f32::consts::TAU * freq * i as f32 / rate as f32).sin() * amp * env * 32767.0) as i16;
            out.extend_from_slice(&v.to_le_bytes());
            out.extend_from_slice(&v.to_le_bytes());
        }
        entries.push(Entry { name, offset, frames: n as u32 });
        offset += n as u64;
    };
    let names = ["A", "C", "D#", "F#"];
    for (r, reg) in REGIONS.iter().enumerate() {
        let key = reg.0 as i32;
        let name = format!("{}{}", names[r % 4], key / 12 - 1);
        let freq = 440.0 * 2f32.powf((key as f32 - 69.0) / 12.0);
        for l in 1..=16 {
            push(format!("{name}v{l}"), freq, 0.1 + 0.05 * l as f32, 1.0, &mut out);
        }
        if r < 23 {
            push(format!("harmL{name}"), freq, 0.05, 0.3, &mut out);
            push(format!("harmS{name}"), freq, 0.05, 0.3, &mut out);
            push(format!("harmV3{name}"), freq, 0.05, 0.3, &mut out);
        }
    }
    for k in 1..=88 {
        push(format!("rel{k}"), 3000.0, 0.2, 0.1, &mut out);
    }
    for k in 1..=2 {
        push(format!("pedalD{k}"), 200.0, 0.3, 0.2, &mut out);
        push(format!("pedalU{k}"), 300.0, 0.3, 0.2, &mut out);
    }
    std::fs::write(dir.join("samples.bin"), &out).unwrap();
    let index = Index { version: 1, sample_rate: rate, samples: entries };
    std::fs::write(dir.join("index.json"), serde_json::to_vec(&index).unwrap()).unwrap();
}
