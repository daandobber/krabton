//! MP3 decoding: MPEG-1, MPEG-2 and MPEG-2.5 audio, Layers I, II and III,
//! mono, stereo, joint stereo (mid/side and intensity), every bitrate, free
//! format and VBR.
//!
//! This is a port to Rust of minimp3 by Lion (lieff) and its contributors
//! (<https://github.com/lieff/minimp3>), whose authors dedicated it to the
//! public domain under CC0 1.0
//! (<http://creativecommons.org/publicdomain/zero/1.0/>). It follows
//! minimp3's plain (non-SIMD) code path line by line, with its tables as they
//! are, so it decodes as minimp3 does; around it is what minimp3_ex does for a
//! whole file: ID3v2, ID3v1 and APE tags are skipped, and a Xing/Info frame
//! with a LAME tag gives the encoder's delay and padding, which are cut off
//! so a loop starts and ends where the original did.
//!
//! It runs on a loading thread, never on the audio thread.

use std::path::Path;

/// The most samples one frame gives (1152 per channel, two channels).
pub const MAX_SAMPLES_PER_FRAME: usize = 1152 * 2;

const HDR_SIZE: usize = 4;
const MAX_FREE_FORMAT_FRAME_SIZE: usize = 2304;
const MAX_FRAME_SYNC_MATCHES: usize = 10;
const MAX_L3_FRAME_PAYLOAD_BYTES: usize = MAX_FREE_FORMAT_FRAME_SIZE;
const MAX_BITRESERVOIR_BYTES: usize = 511;
const SHORT_BLOCK_TYPE: u8 = 2;
const STOP_BLOCK_TYPE: u8 = 3;
const MODE_MONO: u8 = 3;
const MODE_JOINT_STEREO: u8 = 1;
const BITS_DEQUANTIZER_OUT: i32 = -1;
const MAX_SCF: i32 = 255 + BITS_DEQUANTIZER_OUT * 4 - 210;
const MAX_SCFI: i32 = (MAX_SCF + 3) & !3;

/// A whole file decoded: one buffer per side (a mono file has the same on both).
pub struct Mp3Audio {
    pub sample_rate: u32,
    /// 1 or 2: what the file has (a file that changes between them counts as 2).
    pub channels: usize,
    pub left: Vec<f32>,
    pub right: Vec<f32>,
}

/// Decodes a whole MP3 file in memory.
pub fn decode(data: &[u8]) -> Result<Mp3Audio, String> {
    let data = skip_tags(data);
    let mut free_format_bytes = 0;
    let mut frame_size = 0;
    let first = find_frame(data, &mut free_format_bytes, &mut frame_size);
    if frame_size == 0 {
        return Err("no MPEG audio found".into());
    }
    let mut pos = first;
    // A Xing or Info frame first (it plays nothing) says how long the audio
    // really is: the encoder's delay at the start and padding at the end go.
    let mut to_skip = 0usize;
    let mut detected: Option<usize> = None;
    let hdr = &data[first..];
    if 4 - hdr_get_layer(hdr) as usize == 3 {
        let frame = &data[first..(first + frame_size).min(data.len())];
        match check_vbr_tag(frame) {
            VbrTag::None => {}
            VbrTag::NoCount => pos += frame_size,
            VbrTag::Count { frames, delay, padding } => {
                pos += frame_size;
                let mut total = hdr_frame_samples(hdr) as usize * frames as usize;
                to_skip = delay.max(0) as usize;
                total = total.saturating_sub(to_skip);
                if padding > 0 {
                    total = total.saturating_sub(padding as usize);
                }
                if total == 0 {
                    return Err("the MP3 file has no audio".into());
                }
                detected = Some(total);
            }
        }
    }

    let mut dec = Decoder::new();
    let mut pcm = vec![0.0f32; MAX_SAMPLES_PER_FRAME];
    let estimate = detected.unwrap_or_else(|| (data.len() - pos) / frame_size.max(1) * hdr_frame_samples(hdr) as usize);
    let mut left = Vec::with_capacity(estimate + MAX_SAMPLES_PER_FRAME);
    let mut right = Vec::with_capacity(estimate + MAX_SAMPLES_PER_FRAME);
    let (mut hz, mut layer, mut channels) = (0u32, 0u32, 0usize);
    while pos < data.len() {
        let (samples, info) = dec.decode_frame(&data[pos..], &mut pcm);
        pos += info.frame_bytes;
        if samples > 0 {
            if hz == 0 {
                hz = info.hz;
                layer = info.layer;
            } else if info.hz != hz || info.layer != layer {
                // Another stream glued on: stop where this one ends.
                break;
            }
            let ch = info.channels;
            channels = channels.max(ch);
            let skip = to_skip.min(samples);
            to_skip -= skip;
            for f in skip..samples {
                let l = pcm[f * ch];
                left.push(l);
                right.push(if ch > 1 { pcm[f * ch + 1] } else { l });
            }
        }
        if info.frame_bytes == 0 {
            break;
        }
    }
    if let Some(n) = detected {
        left.truncate(n);
        right.truncate(n);
    }
    if left.is_empty() || hz == 0 {
        return Err("no MP3 audio could be decoded".into());
    }
    Ok(Mp3Audio { sample_rate: hz, channels, left, right })
}

/// Reads and decodes an MP3 file.
pub fn decode_file(path: &Path) -> Result<Mp3Audio, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    decode(&data)
}

/// Whether these bytes (the start of a file) look like MPEG audio: an ID3v2
/// tag, or frames that follow each other.
pub fn looks_like_mp3(data: &[u8]) -> bool {
    if data.len() >= 10 && &data[..3] == b"ID3" {
        return true;
    }
    let head = &data[..data.len().min(64 * 1024)];
    let (mut free, mut size) = (0, 0);
    let at = find_frame(head, &mut free, &mut size);
    // Frames from (nearly) the start, not a stray sync word deep inside.
    size > 0 && at < 4096
}

/// The audio part of a file: without an ID3v2 tag in front, or ID3v1 and APE
/// tags at the end.
fn skip_tags(mut buf: &[u8]) -> &[u8] {
    let id3v2 = id3v2_size(buf);
    if id3v2 > 0 {
        buf = &buf[id3v2.min(buf.len())..];
    }
    let mut size = buf.len();
    if size >= 128 && &buf[size - 128..size - 125] == b"TAG" {
        size -= 128;
        if size >= 227 && &buf[size - 227..size - 223] == b"TAG+" {
            size -= 227;
        }
    }
    if size > 32 && &buf[size - 32..size - 24] == b"APETAGEX" {
        size -= 32;
        let t = &buf[size + 12..size + 16];
        let tag_size = u32::from_le_bytes([t[0], t[1], t[2], t[3]]) as usize;
        if size >= tag_size {
            size -= tag_size;
        }
    }
    &buf[..size]
}

fn id3v2_size(buf: &[u8]) -> usize {
    if buf.len() >= 10 && &buf[..3] == b"ID3" && !((buf[5] & 15) != 0 || (buf[6] & 0x80) != 0 || (buf[7] & 0x80) != 0 || (buf[8] & 0x80) != 0 || (buf[9] & 0x80) != 0) {
        let mut size = ((((buf[6] & 0x7f) as usize) << 21) | (((buf[7] & 0x7f) as usize) << 14) | (((buf[8] & 0x7f) as usize) << 7) | (buf[9] & 0x7f) as usize) + 10;
        if buf[5] & 16 != 0 {
            size += 10; // footer
        }
        return size;
    }
    0
}

enum VbrTag {
    None,
    /// A Xing/Info frame that does not count the frames.
    NoCount,
    Count { frames: u32, delay: i32, padding: i32 },
}

/// The Xing/Info tag in the first frame, with the LAME tag's encoder delay
/// and padding (the decoder's own delay of 529 samples included).
fn check_vbr_tag(frame: &[u8]) -> VbrTag {
    const FRAMES_FLAG: u8 = 1;
    const BYTES_FLAG: u8 = 2;
    const TOC_FLAG: u8 = 4;
    const VBR_SCALE_FLAG: u8 = 8;
    let frame_size = frame.len();
    if frame_size < HDR_SIZE + 8 {
        return VbrTag::None;
    }
    let mut bs = Bs::new(&frame[HDR_SIZE..], frame_size - HDR_SIZE);
    if hdr_is_crc(frame) {
        bs.get_bits(16);
    }
    let mut gr = [GrInfo::default(); 4];
    if read_side_info(&mut bs, &mut gr, frame) < 0 {
        return VbrTag::None; // side info corrupted
    }
    let mut tag_off = HDR_SIZE + bs.pos / 8;
    if tag_off > frame_size || frame_size - tag_off < 8 {
        return VbrTag::None;
    }
    let tag = &frame[tag_off..];
    if &tag[..4] != b"Xing" && &tag[..4] != b"Info" {
        return VbrTag::None;
    }
    let flags = tag[7];
    if flags & FRAMES_FLAG == 0 {
        return VbrTag::NoCount;
    }
    tag_off += 8;
    if frame_size - tag_off < 4 {
        return VbrTag::None;
    }
    let t = &frame[tag_off..];
    let frames = u32::from_be_bytes([t[0], t[1], t[2], t[3]]);
    tag_off += 4;
    for (flag, len) in [(BYTES_FLAG, 4), (TOC_FLAG, 100), (VBR_SCALE_FLAG, 4)] {
        if flags & flag != 0 {
            if frame_size - tag_off < len {
                return VbrTag::None;
            }
            tag_off += len;
        }
    }
    let (mut delay, mut padding) = (0, 0);
    if frame_size - tag_off < 1 {
        return VbrTag::None;
    }
    if frame[tag_off] != 0 {
        // An extension (LAME, Lavc, ...): the same layout.
        if frame_size - tag_off <= 35 {
            return VbrTag::None;
        }
        tag_off += 21;
        let t = &frame[tag_off..];
        delay = (((t[0] as i32) << 4) | ((t[1] as i32) >> 4)) + (528 + 1);
        padding = ((((t[1] & 0xF) as i32) << 8) | t[2] as i32) - (528 + 1);
    }
    VbrTag::Count { frames, delay, padding }
}

// ---------------------------------------------------------------------------
// Frame headers

#[inline]
fn hdr_is_mono(h: &[u8]) -> bool {
    h[3] & 0xC0 == 0xC0
}
#[inline]
fn hdr_is_ms_stereo(h: &[u8]) -> bool {
    h[3] & 0xE0 == 0x60
}
#[inline]
fn hdr_is_free_format(h: &[u8]) -> bool {
    h[2] & 0xF0 == 0
}
#[inline]
fn hdr_is_crc(h: &[u8]) -> bool {
    h[1] & 1 == 0
}
#[inline]
fn hdr_test_padding(h: &[u8]) -> bool {
    h[2] & 0x2 != 0
}
#[inline]
fn hdr_test_mpeg1(h: &[u8]) -> bool {
    h[1] & 0x8 != 0
}
#[inline]
fn hdr_test_not_mpeg25(h: &[u8]) -> bool {
    h[1] & 0x10 != 0
}
#[inline]
fn hdr_test_i_stereo(h: &[u8]) -> bool {
    h[3] & 0x10 != 0
}
#[inline]
fn hdr_test_ms_stereo(h: &[u8]) -> bool {
    h[3] & 0x20 != 0
}
#[inline]
fn hdr_get_stereo_mode(h: &[u8]) -> u8 {
    (h[3] >> 6) & 3
}
#[inline]
fn hdr_get_stereo_mode_ext(h: &[u8]) -> u8 {
    (h[3] >> 4) & 3
}
#[inline]
fn hdr_get_layer(h: &[u8]) -> u8 {
    (h[1] >> 1) & 3
}
#[inline]
fn hdr_get_bitrate(h: &[u8]) -> u8 {
    h[2] >> 4
}
#[inline]
fn hdr_get_sample_rate(h: &[u8]) -> u8 {
    (h[2] >> 2) & 3
}
#[inline]
fn hdr_get_my_sample_rate(h: &[u8]) -> u8 {
    hdr_get_sample_rate(h) + (((h[1] >> 3) & 1) + ((h[1] >> 4) & 1)) * 3
}
#[inline]
fn hdr_is_frame_576(h: &[u8]) -> bool {
    h[1] & 14 == 2
}
#[inline]
fn hdr_is_layer_1(h: &[u8]) -> bool {
    h[1] & 6 == 6
}

fn hdr_valid(h: &[u8]) -> bool {
    h.len() >= HDR_SIZE && h[0] == 0xff && ((h[1] & 0xF0) == 0xf0 || (h[1] & 0xFE) == 0xe2) && hdr_get_layer(h) != 0 && hdr_get_bitrate(h) != 15 && hdr_get_sample_rate(h) != 3
}

fn hdr_compare(h1: &[u8], h2: &[u8]) -> bool {
    hdr_valid(h2) && ((h1[1] ^ h2[1]) & 0xFE) == 0 && ((h1[2] ^ h2[2]) & 0x0C) == 0 && !(hdr_is_free_format(h1) ^ hdr_is_free_format(h2))
}

fn hdr_bitrate_kbps(h: &[u8]) -> u32 {
    const HALFRATE: [[[u8; 15]; 3]; 2] = [
        [[0, 4, 8, 12, 16, 20, 24, 28, 32, 40, 48, 56, 64, 72, 80], [0, 4, 8, 12, 16, 20, 24, 28, 32, 40, 48, 56, 64, 72, 80], [0, 16, 24, 28, 32, 40, 48, 56, 64, 72, 80, 88, 96, 112, 128]],
        [[0, 16, 20, 24, 28, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160], [0, 16, 24, 28, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192], [0, 16, 32, 48, 64, 80, 96, 112, 128, 144, 160, 176, 192, 208, 224]],
    ];
    2 * HALFRATE[hdr_test_mpeg1(h) as usize][hdr_get_layer(h) as usize - 1][hdr_get_bitrate(h) as usize] as u32
}

fn hdr_sample_rate_hz(h: &[u8]) -> u32 {
    const HZ: [u32; 3] = [44100, 48000, 32000];
    HZ[hdr_get_sample_rate(h) as usize] >> (!hdr_test_mpeg1(h) as u32) >> (!hdr_test_not_mpeg25(h) as u32)
}

fn hdr_frame_samples(h: &[u8]) -> u32 {
    if hdr_is_layer_1(h) {
        384
    } else {
        1152 >> (hdr_is_frame_576(h) as u32)
    }
}

fn hdr_frame_bytes(h: &[u8], free_format_size: usize) -> usize {
    let mut frame_bytes = (hdr_frame_samples(h) * hdr_bitrate_kbps(h) * 125 / hdr_sample_rate_hz(h)) as usize;
    if hdr_is_layer_1(h) {
        frame_bytes &= !3; // slot align
    }
    if frame_bytes != 0 {
        frame_bytes
    } else {
        free_format_size
    }
}

fn hdr_padding(h: &[u8]) -> usize {
    if hdr_test_padding(h) {
        if hdr_is_layer_1(h) {
            4
        } else {
            1
        }
    } else {
        0
    }
}

/// Whether `frame_bytes` on from this header, frame after frame, find
/// headers like it.
fn match_frame(hdr: &[u8], mp3_bytes: usize, frame_bytes: usize) -> bool {
    let mut i = 0;
    for nmatch in 0..MAX_FRAME_SYNC_MATCHES {
        i += hdr_frame_bytes(&hdr[i..], frame_bytes) + hdr_padding(&hdr[i..]);
        if i + HDR_SIZE > mp3_bytes {
            return nmatch > 0;
        }
        if !hdr_compare(hdr, &hdr[i..]) {
            return false;
        }
    }
    true
}

