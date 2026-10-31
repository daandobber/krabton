//! Animated GIFs for the Video, read here (the `image` crate Krabton has is
//! built without GIF): each frame drawn over the last as the file says
//! (left, cleared or put back), with its time.

/// A GIF's frames, every one the whole picture in RGBA.
pub struct Gif {
    pub width: u32,
    pub height: u32,
    /// (when it starts in seconds, its pixels).
    pub frames: Vec<(f32, Vec<u8>)>,
    /// How long all of it lasts.
    pub duration: f32,
}

/// Reads a GIF, keeping at most `budget` bytes of frames (the rest is left out).
pub fn decode(data: &[u8], budget: usize) -> Result<Gif, String> {
    if data.len() < 13 || !(data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a")) {
        return Err("not a GIF".into());
    }
    let u16_at = |o: usize| u16::from_le_bytes([data[o], data[o + 1]]) as usize;
    let (w, h) = (u16_at(6), u16_at(8));
    if w == 0 || h == 0 || w > 8192 || h > 8192 {
        return Err(format!("a GIF of {w}x{h}"));
    }
    let packed = data[10];
    let mut p = 13;
    let mut global: Vec<[u8; 3]> = Vec::new();
    if packed & 0x80 != 0 {
        let n = 2usize << (packed & 7);
        global = read_palette(data, p, n)?;
        p += 3 * n;
    }
    let mut canvas = vec![0u8; w * h * 4];
    let mut frames: Vec<(f32, Vec<u8>)> = Vec::new();
    let mut t = 0.0f32;
    // The graphic control for the next image: delay (s), disposal, transparent index.
    let mut delay = 0.1f32;
    let mut disposal = 0u8;
    let mut transparent: Option<u8> = None;
    let mut used = 0usize;
    loop {
        let Some(&block) = data.get(p) else { break };
        p += 1;
        match block {
            // The end.
            0x3B => break,
            0x21 => {
                let label = *data.get(p).ok_or("cut short")?;
                p += 1;
                if label == 0xF9 && data.get(p) == Some(&4) && p + 5 < data.len() {
                    let flags = data[p + 1];
                    let cs = u16_at(p + 2);
                    // 0 and 1 hundredths are played at a tenth, as browsers do.
                    delay = if cs < 2 { 0.1 } else { cs as f32 / 100.0 };
                    disposal = (flags >> 2) & 7;
                    transparent = (flags & 1 != 0).then_some(data[p + 4]);
                }
                p = skip_blocks(data, p)?;
            }
            0x2C => {
                if p + 9 > data.len() {
                    return Err("cut short".into());
                }
                let (fx, fy, fw, fh) = (u16_at(p), u16_at(p + 2), u16_at(p + 4), u16_at(p + 6));
                let flags = data[p + 8];
                p += 9;
                let mut palette = &global;
                let local;
                if flags & 0x80 != 0 {
                    let n = 2usize << (flags & 7);
                    local = read_palette(data, p, n)?;
                    p += 3 * n;
                    palette = &local;
                }
                let min_code = *data.get(p).ok_or("cut short")?;
                p += 1;
                let (packed_data, next) = gather_blocks(data, p)?;
                p = next;
                if !(2..=11).contains(&min_code) {
                    return Err(format!("a code size of {min_code}"));
                }
                let mut indices = Vec::with_capacity(fw * fh);
                // What decodes is kept, even when the end is broken.
                let _ = weezl::decode::Decoder::new(weezl::BitOrder::Lsb, min_code).into_vec(&mut indices).decode_all(&packed_data);
                let saved = (disposal == 3).then(|| canvas.clone());
                let interlaced = flags & 0x40 != 0;
                for (i, &ix) in indices.iter().take(fw * fh).enumerate() {
                    if Some(ix) == transparent {
                        continue;
                    }
                    let row = if interlaced { interlaced_row(i / fw, fh) } else { i / fw };
                    let (x, y) = (fx + i % fw, fy + row);
                    if x >= w || y >= h {
                        continue;
                    }
                    let c = palette.get(ix as usize).copied().unwrap_or([0, 0, 0]);
                    let o = (y * w + x) * 4;
                    canvas[o..o + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
                }
                used += canvas.len();
                if used > budget && !frames.is_empty() {
                    break;
                }
                frames.push((t, canvas.clone()));
                t += delay;
                match disposal {
                    // Back to nothing where it was.
                    2 => {
                        for y in fy..(fy + fh).min(h) {
                            for x in fx..(fx + fw).min(w) {
                                let o = (y * w + x) * 4;
                                canvas[o..o + 4].copy_from_slice(&[0, 0, 0, 0]);
                            }
                        }
                    }
                    // Back to what was before it.
                    3 => {
                        if let Some(s) = saved {
                            canvas = s;
                        }
                    }
                    _ => {}
                }
                delay = 0.1;
                disposal = 0;
                transparent = None;
            }
            // Anything else: a broken file; what was read so far stays.
            _ => break,
        }
    }
    if frames.is_empty() {
        return Err("a GIF without pictures".into());
    }
    let duration = if frames.len() > 1 { t } else { 0.0 };
    Ok(Gif { width: w as u32, height: h as u32, frames, duration })
}

/// Writes a GIF: frames of palette indices (each the whole picture), with
/// a delay in hundredths of a second, a disposal method and a see-through
/// index. For the debug script's test clip and the tests.
pub fn encode(w: u16, h: u16, palette: &[[u8; 3]], frames: &[(Vec<u8>, u16, u8, Option<u8>)]) -> Vec<u8> {
    // The palette in a power of two, at least four.
    let bits = (1..=8).find(|b| 1usize << b >= palette.len()).unwrap_or(8).max(2);
    let mut out = b"GIF89a".to_vec();
    out.extend_from_slice(&w.to_le_bytes());
    out.extend_from_slice(&h.to_le_bytes());
    out.extend_from_slice(&[0x80 | (bits as u8 - 1), 0, 0]);
    for i in 0..1usize << bits {
        out.extend_from_slice(&palette.get(i).copied().unwrap_or([0, 0, 0]));
    }
    // Round and round.
    out.extend_from_slice(&[0x21, 0xFF, 11]);
    out.extend_from_slice(b"NETSCAPE2.0");
    out.extend_from_slice(&[3, 1, 0, 0, 0]);
    for (px, delay, disposal, transparent) in frames {
        out.extend_from_slice(&[0x21, 0xF9, 4, (disposal << 2) | transparent.is_some() as u8]);
        out.extend_from_slice(&delay.to_le_bytes());
        out.extend_from_slice(&[transparent.unwrap_or(0), 0]);
        out.push(0x2C);
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&w.to_le_bytes());
        out.extend_from_slice(&h.to_le_bytes());
        out.push(0);
        out.push(bits as u8);
        let packed = weezl::encode::Encoder::new(weezl::BitOrder::Lsb, bits as u8).encode(px).unwrap_or_default();
        for chunk in packed.chunks(255) {
            out.push(chunk.len() as u8);
            out.extend_from_slice(chunk);
        }
        out.push(0);
    }
    out.push(0x3B);
    out
}

