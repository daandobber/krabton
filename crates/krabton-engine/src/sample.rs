//! Audio sample storage and loading (WAV and FLAC).

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

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
    /// Its transients and tempo, found once (see [`SampleData::analysis`]).
    analysis: OnceLock<crate::analysis::Analysis>,
    /// The sample backwards, made once when a clip plays it reversed.
    reversed: OnceLock<Arc<SampleData>>,
}

impl SampleData {
    pub fn from_buffers(name: impl Into<String>, path: Option<PathBuf>, sample_rate: u32, left: Vec<f32>, right: Vec<f32>) -> Self {
        let peaks = compute_peaks(&left, &right);
        Self { name: name.into(), path, sample_rate: sample_rate.max(1), left, right, peaks, analysis: OnceLock::new(), reversed: OnceLock::new() }
    }

    /// Its transients and tempo: found on the first call (which takes a
    /// moment for a whole song, so never on the audio thread), kept after.
    pub fn analysis(&self) -> &crate::analysis::Analysis {
        self.analysis.get_or_init(|| crate::analysis::analyze(self))
    }

    /// The analysis if it was made already: what the audio thread reads.
    pub fn analysis_ready(&self) -> Option<&crate::analysis::Analysis> {
        self.analysis.get()
    }

    /// The sample backwards (made on the first call, with its analysis).
    pub fn reversed(&self) -> &SampleData {
        self.reversed.get_or_init(|| {
            let (mut l, mut r) = (self.left.clone(), self.right.clone());
            l.reverse();
            r.reverse();
            let rev = SampleData::from_buffers(self.name.clone(), self.path.clone(), self.sample_rate, l, r);
            rev.analysis();
            Arc::new(rev)
        })
    }

    /// The reversed sample if it was made already.
    pub fn reversed_ready(&self) -> Option<&SampleData> {
        self.reversed.get().map(|a| &**a)
    }

    /// The loudest sample, either side.
    pub fn peak(&self) -> f32 {
        self.left.iter().chain(&self.right).fold(0.0f32, |m, x| m.max(x.abs()))
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

    /// Loads an audio file: WAV, FLAC, MP3 or AIFF, known by what is in it
    /// (a file whose extension lies still loads), else by its extension.
    /// Other formats go to [`decode_with_external`].
    pub fn load(path: &Path) -> Result<Self, String> {
        match AudioFormat::of(path) {
            AudioFormat::Wav => Self::from_wav(path),
            AudioFormat::Flac => Self::from_flac(path),
            AudioFormat::Mp3 => Self::from_mp3(path),
            AudioFormat::Aiff => Self::from_aiff(path),
            AudioFormat::Other => decode_with_external(path),
        }
    }

    pub fn from_mp3(path: &Path) -> Result<Self, String> {
        let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::from_mp3_bytes(path, &data)
    }

    fn from_mp3_bytes(path: &Path, data: &[u8]) -> Result<Self, String> {
        // The decoder checks what it reads, but a damaged file must never
        // take the program down: a panic in it is a file that does not load.
        let decoded = std::panic::catch_unwind(|| crate::mp3_decode::decode(data)).unwrap_or_else(|_| Err("the MP3 data is damaged".into()));
        let a = decoded.map_err(|e| format!("{}: {e}", path.display()))?;
        let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "sample".into());
        Ok(Self::from_buffers(name, Some(path.to_path_buf()), a.sample_rate, a.left, a.right))
    }

    /// An AIFF or AIFF-C file: big-endian PCM of 8 to 32 bits, and in AIFF-C
    /// also little-endian PCM ('sowt') and 32- or 64-bit float.
    pub fn from_aiff(path: &Path) -> Result<Self, String> {
        let d = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let (rate, channels, interleaved) = parse_aiff(&d).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::from_interleaved(path, rate, channels, interleaved)
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
            (3, 64) => data.chunks_exact(8).map(|c| f64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]) as f32).collect(),
            // MPEG audio inside a WAV (as some old tools wrote it).
            (0x50 | 0x55, _) => return Self::from_mp3_bytes(path, data),
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

/// The audio files Krabton reads itself, by extension (lower case). The
/// browser lists them, file dialogs offer them and drops take them.
pub const AUDIO_EXTENSIONS: &[&str] = &["wav", "wave", "flac", "mp3", "aif", "aiff", "aifc"];