/// Where the next frame starts (or the length when there is none), and its
/// size with padding in `ptr_frame_bytes` (0 when none was found).
fn find_frame(mp3: &[u8], free_format_bytes: &mut usize, ptr_frame_bytes: &mut usize) -> usize {
    let mp3_bytes = mp3.len();
    let mut i = 0;
    while i + HDR_SIZE < mp3_bytes {
        let h = &mp3[i..];
        if hdr_valid(h) {
            let mut frame_bytes = hdr_frame_bytes(h, *free_format_bytes);
            let mut frame_and_padding = frame_bytes + hdr_padding(h);
            let mut k = HDR_SIZE;
            while frame_bytes == 0 && k < MAX_FREE_FORMAT_FRAME_SIZE && i + 2 * k < mp3_bytes - HDR_SIZE {
                if hdr_compare(h, &h[k..]) {
                    let fb = k - hdr_padding(h);
                    let nextfb = fb + hdr_padding(&h[k..]);
                    if i + k + nextfb + HDR_SIZE > mp3_bytes || !hdr_compare(h, &h[k + nextfb..]) {
                        k += 1;
                        continue;
                    }
                    frame_and_padding = k;
                    frame_bytes = fb;
                    *free_format_bytes = fb;
                }
                k += 1;
            }
            if (frame_bytes != 0 && i + frame_and_padding <= mp3_bytes && match_frame(h, mp3_bytes - i, frame_bytes)) || (i == 0 && frame_and_padding == mp3_bytes) {
                *ptr_frame_bytes = frame_and_padding;
                return i;
            }
            *free_format_bytes = 0;
        }
        i += 1;
    }
    *ptr_frame_bytes = 0;
    mp3_bytes
}

// ---------------------------------------------------------------------------
// The bit reader

struct Bs<'a> {
    buf: &'a [u8],
    pos: usize,
    limit: usize,
}

impl<'a> Bs<'a> {
    fn new(buf: &'a [u8], bytes: usize) -> Self {
        Self { buf, pos: 0, limit: bytes * 8 }
    }

    #[inline]
    fn byte(&self, i: usize) -> u32 {
        self.buf.get(i).copied().unwrap_or(0) as u32
    }

    fn get_bits(&mut self, n: u32) -> u32 {
        let s = (self.pos & 7) as i32;
        let mut shl = n as i32 + s;
        let mut p = self.pos >> 3;
        self.pos += n as usize;
        if self.pos > self.limit {
            return 0;
        }
        let mut next = self.byte(p) & (255 >> s);
        p += 1;
        let mut cache = 0u32;
        loop {
            shl -= 8;
            if shl <= 0 {
                break;
            }
            cache |= next << shl;
            next = self.byte(p);
            p += 1;
        }
        cache | (next >> -shl)
    }
}

// ---------------------------------------------------------------------------
// The decoder

/// What one frame was.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameInfo {
    /// Bytes to move on by (any junk before the frame included).
    pub frame_bytes: usize,
    pub frame_offset: usize,
    pub channels: usize,
    pub hz: u32,
    pub layer: u32,
    pub bitrate_kbps: u32,
}

#[derive(Clone, Copy)]
struct GrInfo {
    sfbtab: &'static [u8],
    part_23_length: u16,
    big_values: u16,
    scalefac_compress: u16,
    global_gain: u8,
    block_type: u8,
    mixed_block_flag: u8,
    n_long_sfb: u8,
    n_short_sfb: u8,
    table_select: [u8; 3],
    region_count: [u8; 3],
    subblock_gain: [u8; 3],
    preflag: u8,
    scalefac_scale: u8,
    count1_table: u8,
    scfsi: u8,
}

impl Default for GrInfo {
    fn default() -> Self {
        Self {
            sfbtab: &SCF_LONG[0],
            part_23_length: 0,
            big_values: 0,
            scalefac_compress: 0,
            global_gain: 0,
            block_type: 0,
            mixed_block_flag: 0,
            n_long_sfb: 0,
            n_short_sfb: 0,
            table_select: [0; 3],
            region_count: [0; 3],
            subblock_gain: [0; 3],
            preflag: 0,
            scalefac_scale: 0,
            count1_table: 0,
            scfsi: 0,
        }
    }
}

/// Working memory for one frame (minimp3 keeps it on the stack).
struct Scratch {
    maindata: [u8; MAX_BITRESERVOIR_BYTES + MAX_L3_FRAME_PAYLOAD_BYTES],
    /// The bit reader over `maindata`: where it is and where it ends.
    bs_pos: usize,
    bs_limit: usize,
    gr_info: [GrInfo; 4],
    /// Both channels, the right one from 576.
    grbuf: [f32; 576 * 2],
    scf: [f32; 40],
    syn: [f32; (18 + 15) * 2 * 32],
    ist_pos: [[u8; 39]; 2],
}

/// An MP3 decoder: one frame at a time, keeping what carries over between
/// frames (overlap, filter bank state, the bit reservoir).
pub struct Decoder {
    mdct_overlap: [[f32; 9 * 32]; 2],
    qmf_state: [f32; 15 * 2 * 32],
    reserv: usize,
    free_format_bytes: usize,
    header: [u8; 4],
    reserv_buf: [u8; 511],
    scratch: Box<Scratch>,
}

impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder {
    pub fn new() -> Self {
        Self {
            mdct_overlap: [[0.0; 9 * 32]; 2],
            qmf_state: [0.0; 15 * 2 * 32],
            reserv: 0,
            free_format_bytes: 0,
            header: [0; 4],
            reserv_buf: [0; 511],
            scratch: Box::new(Scratch {
                maindata: [0; MAX_BITRESERVOIR_BYTES + MAX_L3_FRAME_PAYLOAD_BYTES],
                bs_pos: 0,
                bs_limit: 0,
                gr_info: [GrInfo::default(); 4],
                grbuf: [0.0; 576 * 2],
                scf: [0.0; 40],
                syn: [0.0; (18 + 15) * 2 * 32],
                ist_pos: [[0; 39]; 2],
            }),
        }
    }

    /// Back to the start, as after a jump (minimp3 clears the whole decoder).
    fn reset(&mut self) {
        self.mdct_overlap = [[0.0; 9 * 32]; 2];
        self.qmf_state = [0.0; 15 * 2 * 32];
        self.reserv = 0;
        self.free_format_bytes = 0;
        self.header = [0; 4];
        self.reserv_buf = [0; 511];
    }

    /// Decodes the next frame in `mp3` into `pcm` (interleaved, -1..1, room
    /// for [`MAX_SAMPLES_PER_FRAME`]). Returns the samples per channel (0
    /// when the frame gave none: not enough bit reservoir yet, or damage)
    /// and what the frame was; `frame_bytes` 0 means no frame was left.
    pub fn decode_frame(&mut self, mp3: &[u8], pcm: &mut [f32]) -> (usize, FrameInfo) {
        let mp3_bytes = mp3.len();
        let mut info = FrameInfo::default();
        let mut i = 0;
        let mut frame_size = 0;
        if mp3_bytes > 4 && self.header[0] == 0xff && hdr_compare(&self.header, mp3) {
            frame_size = hdr_frame_bytes(mp3, self.free_format_bytes) + hdr_padding(mp3);
            if frame_size != mp3_bytes && (frame_size + HDR_SIZE > mp3_bytes || !hdr_compare(mp3, &mp3[frame_size..])) {
                frame_size = 0;
            }
        }
        if frame_size == 0 {
            self.reset();
            i = find_frame(mp3, &mut self.free_format_bytes, &mut frame_size);
            if frame_size == 0 || i + frame_size > mp3_bytes {
                info.frame_bytes = i;
                return (0, info);
            }
        }

        if frame_size <= HDR_SIZE {
            // Not a frame after all (a free-format guess on junk).
            info.frame_bytes = i + frame_size.max(1);
            self.header[0] = 0;
            return (0, info);
        }
        let hdr = &mp3[i..i + HDR_SIZE];
        self.header.copy_from_slice(hdr);
        info.frame_bytes = i + frame_size;
        info.frame_offset = i;
        info.channels = if hdr_is_mono(hdr) { 1 } else { 2 };
        info.hz = hdr_sample_rate_hz(hdr);
        info.layer = 4 - hdr_get_layer(hdr) as u32;
        info.bitrate_kbps = hdr_bitrate_kbps(hdr);

        let mut bs_frame = Bs::new(&mp3[i + HDR_SIZE..i + frame_size], frame_size - HDR_SIZE);
        if hdr_is_crc(hdr) {
            bs_frame.get_bits(16);
        }
        let nch = info.channels;
        let mut success = true;
        if info.layer == 3 {
            let main_data_begin = read_side_info(&mut bs_frame, &mut self.scratch.gr_info, hdr);
            if main_data_begin < 0 || bs_frame.pos > bs_frame.limit {
                self.header[0] = 0;
                return (0, info);
            }
            success = self.restore_reservoir(&bs_frame, main_data_begin as usize);
            if success {
                let granules = if hdr_test_mpeg1(hdr) { 2 } else { 1 };
                for igr in 0..granules {
                    self.scratch.grbuf.fill(0.0);
                    self.l3_decode(igr * nch, nch);
                    let s = &mut *self.scratch;
                    synth_granule(&mut self.qmf_state, &mut s.grbuf, 18, nch, pcm, igr * 576 * nch, &mut s.syn);
                }
            }
            self.save_reservoir();
        } else {
            let mut sci = L12ScaleInfo::default();
            l12_read_scale_info(hdr, &mut bs_frame, &mut sci);
            let s = &mut *self.scratch;
            s.grbuf.fill(0.0);
            let mut k = 0;
            let mut out = 0;
            for igr in 0..3 {
                k += l12_dequantize_granule(&mut s.grbuf, k, &mut bs_frame, &sci, info.layer as usize | 1);
                if k == 12 {
                    k = 0;
                    l12_apply_scf_384(&sci, igr, &mut s.grbuf);
                    synth_granule(&mut self.qmf_state, &mut s.grbuf, 12, nch, pcm, out, &mut s.syn);
                    s.grbuf.fill(0.0);
                    out += 384 * nch;
                }
                if bs_frame.pos > bs_frame.limit {
                    self.header[0] = 0;
                    return (0, info);
                }
            }
        }
        (if success { hdr_frame_samples(&self.header) as usize } else { 0 }, info)
    }

    /// Puts the bytes this frame's main data starts with (from earlier
    /// frames) in front of its own; false when not enough came before.
    fn restore_reservoir(&mut self, bs: &Bs, main_data_begin: usize) -> bool {
        let s = &mut *self.scratch;
        let frame_bytes = ((bs.limit - bs.pos) / 8).min(MAX_L3_FRAME_PAYLOAD_BYTES);
        let bytes_have = self.reserv.min(main_data_begin);
        let from = self.reserv.saturating_sub(main_data_begin);
        s.maindata[..bytes_have].copy_from_slice(&self.reserv_buf[from..from + bytes_have]);
        let start = bs.pos / 8;
        let avail = bs.buf.len().saturating_sub(start).min(frame_bytes);
        s.maindata[bytes_have..bytes_have + avail].copy_from_slice(&bs.buf[start..start + avail]);
        s.maindata[bytes_have + avail..bytes_have + frame_bytes].fill(0);
        s.bs_pos = 0;
        s.bs_limit = (bytes_have + frame_bytes) * 8;
        self.reserv >= main_data_begin
    }

    /// Keeps what is left of this frame's main data for the frames after it.
    fn save_reservoir(&mut self) {
        let s = &*self.scratch;
        let mut pos = s.bs_pos.div_ceil(8);
        let total = s.bs_limit / 8;
        let mut remains = total as isize - pos as isize;
        if remains > MAX_BITRESERVOIR_BYTES as isize {
            pos += remains as usize - MAX_BITRESERVOIR_BYTES;
            remains = MAX_BITRESERVOIR_BYTES as isize;
        }
        let remains = remains.max(0) as usize;
        if remains > 0 {
            self.reserv_buf[..remains].copy_from_slice(&s.maindata[pos..pos + remains]);
        }
        self.reserv = remains;
    }

    /// One granule of Layer III: scale factors and Huffman for each channel,
    /// stereo, reordering, alias reduction and the inverse MDCT.
    fn l3_decode(&mut self, gr0: usize, nch: usize) {
        let hdr = self.header;
        let Scratch { maindata, bs_pos, bs_limit, gr_info, grbuf, scf, syn, ist_pos } = &mut *self.scratch;
        let mut bs = Bs { buf: &maindata[..], pos: *bs_pos, limit: *bs_limit };
        for ch in 0..nch {
            let gr = &gr_info[gr0 + ch];
            let layer3gr_limit = bs.pos + gr.part_23_length as usize;
            decode_scalefactors(&hdr, &mut ist_pos[ch], &mut bs, gr, scf, ch);
            huffman(&mut grbuf[576 * ch..576 * (ch + 1)], &mut bs, gr, scf, layer3gr_limit);
        }
        *bs_pos = bs.pos;

        if hdr_test_i_stereo(&hdr) {
            intensity_stereo(grbuf, &mut ist_pos[1], &gr_info[gr0..gr0 + 2], &hdr);
        } else if hdr_is_ms_stereo(&hdr) {
            midside_stereo(grbuf, 0, 576);
        }

        for ch in 0..nch {
            let gr = &gr_info[gr0 + ch];
            let mut aa_bands = 31i32;
            let n_long_bands = (if gr.mixed_block_flag != 0 { 2usize } else { 0 }) << (hdr_get_my_sample_rate(&hdr) == 2) as usize;
            let g = &mut grbuf[576 * ch..576 * (ch + 1)];
            if gr.n_short_sfb != 0 {
                aa_bands = n_long_bands as i32 - 1;
                reorder(g, n_long_bands * 18, syn, &gr.sfbtab[gr.n_long_sfb as usize..]);
            }
            antialias(g, aa_bands);
            imdct_gr(g, &mut self.mdct_overlap[ch], gr.block_type, n_long_bands);
            change_sign(g);
        }
    }
}

// ---------------------------------------------------------------------------
// Layers I and II

struct L12ScaleInfo {
    scf: [f32; 3 * 64],
    total_bands: u8,
    stereo_bands: u8,
    bitalloc: [u8; 64],
    scfcod: [u8; 64],
}

impl Default for L12ScaleInfo {
    fn default() -> Self {
        Self { scf: [0.0; 3 * 64], total_bands: 0, stereo_bands: 0, bitalloc: [0; 64], scfcod: [0; 64] }
    }
}

#[derive(Clone, Copy)]
struct SubbandAlloc {
    tab_offset: u8,
    code_tab_width: u8,
    band_count: u8,
}

const fn sa(tab_offset: u8, code_tab_width: u8, band_count: u8) -> SubbandAlloc {
    SubbandAlloc { tab_offset, code_tab_width, band_count }
}

