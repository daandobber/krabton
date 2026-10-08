//! The host side of the CLAP ABI: the `clap_host` struct each plugin
//! instance gets, and the host extensions plugins may ask for.

use clap_sys::ext::audio_ports::{clap_host_audio_ports, CLAP_EXT_AUDIO_PORTS};
use clap_sys::ext::gui::{clap_host_gui, CLAP_EXT_GUI};
use clap_sys::ext::latency::{clap_host_latency, CLAP_EXT_LATENCY};
use clap_sys::ext::log::{clap_host_log, clap_log_severity, CLAP_EXT_LOG};
use clap_sys::ext::note_ports::{clap_host_note_ports, clap_note_dialect, CLAP_EXT_NOTE_PORTS, CLAP_NOTE_DIALECT_CLAP, CLAP_NOTE_DIALECT_MIDI};
use clap_sys::ext::params::{clap_host_params, clap_param_clear_flags, clap_param_rescan_flags, CLAP_EXT_PARAMS};
use clap_sys::ext::state::{clap_host_state, CLAP_EXT_STATE};
use clap_sys::ext::tail::{clap_host_tail, CLAP_EXT_TAIL};
use clap_sys::ext::thread_check::{clap_host_thread_check, CLAP_EXT_THREAD_CHECK};
use clap_sys::host::clap_host;
use clap_sys::id::clap_id;
use clap_sys::version::CLAP_VERSION;
use std::ffi::{c_char, c_void, CStr};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::thread::ThreadId;

static MAIN_THREAD: OnceLock<ThreadId> = OnceLock::new();

pub(crate) fn set_main_thread() {
    let _ = MAIN_THREAD.set(std::thread::current().id());
}

fn on_main_thread() -> bool {
    MAIN_THREAD.get().map(|t| *t == std::thread::current().id()).unwrap_or(false)
}

/// Per-instance host data. `clap.host_data` points back at this struct.
pub(crate) struct HostData {
    pub clap: clap_host,
    pub callback_requested: AtomicBool,
    pub restart_requested: AtomicBool,
    pub state_dirty: AtomicBool,
    pub gui_closed: AtomicBool,
    /// Requested editor size as (w << 32 | h); 0 = none.
    pub resize_request: AtomicU64,
}

impl HostData {
    pub fn new() -> Box<HostData> {
        let mut b = Box::new(HostData {
            clap: clap_host {
                clap_version: CLAP_VERSION,
                host_data: std::ptr::null_mut(),
                name: c"Krabton".as_ptr(),
                vendor: c"Krabton contributors".as_ptr(),
                url: c"https://github.com/krabton".as_ptr(),
                version: c"0.1.0".as_ptr(),
                get_extension: Some(get_extension),
                request_restart: Some(request_restart),
                request_process: Some(request_process),
                request_callback: Some(request_callback),
            },
            callback_requested: AtomicBool::new(false),
            restart_requested: AtomicBool::new(false),
            state_dirty: AtomicBool::new(false),
            gui_closed: AtomicBool::new(false),
            resize_request: AtomicU64::new(0),
        });
        b.clap.host_data = &*b as *const HostData as *mut c_void;
        b
    }
}

unsafe fn data<'a>(host: *const clap_host) -> Option<&'a HostData> {
    if host.is_null() {
        return None;
    }
    ((*host).host_data as *const HostData).as_ref()
}

unsafe extern "C" fn get_extension(_host: *const clap_host, id: *const c_char) -> *const c_void {
    if id.is_null() {
        return std::ptr::null();
    }
    let id = CStr::from_ptr(id);
    let ptr: *const c_void = if id == CLAP_EXT_THREAD_CHECK {
        &THREAD_CHECK as *const _ as *const c_void
    } else if id == CLAP_EXT_LOG {
        &LOG as *const _ as *const c_void
    } else if id == CLAP_EXT_GUI {
        &GUI as *const _ as *const c_void
    } else if id == CLAP_EXT_STATE {
        &STATE as *const _ as *const c_void
    } else if id == CLAP_EXT_PARAMS {
        &PARAMS as *const _ as *const c_void
    } else if id == CLAP_EXT_LATENCY {
        &LATENCY as *const _ as *const c_void
    } else if id == CLAP_EXT_TAIL {
        &TAIL as *const _ as *const c_void
    } else if id == CLAP_EXT_AUDIO_PORTS {
        &AUDIO_PORTS as *const _ as *const c_void
    } else if id == CLAP_EXT_NOTE_PORTS {
        &NOTE_PORTS as *const _ as *const c_void
    } else {
        std::ptr::null()
    };
    ptr
}