/// Audio files that need ffmpeg (see [`decode_with_external`]).
pub const EXTERNAL_AUDIO_EXTENSIONS: &[&str] = &["ogg", "oga", "opus", "m4a", "mp4", "aac", "wma", "webm", "mka", "ac3", "ape", "wv", "caf", "mp2", "amr"];

/// Whether Krabton reads this file itself (by its extension).
pub fn is_audio_file(path: &Path) -> bool {
    has_extension(path, AUDIO_EXTENSIONS)
}

/// Whether this is a sound file of a kind ffmpeg would have to read.
pub fn is_external_audio_file(path: &Path) -> bool {
    has_extension(path, EXTERNAL_AUDIO_EXTENSIONS)
}

fn has_extension(path: &Path, list: &[&str]) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| list.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

/// Which decoder a file goes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioFormat {
    Wav,
    Flac,
    Mp3,
    Aiff,
    /// Something else: ffmpeg's to read.
    Other,
}

impl AudioFormat {
    /// By the file's first bytes, or its extension when they say nothing.
    pub fn of(path: &Path) -> Self {
        let mut head = vec![0u8; 64 * 1024];
        let n = std::fs::File::open(path)
            .and_then(|mut f| {
                use std::io::Read;
                let mut n = 0;
                while n < head.len() {
                    match f.read(&mut head[n..])? {
                        0 => break,
                        k => n += k,
                    }
                }
                Ok(n)
            })
            .unwrap_or(0);
        head.truncate(n);
        match Self::sniff(&head) {
            // An ID3 tag is put in front of other files too (FLAC, say).
            Some(Self::Mp3) if head.starts_with(b"ID3") && matches!(Self::by_extension(path), Self::Flac | Self::Aiff | Self::Other) => Self::by_extension(path),
            Some(f) => f,
            None => Self::by_extension(path),
        }
    }

    /// What the start of a file says it is.
    pub fn sniff(head: &[u8]) -> Option<Self> {
        if head.len() >= 12 && (&head[..4] == b"RIFF" || &head[..4] == b"RF64") && &head[8..12] == b"WAVE" {
            Some(Self::Wav)
        } else if head.len() >= 4 && &head[..4] == b"fLaC" {
            Some(Self::Flac)
        } else if head.len() >= 12 && &head[..4] == b"FORM" && (&head[8..12] == b"AIFF" || &head[8..12] == b"AIFC") {
            Some(Self::Aiff)
        } else if head.len() >= 4 && (&head[..4] == b"OggS" || &head[4..8.min(head.len())] == b"ftyp" || head[..4] == [0x30, 0x26, 0xB2, 0x75]) {
            // Ogg, MP4/M4A and ASF/WMA containers.
            Some(Self::Other)
        } else if crate::mp3_decode::looks_like_mp3(head) {
            Some(Self::Mp3)
        } else {
            None
        }
    }

    pub fn by_extension(path: &Path) -> Self {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        match ext.as_str() {
            "flac" => Self::Flac,
            "mp3" => Self::Mp3,
            "aif" | "aiff" | "aifc" => Self::Aiff,
            _ if is_external_audio_file(path) => Self::Other,
            _ => Self::Wav,
        }
    }
}

/// The rate ffmpeg hands decoded sound over at.
const EXTERNAL_RATE: u32 = 48_000;

/// The one way in for sound files Krabton does not decode itself (Ogg
/// Vorbis, Opus, AAC/M4A, WMA, ...): ffmpeg decodes them, to interleaved
/// 32-bit float stereo at 48 kHz, and the result goes through
/// [`SampleData::from_decoded`]. Without ffmpeg, it says what is missing.
pub fn decode_with_external(path: &Path) -> Result<SampleData, String> {
    let ext = path.extension().map(|e| e.to_string_lossy().to_uppercase()).unwrap_or_else(|| "this".into());
    let Some(exe) = crate::ffmpeg::find() else {
        return Err(format!("{}: {ext} files need ffmpeg, which Krabton could not find (WAV, FLAC, MP3 and AIFF load without it; the Video's Get ffmpeg fetches it)", path.display()));
    };
    let out = crate::ffmpeg::command(&exe)
        .args(external_args(path))
        .output()
        .map_err(|e| format!("{}: ffmpeg did not start: {e}", path.display()))?;
    if !out.status.success() || out.stdout.len() < 8 {
        let why = String::from_utf8_lossy(&out.stderr);
        let why = why.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("no sound in it").trim().to_string();
        return Err(format!("{}: ffmpeg could not read it ({why})", path.display()));
    }
    let samples: Vec<f32> = out.stdout.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
    SampleData::from_decoded(path, EXTERNAL_RATE, 2, samples)
}