fn l12_subband_alloc_table(hdr: &[u8], sci: &mut L12ScaleInfo) -> &'static [SubbandAlloc] {
    const ALLOC_L1: [SubbandAlloc; 1] = [sa(76, 4, 32)];
    const ALLOC_L2M2: [SubbandAlloc; 3] = [sa(60, 4, 4), sa(44, 3, 7), sa(44, 2, 19)];
    const ALLOC_L2M1: [SubbandAlloc; 4] = [sa(0, 4, 3), sa(16, 4, 8), sa(32, 3, 12), sa(40, 2, 7)];
    const ALLOC_L2M1_LOWRATE: [SubbandAlloc; 2] = [sa(44, 4, 2), sa(44, 3, 10)];
    let mode = hdr_get_stereo_mode(hdr);
    let stereo_bands = if mode == MODE_MONO {
        0
    } else if mode == MODE_JOINT_STEREO {
        (hdr_get_stereo_mode_ext(hdr) << 2) + 4
    } else {
        32
    };
    let alloc: &'static [SubbandAlloc];
    let nbands;
    if hdr_is_layer_1(hdr) {
        alloc = &ALLOC_L1;
        nbands = 32;
    } else if !hdr_test_mpeg1(hdr) {
        alloc = &ALLOC_L2M2;
        nbands = 30;
    } else {
        let sample_rate_idx = hdr_get_sample_rate(hdr);
        let mut kbps = hdr_bitrate_kbps(hdr) >> (mode != MODE_MONO) as u32;
        if kbps == 0 {
            kbps = 192; // free format
        }
        if kbps < 56 {
            alloc = &ALLOC_L2M1_LOWRATE;
            nbands = if sample_rate_idx == 2 { 12 } else { 8 };
        } else {
            alloc = &ALLOC_L2M1;
            nbands = if kbps >= 96 && sample_rate_idx != 1 { 30 } else { 27 };
        }
    }
    sci.total_bands = nbands;
    sci.stereo_bands = stereo_bands.min(nbands);
    alloc
}

fn l12_read_scalefactors(bs: &mut Bs, pba: &[u8], scfcod: &[u8], bands: usize, scf: &mut [f32]) {
    const DEQ_BASE: [f32; 3] = [9.53674316e-07, 7.56931807e-07, 6.00777173e-07];
    const DEQ_DIV: [f32; 18] = [3.0, 7.0, 15.0, 31.0, 63.0, 127.0, 255.0, 511.0, 1023.0, 2047.0, 4095.0, 8191.0, 16383.0, 32767.0, 65535.0, 3.0, 5.0, 9.0];
    let mut o = 0;
    for i in 0..bands {
        let mut s = 0.0f32;
        let ba = pba[i] as usize;
        let mask = if ba != 0 { 4 + ((19 >> scfcod[i]) & 3) } else { 0 };
        let mut m = 4;
        while m != 0 {
            if mask & m != 0 {
                let b = bs.get_bits(6) as usize;
                let k = (ba * 3 + b % 3).saturating_sub(6);
                s = (DEQ_BASE[k % 3] / DEQ_DIV[(k / 3).min(17)]) * ((1i32 << 21) >> (b / 3)) as f32;
            }
            scf[o] = s;
            o += 1;
            m >>= 1;
        }
    }
}

fn l12_read_scale_info(hdr: &[u8], bs: &mut Bs, sci: &mut L12ScaleInfo) {
    let subband_alloc = l12_subband_alloc_table(hdr, sci);
    let mut k = 0usize;
    let mut ba_bits = 0u32;
    let mut ba_code_tab = 0usize;
    let mut next_alloc = 0;
    for i in 0..sci.total_bands as usize {
        if i == k {
            let a = subband_alloc[next_alloc.min(subband_alloc.len() - 1)];
            k += a.band_count as usize;
            ba_bits = a.code_tab_width as u32;
            ba_code_tab = a.tab_offset as usize;
            next_alloc += 1;
        }
        let mut ba = BITALLOC_CODE_TAB[ba_code_tab + bs.get_bits(ba_bits) as usize];
        sci.bitalloc[2 * i] = ba;
        if i < sci.stereo_bands as usize {
            ba = BITALLOC_CODE_TAB[ba_code_tab + bs.get_bits(ba_bits) as usize];
        }
        sci.bitalloc[2 * i + 1] = if sci.stereo_bands != 0 { ba } else { 0 };
    }
    for i in 0..2 * sci.total_bands as usize {
        sci.scfcod[i] = if sci.bitalloc[i] != 0 {
            if hdr_is_layer_1(hdr) {
                2
            } else {
                bs.get_bits(2) as u8
            }
        } else {
            6
        };
    }
    let bands = sci.total_bands as usize * 2;
    let L12ScaleInfo { scf, bitalloc, scfcod, .. } = sci;
    l12_read_scalefactors(bs, &bitalloc[..], &scfcod[..], bands, &mut scf[..]);
    for i in sci.stereo_bands as usize..sci.total_bands as usize {
        sci.bitalloc[2 * i + 1] = 0;
    }
}

fn l12_dequantize_granule(grbuf: &mut [f32], base: usize, bs: &mut Bs, sci: &L12ScaleInfo, group_size: usize) -> usize {
    let mut choff: isize = 576;
    for j in 0..4 {
        let mut dst = (base + group_size * j) as isize;
        for i in 0..2 * sci.total_bands as usize {
            let ba = sci.bitalloc[i] as u32;
            if ba != 0 {
                let d = dst as usize;
                if ba < 17 {
                    let half = (1i32 << (ba - 1)) - 1;
                    for k in 0..group_size {
                        grbuf[d + k] = (bs.get_bits(ba) as i32 - half) as f32;
                    }
                } else {
                    let m = (2u32 << (ba - 17)) + 1; // 3, 5, 9
                    let mut code = bs.get_bits(m + 2 - (m >> 3)); // 5, 7, 10
                    for k in 0..group_size {
                        grbuf[d + k] = ((code % m) as i32 - (m / 2) as i32) as f32;
                        code /= m;
                    }
                }
            }
            dst += choff;
            choff = 18 - choff;
        }
    }
    group_size * 4
}

fn l12_apply_scf_384(sci: &L12ScaleInfo, scf_off: usize, dst: &mut [f32]) {
    let sb = sci.stereo_bands as usize * 18;
    let n = (sci.total_bands as usize - sci.stereo_bands as usize) * 18;
    dst.copy_within(sb..sb + n, 576 + sb);
    for i in 0..sci.total_bands as usize {
        let d = i * 18;
        let s = scf_off + i * 6;
        for k in 0..12 {
            dst[d + k] *= sci.scf[s];
            dst[d + k + 576] *= sci.scf[s + 3];
        }
    }
}

// ---------------------------------------------------------------------------
// Layer III

/// Reads a granule's side info for every channel; returns where its main
/// data begins (bytes back into the reservoir), or -1 when it is damaged.
fn read_side_info(bs: &mut Bs, gr: &mut [GrInfo; 4], hdr: &[u8]) -> i32 {
    let mut scfsi: u32 = 0;
    let main_data_begin: u32;
    let mut part_23_sum = 0usize;
    let mut sr_idx = hdr_get_my_sample_rate(hdr) as usize;
    if sr_idx != 0 {
        sr_idx -= 1;
    }
    let mut gr_count = if hdr_is_mono(hdr) { 1 } else { 2 };
    if hdr_test_mpeg1(hdr) {
        gr_count *= 2;
        main_data_begin = bs.get_bits(9);
        scfsi = bs.get_bits(7 + gr_count);
    } else {
        main_data_begin = bs.get_bits(8 + gr_count) >> gr_count;
    }

    for g in gr.iter_mut().take(gr_count as usize) {
        if hdr_is_mono(hdr) {
            scfsi <<= 4;
        }
        g.part_23_length = bs.get_bits(12) as u16;
        part_23_sum += g.part_23_length as usize;
        g.big_values = bs.get_bits(9) as u16;
        if g.big_values > 288 {
            return -1;
        }
        g.global_gain = bs.get_bits(8) as u8;
        g.scalefac_compress = bs.get_bits(if hdr_test_mpeg1(hdr) { 4 } else { 9 }) as u16;
        g.sfbtab = &SCF_LONG[sr_idx];
        g.n_long_sfb = 22;
        g.n_short_sfb = 0;
        let tables;
        if bs.get_bits(1) != 0 {
            g.block_type = bs.get_bits(2) as u8;
            if g.block_type == 0 {
                return -1;
            }
            g.mixed_block_flag = bs.get_bits(1) as u8;
            g.region_count[0] = 7;
            g.region_count[1] = 255;
            if g.block_type == SHORT_BLOCK_TYPE {
                scfsi &= 0x0F0F;
                if g.mixed_block_flag == 0 {
                    g.region_count[0] = 8;
                    g.sfbtab = &SCF_SHORT[sr_idx];
                    g.n_long_sfb = 0;
                    g.n_short_sfb = 39;
                } else {
                    g.sfbtab = &SCF_MIXED[sr_idx];
                    g.n_long_sfb = if hdr_test_mpeg1(hdr) { 8 } else { 6 };
                    g.n_short_sfb = 30;
                }
            }
            tables = bs.get_bits(10) << 5;
            g.subblock_gain[0] = bs.get_bits(3) as u8;
            g.subblock_gain[1] = bs.get_bits(3) as u8;
            g.subblock_gain[2] = bs.get_bits(3) as u8;
        } else {
            g.block_type = 0;
            g.mixed_block_flag = 0;
            tables = bs.get_bits(15);
            g.region_count[0] = bs.get_bits(4) as u8;
            g.region_count[1] = bs.get_bits(3) as u8;
            g.region_count[2] = 255;
        }
        g.table_select[0] = (tables >> 10) as u8;
        g.table_select[1] = ((tables >> 5) & 31) as u8;
        g.table_select[2] = (tables & 31) as u8;
        g.preflag = if hdr_test_mpeg1(hdr) { bs.get_bits(1) as u8 } else { (g.scalefac_compress >= 500) as u8 };
        g.scalefac_scale = bs.get_bits(1) as u8;
        g.count1_table = bs.get_bits(1) as u8;
        g.scfsi = ((scfsi >> 12) & 15) as u8;
        scfsi <<= 4;
    }

    if part_23_sum + bs.pos > bs.limit + main_data_begin as usize * 8 {
        return -1;
    }
    main_data_begin as i32
}

fn read_scalefactors(scf: &mut [u8], ist_pos: &mut [u8], scf_size: &[u8; 4], scf_count: &[u8], bs: &mut Bs, mut scfsi: i32) {
    let mut o = 0;
    for i in 0..4 {
        let cnt = scf_count[i] as usize;
        if cnt == 0 {
            break;
        }
        if scfsi & 8 != 0 {
            scf[o..o + cnt].copy_from_slice(&ist_pos[o..o + cnt]);
        } else {
            let bits = scf_size[i] as u32;
            if bits == 0 {
                scf[o..o + cnt].fill(0);
                ist_pos[o..o + cnt].fill(0);
            } else {
                let max_scf: i32 = if scfsi < 0 { (1 << bits) - 1 } else { -1 };
                for k in 0..cnt {
                    let s = bs.get_bits(bits) as i32;
                    ist_pos[o + k] = if s == max_scf { 255 } else { s as u8 };
                    scf[o + k] = s as u8;
                }
            }
        }
        o += cnt;
        scfsi = scfsi.wrapping_mul(2);
    }
    scf[o] = 0;
    scf[o + 1] = 0;
    scf[o + 2] = 0;
}

/// `y` times 2 to the power `-exp_q2 / 4`.
fn ldexp_q2(mut y: f32, mut exp_q2: i32) -> f32 {
    loop {
        let e = exp_q2.min(30 * 4);
        y *= EXPFRAC[(e & 3) as usize] * ((1i32 << 30) >> (e >> 2)) as f32;
        exp_q2 -= e;
        if exp_q2 <= 0 {
            break;
        }
    }
    y
}

fn decode_scalefactors(hdr: &[u8], ist_pos: &mut [u8; 39], bs: &mut Bs, gr: &GrInfo, scf: &mut [f32; 40], ch: usize) {
    let row = (gr.n_short_sfb != 0) as usize + (gr.n_long_sfb == 0) as usize;
    let mut part_off = 0;
    let mut scf_size = [0u8; 4];
    let mut iscf = [0u8; 48];
    let scf_shift = gr.scalefac_scale as i32 + 1;
    let mut scfsi = gr.scfsi as i32;

    if hdr_test_mpeg1(hdr) {
        let part = SCFC_DECODE[gr.scalefac_compress as usize & 15];
        scf_size[0] = part >> 2;
        scf_size[1] = part >> 2;
        scf_size[2] = part & 3;
        scf_size[3] = part & 3;
    } else {
        let ist = (hdr_test_i_stereo(hdr) && ch != 0) as usize;
        let mut sfc = (gr.scalefac_compress as i32) >> ist;
        let mut k = ist * 3 * 4;
        while sfc >= 0 && k + 4 <= SFC_MOD.len() {
            let mut modprod = 1i32;
            for i in (0..4).rev() {
                let m = SFC_MOD[k + i] as i32;
                scf_size[i] = (sfc / modprod % m) as u8;
                modprod *= m;
            }
            sfc -= modprod;
            k += 4;
        }
        part_off = k;
        scfsi = -16;
    }
    read_scalefactors(&mut iscf, &mut ist_pos[..], &scf_size, &SCF_PARTITIONS[row][part_off.min(24)..], bs, scfsi);

    let n_long = gr.n_long_sfb as usize;
    let n_short = gr.n_short_sfb as usize;
    if n_short != 0 {
        let sh = 3 - scf_shift;
        for i in (0..n_short).step_by(3) {
            for j in 0..3 {
                iscf[n_long + i + j] = iscf[n_long + i + j].wrapping_add(gr.subblock_gain[j] << sh);
            }
        }
    } else if gr.preflag != 0 {
        for (i, p) in PREAMP.iter().enumerate() {
            iscf[11 + i] = iscf[11 + i].wrapping_add(*p);
        }
    }

    let gain_exp = gr.global_gain as i32 + BITS_DEQUANTIZER_OUT * 4 - 210 - if hdr_is_ms_stereo(hdr) { 2 } else { 0 };
    let gain = ldexp_q2((1 << (MAX_SCFI / 4)) as f32, MAX_SCFI - gain_exp);
    for i in 0..n_long + n_short {
        scf[i] = ldexp_q2(gain, (iscf[i] as i32) << scf_shift);
    }
}

/// `x` to the power 4/3 (with its sign in the table below 0).
fn pow_43(mut x: i32) -> f32 {
    let mut mult = 256.0f32;
    if x < 129 {
        return POW43[(16 + x) as usize];
    }
    if x < 1024 {
        mult = 16.0;
        x <<= 3;
    }
    let sign = 2 * x & 64;
    let frac = ((x & 63) - sign) as f32 / ((x & !63) + sign) as f32;
    POW43[(16 + ((x + sign) >> 6)) as usize] * (1.0 + frac * ((4.0 / 3.0) + frac * (2.0 / 9.0))) * mult
}

