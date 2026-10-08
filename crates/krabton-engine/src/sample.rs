//! Audio sample storage and WAV loading.

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

    pub fn from_wav(path: &Path) -> Result<Self, String> {
        let mut reader = hound::WavReader::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let spec = reader.spec();
        let channels = spec.channels.max(1) as usize;
        let interleaved: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap_or(0.0)).collect(),
            hound::SampleFormat::Int => {
                let scale = 1.0 / (1u64 << (spec.bits_per_sample.max(1) - 1)) as f32;
                reader.samples::<i32>().map(|s| s.unwrap_or(0) as f32 * scale).collect()
            }
        };
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
        Ok(Self::from_buffers(name, Some(path.to_path_buf()), spec.sample_rate, left, right))
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
        SampleData::from_wav(path).map(Self::new)
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