/// ffmpeg's arguments for a sound file: its first sound only, as raw
/// little-endian float stereo at 48 kHz on stdout.
fn external_args(path: &Path) -> Vec<std::ffi::OsString> {
    let mut a: Vec<std::ffi::OsString> = ["-nostdin", "-v", "error", "-i"].iter().map(Into::into).collect();
    a.push(path.as_os_str().to_owned());
    for s in ["-map", "0:a:0", "-vn", "-f", "f32le", "-acodec", "pcm_f32le", "-ac", "2", "-ar", "48000", "-"] {
        a.push(s.into());
    }
    a
}

impl SampleData {
    /// A sample from decoded audio: interleaved frames of `channels`
    /// (the first two are used; mono plays on both sides).
    pub fn from_decoded(path: &Path, sample_rate: u32, channels: usize, interleaved: Vec<f32>) -> Result<Self, String> {
        Self::from_interleaved(path, sample_rate, channels.max(1), interleaved)
    }
}

/// An AIFF/AIFF-C file's sample rate, channels and interleaved samples.
fn parse_aiff(d: &[u8]) -> Result<(u32, usize, Vec<f32>), String> {
    if d.len() < 12 || &d[..4] != b"FORM" || (&d[8..12] != b"AIFF" && &d[8..12] != b"AIFC") {
        return Err("not an AIFF file".into());
    }
    let aifc = &d[8..12] == b"AIFC";
    let be16 = |i: usize| u16::from_be_bytes([d[i], d[i + 1]]);
    let be32 = |i: usize| u32::from_be_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]]);
    let (mut comm, mut ssnd) = (None, None);
    let mut i = 12;
    while i + 8 <= d.len() {
        let id = &d[i..i + 4];
        let n = be32(i + 4) as usize;
        let body = i + 8..(i + 8 + n).min(d.len());
        if id == b"COMM" && body.len() >= 18 {
            let b = body.start;
            let compression = if aifc && body.len() >= 22 { [d[b + 18], d[b + 19], d[b + 20], d[b + 21]] } else { *b"NONE" };
            comm = Some((be16(b).max(1) as usize, be16(b + 6), extended_to_f64(&d[b + 8..b + 18]), compression));
        } else if id == b"SSND" && body.len() >= 8 {
            let offset = be32(body.start) as usize;
            ssnd = Some((body.start + 8 + offset).min(body.end)..body.end);
        }
        i += 8 + n + (n & 1);
    }
    let (channels, bits, rate, compression) = comm.ok_or("no COMM chunk")?;
    let data = &d[ssnd.ok_or("no SSND chunk")?];
    if !(rate.is_finite() && rate >= 1.0 && rate <= 1.0e7) {
        return Err(format!("sample rate {rate}"));
    }
    let bytes = (bits as usize).div_ceil(8);
    let interleaved: Vec<f32> = match (&compression, bytes) {
        (b"NONE" | b"twos", 1) => data.iter().map(|b| *b as i8 as f32 / 128.0).collect(),
        (b"NONE" | b"twos", 2) => data.chunks_exact(2).map(|c| i16::from_be_bytes([c[0], c[1]]) as f32 / 32768.0).collect(),
        (b"NONE" | b"twos" | b"in24", 3) => data.chunks_exact(3).map(|c| (i32::from_be_bytes([c[0], c[1], c[2], 0]) >> 8) as f32 / 8_388_608.0).collect(),
        (b"NONE" | b"twos" | b"in32", 4) => data.chunks_exact(4).map(|c| i32::from_be_bytes([c[0], c[1], c[2], c[3]]) as f32 / 2_147_483_648.0).collect(),
        (b"sowt", 1) => data.iter().map(|b| *b as i8 as f32 / 128.0).collect(),
        (b"sowt", 2) => data.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0).collect(),
        (b"sowt", 3) => data.chunks_exact(3).map(|c| (i32::from_le_bytes([0, c[0], c[1], c[2]]) >> 8) as f32 / 8_388_608.0).collect(),
        (b"sowt", 4) => data.chunks_exact(4).map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f32 / 2_147_483_648.0).collect(),
        (b"fl32" | b"FL32", _) => data.chunks_exact(4).map(|c| f32::from_be_bytes([c[0], c[1], c[2], c[3]])).collect(),
        (b"fl64" | b"FL64", _) => data.chunks_exact(8).map(|c| f64::from_be_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]) as f32).collect(),
        _ => return Err(format!("unsupported AIFF compression '{}' with {bits} bits", String::from_utf8_lossy(&compression))),
    };
    Ok((rate.round() as u32, channels, interleaved))
}