/// The Huffman-coded spectrum of one channel in one granule, scaled.
fn huffman(dst: &mut [f32], bs: &mut Bs, gr: &GrInfo, scf: &[f32; 40], layer3gr_limit: usize) {
    let buf = bs.buf;
    let rd = |i: usize| buf.get(i).copied().unwrap_or(0) as u32;
    let mut one = 0.0f32;
    let mut ireg = 0;
    let mut big_val_cnt = gr.big_values as i32;
    let sfbtab = gr.sfbtab;
    let mut sfb = 0usize;
    let mut scf_i = 0usize;
    let mut next = bs.pos / 8;
    let mut cache: u32 = (((rd(next) * 256 + rd(next + 1)) * 256 + rd(next + 2)) * 256 + rd(next + 3)) << (bs.pos & 7);
    let mut sh: i32 = (bs.pos & 7) as i32 - 8;
    next += 4;
    let mut d = 0usize;

    macro_rules! peek {
        ($n:expr) => {
            ((cache as u64) >> (32 - ($n) as u32)) as u32
        };
    }
    macro_rules! flush {
        ($n:expr) => {{
            let n = ($n) as u32;
            cache = if n >= 32 { 0 } else { cache << n };
            sh += n as i32;
        }};
    }
    macro_rules! check {
        () => {
            while sh >= 0 {
                cache |= rd(next) << sh;
                next += 1;
                sh -= 8;
            }
        };
    }
    let sfb_at = |i: usize| sfbtab.get(i).copied().unwrap_or(0) as i32;
    let scf_at = |i: usize| scf.get(i).copied().unwrap_or(0.0);

    while big_val_cnt > 0 {
        let tab_num = gr.table_select[ireg.min(2)] as usize;
        let mut sfb_cnt = gr.region_count[ireg.min(2)] as i32;
        ireg += 1;
        let codebook = TAB_INDEX[tab_num] as i32;
        let linbits = LINBITS[tab_num] as u32;
        loop {
            let np = sfb_at(sfb) / 2;
            sfb += 1;
            let mut pairs_to_decode = big_val_cnt.min(np);
            one = scf_at(scf_i);
            scf_i += 1;
            while pairs_to_decode > 0 && d + 1 < dst.len() {
                let mut w = 5u32;
                let mut leaf = TABS[(codebook + peek!(w) as i32) as usize] as i32;
                while leaf < 0 {
                    flush!(w);
                    w = (leaf & 7) as u32;
                    let idx = codebook + peek!(w) as i32 - (leaf >> 3);
                    leaf = TABS.get(idx as usize).copied().unwrap_or(0) as i32;
                }
                flush!(leaf >> 8);
                for _ in 0..2 {
                    let mut lsb = leaf & 0x0F;
                    if linbits != 0 && lsb == 15 {
                        lsb += peek!(linbits) as i32;
                        flush!(linbits);
                        check!();
                        dst[d] = one * pow_43(lsb) * if (cache as i32) < 0 { -1.0 } else { 1.0 };
                    } else {
                        dst[d] = POW43[(16 + lsb - 16 * (cache >> 31) as i32) as usize] * one;
                    }
                    flush!(if lsb != 0 { 1 } else { 0 });
                    d += 1;
                    leaf >>= 4;
                }
                check!();
                pairs_to_decode -= 1;
            }
            big_val_cnt -= np;
            if np == 0 || big_val_cnt <= 0 {
                break;
            }
            sfb_cnt -= 1;
            if sfb_cnt < 0 {
                break;
            }
        }
        if ireg > 3 {
            break;
        }
    }

    let mut np = 1 - big_val_cnt;
    let codebook_count1: &[u8] = if gr.count1_table != 0 { &TAB33 } else { &TAB32 };
    'count1: loop {
        let mut leaf = codebook_count1[peek!(4) as usize] as u32;
        if leaf & 8 == 0 {
            let extra = ((cache << 4) as u64 >> (32 - (leaf & 3))) as u32;
            leaf = codebook_count1[((leaf >> 3) + extra) as usize] as u32;
        }
        flush!(leaf & 7);
        let bspos = (next * 8) as isize - 24 + sh as isize;
        if bspos > layer3gr_limit as isize {
            break;
        }
        for half in 0..2 {
            np -= 1;
            if np == 0 {
                np = sfb_at(sfb) / 2;
                sfb += 1;
                if np == 0 {
                    break 'count1;
                }
                one = scf_at(scf_i);
                scf_i += 1;
            }
            for s in half * 2..half * 2 + 2 {
                if leaf & (128 >> s) != 0 {
                    if let Some(v) = dst.get_mut(d + s) {
                        *v = if (cache as i32) < 0 { -one } else { one };
                    }
                    flush!(1);
                }
            }
        }
        check!();
        d += 4;
    }

    bs.pos = layer3gr_limit;
}

fn midside_stereo(buf: &mut [f32], off: usize, n: usize) {
    for i in off..off + n {
        let a = buf[i];
        let b = buf[i + 576];
        buf[i] = a + b;
        buf[i + 576] = a - b;
    }
}

fn intensity_stereo_band(buf: &mut [f32], off: usize, n: usize, kl: f32, kr: f32) {
    for i in off..off + n {
        buf[i + 576] = buf[i] * kr;
        buf[i] *= kl;
    }
}

fn stereo_top_band(right: &[f32], sfb: &[u8], nbands: usize, max_band: &mut [i32; 3]) {
    *max_band = [-1; 3];
    let mut off = 0;
    for i in 0..nbands {
        let n = sfb[i] as usize;
        let mut k = 0;
        while k < n {
            if right[off + k] != 0.0 || right[off + k + 1] != 0.0 {
                max_band[i % 3] = i as i32;
                break;
            }
            k += 2;
        }
        off += n;
    }
}

fn stereo_process(buf: &mut [f32], ist_pos: &[u8; 39], sfb: &[u8], hdr: &[u8], max_band: &[i32; 3], mpeg2_sh: u32) {
    let max_pos = if hdr_test_mpeg1(hdr) { 7 } else { 64 };
    let mut off = 0;
    let mut i = 0;
    while i < sfb.len() && sfb[i] != 0 && i < ist_pos.len() {
        let n = sfb[i] as usize;
        let ipos = ist_pos[i] as u32;
        if i as i32 > max_band[i % 3] && ipos < max_pos {
            let s = if hdr_test_ms_stereo(hdr) { 1.41421356 } else { 1.0 };
            let (mut kl, mut kr);
            if hdr_test_mpeg1(hdr) {
                kl = PAN[2 * ipos as usize];
                kr = PAN[2 * ipos as usize + 1];
            } else {
                kl = 1.0;
                kr = ldexp_q2(1.0, (((ipos + 1) >> 1) << mpeg2_sh) as i32);
                if ipos & 1 != 0 {
                    kl = kr;
                    kr = 1.0;
                }
            }
            intensity_stereo_band(buf, off, n, kl * s, kr * s);
        } else if hdr_test_ms_stereo(hdr) {
            midside_stereo(buf, off, n);
        }
        off += n;
        i += 1;
    }
}

fn intensity_stereo(buf: &mut [f32], ist_pos: &mut [u8; 39], gr: &[GrInfo], hdr: &[u8]) {
    let mut max_band = [0i32; 3];
    let n_sfb = gr[0].n_long_sfb as usize + gr[0].n_short_sfb as usize;
    let max_blocks = if gr[0].n_short_sfb != 0 { 3 } else { 1 };
    stereo_top_band(&buf[576..], gr[0].sfbtab, n_sfb, &mut max_band);
    if gr[0].n_long_sfb != 0 {
        let m = max_band[0].max(max_band[1]).max(max_band[2]);
        max_band = [m; 3];
    }
    for i in 0..max_blocks {
        let default_pos = if hdr_test_mpeg1(hdr) { 3 } else { 0 };
        let itop = n_sfb - max_blocks + i;
        let prev = itop - max_blocks;
        ist_pos[itop] = if max_band[i] >= prev as i32 { default_pos } else { ist_pos[prev] };
    }
    stereo_process(buf, ist_pos, gr[0].sfbtab, hdr, &max_band, (gr[1].scalefac_compress & 1) as u32);
}

/// Short blocks: from window by window to frequency by frequency.
fn reorder(grbuf: &mut [f32], off: usize, scratch: &mut [f32], sfb: &[u8]) {
    let mut src = off;
    let mut dst = 0;
    let mut j = 0;
    while j < sfb.len() && sfb[j] != 0 {
        let len = sfb[j] as usize;
        for i in 0..len {
            scratch[dst] = grbuf[src + i];
            scratch[dst + 1] = grbuf[src + i + len];
            scratch[dst + 2] = grbuf[src + i + 2 * len];
            dst += 3;
        }
        src += 3 * len;
        j += 3;
    }
    grbuf[off..off + dst].copy_from_slice(&scratch[..dst]);
}

fn antialias(grbuf: &mut [f32], nbands: i32) {
    for b in 0..nbands.max(0) as usize {
        let g = &mut grbuf[b * 18..];
        for i in 0..8 {
            let u = g[18 + i];
            let d = g[17 - i];
            g[18 + i] = u * ANTIALIAS[0][i] - d * ANTIALIAS[1][i];
            g[17 - i] = u * ANTIALIAS[1][i] + d * ANTIALIAS[0][i];
        }
    }
}

fn dct3_9(y: &mut [f32; 9]) {
    let (mut s0, mut s1, mut s2, mut s3, mut s4, mut s5, mut s6, mut s7, mut s8);
    let (mut t0, mut t2, mut t4);

    s0 = y[0];
    s2 = y[2];
    s4 = y[4];
    s6 = y[6];
    s8 = y[8];
    t0 = s0 + s6 * 0.5;
    s0 -= s6;
    t4 = (s4 + s2) * 0.93969262;
    t2 = (s8 + s2) * 0.76604444;
    s6 = (s4 - s8) * 0.17364818;
    s4 += s8 - s2;

    s2 = s0 - s4 * 0.5;
    y[4] = s4 + s0;
    s8 = t0 - t2 + s6;
    s0 = t0 - t4 + t2;
    s4 = t0 + t4 - s6;

    s1 = y[1];
    s3 = y[3];
    s5 = y[5];
    s7 = y[7];

    s3 *= 0.86602540;
    t0 = (s5 + s1) * 0.98480775;
    t4 = (s5 - s7) * 0.34202014;
    t2 = (s1 + s7) * 0.64278761;
    s1 = (s1 - s5 - s7) * 0.86602540;

    s5 = t0 - s3 - t2;
    s7 = t4 - s3 - t0;
    s3 = t4 + s3 - t2;

    y[0] = s4 - s7;
    y[1] = s2 + s1;
    y[2] = s0 - s3;
    y[3] = s8 + s5;
    y[5] = s8 - s5;
    y[6] = s0 + s3;
    y[7] = s2 - s1;
    y[8] = s4 + s7;
}

fn imdct36(grbuf: &mut [f32], overlap: &mut [f32], window: &[f32; 18], nbands: usize) {
    for j in 0..nbands {
        let g = &mut grbuf[j * 18..j * 18 + 18];
        let ov = &mut overlap[j * 9..j * 9 + 9];
        let mut co = [0.0f32; 9];
        let mut si = [0.0f32; 9];
        co[0] = -g[0];
        si[0] = g[17];
        for i in 0..4 {
            si[8 - 2 * i] = g[4 * i + 1] - g[4 * i + 2];
            co[1 + 2 * i] = g[4 * i + 1] + g[4 * i + 2];
            si[7 - 2 * i] = g[4 * i + 4] - g[4 * i + 3];
            co[2 + 2 * i] = -(g[4 * i + 3] + g[4 * i + 4]);
        }
        dct3_9(&mut co);
        dct3_9(&mut si);

        si[1] = -si[1];
        si[3] = -si[3];
        si[5] = -si[5];
        si[7] = -si[7];

        for i in 0..9 {
            let ovl = ov[i];
            let sum = co[i] * TWID9[9 + i] + si[i] * TWID9[i];
            ov[i] = co[i] * TWID9[i] - si[i] * TWID9[9 + i];
            g[i] = ovl * window[i] - sum * window[9 + i];
            g[17 - i] = ovl * window[9 + i] + sum * window[i];
        }
    }
}

fn idct3(x0: f32, x1: f32, x2: f32, dst: &mut [f32; 3]) {
    let m1 = x1 * 0.86602540;
    let a1 = x0 - x2 * 0.5;
    dst[1] = x0 + x2;
    dst[0] = a1 + m1;
    dst[2] = a1 - m1;
}

/// One short window: `x` read every third value, six out to `dst`.
fn imdct12(x: &[f32], dst: &mut [f32], overlap: &mut [f32]) {
    let mut co = [0.0f32; 3];
    let mut si = [0.0f32; 3];
    idct3(-x[0], x[6] + x[3], x[12] + x[9], &mut co);
    idct3(x[15], x[12] - x[9], x[6] - x[3], &mut si);
    si[1] = -si[1];
    for i in 0..3 {
        let ovl = overlap[i];
        let sum = co[i] * TWID3[3 + i] + si[i] * TWID3[i];
        overlap[i] = co[i] * TWID3[i] - si[i] * TWID3[3 + i];
        dst[i] = ovl * TWID3[2 - i] - sum * TWID3[5 - i];
        dst[5 - i] = ovl * TWID3[5 - i] + sum * TWID3[2 - i];
    }
}

fn imdct_short(grbuf: &mut [f32], overlap: &mut [f32], nbands: usize) {
    for b in 0..nbands {
        let g = &mut grbuf[b * 18..b * 18 + 18];
        let ov = &mut overlap[b * 9..b * 9 + 9];
        let mut tmp = [0.0f32; 18];
        tmp.copy_from_slice(g);
        g[..6].copy_from_slice(&ov[..6]);
        let (head, state) = ov.split_at_mut(6);
        imdct12(&tmp, &mut g[6..12], state);
        imdct12(&tmp[1..], &mut g[12..18], state);
        imdct12(&tmp[2..], head, state);
    }
}

fn change_sign(grbuf: &mut [f32]) {
    for b in (0..32).step_by(2) {
        let g = &mut grbuf[18 + b * 18..];
        for i in (1..18).step_by(2) {
            g[i] = -g[i];
        }
    }
}

fn imdct_gr(grbuf: &mut [f32], overlap: &mut [f32], block_type: u8, n_long_bands: usize) {
    if n_long_bands > 0 {
        imdct36(grbuf, overlap, &MDCT_WINDOW[0], n_long_bands);
    }
    let g = &mut grbuf[18 * n_long_bands..];
    let ov = &mut overlap[9 * n_long_bands..];
    if block_type == SHORT_BLOCK_TYPE {
        imdct_short(g, ov, 32 - n_long_bands);
    } else {
        imdct36(g, ov, &MDCT_WINDOW[(block_type == STOP_BLOCK_TYPE) as usize], 32 - n_long_bands);
    }
}

// ---------------------------------------------------------------------------
// The synthesis filter bank

fn dct_ii(grbuf: &mut [f32], n: usize) {
    for k in 0..n {
        let mut t = [[0.0f32; 8]; 4];
        for i in 0..8 {
            let x0 = grbuf[k + i * 18];
            let x1 = grbuf[k + (15 - i) * 18];
            let x2 = grbuf[k + (16 + i) * 18];
            let x3 = grbuf[k + (31 - i) * 18];
            let t0 = x0 + x3;
            let t1 = x1 + x2;
            let t2 = (x1 - x2) * DCT_SEC[3 * i];
            let t3 = (x0 - x3) * DCT_SEC[3 * i + 1];
            t[0][i] = t0 + t1;
            t[1][i] = (t0 - t1) * DCT_SEC[3 * i + 2];
            t[2][i] = t3 + t2;
            t[3][i] = (t3 - t2) * DCT_SEC[3 * i + 2];
        }
        for x in t.iter_mut() {
            let (mut x0, mut x1, mut x2, mut x3, mut x4, mut x5, mut x6, mut x7) = (x[0], x[1], x[2], x[3], x[4], x[5], x[6], x[7]);
            let mut xt = x0 - x7;
            x0 += x7;
            x7 = x1 - x6;
            x1 += x6;
            x6 = x2 - x5;
            x2 += x5;
            x5 = x3 - x4;
            x3 += x4;
            x4 = x0 - x3;
            x0 += x3;
            x3 = x1 - x2;
            x1 += x2;
            x[0] = x0 + x1;
            x[4] = (x0 - x1) * 0.70710677;
            x5 += x6;
            x6 = (x6 + x7) * 0.70710677;
            x7 += xt;
            x3 = (x3 + x4) * 0.70710677;
            x5 -= x7 * 0.198912367; // rotate by PI/8
            x7 += x5 * 0.382683432;
            x5 -= x7 * 0.198912367;
            x0 = xt - x6;
            xt += x6;
            x[1] = (xt + x7) * 0.50979561;
            x[2] = (x4 + x3) * 0.54119611;
            x[3] = (x0 - x5) * 0.60134488;
            x[5] = (x0 + x5) * 0.89997619;
            x[6] = (x4 - x3) * 1.30656302;
            x[7] = (xt - x7) * 2.56291556;
        }
        let mut y = k;
        for i in 0..7 {
            grbuf[y] = t[0][i];
            grbuf[y + 18] = t[2][i] + t[3][i] + t[3][i + 1];
            grbuf[y + 2 * 18] = t[1][i] + t[1][i + 1];
            grbuf[y + 3 * 18] = t[2][i + 1] + t[3][i] + t[3][i + 1];
            y += 4 * 18;
        }
        grbuf[y] = t[0][7];
        grbuf[y + 18] = t[2][7] + t[3][7];
        grbuf[y + 2 * 18] = t[1][7];
        grbuf[y + 3 * 18] = t[3][7];
    }
}