/// A test clip: colour bars sliding by and a white ball bouncing, about two
/// seconds round (for the debug script, where there is no film at hand).
pub fn test_pattern() -> Vec<u8> {
    let (w, h, n) = (320usize, 180usize, 24usize);
    let mut palette = vec![[0u8, 0, 0], [255, 255, 255], [40, 40, 48]];
    palette.extend_from_slice(&[[230, 60, 60], [240, 150, 40], [235, 220, 60], [80, 200, 90], [60, 190, 210], [70, 110, 230], [160, 80, 220], [230, 80, 170]]);
    let frames: Vec<(Vec<u8>, u16, u8, Option<u8>)> = (0..n)
        .map(|f| {
            let mut px = vec![2u8; w * h];
            let shift = f * w / n;
            let phase = f as f32 / n as f32;
            let (bx, by) = (40.0 + 240.0 * phase, 90.0 - 60.0 * (std::f32::consts::PI * 2.0 * phase).sin().abs());
            for y in 0..h {
                for x in 0..w {
                    let i = y * w + x;
                    if y < 120 {
                        px[i] = 3 + (((x + shift) % w) * 8 / w) as u8;
                    } else if (x / 20 + f) % 2 == 0 && y > 150 {
                        px[i] = 1;
                    }
                    let (dx, dy) = (x as f32 - bx, y as f32 - by);
                    let d = dx * dx + dy * dy;
                    if d < 18.0 * 18.0 {
                        px[i] = 1;
                    } else if d < 21.0 * 21.0 {
                        px[i] = 0;
                    }
                }
            }
            (px, 8, 1, None)
        })
        .collect();
    encode(w as u16, h as u16, &palette, &frames)
}

fn read_palette(data: &[u8], at: usize, n: usize) -> Result<Vec<[u8; 3]>, String> {
    let raw = data.get(at..at + 3 * n).ok_or("cut short")?;
    Ok(raw.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect())
}

/// Past a run of sub-blocks (each its length, then that many bytes; 0 ends).
fn skip_blocks(data: &[u8], mut p: usize) -> Result<usize, String> {
    loop {
        let n = *data.get(p).ok_or("cut short")? as usize;
        p += 1 + n;
        if n == 0 {
            return Ok(p);
        }
    }
}

/// A run of sub-blocks joined, and where it ends.
fn gather_blocks(data: &[u8], mut p: usize) -> Result<(Vec<u8>, usize), String> {
    let mut out = Vec::new();
    loop {
        let n = *data.get(p).ok_or("cut short")? as usize;
        p += 1;
        if n == 0 {
            return Ok((out, p));
        }
        // A last block cut short: what is there.
        let end = (p + n).min(data.len());
        out.extend_from_slice(&data[p..end]);
        p += n;
        if p > data.len() {
            return Ok((out, p));
        }
    }
}

