//! Sound through PulseAudio's "simple" API, loaded when Krabton starts. On
//! Linux this reaches PulseAudio, and PipeWire through pipewire-pulse,
//! directly: no ALSA plugin package is needed, and nothing extra is needed
//! to build. When the library or the server is not there, opening fails and
//! Krabton uses ALSA.

use libloading::Library;
use std::ffi::{c_char, c_int, c_void, CString};

/// `pa_sample_spec`.
#[repr(C)]
struct SampleSpec {
    format: c_int,
    rate: u32,
    channels: u8,
}

/// `pa_buffer_attr` (u32::MAX: the server's default).
#[repr(C)]
struct BufferAttr {
    maxlength: u32,
    tlength: u32,
    prebuf: u32,
    minreq: u32,
    fragsize: u32,
}

const SAMPLE_FLOAT32LE: c_int = 5;
const STREAM_PLAYBACK: c_int = 1;
const STREAM_RECORD: c_int = 2;

type NewFn = unsafe extern "C" fn(*const c_char, *const c_char, c_int, *const c_char, *const c_char, *const SampleSpec, *const c_void, *const BufferAttr, *mut c_int) -> *mut c_void;
type IoFn = unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *mut c_int) -> c_int;
type FreeFn = unsafe extern "C" fn(*mut c_void);

/// Name shown for this output in the Options menu.
pub const NAME: &str = "PulseAudio / PipeWire";

/// Whether a PulseAudio-compatible server seems to be running for this user.
pub fn server_running() -> bool {
    if std::env::var_os("PULSE_SERVER").is_some() {
        return true;
    }
    std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from).is_some_and(|d| d.join("pulse/native").exists())
}

/// A playback or record stream of 32-bit float stereo.
pub struct PulseStream {
    s: *mut c_void,
    io: IoFn,
    free: FreeFn,
    // Last, so the library is unloaded after the stream is freed.
    _lib: Library,
}

// The stream is only used from the thread it is moved to.
unsafe impl Send for PulseStream {}

impl PulseStream {
    /// Opens a stream at `rate` with about `latency_frames` of buffer.
    pub fn open(record: bool, rate: u32, latency_frames: u32) -> Result<Self, String> {
        // SAFETY: loading a well-known system library; its init code is the
        // library's own.
        let lib = unsafe { Library::new("libpulse-simple.so.0") }.map_err(|e| format!("libpulse-simple: {e}"))?;
        // SAFETY: the symbols have these C signatures (pulse/simple.h).
        let (new, io, free) = unsafe {
            let new: NewFn = *lib.get::<NewFn>(b"pa_simple_new\0").map_err(|e| e.to_string())?;
            let io: IoFn = *lib.get::<IoFn>(if record { b"pa_simple_read\0" } else { b"pa_simple_write\0" }).map_err(|e| e.to_string())?;
            let free: FreeFn = *lib.get::<FreeFn>(b"pa_simple_free\0").map_err(|e| e.to_string())?;
            (new, io, free)
        };
        let spec = SampleSpec { format: SAMPLE_FLOAT32LE, rate, channels: 2 };
        let bytes = latency_frames * 8;
        let attr = if record {
            BufferAttr { maxlength: u32::MAX, tlength: u32::MAX, prebuf: u32::MAX, minreq: u32::MAX, fragsize: bytes / 2 }
        } else {
            BufferAttr { maxlength: u32::MAX, tlength: bytes, prebuf: u32::MAX, minreq: u32::MAX, fragsize: u32::MAX }
        };
        let app = CString::new("Krabton").unwrap_or_default();
        let stream = CString::new(if record { "Krabton input" } else { "Krabton output" }).unwrap_or_default();
        let mut err: c_int = 0;
        // SAFETY: every pointer is valid for the call; NULL server, device
        // and channel map mean the defaults.
        let s = unsafe {
            new(
                std::ptr::null(),
                app.as_ptr(),
                if record { STREAM_RECORD } else { STREAM_PLAYBACK },
                std::ptr::null(),
                stream.as_ptr(),
                &spec,
                std::ptr::null(),
                &attr,
                &mut err,
            )
        };
        if s.is_null() {
            return Err(format!("PulseAudio: could not connect (error {err})"));
        }
        Ok(Self { s, io, free, _lib: lib })
    }

    /// Plays interleaved stereo; blocks while the server's buffer is full.
    pub fn write(&mut self, interleaved: &[f32]) -> Result<(), String> {
        let mut err: c_int = 0;
        // SAFETY: `interleaved` is valid for its byte length; the stream is open.
        let r = unsafe { (self.io)(self.s, interleaved.as_ptr() as *mut c_void, std::mem::size_of_val(interleaved), &mut err) };
        if r < 0 {
            Err(format!("PulseAudio write error {err}"))
        } else {
            Ok(())
        }
    }

    /// Records interleaved stereo into `buf`; blocks until it is full.
    pub fn read(&mut self, buf: &mut [f32]) -> Result<(), String> {
        let mut err: c_int = 0;
        // SAFETY: `buf` is valid and writable for its byte length.
        let r = unsafe { (self.io)(self.s, buf.as_mut_ptr() as *mut c_void, std::mem::size_of_val(buf), &mut err) };
        if r < 0 {
            Err(format!("PulseAudio read error {err}"))
        } else {
            Ok(())
        }
    }
}

impl Drop for PulseStream {
    fn drop(&mut self) {
        // SAFETY: `s` came from pa_simple_new and is freed once.
        unsafe { (self.free)(self.s) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_match_pulse() {
        // pa_sample_spec: enum, uint32, uint8 (padded); pa_buffer_attr: five uint32.
        assert_eq!(std::mem::size_of::<SampleSpec>(), 12);
        assert_eq!(std::mem::size_of::<BufferAttr>(), 20);
    }

    #[test]
    fn a_missing_library_is_an_error_not_a_crash() {
        if cfg!(target_os = "linux") && server_running() {
            return;
        }
        assert!(PulseStream::open(false, 48000, 1024).is_err());
    }
}