const PCM_SCALE: f32 = 1.0 / 32768.0;

fn synth_pair(pcm: &mut [f32], o: usize, nch: usize, z: &[f32], zo: usize) {
    let mut a;
    a = (z[zo + 14 * 64] - z[zo]) * 29.0;
    a += (z[zo + 64] + z[zo + 13 * 64]) * 213.0;
    a += (z[zo + 12 * 64] - z[zo + 2 * 64]) * 459.0;
    a += (z[zo + 3 * 64] + z[zo + 11 * 64]) * 2037.0;
    a += (z[zo + 10 * 64] - z[zo + 4 * 64]) * 5153.0;
    a += (z[zo + 5 * 64] + z[zo + 9 * 64]) * 6574.0;
    a += (z[zo + 8 * 64] - z[zo + 6 * 64]) * 37489.0;
    a += z[zo + 7 * 64] * 75038.0;
    pcm[o] = a * PCM_SCALE;

    let zo = zo + 2;
    a = z[zo + 14 * 64] * 104.0;
    a += z[zo + 12 * 64] * 1567.0;
    a += z[zo + 10 * 64] * 9727.0;
    a += z[zo + 8 * 64] * 64019.0;
    a += z[zo + 6 * 64] * -9975.0;
    a += z[zo + 4 * 64] * -45.0;
    a += z[zo + 2 * 64] * 146.0;
    a += z[zo] * -5.0;
    pcm[o + 16 * nch] = a * PCM_SCALE;
}

/// 64 samples per channel out of two subband slots (`xl` into `grbuf`).
fn synth(grbuf: &[f32], xl: usize, pcm: &mut [f32], dl: usize, nch: usize, lins: &mut [f32], lb: usize) {
    let xr = xl + 576 * (nch - 1);
    let dr = dl + (nch - 1);
    let zlin = lb + 15 * 64;

    lins[zlin + 4 * 15] = grbuf[xl + 18 * 16];
    lins[zlin + 4 * 15 + 1] = grbuf[xr + 18 * 16];
    lins[zlin + 4 * 15 + 2] = grbuf[xl];
    lins[zlin + 4 * 15 + 3] = grbuf[xr];

    lins[zlin + 4 * 31] = grbuf[xl + 1 + 18 * 16];
    lins[zlin + 4 * 31 + 1] = grbuf[xr + 1 + 18 * 16];
    lins[zlin + 4 * 31 + 2] = grbuf[xl + 1];
    lins[zlin + 4 * 31 + 3] = grbuf[xr + 1];

    synth_pair(pcm, dr, nch, lins, lb + 4 * 15 + 1);
    synth_pair(pcm, dr + 32 * nch, nch, lins, lb + 4 * 15 + 64 + 1);
    synth_pair(pcm, dl, nch, lins, lb + 4 * 15);
    synth_pair(pcm, dl + 32 * nch, nch, lins, lb + 4 * 15 + 64);

    let mut w = 0;
    for i in (0..15).rev() {
        lins[zlin + 4 * i] = grbuf[xl + 18 * (31 - i)];
        lins[zlin + 4 * i + 1] = grbuf[xr + 18 * (31 - i)];
        lins[zlin + 4 * i + 2] = grbuf[xl + 1 + 18 * (31 - i)];
        lins[zlin + 4 * i + 3] = grbuf[xr + 1 + 18 * (31 - i)];
        lins[zlin + 4 * (i + 16)] = grbuf[xl + 1 + 18 * (1 + i)];
        lins[zlin + 4 * (i + 16) + 1] = grbuf[xr + 1 + 18 * (1 + i)];
        lins[zlin + 4 * i - 64 + 2] = grbuf[xl + 18 * (1 + i)];
        lins[zlin + 4 * i - 64 + 3] = grbuf[xr + 18 * (1 + i)];

        let mut a = [0.0f32; 4];
        let mut b = [0.0f32; 4];
        for k in 0..8 {
            let w0 = SYNTH_WIN[w];
            let w1 = SYNTH_WIN[w + 1];
            w += 2;
            let vz = zlin + 4 * i - k * 64;
            let vy = zlin + 4 * i - (15 - k) * 64;
            for j in 0..4 {
                let z = lins[vz + j];
                let y = lins[vy + j];
                if k == 0 {
                    b[j] = z * w1 + y * w0;
                    a[j] = z * w0 - y * w1;
                } else if k & 1 == 1 {
                    b[j] += z * w1 + y * w0;
                    a[j] += y * w1 - z * w0;
                } else {
                    b[j] += z * w1 + y * w0;
                    a[j] += z * w0 - y * w1;
                }
            }
        }

        pcm[dr + (15 - i) * nch] = a[1] * PCM_SCALE;
        pcm[dr + (17 + i) * nch] = b[1] * PCM_SCALE;
        pcm[dl + (15 - i) * nch] = a[0] * PCM_SCALE;
        pcm[dl + (17 + i) * nch] = b[0] * PCM_SCALE;
        pcm[dr + (47 - i) * nch] = a[3] * PCM_SCALE;
        pcm[dr + (49 + i) * nch] = b[3] * PCM_SCALE;
        pcm[dl + (47 - i) * nch] = a[2] * PCM_SCALE;
        pcm[dl + (49 + i) * nch] = b[2] * PCM_SCALE;
    }
}

fn synth_granule(qmf_state: &mut [f32; 960], grbuf: &mut [f32], nbands: usize, nch: usize, pcm: &mut [f32], out: usize, lins: &mut [f32]) {
    for i in 0..nch {
        dct_ii(&mut grbuf[576 * i..576 * (i + 1)], nbands);
    }
    lins[..15 * 64].copy_from_slice(qmf_state);
    for i in (0..nbands).step_by(2) {
        synth(grbuf, i, pcm, out + 32 * nch * i, nch, lins, i * 64);
    }
    if nch == 1 {
        for i in (0..15 * 64).step_by(2) {
            qmf_state[i] = lins[nbands * 64 + i];
        }
    } else {
        qmf_state.copy_from_slice(&lins[nbands * 64..nbands * 64 + 15 * 64]);
    }
}

// ---------------------------------------------------------------------------
// Tables (minimp3's, as they are)