/// The 80-bit extended float AIFF keeps its sample rate in.
fn extended_to_f64(b: &[u8]) -> f64 {
    let exponent = (((b[0] & 0x7F) as i32) << 8) | b[1] as i32;
    let mantissa = u64::from_be_bytes([b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9]]);
    if exponent == 0 && mantissa == 0 {
        return 0.0;
    }
    let v = mantissa as f64 * 2f64.powi(exponent - 16383 - 63);
    if b[0] & 0x80 != 0 {
        -v
    } else {
        v
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

/// Notes a file a project names that could not be read (a SoundFont bank).
pub(crate) fn note_missing(e: String) {
    MISSING.with(|m| m.borrow_mut().push(e));
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

    /// An AIFF (or AIFF-C with `compression`) file of 44.1 kHz stereo frames.
    fn aiff(compression: Option<&[u8; 4]>, bits: u16, data: &[u8]) -> Vec<u8> {
        let mut comm = Vec::new();
        comm.extend_from_slice(&2u16.to_be_bytes());
        comm.extend_from_slice(&((data.len() / (bits as usize / 8) / 2) as u32).to_be_bytes());
        comm.extend_from_slice(&bits.to_be_bytes());
        // 44100 as an 80-bit extended float.
        comm.extend_from_slice(&[0x40, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0]);
        if let Some(c) = compression {
            comm.extend_from_slice(c);
            comm.extend_from_slice(&[0, 0]); // an empty name, padded
        }
        let mut body = Vec::new();
        body.extend_from_slice(if compression.is_some() { b"AIFC" } else { b"AIFF" });
        body.extend_from_slice(b"COMM");
        body.extend_from_slice(&(comm.len() as u32).to_be_bytes());
        body.extend_from_slice(&comm);
        body.extend_from_slice(b"SSND");
        body.extend_from_slice(&(data.len() as u32 + 8).to_be_bytes());
        body.extend_from_slice(&[0; 8]);
        body.extend_from_slice(data);
        let mut d = b"FORM".to_vec();
        d.extend_from_slice(&(body.len() as u32).to_be_bytes());
        d.extend_from_slice(&body);
        d
    }

    fn load_bytes(name: &str, d: &[u8]) -> Result<SampleData, String> {
        let path = std::env::temp_dir().join(format!("krabton_{}_{name}", std::process::id()));
        std::fs::write(&path, d).unwrap();
        let s = SampleData::load(&path);
        let _ = std::fs::remove_file(&path);
        s
    }

    #[test]
    fn aiff_files_load() {
        // 16-bit big-endian: left 0.5 then 0.25, right -0.5 then -0.25.
        let pcm16: Vec<u8> = [16384i16, -16384, 8192, -8192].iter().flat_map(|v| v.to_be_bytes()).collect();
        let s = load_bytes("a.aiff", &aiff(None, 16, &pcm16)).expect("AIFF loads");
        assert_eq!((s.sample_rate, s.frames()), (44100, 2));
        assert!((s.left[0] - 0.5).abs() < 1e-4 && (s.right[0] + 0.5).abs() < 1e-4 && (s.left[1] - 0.25).abs() < 1e-4);
        // 24-bit.
        let pcm24: Vec<u8> = [0x400000i32, -0x400000, 0x200000, -0x200000].iter().flat_map(|v| v.to_be_bytes()[1..].to_vec()).collect();
        let s = load_bytes("b.aif", &aiff(None, 24, &pcm24)).expect("24-bit AIFF loads");
        assert!((s.left[0] - 0.5).abs() < 1e-4 && (s.right[1] + 0.25).abs() < 1e-4);
        // AIFF-C: little-endian ('sowt', as Macs write it) and float.
        let sowt: Vec<u8> = [16384i16, -16384, 8192, -8192].iter().flat_map(|v| v.to_le_bytes()).collect();
        let s = load_bytes("c.aifc", &aiff(Some(b"sowt"), 16, &sowt)).expect("sowt loads");
        assert!((s.left[0] - 0.5).abs() < 1e-4 && (s.right[0] + 0.5).abs() < 1e-4);
        let fl32: Vec<u8> = [0.5f32, -0.5, 0.25, -0.25].iter().flat_map(|v| v.to_be_bytes()).collect();
        let s = load_bytes("d.aiff", &aiff(Some(b"fl32"), 32, &fl32)).expect("fl32 loads");
        assert_eq!(s.left, vec![0.5, 0.25]);
        assert_eq!(s.right, vec![-0.5, -0.25]);
        assert!(load_bytes("e.aiff", &aiff(Some(b"ima4"), 16, &pcm16)).is_err());
        assert_eq!(extended_to_f64(&[0x40, 0x0E, 0xBB, 0x80, 0, 0, 0, 0, 0, 0]), 48000.0);
    }

    #[test]
    fn files_are_known_by_what_is_in_them() {
        let pcm16: Vec<u8> = [16384i16, -16384, 8192, -8192].iter().flat_map(|v| v.to_be_bytes()).collect();
        // An AIFF named .wav still loads as AIFF.
        let s = load_bytes("aiff_named.wav", &aiff(None, 16, &pcm16)).expect("loads by content");
        assert_eq!(s.frames(), 2);
        assert_eq!(AudioFormat::sniff(b"fLaC\0\0\0\x22"), Some(AudioFormat::Flac));
        assert_eq!(AudioFormat::sniff(b"OggS\0\x02\0\0\0\0\0\0"), Some(AudioFormat::Other));
        assert_eq!(AudioFormat::sniff(b"\0\0\0\x20ftypM4A "), Some(AudioFormat::Other));
        assert_eq!(AudioFormat::sniff(b"RIFF\0\0\0\0WAVEfmt "), Some(AudioFormat::Wav));
        assert_eq!(AudioFormat::by_extension(Path::new("x.MP3")), AudioFormat::Mp3);
        assert_eq!(AudioFormat::by_extension(Path::new("x.ogg")), AudioFormat::Other);
        assert!(is_audio_file(Path::new("Song.Mp3")) && is_audio_file(Path::new("a.aif")) && !is_audio_file(Path::new("a.ogg")));
        // Formats for ffmpeg say so.
        let Err(e) = load_bytes("song.ogg", b"OggS\0\x02\0\0\0\0\0\0\0\0\0\0\0\0") else { panic!("an Ogg file loaded") };
        assert!(e.contains("ffmpeg"), "{e}");
    }

    #[test]
    fn ffmpeg_is_asked_for_float_stereo() {
        let a: Vec<String> = external_args(Path::new("a b.ogg")).iter().map(|s| s.to_string_lossy().into_owned()).collect();
        assert_eq!(a[4], "a b.ogg");
        assert!(a.windows(2).any(|w| w == ["-f", "f32le"]) && a.windows(2).any(|w| w == ["-ac", "2"]) && a.windows(2).any(|w| w == ["-ar", "48000"]));
        assert_eq!(a.last().map(String::as_str), Some("-"));
    }

    #[test]
    fn an_mp3_loads_whatever_its_name() {
        // Ten silent frames: MPEG-1 Layer III, 128 kbit/s, 44.1 kHz, mono.
        let mut d = Vec::new();
        for _ in 0..10 {
            let mut f = vec![0xFF, 0xFB, 0x90, 0xC0];
            f.resize(417, 0);
            d.extend_from_slice(&f);
        }
        for name in ["silence.mp3", "silence.wav", "silence"] {
            let s = load_bytes(name, &d).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!((s.sample_rate, s.frames()), (44100, 11520), "{name}");
        }
        // A set keeps the MP3's path and reads it again when it opens.
        let path = std::env::temp_dir().join(format!("krabton_{}_in_a_set.mp3", std::process::id()));
        std::fs::write(&path, &d).unwrap();
        let s = SampleRef::load(&path).unwrap();
        let json = serde_json::to_string(&s).unwrap();
        let back: SampleRef = serde_json::from_str(&json).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(back.path.as_deref(), Some(path.as_path()));
        assert_eq!((back.frames(), back.name.as_str()), (11520, s.name.as_str()));
    }
}
