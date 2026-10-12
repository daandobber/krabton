//! Audio sample storage and loading (WAV and FLAC).

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Frames per entry in the waveform overview.
pub const PEAK_BLOCK: usize = 256;

pub struct SampleData {
    pub name: String,
    pub path: Option<PathBuf>,
    pub sample_rate: u32,
    pub left: Vec<f32>,
    pub right: Vec<f32>,
    /// (min, max) per `PEAK_BLOCK` frames, mono-summed, for drawing.
    pub peaks: Vec<(f32, f32)>,
}

impl SampleData {
    pub fn from_buffers(name: impl Into<String>, path: Option<PathBuf>, sample_rate: u32, left: Vec<f32>, right: Vec<f32>) -> Self {
        let peaks = compute_peaks(&left, &right);
        Self { name: name.into(), path, sample_rate: sample_rate.max(1), left, right, peaks }
    }

    pub fn silent(name: impl Into<String>, path: Option<PathBuf>) -> Self {
        Self::from_buffers(name, path, 44100, vec![0.0; 64], vec![0.0; 64])
    }

    pub fn frames(&self) -> usize {
        self.left.len()
    }

    pub fn duration(&self) -> f64 {
        self.frames() as f64 / self.sample_rate as f64
    }

    /// Loads a WAV or FLAC file (by its extension).
    pub fn load(path: &Path) -> Result<Self, String> {
        if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("flac")) {
            Self::from_flac(path)
        } else {
            Self::from_wav(path)
        }
    }

    pub fn from_flac(path: &Path) -> Result<Self, String> {
        let mut reader = claxon::FlacReader::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let info = reader.streaminfo();
        let scale = 1.0 / (1u64 << (info.bits_per_sample.max(1) - 1)) as f32;
        let interleaved: Vec<f32> = reader.samples().map(|s| s.unwrap_or(0) as f32 * scale).collect();
        Self::from_interleaved(path, info.sample_rate, info.channels.max(1) as usize, interleaved)
    }

    pub fn from_wav(path: &Path) -> Result<Self, String> {
        let mut reader = match hound::WavReader::open(path) {
            Ok(r) => r,
            // Some recorders write chunks hound does not take (a "fmt " of 20
            // bytes, say); read the plain PCM and float formats ourselves.
            Err(e) => return Self::from_wav_lenient(path).map_err(|e2| format!("{}: {e} ({e2})", path.display())),
        };
        let spec = reader.spec();
        let channels = spec.channels.max(1) as usize;
        let interleaved: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap_or(0.0)).collect(),
            hound::SampleFormat::Int => {
                let scale = 1.0 / (1u64 << (spec.bits_per_sample.max(1) - 1)) as f32;
                reader.samples::<i32>().map(|s| s.unwrap_or(0) as f32 * scale).collect()
            }
        };
        Self::from_interleaved(path, spec.sample_rate, channels, interleaved)
    }

    /// A WAV file read chunk by chunk: PCM of 8 to 32 bits, or 32-bit float,
    /// whatever the size of its "fmt " chunk.
    fn from_wav_lenient(path: &Path) -> Result<Self, String> {
        let d = std::fs::read(path).map_err(|e| e.to_string())?;
        if d.len() < 12 || &d[0..4] != b"RIFF" || &d[8..12] != b"WAVE" {
            return Err("not a WAV file".into());
        }
        let u16_at = |i: usize| u16::from_le_bytes([d[i], d[i + 1]]);
        let u32_at = |i: usize| u32::from_le_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]]);
        let (mut fmt, mut data) = (None, None);
        let mut i = 12;
        while i + 8 <= d.len() {
            let id = &d[i..i + 4];
            let n = u32_at(i + 4) as usize;
            let body = i + 8..(i + 8 + n).min(d.len());
            if id == b"fmt " && body.len() >= 16 {
                let b = body.start;
                let mut format = u16_at(b);
                if format == 0xFFFE && body.len() >= 26 {
                    // WAVE_FORMAT_EXTENSIBLE: the real format is in the sub-format.
                    format = u16_at(b + 24);
                }
                fmt = Some((format, u16_at(b + 2).max(1) as usize, u32_at(b + 4), u16_at(b + 14)));
            } else if id == b"data" {
                data = Some(body);
            }
            i += 8 + n + (n & 1);
        }
        let (format, channels, rate, bits) = fmt.ok_or("no fmt chunk")?;
        let data = &d[data.ok_or("no data chunk")?];
        let interleaved: Vec<f32> = match (format, bits) {
            (1, 8) => data.iter().map(|b| (*b as f32 - 128.0) / 128.0).collect(),
            (1, 16) => data.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0).collect(),
            (1, 24) => data.chunks_exact(3).map(|c| (i32::from_le_bytes([0, c[0], c[1], c[2]]) >> 8) as f32 / 8_388_608.0).collect(),
            (1, 32) => data.chunks_exact(4).map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f32 / 2_147_483_648.0).collect(),
            (3, 32) => data.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect(),
            _ => return Err(format!("unsupported WAV format {format} with {bits} bits")),
        };
        Self::from_interleaved(path, rate, channels, interleaved)
    }

    fn from_interleaved(path: &Path, sample_rate: u32, channels: usize, interleaved: Vec<f32>) -> Result<Self, String> {
        let frames = interleaved.len() / channels;
        let mut left = Vec::with_capacity(frames);
        let mut right = Vec::with_capacity(frames);
        for f in 0..frames {
            let l = interleaved[f * channels];
            let r = if channels > 1 { interleaved[f * channels + 1] } else { l };
            left.push(l);
            right.push(r);
        }
        if frames == 0 {
            return Err(format!("{}: empty file", path.display()));
        }
        let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "sample".into());
        Ok(Self::from_buffers(name, Some(path.to_path_buf()), sample_rate, left, right))
    }

    /// Linear-interpolated stereo read at a fractional frame position.
    #[inline]
    pub fn read(&self, pos: f64) -> (f32, f32) {
        if pos < 0.0 {
            return (0.0, 0.0);
        }
        let i = pos as usize;
        let n = self.left.len();
        if i + 1 >= n {
            if i < n {
                return (self.left[i], self.right[i]);
            }
            return (0.0, 0.0);
        }
        let f = (pos - i as f64) as f32;
        let l = self.left[i] + (self.left[i + 1] - self.left[i]) * f;
        let r = self.right[i] + (self.right[i + 1] - self.right[i]) * f;
        (l, r)
    }
}

