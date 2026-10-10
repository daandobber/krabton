//! A tiny CLAP effect for testing the host: it swaps left and right and
//! halves the level. Built as a library:
//!
//! ```sh
//! cargo build -p krabton-clap --example swap_fx
//! cargo run -p krabton-clap --example fx_probe -- target/debug/examples/swap_fx.dll
//! ```
//! (`libswap_fx.so` on Linux, `libswap_fx.dylib` on macOS.)

use clap_sys::ext::audio_ports::{clap_audio_port_info, clap_plugin_audio_ports, CLAP_AUDIO_PORT_IS_MAIN, CLAP_EXT_AUDIO_PORTS, CLAP_PORT_STEREO};
use clap_sys::entry::clap_plugin_entry;
use clap_sys::factory::plugin_factory::{clap_plugin_factory, CLAP_PLUGIN_FACTORY_ID};
use clap_sys::host::clap_host;
use clap_sys::id::CLAP_INVALID_ID;
use clap_sys::plugin::{clap_plugin, clap_plugin_descriptor};
use clap_sys::process::{clap_process, clap_process_status, CLAP_PROCESS_CONTINUE, CLAP_PROCESS_ERROR};
use clap_sys::version::CLAP_VERSION;
use std::ffi::{c_char, c_void, CStr};

struct Features([*const c_char; 3]);
// SAFETY: pointers to static strings.
unsafe impl Sync for Features {}

static FEATURES: Features = Features([c"audio-effect".as_ptr(), c"stereo".as_ptr(), std::ptr::null()]);

static DESC: clap_plugin_descriptor = clap_plugin_descriptor {
    clap_version: CLAP_VERSION,
    id: c"org.krabton.test.swap".as_ptr(),
    name: c"Krabton Test Swap".as_ptr(),
    vendor: c"Krabton".as_ptr(),
    url: c"".as_ptr(),
    manual_url: c"".as_ptr(),
    support_url: c"".as_ptr(),
    version: c"1.0.0".as_ptr(),
    description: c"Swaps left and right at half level".as_ptr(),
    features: &FEATURES.0 as *const *const c_char,
};

unsafe extern "C" fn init(_: *const clap_plugin) -> bool {
    true
}

unsafe extern "C" fn destroy(plugin: *const clap_plugin) {
    drop(Box::from_raw(plugin as *mut clap_plugin));
}

unsafe extern "C" fn activate(_: *const clap_plugin, _: f64, _: u32, _: u32) -> bool {
    true
}

unsafe extern "C" fn nothing(_: *const clap_plugin) {}

unsafe extern "C" fn start(_: *const clap_plugin) -> bool {
    true
}

unsafe extern "C" fn process(_: *const clap_plugin, p: *const clap_process) -> clap_process_status {
    let p = &*p;
    if p.audio_inputs_count < 1 || p.audio_outputs_count < 1 {
        return CLAP_PROCESS_ERROR;
    }
    let (i, o) = (&*p.audio_inputs, &*p.audio_outputs);
    if i.channel_count < 2 || o.channel_count < 2 {
        return CLAP_PROCESS_ERROR;
    }
    let n = p.frames_count as usize;
    let (il, ir) = (std::slice::from_raw_parts(*i.data32, n), std::slice::from_raw_parts(*i.data32.add(1), n));
    let (ol, or) = (std::slice::from_raw_parts_mut(*o.data32, n), std::slice::from_raw_parts_mut(*o.data32.add(1), n));
    for k in 0..n {
        let (l, r) = (il[k], ir[k]);
        ol[k] = 0.5 * r;
        or[k] = 0.5 * l;
    }
    CLAP_PROCESS_CONTINUE
}

unsafe extern "C" fn ports_count(_: *const clap_plugin, _: bool) -> u32 {
    1
}

unsafe extern "C" fn ports_get(_: *const clap_plugin, index: u32, _: bool, info: *mut clap_audio_port_info) -> bool {
    if index != 0 {
        return false;
    }
    let info = &mut *info;
    info.id = 0;
    info.name = [0; clap_sys::string_sizes::CLAP_NAME_SIZE];
    for (d, s) in info.name.iter_mut().zip(c"main".to_bytes()) {
        *d = *s as c_char;
    }
    info.flags = CLAP_AUDIO_PORT_IS_MAIN;
    info.channel_count = 2;
    info.port_type = CLAP_PORT_STEREO.as_ptr();
    info.in_place_pair = CLAP_INVALID_ID;
    true
}

static PORTS: clap_plugin_audio_ports = clap_plugin_audio_ports { count: Some(ports_count), get: Some(ports_get) };

unsafe extern "C" fn get_extension(_: *const clap_plugin, id: *const c_char) -> *const c_void {
    if !id.is_null() && CStr::from_ptr(id) == CLAP_EXT_AUDIO_PORTS {
        return &PORTS as *const clap_plugin_audio_ports as *const c_void;
    }
    std::ptr::null()
}

unsafe extern "C" fn count(_: *const clap_plugin_factory) -> u32 {
    1
}

unsafe extern "C" fn descriptor(_: *const clap_plugin_factory, index: u32) -> *const clap_plugin_descriptor {
    if index == 0 {
        &DESC
    } else {
        std::ptr::null()
    }
}

unsafe extern "C" fn create(_: *const clap_plugin_factory, _: *const clap_host, id: *const c_char) -> *const clap_plugin {
    if id.is_null() || CStr::from_ptr(id) != CStr::from_ptr(DESC.id) {
        return std::ptr::null();
    }
    Box::into_raw(Box::new(clap_plugin {
        desc: &DESC,
        plugin_data: std::ptr::null_mut(),
        init: Some(init),
        destroy: Some(destroy),
        activate: Some(activate),
        deactivate: Some(nothing),
        start_processing: Some(start),
        stop_processing: Some(nothing),
        reset: Some(nothing),
        process: Some(process),
        get_extension: Some(get_extension),
        on_main_thread: Some(nothing),
    }))
}

static FACTORY: clap_plugin_factory = clap_plugin_factory { get_plugin_count: Some(count), get_plugin_descriptor: Some(descriptor), create_plugin: Some(create) };

unsafe extern "C" fn entry_init(_: *const c_char) -> bool {
    true
}

unsafe extern "C" fn entry_deinit() {}

unsafe extern "C" fn get_factory(id: *const c_char) -> *const c_void {
    if !id.is_null() && CStr::from_ptr(id) == CLAP_PLUGIN_FACTORY_ID {
        return &FACTORY as *const clap_plugin_factory as *const c_void;
    }
    std::ptr::null()
}

#[allow(non_upper_case_globals)]
#[no_mangle]
pub static clap_entry: clap_plugin_entry = clap_plugin_entry { clap_version: CLAP_VERSION, init: Some(entry_init), deinit: Some(entry_deinit), get_factory: Some(get_factory) };