unsafe extern "C" fn request_restart(host: *const clap_host) {
    if let Some(d) = data(host) {
        d.restart_requested.store(true, Ordering::Relaxed);
    }
}

unsafe extern "C" fn request_process(_host: *const clap_host) {
    // Instruments are always processed.
}

unsafe extern "C" fn request_callback(host: *const clap_host) {
    if let Some(d) = data(host) {
        d.callback_requested.store(true, Ordering::Relaxed);
    }
}

unsafe extern "C" fn is_main_thread(_host: *const clap_host) -> bool {
    on_main_thread()
}

unsafe extern "C" fn is_audio_thread(_host: *const clap_host) -> bool {
    !on_main_thread()
}

static THREAD_CHECK: clap_host_thread_check = clap_host_thread_check { is_main_thread: Some(is_main_thread), is_audio_thread: Some(is_audio_thread) };

unsafe extern "C" fn log(_host: *const clap_host, severity: clap_log_severity, msg: *const c_char) {
    if !msg.is_null() && severity >= 2 {
        eprintln!("[clap] {}", CStr::from_ptr(msg).to_string_lossy());
    }
}

static LOG: clap_host_log = clap_host_log { log: Some(log) };

unsafe extern "C" fn noop(_host: *const clap_host) {}

unsafe extern "C" fn yes(_host: *const clap_host) -> bool {
    true
}

unsafe extern "C" fn request_resize(host: *const clap_host, width: u32, height: u32) -> bool {
    if let Some(d) = data(host) {
        d.resize_request.store(((width as u64) << 32) | height.max(1) as u64, Ordering::Relaxed);
        return true;
    }
    false
}

unsafe extern "C" fn gui_closed(host: *const clap_host, _was_destroyed: bool) {
    if let Some(d) = data(host) {
        d.gui_closed.store(true, Ordering::Relaxed);
    }
}

static GUI: clap_host_gui = clap_host_gui {
    resize_hints_changed: Some(noop),
    request_resize: Some(request_resize),
    request_show: Some(yes),
    request_hide: Some(yes),
    closed: Some(gui_closed),
};

unsafe extern "C" fn mark_dirty(host: *const clap_host) {
    if let Some(d) = data(host) {
        d.state_dirty.store(true, Ordering::Relaxed);
    }
}

static STATE: clap_host_state = clap_host_state { mark_dirty: Some(mark_dirty) };

unsafe extern "C" fn params_rescan(_host: *const clap_host, _flags: clap_param_rescan_flags) {}

unsafe extern "C" fn params_clear(_host: *const clap_host, _id: clap_id, _flags: clap_param_clear_flags) {}

static PARAMS: clap_host_params = clap_host_params { rescan: Some(params_rescan), clear: Some(params_clear), request_flush: Some(noop) };

static LATENCY: clap_host_latency = clap_host_latency { changed: Some(noop) };

static TAIL: clap_host_tail = clap_host_tail { changed: Some(noop) };

unsafe extern "C" fn rescan_flag_supported(_host: *const clap_host, _flag: u32) -> bool {
    false
}

unsafe extern "C" fn rescan(_host: *const clap_host, _flags: u32) {}

static AUDIO_PORTS: clap_host_audio_ports = clap_host_audio_ports { is_rescan_flag_supported: Some(rescan_flag_supported), rescan: Some(rescan) };

unsafe extern "C" fn supported_dialects(_host: *const clap_host) -> clap_note_dialect {
    CLAP_NOTE_DIALECT_CLAP | CLAP_NOTE_DIALECT_MIDI
}

static NOTE_PORTS: clap_host_note_ports = clap_host_note_ports { supported_dialects: Some(supported_dialects), rescan: Some(rescan) };
