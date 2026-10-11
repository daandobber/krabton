//! MP3 files through LAME. Krabton does not ship an MP3 encoder (LAME is
//! LGPL, and there is no permissively licensed one); it uses one that is
//! already on the computer: the LAME library, loaded when needed, or else
//! the `lame` or `ffmpeg` program.

use libloading::Library;
use std::ffi::{c_int, c_void};
use std::path::Path;

/// Constant bit rate of the files Krabton writes (kbit/s).
pub const BITRATE: i32 = 320;

/// Writes interleaved stereo samples (-1..1) as an MP3 file.
pub fn encode(path: &Path, sample_rate: u32, interleaved: &[f32]) -> Result<(), String> {
    let lib_err = match with_library(path, sample_rate, interleaved) {
        Ok(()) => return Ok(()),
        Err(e) => e,
    };
    with_program(path, sample_rate, interleaved).map_err(|prog_err| {
        format!("MP3 export needs LAME or ffmpeg on this computer (Krabton cannot include an MP3 encoder). Install one of them, or export as WAV. ({lib_err}; {prog_err})")
    })
}

fn library_names() -> &'static [&'static str] {
    if cfg!(windows) {
        &["libmp3lame.dll", "libmp3lame-0.dll", "lame.dll"]
    } else if cfg!(target_os = "macos") {
        &["libmp3lame.0.dylib", "libmp3lame.dylib", "/opt/homebrew/lib/libmp3lame.dylib", "/usr/local/lib/libmp3lame.dylib", "/opt/local/lib/libmp3lame.dylib"]
    } else {
        &["libmp3lame.so.0", "libmp3lame.so"]
    }
}

type InitFn = unsafe extern "C" fn() -> *mut c_void;
type SetFn = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
type GfFn = unsafe extern "C" fn(*mut c_void) -> c_int;
type EncodeFn = unsafe extern "C" fn(*mut c_void, *const f32, *const f32, c_int, *mut u8, c_int) -> c_int;
type FlushFn = unsafe extern "C" fn(*mut c_void, *mut u8, c_int) -> c_int;

fn with_library(path: &Path, sample_rate: u32, interleaved: &[f32]) -> Result<(), String> {
    // SAFETY: loading LAME runs no code of ours; its functions are called
    // with the signatures of lame.h, on buffers sized as lame.h asks.
    unsafe {
        let lib = library_names().iter().find_map(|n| Library::new(n).ok()).ok_or("no LAME library")?;
        let sym = |name: &[u8]| -> Result<*mut c_void, String> {
            lib.get::<*mut c_void>(name).map(|s| *s).map_err(|e| format!("LAME: {e}"))
        };
        let init: InitFn = std::mem::transmute(sym(b"lame_init\0")?);
        let set_rate: SetFn = std::mem::transmute(sym(b"lame_set_in_samplerate\0")?);
        let set_channels: SetFn = std::mem::transmute(sym(b"lame_set_num_channels\0")?);
        let set_brate: SetFn = std::mem::transmute(sym(b"lame_set_brate\0")?);
        let set_quality: SetFn = std::mem::transmute(sym(b"lame_set_quality\0")?);
        let init_params: GfFn = std::mem::transmute(sym(b"lame_init_params\0")?);
        let close: GfFn = std::mem::transmute(sym(b"lame_close\0")?);
        let flush: FlushFn = std::mem::transmute(sym(b"lame_encode_flush\0")?);
        // The -1..1 entry point is newer; the older one wants 16-bit scale.
        let (encode, scale): (EncodeFn, f32) = match sym(b"lame_encode_buffer_ieee_float\0") {
            Ok(f) => (std::mem::transmute(f), 1.0),
            Err(_) => (std::mem::transmute(sym(b"lame_encode_buffer_float\0")?), 32768.0),
        };
        let gf = init();
        if gf.is_null() {
            return Err("LAME did not start".into());
        }
        set_rate(gf, sample_rate as c_int);
        set_channels(gf, 2);
        set_brate(gf, BITRATE);
        set_quality(gf, 2);
        if init_params(gf) < 0 {
            close(gf);
            return Err(format!("LAME cannot encode at {sample_rate} Hz"));
        }
        let frames = interleaved.len() / 2;
        let mut out = Vec::with_capacity(frames / 4);
        const CHUNK: usize = 1152 * 16;
        let mut buf = vec![0u8; CHUNK * 5 / 4 + 7200];
        let (mut l, mut r) = (Vec::with_capacity(CHUNK), Vec::with_capacity(CHUNK));
        for block in interleaved.chunks(CHUNK * 2) {
            l.clear();
            r.clear();
            for fr in block.chunks_exact(2) {
                l.push(fr[0].clamp(-1.0, 1.0) * scale);
                r.push(fr[1].clamp(-1.0, 1.0) * scale);
            }
            let n = encode(gf, l.as_ptr(), r.as_ptr(), l.len() as c_int, buf.as_mut_ptr(), buf.len() as c_int);
            if n < 0 {
                close(gf);
                return Err(format!("LAME failed ({n})"));
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        let n = flush(gf, buf.as_mut_ptr(), buf.len() as c_int);
        if n > 0 {
            out.extend_from_slice(&buf[..n as usize]);
        }
        close(gf);
        std::fs::write(path, out).map_err(|e| format!("{}: {e}", path.display()))
    }
}

fn with_program(path: &Path, sample_rate: u32, interleaved: &[f32]) -> Result<(), String> {
    let tmp = std::env::temp_dir().join(format!("krabton-mp3-{}.wav", std::process::id()));
    let spec = hound::WavSpec { channels: 2, sample_rate, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(&tmp, spec).map_err(|e| e.to_string())?;
    for &s in interleaved {
        w.write_sample((s.clamp(-1.0, 1.0) * 32767.0).round() as i16).map_err(|e| e.to_string())?;
    }
    w.finalize().map_err(|e| e.to_string())?;
    let rate = format!("{BITRATE}");
    let kbps = format!("{BITRATE}k");
    let tries: [(&str, Vec<&std::ffi::OsStr>); 2] = [
        ("lame", vec!["--quiet".as_ref(), "-b".as_ref(), rate.as_ref(), tmp.as_os_str(), path.as_os_str()]),
        ("ffmpeg", vec!["-y".as_ref(), "-hide_banner".as_ref(), "-loglevel".as_ref(), "error".as_ref(), "-i".as_ref(), tmp.as_os_str(), "-b:a".as_ref(), kbps.as_ref(), path.as_os_str()]),
    ];
    let mut result = Err("no lame or ffmpeg program".to_string());
    for (prog, args) in tries {
        let mut cmd = std::process::Command::new(prog);
        cmd.args(args).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // No console window popping up.
            cmd.creation_flags(0x0800_0000);
        }
        if let Ok(status) = cmd.status() {
            if status.success() && path.exists() {
                result = Ok(());
                break;
            }
            result = Err(format!("{prog} failed"));
        }
    }
    let _ = std::fs::remove_file(&tmp);
    result
}