fn compute_peaks(left: &[f32], right: &[f32]) -> Vec<(f32, f32)> {
    left.chunks(PEAK_BLOCK)
        .zip(right.chunks(PEAK_BLOCK))
        .map(|(l, r)| {
            let mut mn = 0.0f32;
            let mut mx = 0.0f32;
            for (a, b) in l.iter().zip(r) {
                let m = (a + b) * 0.5;
                mn = mn.min(m);
                mx = mx.max(m);
            }
            (mn, mx)
        })
        .collect()
}

/// Shared, immutable sample. Equality is pointer identity so that comparing
/// projects never walks audio data.
#[derive(Clone)]
pub struct SampleRef(pub Arc<SampleData>);

impl SampleRef {
    pub fn new(data: SampleData) -> Self {
        Self(Arc::new(data))
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        SampleData::load(path).map(Self::new)
    }
}

impl std::ops::Deref for SampleRef {
    type Target = SampleData;
    fn deref(&self) -> &SampleData {
        &self.0
    }
}

impl PartialEq for SampleRef {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl fmt::Debug for SampleRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sample({}, {} frames)", self.0.name, self.0.frames())
    }
}

#[derive(Serialize, Deserialize)]
struct SampleRefOnDisk {
    name: String,
    path: Option<PathBuf>,
}

thread_local! {
    static LOAD_CACHE: RefCell<Option<HashMap<PathBuf, SampleRef>>> = const { RefCell::new(None) };
    static MISSING: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// While the returned guard lives, deserializing the same sample path twice
/// shares one buffer.
pub(crate) struct LoadCacheGuard;

impl LoadCacheGuard {
    pub(crate) fn new() -> Self {
        LOAD_CACHE.with(|c| *c.borrow_mut() = Some(HashMap::new()));
        MISSING.with(|m| m.borrow_mut().clear());
        LoadCacheGuard
    }

    pub(crate) fn missing(&self) -> Vec<String> {
        MISSING.with(|m| m.borrow().clone())
    }
}

impl Drop for LoadCacheGuard {
    fn drop(&mut self) {
        LOAD_CACHE.with(|c| *c.borrow_mut() = None);
    }
}

impl Serialize for SampleRef {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        SampleRefOnDisk { name: self.0.name.clone(), path: self.0.path.clone() }.serialize(s)
    }
}

impl<'de> Deserialize<'de> for SampleRef {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let disk = SampleRefOnDisk::deserialize(d)?;
        let Some(path) = disk.path.clone() else {
            return Ok(SampleRef::new(SampleData::silent(disk.name, None)));
        };
        if let Some(hit) = LOAD_CACHE.with(|c| c.borrow().as_ref().and_then(|m| m.get(&path).cloned())) {
            return Ok(hit);
        }
        let s = match SampleRef::load(&path) {
            Ok(s) => s,
            Err(e) => {
                MISSING.with(|m| m.borrow_mut().push(e));
                SampleRef::new(SampleData::silent(disk.name, Some(path.clone())))
            }
        };
        LOAD_CACHE.with(|c| {
            if let Some(m) = c.borrow_mut().as_mut() {
                m.insert(path, s.clone());
            }
        });
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wav_with_an_unusual_fmt_chunk_still_loads() {
        // 16-bit stereo PCM whose "fmt " chunk is 20 bytes (as some recorders write).
        let samples: [i16; 4] = [16384, -16384, 8192, -8192];
        let mut d = Vec::new();
        d.extend_from_slice(b"RIFF");
        d.extend_from_slice(&(4 + 28 + 8 + 8u32).to_le_bytes());
        d.extend_from_slice(b"WAVEfmt ");
        d.extend_from_slice(&20u32.to_le_bytes());
        for v in [1u16, 2] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        d.extend_from_slice(&44100u32.to_le_bytes());
        d.extend_from_slice(&176400u32.to_le_bytes());
        for v in [4u16, 16, 0, 0] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        d.extend_from_slice(b"data");
        d.extend_from_slice(&8u32.to_le_bytes());
        for s in samples {
            d.extend_from_slice(&s.to_le_bytes());
        }
        let path = std::env::temp_dir().join("krabton_fmt20_test.wav");
        std::fs::write(&path, &d).unwrap();
        let s = SampleData::load(&path).expect("loads");
        let _ = std::fs::remove_file(&path);
        assert_eq!(s.frames(), 2);
        assert!((s.left[0] - 0.5).abs() < 1e-4 && (s.right[0] + 0.5).abs() < 1e-4);
        assert!((s.left[1] - 0.25).abs() < 1e-4);
    }
}