/// The row of the `k`th line of an interlaced image: every eighth from 0,
/// every eighth from 4, every fourth from 2, every second from 1.
fn interlaced_row(k: usize, h: usize) -> usize {
    let passes = [(0, 8), (4, 8), (2, 4), (1, 2)];
    let mut k = k;
    for (start, step) in passes {
        let rows = if h > start { (h - start).div_ceil(step) } else { 0 };
        if k < rows {
            return start + k * step;
        }
        k -= rows;
    }
    h.saturating_sub(1)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A GIF of `frames` (indices into a four-colour palette: black, red,
    /// green, blue), each the whole picture, with a delay in hundredths, a
    /// disposal method and a see-through index.
    pub(crate) fn make_gif(w: u16, h: u16, frames: &[(Vec<u8>, u16, u8, Option<u8>)]) -> Vec<u8> {
        encode(w, h, &[[0, 0, 0], [255, 0, 0], [0, 255, 0], [0, 0, 255]], frames)
    }

    #[test]
    fn the_test_clip_reads_back() {
        let g = decode(&test_pattern(), usize::MAX).expect("decodes");
        assert_eq!((g.width, g.height, g.frames.len()), (320, 180, 24));
        assert!((g.duration - 1.92).abs() < 1e-4);
        // The bars: red at the top left of the first frame.
        assert_eq!(&g.frames[0].1[..4], &[230, 60, 60, 255]);
    }

    #[test]
    fn frames_come_with_their_times_and_draw_over_each_other() {
        let red = vec![1u8; 16];
        // The second frame is transparent (index 3) but for its first pixel, green.
        let mut mostly_clear = vec![3u8; 16];
        mostly_clear[0] = 2;
        let data = make_gif(4, 4, &[(red, 20, 1, None), (mostly_clear, 0, 1, Some(3))]);
        let g = decode(&data, usize::MAX).expect("decodes");
        assert_eq!((g.width, g.height, g.frames.len()), (4, 4, 2));
        assert_eq!(g.frames[0].0, 0.0);
        assert!((g.frames[1].0 - 0.2).abs() < 1e-6);
        // A delay of 0 plays at a tenth.
        assert!((g.duration - 0.3).abs() < 1e-6);
        assert_eq!(&g.frames[0].1[..4], &[255, 0, 0, 255]);
        // Drawn over the red: green where it is not see-through.
        assert_eq!(&g.frames[1].1[..4], &[0, 255, 0, 255]);
        assert_eq!(&g.frames[1].1[4..8], &[255, 0, 0, 255]);
    }

    #[test]
    fn disposal_clears_or_puts_back() {
        let blue = vec![3u8; 4];
        let dot = vec![1u8, 0, 0, 0];
        // Cleared after the first: the second, see-through but its dot, shows nothing else.
        let data = make_gif(2, 2, &[(blue.clone(), 10, 2, None), (dot.clone(), 10, 1, Some(0))]);
        let g = decode(&data, usize::MAX).unwrap();
        assert_eq!(&g.frames[1].1[4..8], &[0, 0, 0, 0]);
        // Put back after the second: the third sees the first again.
        let data = make_gif(2, 2, &[(blue, 10, 1, None), (dot, 10, 3, Some(0)), (vec![0u8; 4], 10, 1, Some(0))]);
        let g = decode(&data, usize::MAX).unwrap();
        assert_eq!(&g.frames[1].1[..4], &[255, 0, 0, 255]);
        assert_eq!(&g.frames[2].1[..4], &[0, 0, 255, 255]);
    }

    #[test]
    fn a_still_gif_has_no_length_and_broken_files_say_so() {
        let g = decode(&make_gif(3, 2, &[(vec![2u8; 6], 0, 0, None)]), usize::MAX).unwrap();
        assert_eq!(g.duration, 0.0);
        assert_eq!(&g.frames[0].1[..4], &[0, 255, 0, 255]);
        assert!(decode(b"GIF89a", usize::MAX).is_err());
        assert!(decode(b"\x89PNG\r\n\x1a\n........", usize::MAX).is_err());
        // Cut off in the middle: the frames before it.
        let data = make_gif(2, 2, &[(vec![1u8; 4], 10, 1, None), (vec![2u8; 4], 10, 1, None)]);
        let cut = &data[..data.len() - 8];
        assert!(decode(cut, usize::MAX).map(|g| !g.frames.is_empty()).unwrap_or(true));
        // Over the budget: the first frame still.
        let g = decode(&data, 10).unwrap();
        assert_eq!(g.frames.len(), 1);
    }

    #[test]
    fn interlaced_rows_go_in_four_passes() {
        let rows: Vec<usize> = (0..10).map(|k| interlaced_row(k, 10)).collect();
        assert_eq!(rows, vec![0, 8, 4, 2, 6, 1, 3, 5, 7, 9]);
    }
}