const TABS: [i16; 2164] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    785, 785, 785, 785, 784, 784, 784, 784, 513, 513, 513, 513, 513, 513, 513, 513,
    256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256,
    -255, 1313, 1298, 1282, 785, 785, 785, 785, 784, 784, 784, 784, 769, 769, 769, 769,
    256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256,
    290, 288, -255, 1313, 1298, 1282, 769, 769, 769, 769, 529, 529, 529, 529, 529, 529,
    529, 529, 528, 528, 528, 528, 528, 528, 528, 528, 512, 512, 512, 512, 512, 512,
    512, 512, 290, 288, -253, -318, -351, -367, 785, 785, 785, 785, 784, 784, 784, 784,
    769, 769, 769, 769, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256,
    256, 256, 256, 256, 819, 818, 547, 547, 275, 275, 275, 275, 561, 560, 515, 546,
    289, 274, 288, 258, -254, -287, 1329, 1299, 1314, 1312, 1057, 1057, 1042, 1042, 1026, 1026,
    784, 784, 784, 784, 529, 529, 529, 529, 529, 529, 529, 529, 769, 769, 769, 769,
    768, 768, 768, 768, 563, 560, 306, 306, 291, 259, -252, -413, -477, -542, 1298, -575,
    1041, 1041, 784, 784, 784, 784, 769, 769, 769, 769, 256, 256, 256, 256, 256, 256,
    256, 256, 256, 256, 256, 256, 256, 256, 256, 256, -383, -399, 1107, 1092, 1106, 1061,
    849, 849, 789, 789, 1104, 1091, 773, 773, 1076, 1075, 341, 340, 325, 309, 834, 804,
    577, 577, 532, 532, 516, 516, 832, 818, 803, 816, 561, 561, 531, 531, 515, 546,
    289, 289, 288, 258, -252, -429, -493, -559, 1057, 1057, 1042, 1042, 529, 529, 529, 529,
    529, 529, 529, 529, 784, 784, 784, 784, 769, 769, 769, 769, 512, 512, 512, 512,
    512, 512, 512, 512, -382, 1077, -415, 1106, 1061, 1104, 849, 849, 789, 789, 1091, 1076,
    1029, 1075, 834, 834, 597, 581, 340, 340, 339, 324, 804, 833, 532, 532, 832, 772,
    818, 803, 817, 787, 816, 771, 290, 290, 290, 290, 288, 258, -253, -349, -414, -447,
    -463, 1329, 1299, -479, 1314, 1312, 1057, 1057, 1042, 1042, 1026, 1026, 785, 785, 785, 785,
    784, 784, 784, 784, 769, 769, 769, 769, 768, 768, 768, 768, -319, 851, 821, -335,
    836, 850, 805, 849, 341, 340, 325, 336, 533, 533, 579, 579, 564, 564, 773, 832,
    578, 548, 563, 516, 321, 276, 306, 291, 304, 259, -251, -572, -733, -830, -863, -879,
    1041, 1041, 784, 784, 784, 784, 769, 769, 769, 769, 256, 256, 256, 256, 256, 256,
    256, 256, 256, 256, 256, 256, 256, 256, 256, 256, -511, -527, -543, 1396, 1351, 1381,
    1366, 1395, 1335, 1380, -559, 1334, 1138, 1138, 1063, 1063, 1350, 1392, 1031, 1031, 1062, 1062,
    1364, 1363, 1120, 1120, 1333, 1348, 881, 881, 881, 881, 375, 374, 359, 373, 343, 358,
    341, 325, 791, 791, 1123, 1122, -703, 1105, 1045, -719, 865, 865, 790, 790, 774, 774,
    1104, 1029, 338, 293, 323, 308, -799, -815, 833, 788, 772, 818, 803, 816, 322, 292,
    307, 320, 561, 531, 515, 546, 289, 274, 288, 258, -251, -525, -605, -685, -765, -831,
    -846, 1298, 1057, 1057, 1312, 1282, 785, 785, 785, 785, 784, 784, 784, 784, 769, 769,
    769, 769, 512, 512, 512, 512, 512, 512, 512, 512, 1399, 1398, 1383, 1367, 1382, 1396,
    1351, -511, 1381, 1366, 1139, 1139, 1079, 1079, 1124, 1124, 1364, 1349, 1363, 1333, 882, 882,
    882, 882, 807, 807, 807, 807, 1094, 1094, 1136, 1136, 373, 341, 535, 535, 881, 775,
    867, 822, 774, -591, 324, 338, -671, 849, 550, 550, 866, 864, 609, 609, 293, 336,
    534, 534, 789, 835, 773, -751, 834, 804, 308, 307, 833, 788, 832, 772, 562, 562,
    547, 547, 305, 275, 560, 515, 290, 290, -252, -397, -477, -557, -622, -653, -719, -735,
    -750, 1329, 1299, 1314, 1057, 1057, 1042, 1042, 1312, 1282, 1024, 1024, 785, 785, 785, 785,
    784, 784, 784, 784, 769, 769, 769, 769, -383, 1127, 1141, 1111, 1126, 1140, 1095, 1110,
    869, 869, 883, 883, 1079, 1109, 882, 882, 375, 374, 807, 868, 838, 881, 791, -463,
    867, 822, 368, 263, 852, 837, 836, -543, 610, 610, 550, 550, 352, 336, 534, 534,
    865, 774, 851, 821, 850, 805, 593, 533, 579, 564, 773, 832, 578, 578, 548, 548,
    577, 577, 307, 276, 306, 291, 516, 560, 259, 259, -250, -2107, -2507, -2764, -2909, -2974,
    -3007, -3023, 1041, 1041, 1040, 1040, 769, 769, 769, 769, 256, 256, 256, 256, 256, 256,
    256, 256, 256, 256, 256, 256, 256, 256, 256, 256, -767, -1052, -1213, -1277, -1358, -1405,
    -1469, -1535, -1550, -1582, -1614, -1647, -1662, -1694, -1726, -1759, -1774, -1807, -1822, -1854, -1886, 1565,
    -1919, -1935, -1951, -1967, 1731, 1730, 1580, 1717, -1983, 1729, 1564, -1999, 1548, -2015, -2031, 1715,
    1595, -2047, 1714, -2063, 1610, -2079, 1609, -2095, 1323, 1323, 1457, 1457, 1307, 1307, 1712, 1547,
    1641, 1700, 1699, 1594, 1685, 1625, 1442, 1442, 1322, 1322, -780, -973, -910, 1279, 1278, 1277,
    1262, 1276, 1261, 1275, 1215, 1260, 1229, -959, 974, 974, 989, 989, -943, 735, 478, 478,
    495, 463, 506, 414, -1039, 1003, 958, 1017, 927, 942, 987, 957, 431, 476, 1272, 1167,
    1228, -1183, 1256, -1199, 895, 895, 941, 941, 1242, 1227, 1212, 1135, 1014, 1014, 490, 489,
    503, 487, 910, 1013, 985, 925, 863, 894, 970, 955, 1012, 847, -1343, 831, 755, 755,
    984, 909, 428, 366, 754, 559, -1391, 752, 486, 457, 924, 997, 698, 698, 983, 893,
    740, 740, 908, 877, 739, 739, 667, 667, 953, 938, 497, 287, 271, 271, 683, 606,
    590, 712, 726, 574, 302, 302, 738, 736, 481, 286, 526, 725, 605, 711, 636, 724,
    696, 651, 589, 681, 666, 710, 364, 467, 573, 695, 466, 466, 301, 465, 379, 379,
    709, 604, 665, 679, 316, 316, 634, 633, 436, 436, 464, 269, 424, 394, 452, 332,
    438, 363, 347, 408, 393, 448, 331, 422, 362, 407, 392, 421, 346, 406, 391, 376,
    375, 359, 1441, 1306, -2367, 1290, -2383, 1337, -2399, -2415, 1426, 1321, -2431, 1411, 1336, -2447,
    -2463, -2479, 1169, 1169, 1049, 1049, 1424, 1289, 1412, 1352, 1319, -2495, 1154, 1154, 1064, 1064,
    1153, 1153, 416, 390, 360, 404, 403, 389, 344, 374, 373, 343, 358, 372, 327, 357,
    342, 311, 356, 326, 1395, 1394, 1137, 1137, 1047, 1047, 1365, 1392, 1287, 1379, 1334, 1364,
    1349, 1378, 1318, 1363, 792, 792, 792, 792, 1152, 1152, 1032, 1032, 1121, 1121, 1046, 1046,
    1120, 1120, 1030, 1030, -2895, 1106, 1061, 1104, 849, 849, 789, 789, 1091, 1076, 1029, 1090,
    1060, 1075, 833, 833, 309, 324, 532, 532, 832, 772, 818, 803, 561, 561, 531, 560,
    515, 546, 289, 274, 288, 258, -250, -1179, -1579, -1836, -1996, -2124, -2253, -2333, -2413, -2477,
    -2542, -2574, -2607, -2622, -2655, 1314, 1313, 1298, 1312, 1282, 785, 785, 785, 785, 1040, 1040,
    1025, 1025, 768, 768, 768, 768, -766, -798, -830, -862, -895, -911, -927, -943, -959, -975,
    -991, -1007, -1023, -1039, -1055, -1070, 1724, 1647, -1103, -1119, 1631, 1767, 1662, 1738, 1708, 1723,
    -1135, 1780, 1615, 1779, 1599, 1677, 1646, 1778, 1583, -1151, 1777, 1567, 1737, 1692, 1765, 1722,
    1707, 1630, 1751, 1661, 1764, 1614, 1736, 1676, 1763, 1750, 1645, 1598, 1721, 1691, 1762, 1706,
    1582, 1761, 1566, -1167, 1749, 1629, 767, 766, 751, 765, 494, 494, 735, 764, 719, 749,
    734, 763, 447, 447, 748, 718, 477, 506, 431, 491, 446, 476, 461, 505, 415, 430,
    475, 445, 504, 399, 460, 489, 414, 503, 383, 474, 429, 459, 502, 502, 746, 752,
    488, 398, 501, 473, 413, 472, 486, 271, 480, 270, -1439, -1455, 1357, -1471, -1487, -1503,
    1341, 1325, -1519, 1489, 1463, 1403, 1309, -1535, 1372, 1448, 1418, 1476, 1356, 1462, 1387, -1551,
    1475, 1340, 1447, 1402, 1386, -1567, 1068, 1068, 1474, 1461, 455, 380, 468, 440, 395, 425,
    410, 454, 364, 467, 466, 464, 453, 269, 409, 448, 268, 432, 1371, 1473, 1432, 1417,
    1308, 1460, 1355, 1446, 1459, 1431, 1083, 1083, 1401, 1416, 1458, 1445, 1067, 1067, 1370, 1457,
    1051, 1051, 1291, 1430, 1385, 1444, 1354, 1415, 1400, 1443, 1082, 1082, 1173, 1113, 1186, 1066,
    1185, 1050, -1967, 1158, 1128, 1172, 1097, 1171, 1081, -1983, 1157, 1112, 416, 266, 375, 400,
    1170, 1142, 1127, 1065, 793, 793, 1169, 1033, 1156, 1096, 1141, 1111, 1155, 1080, 1126, 1140,
    898, 898, 808, 808, 897, 897, 792, 792, 1095, 1152, 1032, 1125, 1110, 1139, 1079, 1124,
    882, 807, 838, 881, 853, 791, -2319, 867, 368, 263, 822, 852, 837, 866, 806, 865,
    -2399, 851, 352, 262, 534, 534, 821, 836, 594, 594, 549, 549, 593, 593, 533, 533,
    848, 773, 579, 579, 564, 578, 548, 563, 276, 276, 577, 576, 306, 291, 516, 560,
    305, 305, 275, 259, -251, -892, -2058, -2620, -2828, -2957, -3023, -3039, 1041, 1041, 1040, 1040,
    769, 769, 769, 769, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256,
    256, 256, 256, 256, -511, -527, -543, -559, 1530, -575, -591, 1528, 1527, 1407, 1526, 1391,
    1023, 1023, 1023, 1023, 1525, 1375, 1268, 1268, 1103, 1103, 1087, 1087, 1039, 1039, 1523, -604,
    815, 815, 815, 815, 510, 495, 509, 479, 508, 463, 507, 447, 431, 505, 415, 399,
    -734, -782, 1262, -815, 1259, 1244, -831, 1258, 1228, -847, -863, 1196, -879, 1253, 987, 987,
    748, -767, 493, 493, 462, 477, 414, 414, 686, 669, 478, 446, 461, 445, 474, 429,
    487, 458, 412, 471, 1266, 1264, 1009, 1009, 799, 799, -1019, -1276, -1452, -1581, -1677, -1757,
    -1821, -1886, -1933, -1997, 1257, 1257, 1483, 1468, 1512, 1422, 1497, 1406, 1467, 1496, 1421, 1510,
    1134, 1134, 1225, 1225, 1466, 1451, 1374, 1405, 1252, 1252, 1358, 1480, 1164, 1164, 1251, 1251,
    1238, 1238, 1389, 1465, -1407, 1054, 1101, -1423, 1207, -1439, 830, 830, 1248, 1038, 1237, 1117,
    1223, 1148, 1236, 1208, 411, 426, 395, 410, 379, 269, 1193, 1222, 1132, 1235, 1221, 1116,
    976, 976, 1192, 1162, 1177, 1220, 1131, 1191, 963, 963, -1647, 961, 780, -1663, 558, 558,
    994, 993, 437, 408, 393, 407, 829, 978, 813, 797, 947, -1743, 721, 721, 377, 392,
    844, 950, 828, 890, 706, 706, 812, 859, 796, 960, 948, 843, 934, 874, 571, 571,
    -1919, 690, 555, 689, 421, 346, 539, 539, 944, 779, 918, 873, 932, 842, 903, 888,
    570, 570, 931, 917, 674, 674, -2575, 1562, -2591, 1609, -2607, 1654, 1322, 1322, 1441, 1441,
    1696, 1546, 1683, 1593, 1669, 1624, 1426, 1426, 1321, 1321, 1639, 1680, 1425, 1425, 1305, 1305,
    1545, 1668, 1608, 1623, 1667, 1592, 1638, 1666, 1320, 1320, 1652, 1607, 1409, 1409, 1304, 1304,
    1288, 1288, 1664, 1637, 1395, 1395, 1335, 1335, 1622, 1636, 1394, 1394, 1319, 1319, 1606, 1621,
    1392, 1392, 1137, 1137, 1137, 1137, 345, 390, 360, 375, 404, 373, 1047, -2751, -2767, -2783,
    1062, 1121, 1046, -2799, 1077, -2815, 1106, 1061, 789, 789, 1105, 1104, 263, 355, 310, 340,
    325, 354, 352, 262, 339, 324, 1091, 1076, 1029, 1090, 1060, 1075, 833, 833, 788, 788,
    1088, 1028, 818, 818, 803, 803, 561, 561, 531, 531, 816, 771, 546, 546, 289, 274,
    288, 258, -253, -317, -381, -446, -478, -509, 1279, 1279, -811, -1179, -1451, -1756, -1900, -2028,
    -2189, -2253, -2333, -2414, -2445, -2511, -2526, 1313, 1298, -2559, 1041, 1041, 1040, 1040, 1025, 1025,
    1024, 1024, 1022, 1007, 1021, 991, 1020, 975, 1019, 959, 687, 687, 1018, 1017, 671, 671,
    655, 655, 1016, 1015, 639, 639, 758, 758, 623, 623, 757, 607, 756, 591, 755, 575,
    754, 559, 543, 543, 1009, 783, -575, -621, -685, -749, 496, -590, 750, 749, 734, 748,
    974, 989, 1003, 958, 988, 973, 1002, 942, 987, 957, 972, 1001, 926, 986, 941, 971,
    956, 1000, 910, 985, 925, 999, 894, 970, -1071, -1087, -1102, 1390, -1135, 1436, 1509, 1451,
    1374, -1151, 1405, 1358, 1480, 1420, -1167, 1507, 1494, 1389, 1342, 1465, 1435, 1450, 1326, 1505,
    1310, 1493, 1373, 1479, 1404, 1492, 1464, 1419, 428, 443, 472, 397, 736, 526, 464, 464,
    486, 457, 442, 471, 484, 482, 1357, 1449, 1434, 1478, 1388, 1491, 1341, 1490, 1325, 1489,
    1463, 1403, 1309, 1477, 1372, 1448, 1418, 1433, 1476, 1356, 1462, 1387, -1439, 1475, 1340, 1447,
    1402, 1474, 1324, 1461, 1371, 1473, 269, 448, 1432, 1417, 1308, 1460, -1711, 1459, -1727, 1441,
    1099, 1099, 1446, 1386, 1431, 1401, -1743, 1289, 1083, 1083, 1160, 1160, 1458, 1445, 1067, 1067,
    1370, 1457, 1307, 1430, 1129, 1129, 1098, 1098, 268, 432, 267, 416, 266, 400, -1887, 1144,
    1187, 1082, 1173, 1113, 1186, 1066, 1050, 1158, 1128, 1143, 1172, 1097, 1171, 1081, 420, 391,
    1157, 1112, 1170, 1142, 1127, 1065, 1169, 1049, 1156, 1096, 1141, 1111, 1155, 1080, 1126, 1154,
    1064, 1153, 1140, 1095, 1048, -2159, 1125, 1110, 1137, -2175, 823, 823, 1139, 1138, 807, 807,
    384, 264, 368, 263, 868, 838, 853, 791, 867, 822, 852, 837, 866, 806, 865, 790,
    -2319, 851, 821, 836, 352, 262, 850, 805, 849, -2399, 533, 533, 835, 820, 336, 261,
    578, 548, 563, 577, 532, 532, 832, 772, 562, 562, 547, 547, 305, 275, 560, 515,
    290, 290, 288, 258,
];

const TAB32: [u8; 28] = [
    130, 162, 193, 209, 44, 28, 76, 140, 9, 9, 9, 9, 9, 9, 9, 9,
    190, 254, 222, 238, 126, 94, 157, 157, 109, 61, 173, 205,
];

const TAB33: [u8; 16] = [
    252, 236, 220, 204, 188, 172, 156, 140, 124, 108, 92, 76, 60, 44, 28, 12,
];

const TAB_INDEX: [i16; 32] = [
    0, 32, 64, 98, 0, 132, 180, 218, 292, 364, 426, 538, 648, 746, 0, 1126,
    1460, 1460, 1460, 1460, 1460, 1460, 1460, 1460, 1842, 1842, 1842, 1842, 1842, 1842, 1842, 1842,
];

const LINBITS: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    1, 2, 3, 4, 6, 8, 10, 13, 4, 5, 6, 7, 8, 9, 11, 13,
];

const POW43: [f32; 145] = [
    0.0, -1.0, -2.519842, -4.326749, -6.349604, -8.549880, -10.902724, -13.390518, -16.000000, -18.720754, -21.544347, -24.463781, -27.473142, -30.567351, -33.741992, -36.993181,
    0.0, 1.0, 2.519842, 4.326749, 6.349604, 8.549880, 10.902724, 13.390518, 16.000000, 18.720754, 21.544347, 24.463781, 27.473142, 30.567351, 33.741992, 36.993181,
    40.317474, 43.711787, 47.173345, 50.699631, 54.288352, 57.937408, 61.644865, 65.408941, 69.227979, 73.100443, 77.024898, 81.000000, 85.024491, 89.097188, 93.216975, 97.382800,
    101.593667, 105.848633, 110.146801, 114.487321, 118.869381, 123.292209, 127.755065, 132.257246, 136.798076, 141.376907, 145.993119, 150.646117, 155.335327, 160.060199, 164.820202, 169.614826,
    174.443577, 179.305980, 184.201575, 189.129918, 194.090580, 199.083145, 204.107210, 209.162385, 214.248292, 219.364564, 224.510845, 229.686789, 234.892058, 240.126328, 245.389280, 250.680604,
    256.000000, 261.347174, 266.721841, 272.123723, 277.552547, 283.008049, 288.489971, 293.998060, 299.532071, 305.091761, 310.676898, 316.287249, 321.922592, 327.582707, 333.267377, 338.976394,
    344.709550, 350.466646, 356.247482, 362.051866, 367.879608, 373.730522, 379.604427, 385.501143, 391.420496, 397.362314, 403.326427, 409.312672, 415.320884, 421.350905, 427.402579, 433.475750,
    439.570269, 445.685987, 451.822757, 457.980436, 464.158883, 470.357960, 476.577530, 482.817459, 489.077615, 495.357868, 501.658090, 507.978156, 514.317941, 520.677324, 527.056184, 533.454404,
    539.871867, 546.308458, 552.764065, 559.238575, 565.731879, 572.243870, 578.774440, 585.323483, 591.890898, 598.476581, 605.080431, 611.702349, 618.342238, 625.000000, 631.675540, 638.368763,
    645.079578,
];

const SYNTH_WIN: [f32; 240] = [
    -1.0, 26.0, -31.0, 208.0, 218.0, 401.0, -519.0, 2063.0, 2000.0, 4788.0, -5517.0, 7134.0, 5959.0, 35640.0, -39336.0, 74992.0,
    -1.0, 24.0, -35.0, 202.0, 222.0, 347.0, -581.0, 2080.0, 1952.0, 4425.0, -5879.0, 7640.0, 5288.0, 33791.0, -41176.0, 74856.0,
    -1.0, 21.0, -38.0, 196.0, 225.0, 294.0, -645.0, 2087.0, 1893.0, 4063.0, -6237.0, 8092.0, 4561.0, 31947.0, -43006.0, 74630.0,
    -1.0, 19.0, -41.0, 190.0, 227.0, 244.0, -711.0, 2085.0, 1822.0, 3705.0, -6589.0, 8492.0, 3776.0, 30112.0, -44821.0, 74313.0,
    -1.0, 17.0, -45.0, 183.0, 228.0, 197.0, -779.0, 2075.0, 1739.0, 3351.0, -6935.0, 8840.0, 2935.0, 28289.0, -46617.0, 73908.0,
    -1.0, 16.0, -49.0, 176.0, 228.0, 153.0, -848.0, 2057.0, 1644.0, 3004.0, -7271.0, 9139.0, 2037.0, 26482.0, -48390.0, 73415.0,
    -2.0, 14.0, -53.0, 169.0, 227.0, 111.0, -919.0, 2032.0, 1535.0, 2663.0, -7597.0, 9389.0, 1082.0, 24694.0, -50137.0, 72835.0,
    -2.0, 13.0, -58.0, 161.0, 224.0, 72.0, -991.0, 2001.0, 1414.0, 2330.0, -7910.0, 9592.0, 70.0, 22929.0, -51853.0, 72169.0,
    -2.0, 11.0, -63.0, 154.0, 221.0, 36.0, -1064.0, 1962.0, 1280.0, 2006.0, -8209.0, 9750.0, -998.0, 21189.0, -53534.0, 71420.0,
    -2.0, 10.0, -68.0, 147.0, 215.0, 2.0, -1137.0, 1919.0, 1131.0, 1692.0, -8491.0, 9863.0, -2122.0, 19478.0, -55178.0, 70590.0,
    -3.0, 9.0, -73.0, 139.0, 208.0, -29.0, -1210.0, 1870.0, 970.0, 1388.0, -8755.0, 9935.0, -3300.0, 17799.0, -56778.0, 69679.0,
    -3.0, 8.0, -79.0, 132.0, 200.0, -57.0, -1283.0, 1817.0, 794.0, 1095.0, -8998.0, 9966.0, -4533.0, 16155.0, -58333.0, 68692.0,
    -4.0, 7.0, -85.0, 125.0, 189.0, -83.0, -1356.0, 1759.0, 605.0, 814.0, -9219.0, 9959.0, -5818.0, 14548.0, -59838.0, 67629.0,
    -4.0, 7.0, -91.0, 117.0, 177.0, -106.0, -1428.0, 1698.0, 402.0, 545.0, -9416.0, 9916.0, -7154.0, 12980.0, -61289.0, 66494.0,
    -5.0, 6.0, -97.0, 111.0, 163.0, -127.0, -1498.0, 1634.0, 185.0, 288.0, -9585.0, 9838.0, -8540.0, 11455.0, -62684.0, 65290.0,
];

const BITALLOC_CODE_TAB: [u8; 92] = [
    0, 17, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
    0, 17, 18, 3, 19, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 16,
    0, 17, 18, 3, 19, 4, 5, 16, 0, 17, 18, 16, 0, 17, 18, 19,
    4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 17, 18, 3,
    19, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 0, 2, 3, 4,
    5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
];

const DCT_SEC: [f32; 24] = [
    10.19000816, 0.50060302, 0.50241929, 3.40760851, 0.50547093, 0.52249861, 2.05778098, 0.51544732, 0.56694406, 1.48416460, 0.53104258, 0.64682180, 1.16943991, 0.55310392, 0.78815460, 0.97256821,
    0.58293498, 1.06067765, 0.83934963, 0.62250412, 1.72244716, 0.74453628, 0.67480832, 5.10114861,
];

const TWID9: [f32; 18] = [
    0.73727734, 0.79335334, 0.84339145, 0.88701083, 0.92387953, 0.95371695, 0.97629601, 0.99144486, 0.99904822, 0.67559021, 0.60876143, 0.53729961, 0.46174861, 0.38268343, 0.30070580, 0.21643961,
    0.13052619, 0.04361938,
];

const TWID3: [f32; 6] = [
    0.79335334, 0.92387953, 0.99144486, 0.60876143, 0.38268343, 0.13052619,
];

const PAN: [f32; 14] = [
    0.0, 1.0, 0.21132487, 0.78867513, 0.36602540, 0.63397460, 0.5, 0.5, 0.63397460, 0.36602540, 0.78867513, 0.21132487, 1.0, 0.0,
];

const EXPFRAC: [f32; 4] = [
    9.31322575e-10, 7.83145814e-10, 6.58544508e-10, 5.53767716e-10,
];

const PREAMP: [u8; 10] = [
    1, 1, 1, 1, 2, 2, 3, 3, 3, 2,
];

const SFC_MOD: [u8; 24] = [
    5, 5, 4, 4, 5, 5, 4, 1, 4, 3, 1, 1, 5, 6, 6, 1,
    4, 4, 4, 1, 4, 3, 1, 1,
];

const SCFC_DECODE: [u8; 16] = [
    0, 1, 2, 3, 12, 5, 6, 7, 9, 10, 11, 13, 14, 15, 18, 19,
];

const SCF_LONG: [[u8; 23]; 8] = [
    [6, 6, 6, 6, 6, 6, 8, 10, 12, 14, 16, 20, 24, 28, 32, 38, 46, 52, 60, 68, 58, 54, 0],
    [12, 12, 12, 12, 12, 12, 16, 20, 24, 28, 32, 40, 48, 56, 64, 76, 90, 2, 2, 2, 2, 2, 0],
    [6, 6, 6, 6, 6, 6, 8, 10, 12, 14, 16, 20, 24, 28, 32, 38, 46, 52, 60, 68, 58, 54, 0],
    [6, 6, 6, 6, 6, 6, 8, 10, 12, 14, 16, 18, 22, 26, 32, 38, 46, 54, 62, 70, 76, 36, 0],
    [6, 6, 6, 6, 6, 6, 8, 10, 12, 14, 16, 20, 24, 28, 32, 38, 46, 52, 60, 68, 58, 54, 0],
    [4, 4, 4, 4, 4, 4, 6, 6, 8, 8, 10, 12, 16, 20, 24, 28, 34, 42, 50, 54, 76, 158, 0],
    [4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 10, 12, 16, 18, 22, 28, 34, 40, 46, 54, 54, 192, 0],
    [4, 4, 4, 4, 4, 4, 6, 6, 8, 10, 12, 16, 20, 24, 30, 38, 46, 56, 68, 84, 102, 26, 0],
];

const SCF_SHORT: [[u8; 40]; 8] = [
    [4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18, 18, 24, 24, 24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0],
    [8, 8, 8, 8, 8, 8, 8, 8, 8, 12, 12, 12, 16, 16, 16, 20, 20, 20, 24, 24, 24, 28, 28, 28, 36, 36, 36, 2, 2, 2, 2, 2, 2, 2, 2, 2, 26, 26, 26, 0],
    [4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 14, 14, 14, 18, 18, 18, 26, 26, 26, 32, 32, 32, 42, 42, 42, 18, 18, 18, 0],
    [4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18, 18, 24, 24, 24, 32, 32, 32, 44, 44, 44, 12, 12, 12, 0],
    [4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18, 18, 24, 24, 24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0],
    [4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18, 18, 22, 22, 22, 30, 30, 30, 56, 56, 56, 0],
    [4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 6, 6, 6, 10, 10, 10, 12, 12, 12, 14, 14, 14, 16, 16, 16, 20, 20, 20, 26, 26, 26, 66, 66, 66, 0],
    [4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 12, 12, 12, 16, 16, 16, 20, 20, 20, 26, 26, 26, 34, 34, 34, 42, 42, 42, 12, 12, 12, 0],
];

const SCF_MIXED: [[u8; 40]; 8] = [
    [6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18, 18, 24, 24, 24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0, 0, 0, 0],
    [12, 12, 12, 4, 4, 4, 8, 8, 8, 12, 12, 12, 16, 16, 16, 20, 20, 20, 24, 24, 24, 28, 28, 28, 36, 36, 36, 2, 2, 2, 2, 2, 2, 2, 2, 2, 26, 26, 26, 0],
    [6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 14, 14, 14, 18, 18, 18, 26, 26, 26, 32, 32, 32, 42, 42, 42, 18, 18, 18, 0, 0, 0, 0],
    [6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18, 18, 24, 24, 24, 32, 32, 32, 44, 44, 44, 12, 12, 12, 0, 0, 0, 0],
    [6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18, 18, 24, 24, 24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0, 0, 0, 0],
    [4, 4, 4, 4, 4, 4, 6, 6, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18, 18, 22, 22, 22, 30, 30, 30, 56, 56, 56, 0, 0],
    [4, 4, 4, 4, 4, 4, 6, 6, 4, 4, 4, 6, 6, 6, 6, 6, 6, 10, 10, 10, 12, 12, 12, 14, 14, 14, 16, 16, 16, 20, 20, 20, 26, 26, 26, 66, 66, 66, 0, 0],
    [4, 4, 4, 4, 4, 4, 6, 6, 4, 4, 4, 6, 6, 6, 8, 8, 8, 12, 12, 12, 16, 16, 16, 20, 20, 20, 26, 26, 26, 34, 34, 34, 42, 42, 42, 12, 12, 12, 0, 0],
];

const SCF_PARTITIONS: [[u8; 28]; 3] = [
    [6, 5, 5, 5, 6, 5, 5, 5, 6, 5, 7, 3, 11, 10, 0, 0, 7, 7, 7, 0, 6, 6, 6, 3, 8, 8, 5, 0],
    [8, 9, 6, 12, 6, 9, 9, 9, 6, 9, 12, 6, 15, 18, 0, 0, 6, 15, 12, 0, 6, 12, 9, 6, 6, 18, 9, 0],
    [9, 9, 6, 12, 9, 9, 9, 9, 9, 9, 12, 6, 18, 18, 0, 0, 12, 12, 12, 0, 12, 9, 9, 6, 15, 12, 9, 0],
];

const MDCT_WINDOW: [[f32; 18]; 2] = [
    [0.99904822, 0.99144486, 0.97629601, 0.95371695, 0.92387953, 0.88701083, 0.84339145, 0.79335334, 0.73727734, 0.04361938, 0.13052619, 0.21643961, 0.30070580, 0.38268343, 0.46174861, 0.53729961, 0.60876143, 0.67559021],
    [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.99144486, 0.92387953, 0.79335334, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.13052619, 0.38268343, 0.60876143],
];

const ANTIALIAS: [[f32; 8]; 2] = [
    [0.85749293, 0.88174200, 0.94962865, 0.98331459, 0.99551782, 0.99916056, 0.99989920, 0.99999316],
    [0.51449576, 0.47173197, 0.31337745, 0.18191320, 0.09457419, 0.04096558, 0.01419856, 0.00369997],
];


#[cfg(test)]
mod tests {
    use super::*;

    /// Writes bits, most significant first.
    #[derive(Default)]
    struct BitWriter {
        bytes: Vec<u8>,
        bits: usize,
    }

    impl BitWriter {
        fn put(&mut self, v: u32, n: u32) {
            for i in (0..n).rev() {
                if self.bits % 8 == 0 {
                    self.bytes.push(0);
                }
                if (v >> i) & 1 != 0 {
                    *self.bytes.last_mut().unwrap() |= 0x80 >> (self.bits % 8);
                }
                self.bits += 1;
            }
        }
    }

    /// MPEG-1 Layer III, 128 kbit/s, 44.1 kHz, mono, no CRC: 417 bytes a frame.
    const MONO_128: [u8; 4] = [0xFF, 0xFB, 0x90, 0xC0];
    const FRAME: usize = 417;
    /// Header and mono MPEG-1 side info.
    const MAIN_START: usize = 4 + 17;

    /// One granule's spectrum: a single line of value 1 (Huffman table 1) at
    /// `line`, or nothing. Returns its big_values and how many bits it took.
    fn granule(w: &mut BitWriter, line: Option<usize>) -> (u32, u32) {
        let Some(k) = line else { return (0, 0) };
        let start = w.bits;
        for _ in 0..k / 2 {
            w.put(0b1, 1); // (0, 0)
        }
        if k % 2 == 0 {
            w.put(0b01, 2); // (1, 0)
        } else {
            w.put(0b001, 3); // (0, 1)
        }
        w.put(0, 1); // its sign: +
        ((k / 2 + 1) as u32, (w.bits - start) as u32)
    }

    /// The side info of a mono MPEG-1 frame whose granules are (part2_3_length, big_values).
    fn side_info(main_data_begin: u32, grs: [(u32, u32); 2], global_gain: u32) -> Vec<u8> {
        let mut w = BitWriter::default();
        w.put(main_data_begin, 9);
        w.put(0, 5); // private bits
        w.put(0, 4); // scfsi
        for (part23, big) in grs {
            w.put(part23, 12);
            w.put(big, 9);
            w.put(global_gain, 8);
            w.put(0, 4); // scalefac_compress: no scale factors
            w.put(0, 1); // no window switching
            w.put(1, 5); // table_select: 1, 0, 0
            w.put(0, 5);
            w.put(0, 5);
            w.put(15, 4); // region0_count
            w.put(7, 3); // region1_count
            w.put(0, 3); // preflag, scalefac_scale, count1table
        }
        assert_eq!(w.bytes.len(), 17);
        w.bytes
    }

    /// A frame's side info and main data (both granules), the main data
    /// padded to `pad` bytes.
    fn frame_parts(line: Option<usize>, global_gain: u32, main_data_begin: u32, pad: usize) -> (Vec<u8>, Vec<u8>) {
        let mut w = BitWriter::default();
        let g0 = granule(&mut w, line);
        let g1 = granule(&mut w, line);
        let mut main = w.bytes;
        main.resize(main.len().max(pad), 0);
        (side_info(main_data_begin, [(g0.1, g0.0), (g1.1, g1.0)], global_gain), main)
    }

    /// `n` frames of a steady tone, each frame's main data in itself.
    fn tone_stream(n: usize, line: usize, global_gain: u32) -> Vec<u8> {
        let mut out = Vec::new();
        for _ in 0..n {
            let (si, main) = frame_parts(Some(line), global_gain, 0, 0);
            let mut f = MONO_128.to_vec();
            f.extend_from_slice(&si);
            f.extend_from_slice(&main);
            f.resize(FRAME, 0);
            out.extend_from_slice(&f);
        }
        out
    }

    /// An Info frame with a LAME tag: `frames` frames follow, the encoder
    /// added `delay` samples at the start and `padding` at the end.
    fn info_frame(frames: u32, delay: u32, padding: u32) -> Vec<u8> {
        let mut f = MONO_128.to_vec();
        f.extend_from_slice(&[0; 17]);
        f.extend_from_slice(b"Info");
        f.extend_from_slice(&1u32.to_be_bytes()); // the frame count is there
        f.extend_from_slice(&frames.to_be_bytes());
        f.extend_from_slice(b"LAME3.100");
        f.extend_from_slice(&[0; 12]);
        f.push((delay >> 4) as u8);
        f.push((((delay & 15) << 4) | (padding >> 8)) as u8);
        f.push((padding & 255) as u8);
        f.resize(FRAME, 0);
        f
    }

    fn goertzel(x: &[f32], freq: f64, rate: f64) -> f64 {
        let w = 2.0 * std::f64::consts::PI * freq / rate;
        let c = 2.0 * w.cos();
        let (mut s1, mut s2) = (0.0f64, 0.0f64);
        for &v in x {
            let s = v as f64 + c * s1 - s2;
            s2 = s1;
            s1 = s;
        }
        (s1 * s1 + s2 * s2 - c * s1 * s2).sqrt() / x.len() as f64
    }

    #[test]
    fn frame_headers_give_the_right_sizes() {
        let cases: [([u8; 4], usize, u32, u32, u32, u32); 7] = [
            // MPEG-1 Layer III, 128 kbit/s, 44.1 kHz
            ([0xFF, 0xFB, 0x90, 0x64], 417, 1152, 44100, 128, 3),
            // ... padded
            ([0xFF, 0xFB, 0x92, 0x64], 418, 1152, 44100, 128, 3),
            // MPEG-1 Layer III, 320 kbit/s, 48 kHz
            ([0xFF, 0xFB, 0xE4, 0x44], 960, 1152, 48000, 320, 3),
            // MPEG-2 Layer III, 64 kbit/s, 22.05 kHz
            ([0xFF, 0xF3, 0x80, 0xC4], 208, 576, 22050, 64, 3),
            // MPEG-2.5 Layer III, 64 kbit/s, 11.025 kHz
            ([0xFF, 0xE3, 0x80, 0xC4], 417, 576, 11025, 64, 3),
            // MPEG-1 Layer II, 256 kbit/s, 48 kHz
            ([0xFF, 0xFD, 0xC4, 0x04], 768, 1152, 48000, 256, 2),
            // MPEG-1 Layer I, 384 kbit/s, 44.1 kHz (slots of four bytes)
            ([0xFF, 0xFF, 0xC0, 0x04], 416, 384, 44100, 384, 1),
        ];
        for (h, bytes, samples, hz, kbps, layer) in cases {
            assert!(hdr_valid(&h), "{h:02X?}");
            assert_eq!(hdr_frame_bytes(&h, 0) + hdr_padding(&h), bytes, "{h:02X?}");
            assert_eq!(hdr_frame_samples(&h), samples, "{h:02X?}");
            assert_eq!(hdr_sample_rate_hz(&h), hz, "{h:02X?}");
            assert_eq!(hdr_bitrate_kbps(&h), kbps, "{h:02X?}");
            assert_eq!(4 - hdr_get_layer(&h) as u32, layer, "{h:02X?}");
        }
        assert!(hdr_is_mono(&MONO_128));
        assert!(hdr_is_ms_stereo(&[0xFF, 0xFB, 0x90, 0x64]));
        // Bitrate 15, sample rate 3 and layer 0 are not frames.
        assert!(!hdr_valid(&[0xFF, 0xFB, 0xF0, 0x64]));
        assert!(!hdr_valid(&[0xFF, 0xFB, 0x9C, 0x64]));
        assert!(!hdr_valid(&[0xFF, 0xF9, 0x90, 0x64]));
        assert!(!hdr_valid(&[0xFE, 0xFB, 0x90, 0x64]));
    }

    /// Decodes one big_values codeword (left-aligned in `code`) with a table:
    /// (x, y, bits used).
    fn pair(table: usize, code: &str) -> (i32, i32, u32) {
        let mut cache = u32::from_str_radix(code, 2).unwrap() << (32 - code.len());
        let base = TAB_INDEX[table] as i32;
        let peek = |c: u32, n: u32| ((c as u64) >> (32 - n)) as i32;
        let mut w = 5;
        let mut used = 0;
        let mut leaf = TABS[(base + peek(cache, w)) as usize] as i32;
        while leaf < 0 {
            used += w;
            cache <<= w;
            w = (leaf & 7) as u32;
            leaf = TABS[(base + peek(cache, w) - (leaf >> 3)) as usize] as i32;
        }
        used += (leaf >> 8) as u32;
        (leaf & 15, (leaf >> 4) & 15, used)
    }

    /// Decodes one count1 codeword: (v, w, x, y as bits 3..0, bits used).
    fn quad(table_b: bool, code: &str) -> (u32, u32) {
        let cache = u32::from_str_radix(code, 2).unwrap() << (32 - code.len());
        let tab: &[u8] = if table_b { &TAB33 } else { &TAB32 };
        let mut leaf = tab[(cache >> 28) as usize] as u32;
        if leaf & 8 == 0 {
            leaf = tab[((leaf >> 3) + ((cache << 4) >> (32 - (leaf & 3)))) as usize] as u32;
        }
        (leaf >> 4, leaf & 7)
    }

    #[test]
    fn huffman_tables_match_the_standard() {
        // ISO 11172-3 table 1 (2 x 2) and table 2 (3 x 3): (x, y) -> code.
        for (x, y, code) in [(0, 0, "1"), (0, 1, "001"), (1, 0, "01"), (1, 1, "000")] {
            assert_eq!(pair(1, code), (x, y, code.len() as u32), "table 1 {code}");
        }
        for (x, y, code) in [(0, 0, "1"), (0, 1, "010"), (0, 2, "000001"), (1, 0, "011"), (1, 1, "001"), (1, 2, "00001"), (2, 0, "00011"), (2, 1, "00010"), (2, 2, "000000")] {
            assert_eq!(pair(2, code), (x, y, code.len() as u32), "table 2 {code}");
        }
        // Count1 table A, and table B (four bits, inverted).
        let a = ["1", "0101", "0100", "00101", "0110", "000101", "00100", "000100", "0111", "00011", "00110", "000000", "00111", "000010", "000011", "000001"];
        for (vwxy, code) in a.iter().enumerate() {
            assert_eq!(quad(false, code), (vwxy as u32, code.len() as u32), "table A {code}");
            let b = format!("{:04b}", 15 - vwxy);
            assert_eq!(quad(true, &b), (vwxy as u32, 4), "table B {b}");
        }
        // Tables 4 and 14 do not exist; 16..23 and 24..31 share their codes
        // with growing escape (linbits) lengths.
        assert_eq!(TAB_INDEX[4], 0);
        assert_eq!(TAB_INDEX[14], 0);
        assert!(TAB_INDEX[16..24].iter().all(|&t| t == TAB_INDEX[16]));
        assert!(TAB_INDEX[24..32].iter().all(|&t| t == TAB_INDEX[24]));
        assert_eq!(LINBITS[16..24], [1, 2, 3, 4, 6, 8, 10, 13]);
        assert_eq!(LINBITS[24..32], [4, 5, 6, 7, 8, 9, 11, 13]);
        // Every scale factor band layout covers the 576 lines of a granule.
        for row in SCF_LONG.iter().map(|r| &r[..]).chain(SCF_SHORT.iter().map(|r| &r[..])).chain(SCF_MIXED.iter().map(|r| &r[..])) {
            assert_eq!(row.iter().map(|&w| w as usize).sum::<usize>(), 576);
        }
        // x^(4/3) beyond the table stays on the curve.
        for x in [129, 200, 1000, 1023, 1024, 4000, 8191] {
            let want = (x as f64).powf(4.0 / 3.0);
            assert!((pow_43(x) as f64 - want).abs() / want < 1e-4, "{x}");
        }
    }

    #[test]
    fn silent_frames_decode_to_silence() {
        let mut data = Vec::new();
        for _ in 0..10 {
            let mut f = MONO_128.to_vec();
            f.extend_from_slice(&side_info(0, [(0, 0), (0, 0)], 0));
            f.resize(FRAME, 0);
            data.extend_from_slice(&f);
        }
        let a = decode(&data).expect("decodes");
        assert_eq!((a.sample_rate, a.channels), (44100, 1));
        assert_eq!(a.left.len(), 10 * 1152);
        assert!(a.left.iter().chain(&a.right).all(|&s| s == 0.0));
    }

    #[test]
    fn a_spectral_line_decodes_to_its_tone() {
        let line = 20;
        let a = decode(&tone_stream(40, line, 190)).expect("decodes");
        assert_eq!(a.left.len(), 40 * 1152);
        assert_eq!(a.left, a.right);
        // Past the first frames (the filter banks filling), a steady tone at
        // the line's frequency: (line + 1/2) * rate / 1152.
        let x = &a.left[4 * 1152..];
        let peak = x.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak > 1e-3 && peak < 2.0, "peak {peak}");
        let f = (line as f64 + 0.5) * 44100.0 / 1152.0;
        // (One line held still over frames is a tone with close side bands.)
        let loudest = (5..800).map(|k| k as f64 * 10.0).map(|fr| (fr, goertzel(x, fr, 44100.0))).fold((0.0, 0.0), |m, (fr, e)| if e > m.1 { (fr, e) } else { m });
        assert!((loudest.0 - f).abs() < 40.0, "loudest at {} Hz, not {f} Hz", loudest.0);
        for other in [f * 0.5, f * 2.0, f * 3.0, 5000.0] {
            assert!(loudest.1 > 10.0 * goertzel(x, other, 44100.0), "{f} Hz against {other} Hz");
        }
        // The same frame decodes the same each time: as loud at the end as
        // after the start.
        let rms = |s: &[f32]| (s.iter().map(|v| (v * v) as f64).sum::<f64>() / s.len() as f64).sqrt();
        let (early, late) = (rms(&a.left[4 * 1152..10 * 1152]), rms(&a.left[34 * 1152..40 * 1152]));
        assert!((early - late).abs() < early * 0.01, "{early} {late}");
    }

    #[test]
    fn the_bit_reservoir_carries_main_data_across_frames() {
        // The same frames twice: once with each frame's main data in itself,
        // once with every frame (but the first) starting its main data 50
        // bytes back, in the frame before.
        let n = 12;
        let lines = |i: usize| 8 + (i * 7) % 60;
        let plain: Vec<u8> = (0..n)
            .flat_map(|i| {
                let (si, main) = frame_parts(Some(lines(i)), 230, 0, 0);
                let mut f = MONO_128.to_vec();
                f.extend_from_slice(&si);
                f.extend_from_slice(&main);
                f.resize(FRAME, 0);
                f
            })
            .collect();
        const BACK: usize = 50;
        let area = FRAME - MAIN_START;
        let mut frames: Vec<Vec<u8>> = Vec::new();
        for i in 0..n {
            let mdb = if i == 0 { 0 } else { BACK as u32 };
            let (si, main) = frame_parts(Some(lines(i)), 230, mdb, BACK + 10);
            let mut f = MONO_128.to_vec();
            f.extend_from_slice(&si);
            f.resize(FRAME, 0);
            if i == 0 {
                f[MAIN_START..MAIN_START + main.len()].copy_from_slice(&main);
            } else {
                // The first 50 bytes go at the end of the frame before.
                let prev = frames.last_mut().unwrap();
                prev[MAIN_START + area - BACK..].copy_from_slice(&main[..BACK]);
                f[MAIN_START..MAIN_START + main.len() - BACK].copy_from_slice(&main[BACK..]);
            }
            frames.push(f);
        }
        let reservoir: Vec<u8> = frames.concat();
        let a = decode(&plain).expect("plain decodes");
        let b = decode(&reservoir).expect("reservoir decodes");
        assert_eq!(a.left.len(), b.left.len());
        assert!(a.left.iter().any(|&s| s != 0.0));
        assert_eq!(a.left, b.left);
    }

    #[test]
    fn the_lame_tag_trims_the_encoder_delay_and_padding() {
        let frames = 20;
        let (delay, padding) = (576, 1000);
        let mut data = info_frame(frames, delay, padding);
        data.extend_from_slice(&tone_stream(frames as usize, 30, 240));
        let a = decode(&data).expect("decodes");
        // The decoder's own delay (529 samples) is counted in, as LAME means it.
        let want = frames as usize * 1152 - (delay as usize + 529) - (padding as usize - 529);
        assert_eq!(a.left.len(), want);
        // Without the tag every sample of every frame is there.
        let plain = decode(&tone_stream(frames as usize, 30, 240)).unwrap();
        assert_eq!(plain.left.len(), frames as usize * 1152);
        // The trimmed audio is the untrimmed audio, shifted.
        assert_eq!(a.left[..], plain.left[delay as usize + 529..delay as usize + 529 + want]);
    }

    #[test]
    fn tags_around_the_audio_are_skipped() {
        let audio = tone_stream(8, 12, 230);
        let mut tagged = b"ID3\x04\x00\x00\x00\x00\x02\x00".to_vec(); // 256 bytes of ID3v2
        tagged.extend((0..256).map(|i| (i * 37) as u8 | 0x80));
        tagged.extend_from_slice(&audio);
        let mut v1 = b"TAG".to_vec();
        v1.resize(128, b'x');
        tagged.extend_from_slice(&v1);
        let a = decode(&audio).unwrap();
        let b = decode(&tagged).unwrap();
        assert_eq!(a.left, b.left);
        assert!(looks_like_mp3(&tagged));
        assert!(looks_like_mp3(&audio));
        assert!(!looks_like_mp3(b"RIFF\x00\x00\x00\x00WAVEfmt "));
        assert!(!looks_like_mp3(&[0u8; 5000]));
    }

    #[test]
    fn junk_and_cut_files_do_not_panic() {
        let audio = tone_stream(6, 12, 230);
        // Cut anywhere, bits flipped: it decodes what it can, or says no.
        for cut in [0, 3, 4, 100, 417, 418, 1000, audio.len() - 1] {
            let _ = decode(&audio[..cut]);
        }
        let mut seed = 12345u32;
        for _ in 0..200 {
            let mut d = audio.clone();
            for _ in 0..20 {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                let i = (seed >> 8) as usize % d.len();
                d[i] ^= (seed >> 3) as u8;
            }
            let _ = decode(&d);
        }
        let mut seed = 777u32;
        let noise: Vec<u8> = (0..20000)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                (seed >> 24) as u8
            })
            .collect();
        let _ = decode(&noise);
    }

    #[test]
    fn four_minutes_decode_quickly() {
        // 4 minutes of 44.1 kHz mono frames.
        let frames = 240 * 44100 / 1152;
        let data = tone_stream(frames, 40, 230);
        let t = std::time::Instant::now();
        let a = decode(&data).unwrap();
        let took = t.elapsed();
        assert_eq!(a.left.len(), frames * 1152);
        eprintln!("decoded {frames} frames ({:.1} MB) in {took:?}", data.len() as f64 / 1e6);
        // Generous, for unoptimized test builds on slow machines.
        assert!(took.as_secs_f64() < 30.0);
    }

    /// The ISO and minimp3 test vectors, when a folder of them (.bit with a
    /// .pcm of 16-bit samples beside each) is named in KRABTON_MP3_VECTORS.
    /// They are not part of Krabton; minimp3's repository has them.
    #[test]
    fn decodes_the_test_vectors_when_given() {
        let Ok(dir) = std::env::var("KRABTON_MP3_VECTORS") else { return };
        let mut names: Vec<_> = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "bit")).collect();
        names.sort();
        let mut failed = Vec::new();
        for bit in names {
            let Ok(want) = std::fs::read(bit.with_extension("pcm")) else { continue };
            let want: Vec<i16> = want.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect();
            let data = std::fs::read(&bit).unwrap();
            let name = bit.file_name().unwrap().to_string_lossy().to_string();
            let to16 = |v: f32| {
                let s = v * 32768.0;
                if s >= 32766.5 {
                    32767
                } else if s <= -32767.5 {
                    -32768
                } else {
                    let i = (s + 0.5) as i32;
                    (i - (i < 0) as i32) as i16
                }
            };
            // Frame by frame, each frame's samples as they come (as the
            // references were made: a mono frame in a stereo stream gives
            // mono samples) ...
            let t = std::time::Instant::now();
            let mut raw = Vec::new();
            let mut dec = Decoder::new();
            let mut pcm = vec![0.0f32; MAX_SAMPLES_PER_FRAME];
            let mut rest = skip_tags(&data);
            loop {
                let (samples, info) = dec.decode_frame(rest, &mut pcm);
                raw.extend(pcm[..samples * info.channels].iter().map(|&v| to16(v)));
                if info.frame_bytes == 0 {
                    break;
                }
                rest = &rest[info.frame_bytes..];
            }
            let took = t.elapsed();
            // ... and as a whole file (trimmed by a LAME tag), interleaved.
            let whole: Vec<i16> = decode(&data).map(|a| (0..a.left.len()).flat_map(|i| if a.channels == 2 { vec![to16(a.left[i]), to16(a.right[i])] } else { vec![to16(a.left[i])] }).collect()).unwrap_or_default();
            // The references for Layers I and II go on with the filter bank's
            // tail and are then padded with silence to a power of two; some
            // others lack the last frame.
            let compare = |got: &[i16]| -> (bool, i32, f64) {
                let n = got.len().min(want.len());
                let (mut max, mut sq) = (0i32, 0f64);
                for i in 0..n {
                    let d = (got[i] as i32 - want[i] as i32).abs();
                    max = max.max(d);
                    sq += (d * d) as f64;
                }
                let length_ok = if got.len() >= want.len() { got.len() - want.len() <= 2304 } else { want[(got.len() + 2304).min(want.len())..].iter().all(|&s| s == 0) };
                (length_ok && max <= 1, max, (sq / n.max(1) as f64).sqrt())
            };
            let (a, b) = (compare(&raw), compare(&whole));
            let (ok, max, rms, len) = if a.0 || !b.0 { (a.0, a.1, a.2, raw.len()) } else { (b.0, b.1, b.2, whole.len()) };
            eprintln!("{name}: {len} of {} samples, max diff {max}, rms {rms:.3}, {took:?}{}", want.len(), if ok { "" } else { "  FAILED" });
            if !ok {
                failed.push(name);
            }
        }
        assert!(failed.is_empty(), "failed: {failed:?}");
    }
}
