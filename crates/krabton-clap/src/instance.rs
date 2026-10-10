//! A running plugin: the main-thread [`PluginInstance`] (state, editor) and
//! the audio-thread [`ClapProcessor`].

use crate::host::HostData;
use crate::library::{cstr, PluginLibrary};
use clap_sys::audio_buffer::clap_audio_buffer;
use clap_sys::events::*;
use clap_sys::ext::audio_ports::{clap_audio_port_info, clap_plugin_audio_ports, CLAP_EXT_AUDIO_PORTS};
use clap_sys::ext::gui::*;
use clap_sys::ext::note_ports::*;
use clap_sys::ext::state::{clap_plugin_state, CLAP_EXT_STATE};
use clap_sys::plugin::clap_plugin;
use clap_sys::process::{clap_process, CLAP_PROCESS_ERROR};
use clap_sys::stream::{clap_istream, clap_ostream};
use krabton_engine::plugin::{AudioProcessor, PluginEvent, PluginEventKind};
use std::ffi::{c_void, CStr, CString};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Dialect {
    Clap,
    Midi,
    None,
}

/// Owns the `clap_plugin`. Dropping it deactivates and destroys the plugin,
/// which must happen on the main thread: both the instance and the
/// processor hold an `Arc`, and processors are always handed back to the UI
/// thread before they are dropped.
pub(crate) struct Shared {
    plugin: *const clap_plugin,
    pub host: Box<HostData>,
    _lib: Arc<PluginLibrary>,
    pub dialect: Dialect,
    pub in_channels: Vec<u32>,
    pub out_channels: Vec<u32>,
    pub max_frames: u32,
    active: AtomicBool,
}

// SAFETY: CLAP defines which thread may call which function; Krabton calls
// process functions only from the processor and the rest from the UI thread.
unsafe impl Send for Shared {}
unsafe impl Sync for Shared {}

impl Shared {
    fn plugin(&self) -> &clap_plugin {
        // SAFETY: valid until Drop.
        unsafe { &*self.plugin }
    }

    /// A plugin extension struct, if the plugin implements it.
    fn ext<T>(&self, id: &CStr) -> Option<&T> {
        let get = self.plugin().get_extension?;
        // SAFETY: the plugin returns a pointer to a static extension struct of type T.
        unsafe { (get(self.plugin, id.as_ptr()) as *const T).as_ref() }
    }
}

impl Drop for Shared {
    fn drop(&mut self) {
        let p = self.plugin();
        // SAFETY: last owner; no processing is running any more.
        unsafe {
            if self.active.swap(false, Ordering::SeqCst) {
                if let Some(f) = p.deactivate {
                    f(self.plugin);
                }
            }
            if let Some(f) = p.destroy {
                f(self.plugin);
            }
        }
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
struct Editor {
    floating: bool,
    #[cfg(windows)]
    hwnd: isize,
    can_resize: bool,
}

/// The main-thread handle of a plugin instance.
pub struct PluginInstance {
    shared: Arc<Shared>,
    pub name: String,
    editor: Option<Editor>,
}

impl PluginInstance {
    pub(crate) fn new(lib: Arc<PluginLibrary>, plugin_id: &str, sample_rate: f64, max_frames: u32) -> Result<Self, String> {
        let host = HostData::new();
        let id = CString::new(plugin_id).map_err(|e| e.to_string())?;
        // SAFETY: valid factory for the life of the library.
        let factory = unsafe { &*lib.factory };
        let create = factory.create_plugin.ok_or("the plugin factory can't create plugins")?;
        // SAFETY: host outlives the plugin (both owned by Shared).
        let plugin = unsafe { create(lib.factory, &host.clap, id.as_ptr()) };
        if plugin.is_null() {
            return Err(format!("could not create plugin {plugin_id}"));
        }
        // SAFETY: non-null plugin from the factory.
        let name = unsafe {
            let desc = (*plugin).desc;
            if desc.is_null() {
                plugin_id.to_string()
            } else {
                cstr((*desc).name)
            }
        };
        // From here on, dropping `shared` destroys the plugin.
        let mut shared = Shared {
            plugin,
            host,
            _lib: lib,
            dialect: Dialect::None,
            in_channels: Vec::new(),
            out_channels: Vec::new(),
            max_frames,
            active: AtomicBool::new(false),
        };
        let p = shared.plugin();
        let init = p.init.ok_or("plugin has no init")?;
        // SAFETY: called once, on the main thread.
        if !unsafe { init(plugin) } {
            return Err(format!("{name}: init failed"));
        }
        shared.dialect = note_dialect(&shared);
        let (ins, outs) = audio_ports(&shared);
        shared.in_channels = ins;
        shared.out_channels = outs;
        let activate = shared.plugin().activate.ok_or("plugin has no activate")?;
        // SAFETY: main thread, plugin initialised and not active.
        if !unsafe { activate(plugin, sample_rate, 1, max_frames) } {
            return Err(format!("{name}: activate failed"));
        }
        shared.active.store(true, Ordering::SeqCst);
        Ok(Self { shared: Arc::new(shared), name, editor: None })
    }

    /// A new audio-thread processor for this instance. Only one should be
    /// in use at a time.
    pub fn processor(&self) -> Box<dyn AudioProcessor> {
        Box::new(ClapProcessor::new(self.shared.clone()))
    }

    pub fn accepts_notes(&self) -> bool {
        self.shared.dialect != Dialect::None
    }

    pub fn save_state(&self) -> Option<Vec<u8>> {
        let st = self.shared.ext::<clap_plugin_state>(CLAP_EXT_STATE)?;
        let save = st.save?;
        let mut buf: Vec<u8> = Vec::new();
        let stream = clap_ostream { ctx: &mut buf as *mut Vec<u8> as *mut c_void, write: Some(stream_write) };
        // SAFETY: main thread; stream valid for the call.
        unsafe { save(self.shared.plugin, &stream) }.then_some(buf)
    }

    pub fn load_state(&self, data: &[u8]) -> bool {
        let Some(st) = self.shared.ext::<clap_plugin_state>(CLAP_EXT_STATE) else { return false };
        let Some(load) = st.load else { return false };
        let mut reader = Reader { data, pos: 0 };
        let stream = clap_istream { ctx: &mut reader as *mut Reader as *mut c_void, read: Some(stream_read) };
        // SAFETY: main thread; stream valid for the call.
        unsafe { load(self.shared.plugin, &stream) }
    }

    fn gui(&self) -> Option<&clap_plugin_gui> {
        self.shared.ext::<clap_plugin_gui>(CLAP_EXT_GUI)
    }

    pub fn has_editor(&self) -> bool {
        self.gui().is_some()
    }

    pub fn editor_open(&self) -> bool {
        self.editor.is_some()
    }

    /// Opens the plugin's own editor, embedded in a native window when the
    /// platform allows it, otherwise as the plugin's floating window.
    pub fn open_editor(&mut self, title: &str) -> Result<(), String> {
        if let Some(_ed) = &self.editor {
            #[cfg(windows)]
            {
                crate::win32::focus_window(_ed.hwnd);
            }
            return Ok(());
        }
        let gui = *self.gui().ok_or("this plugin has no editor")?;
        let p = self.shared.plugin;
        let supported = |api: &CStr, floating: bool| gui.is_api_supported.map(|f| unsafe { f(p, api.as_ptr(), floating) }).unwrap_or(false);

        #[cfg(windows)]
        {
        if supported(CLAP_WINDOW_API_WIN32, false) {
            // SAFETY: GUI calls happen on the main thread in the order CLAP requires.
            unsafe {
                let create = gui.create.ok_or("no gui.create")?;
                if !create(p, CLAP_WINDOW_API_WIN32.as_ptr(), false) {
                    return Err("the plugin could not create its editor".into());
                }
                let can_resize = gui.can_resize.map(|f| f(p)).unwrap_or(false);
                let hwnd = crate::win32::create_window(title, 800, 600, can_resize)?;
                if let Some(f) = gui.set_scale {
                    f(p, crate::win32::dpi_scale(hwnd));
                }
                let (mut w, mut h) = (800u32, 600u32);
                if let Some(f) = gui.get_size {
                    f(p, &mut w, &mut h);
                }
                crate::win32::resize_window(hwnd, w, h);
                let window = clap_window { api: CLAP_WINDOW_API_WIN32.as_ptr(), specific: clap_window_handle { win32: hwnd as *mut c_void } };
                let parented = gui.set_parent.map(|f| f(p, &window)).unwrap_or(false);
                if !parented {
                    if let Some(f) = gui.destroy {
                        f(p);
                    }
                    crate::win32::destroy_window(hwnd);
                    return Err("the plugin could not attach its editor".into());
                }
                if let Some(f) = gui.show {
                    f(p);
                }
                crate::win32::show_window(hwnd);
                self.editor = Some(Editor { floating: false, hwnd, can_resize });
                return Ok(());
            }
        }
        }

        let api = if cfg!(windows) {
            CLAP_WINDOW_API_WIN32
        } else if cfg!(target_os = "macos") {
            CLAP_WINDOW_API_COCOA
        } else {
            CLAP_WINDOW_API_X11
        };
        if supported(api, true) {
            // SAFETY: as above, floating variant.
            unsafe {
                let create = gui.create.ok_or("no gui.create")?;
                if !create(p, api.as_ptr(), true) {
                    return Err("the plugin could not create its editor".into());
                }
                if let (Some(f), Ok(t)) = (gui.suggest_title, CString::new(title)) {
                    f(p, t.as_ptr());
                }
                if let Some(f) = gui.show {
                    f(p);
                }
            }
            self.editor = Some(Editor {
                floating: true,
                #[cfg(windows)]
                hwnd: 0,
                can_resize: false,
            });
            return Ok(());
        }
        Err("no supported editor window API on this platform".into())
    }

    pub fn close_editor(&mut self) {
        let Some(ed) = self.editor.take() else { return };
        if let Some(gui) = self.gui().copied() {
            // SAFETY: main thread, editor was created.
            unsafe {
                if let Some(f) = gui.hide {
                    f(self.shared.plugin);
                }
                if let Some(f) = gui.destroy {
                    f(self.shared.plugin);
                }
            }
        }
        #[cfg(windows)]
        {
            if !ed.floating {
                crate::win32::destroy_window(ed.hwnd);
            }
        }
        let _ = ed;
    }

    /// Call regularly on the main thread. Runs requested main-thread
    /// callbacks and editor housekeeping. Returns true when the plugin
    /// reported that its state changed.
    pub fn idle(&mut self) -> bool {
        let shared = self.shared.clone();
        let h = &shared.host;
        if h.callback_requested.swap(false, Ordering::Relaxed) {
            if let Some(f) = self.shared.plugin().on_main_thread {
                // SAFETY: main thread.
                unsafe { f(self.shared.plugin) };
            }
        }
        if h.restart_requested.swap(false, Ordering::Relaxed) {
            eprintln!("[clap] {} requested a restart (not supported yet)", self.name);
        }
        let closed_by_plugin = h.gui_closed.swap(false, Ordering::Relaxed);
        let resize = h.resize_request.swap(0, Ordering::Relaxed);
        #[cfg(windows)]
        {
        if let Some(ed) = &self.editor {
            if !ed.floating {
                let hwnd = ed.hwnd;
                if resize != 0 {
                    crate::win32::resize_window(hwnd, (resize >> 32) as u32, (resize & 0xffff_ffff) as u32);
                }
                if ed.can_resize {
                    if let (Some((w, hgt)), Some(gui)) = (crate::win32::take_resized(hwnd), self.gui().copied()) {
                        // SAFETY: main thread, editor exists.
                        unsafe {
                            let (mut w, mut hgt) = (w, hgt);
                            if let Some(f) = gui.adjust_size {
                                f(self.shared.plugin, &mut w, &mut hgt);
                            }
                            if let Some(f) = gui.set_size {
                                f(self.shared.plugin, w, hgt);
                            }
                        }
                    }
                }
                if crate::win32::take_closed(hwnd) {
                    self.close_editor();
                }
            }
        }
        }
        let _ = resize;
        if closed_by_plugin {
            self.editor = None;
        }
        h.state_dirty.swap(false, Ordering::Relaxed)
    }
}

impl Drop for PluginInstance {
    fn drop(&mut self) {
        self.close_editor();
    }
}

fn note_dialect(shared: &Shared) -> Dialect {
    let Some(np) = shared.ext::<clap_plugin_note_ports>(CLAP_EXT_NOTE_PORTS) else { return Dialect::None };
    let p = shared.plugin;
    // SAFETY: main thread, before activation.
    unsafe {
        let count = np.count.map(|f| f(p, true)).unwrap_or(0);
        if count == 0 {
            return Dialect::None;
        }
        let mut info: clap_note_port_info = std::mem::zeroed();
        if !np.get.map(|f| f(p, 0, true, &mut info)).unwrap_or(false) {
            return Dialect::Midi;
        }
        if info.preferred_dialect == CLAP_NOTE_DIALECT_MIDI && info.supported_dialects & CLAP_NOTE_DIALECT_MIDI != 0 {
            Dialect::Midi
        } else if info.supported_dialects & CLAP_NOTE_DIALECT_CLAP != 0 {
            Dialect::Clap
        } else if info.supported_dialects & CLAP_NOTE_DIALECT_MIDI != 0 {
            Dialect::Midi
        } else {
            Dialect::None
        }
    }
}

fn audio_ports(shared: &Shared) -> (Vec<u32>, Vec<u32>) {
    let Some(ap) = shared.ext::<clap_plugin_audio_ports>(CLAP_EXT_AUDIO_PORTS) else { return (Vec::new(), vec![2]) };
    let p = shared.plugin;
    let mut res = (Vec::new(), Vec::new());
    for is_input in [true, false] {
        // SAFETY: main thread, before activation.
        unsafe {
            let n = ap.count.map(|f| f(p, is_input)).unwrap_or(0);
            for i in 0..n {
                let mut info: clap_audio_port_info = std::mem::zeroed();
                let ok = ap.get.map(|f| f(p, i, is_input, &mut info)).unwrap_or(false);
                let ch = if ok { info.channel_count.clamp(1, 16) } else { 2 };
                if is_input {
                    res.0.push(ch);
                } else {
                    res.1.push(ch);
                }
            }
        }
    }
    res
}

unsafe extern "C" fn stream_write(stream: *const clap_ostream, buffer: *const c_void, size: u64) -> i64 {
    let v = &mut *((*stream).ctx as *mut Vec<u8>);
    v.extend_from_slice(std::slice::from_raw_parts(buffer as *const u8, size as usize));
    size as i64
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

unsafe extern "C" fn stream_read(stream: *const clap_istream, buffer: *mut c_void, size: u64) -> i64 {
    let r = &mut *((*stream).ctx as *mut Reader);
    let n = (size as usize).min(r.data.len() - r.pos);
    std::ptr::copy_nonoverlapping(r.data.as_ptr().add(r.pos), buffer as *mut u8, n);
    r.pos += n;
    n as i64
}

/// A view of our event storage as a `clap_input_events` list.
struct EventsCtx {
    base: *const u8,
    stride: usize,
    len: u32,
}

unsafe extern "C" fn events_size(list: *const clap_input_events) -> u32 {
    (*((*list).ctx as *const EventsCtx)).len
}

unsafe extern "C" fn events_get(list: *const clap_input_events, index: u32) -> *const clap_event_header {
    let c = &*((*list).ctx as *const EventsCtx);
    if index >= c.len {
        return std::ptr::null();
    }
    c.base.add(index as usize * c.stride) as *const clap_event_header
}

unsafe extern "C" fn events_push(_list: *const clap_output_events, _event: *const clap_event_header) -> bool {
    true
}

/// The audio-thread half of a plugin instance.
pub struct ClapProcessor {
    shared: Arc<Shared>,
    started: bool,
    failed: bool,
    steady: i64,
    max: usize,
    in_data: Vec<Vec<f32>>,
    out_data: Vec<Vec<f32>>,
    in_ptrs: Vec<*mut f32>,
    out_ptrs: Vec<*mut f32>,
    in_bufs: Vec<clap_audio_buffer>,
    out_bufs: Vec<clap_audio_buffer>,
    notes: Vec<clap_event_note>,
    midis: Vec<clap_event_midi>,
}

// SAFETY: the raw pointers point into this struct's own heap buffers.
unsafe impl Send for ClapProcessor {}

impl ClapProcessor {
    fn new(shared: Arc<Shared>) -> Self {
        let max = shared.max_frames.max(1) as usize;
        let alloc = |ports: &[u32]| -> Vec<Vec<f32>> { ports.iter().flat_map(|&c| (0..c).map(|_| vec![0.0f32; max])).collect() };
        let mut s = Self {
            in_data: alloc(&shared.in_channels),
            out_data: alloc(&shared.out_channels),
            in_ptrs: Vec::new(),
            out_ptrs: Vec::new(),
            in_bufs: Vec::new(),
            out_bufs: Vec::new(),
            notes: Vec::with_capacity(512),
            midis: Vec::with_capacity(512),
            started: false,
            failed: false,
            steady: 0,
            max,
            shared,
        };
        s.in_ptrs = s.in_data.iter_mut().map(|v| v.as_mut_ptr()).collect();
        s.out_ptrs = s.out_data.iter_mut().map(|v| v.as_mut_ptr()).collect();
        let mut off = 0;
        for &c in &s.shared.in_channels {
            // SAFETY: `off + c` stays within in_ptrs.
            let data32 = unsafe { s.in_ptrs.as_mut_ptr().add(off) };
            s.in_bufs.push(clap_audio_buffer { data32, data64: std::ptr::null_mut(), channel_count: c, latency: 0, constant_mask: 0 });
            off += c as usize;
        }
        let mut off = 0;
        for &c in &s.shared.out_channels {
            // SAFETY: `off + c` stays within out_ptrs.
            let data32 = unsafe { s.out_ptrs.as_mut_ptr().add(off) };
            s.out_bufs.push(clap_audio_buffer { data32, data64: std::ptr::null_mut(), channel_count: c, latency: 0, constant_mask: 0 });
            off += c as usize;
        }
        s
    }

    fn push_event(&mut self, e: &PluginEvent, time: u32) {
        match self.shared.dialect {
            Dialect::Clap => {
                let (type_, key, channel, velocity) = match e.kind {
                    PluginEventKind::NoteOn { velocity } => (CLAP_EVENT_NOTE_ON, e.pitch as i16, 0, velocity as f64),
                    PluginEventKind::NoteOff => (CLAP_EVENT_NOTE_OFF, e.pitch as i16, 0, 0.0),
                    PluginEventKind::AllNotesOff => (CLAP_EVENT_NOTE_OFF, -1, -1, 0.0),
                };
                if self.notes.len() < self.notes.capacity() {
                    self.notes.push(clap_event_note {
                        header: clap_event_header {
                            size: std::mem::size_of::<clap_event_note>() as u32,
                            time,
                            space_id: CLAP_CORE_EVENT_SPACE_ID,
                            type_,
                            flags: 0,
                        },
                        note_id: -1,
                        port_index: 0,
                        channel,
                        key,
                        velocity,
                    });
                }
            }
            Dialect::Midi => {
                let data = match e.kind {
                    PluginEventKind::NoteOn { velocity } => [0x90, e.pitch & 0x7f, ((velocity * 127.0).round() as u8).clamp(1, 127)],
                    PluginEventKind::NoteOff => [0x80, e.pitch & 0x7f, 0],
                    PluginEventKind::AllNotesOff => [0xB0, 123, 0],
                };
                if self.midis.len() < self.midis.capacity() {
                    self.midis.push(clap_event_midi {
                        header: clap_event_header {
                            size: std::mem::size_of::<clap_event_midi>() as u32,
                            time,
                            space_id: CLAP_CORE_EVENT_SPACE_ID,
                            type_: CLAP_EVENT_MIDI,
                            flags: 0,
                        },
                        port_index: 0,
                        data,
                    });
                }
            }
            Dialect::None => {}
        }
    }

    /// Processes at most `self.max` frames. An instrument adds what it plays
    /// to `left`/`right`; an `effect` gets them as its input and replaces them.
    fn run(&mut self, events: &[PluginEvent], from: usize, left: &mut [f32], right: &mut [f32], last: bool, effect: bool) {
        let n = left.len();
        let plugin_ptr = self.shared.plugin;
        let (start_fn, process_fn) = {
            let p = self.shared.plugin();
            (p.start_processing, p.process)
        };
        if !self.started {
            // SAFETY: audio thread, plugin active.
            let ok = start_fn.map(|f| unsafe { f(plugin_ptr) }).unwrap_or(true);
            if !ok {
                self.failed = true;
                return;
            }
            self.started = true;
        }
        self.notes.clear();
        self.midis.clear();
        for e in events {
            let off = e.offset as usize;
            if (off >= from && off < from + n) || (last && off >= from + n) {
                let t = (off.saturating_sub(from)).min(n.saturating_sub(1)) as u32;
                self.push_event(e, t);
            }
        }
        let ctx = match self.shared.dialect {
            Dialect::Clap => EventsCtx { base: self.notes.as_ptr() as *const u8, stride: std::mem::size_of::<clap_event_note>(), len: self.notes.len() as u32 },
            Dialect::Midi => EventsCtx { base: self.midis.as_ptr() as *const u8, stride: std::mem::size_of::<clap_event_midi>(), len: self.midis.len() as u32 },
            Dialect::None => EventsCtx { base: std::ptr::null(), stride: 0, len: 0 },
        };
        let in_events = clap_input_events { ctx: &ctx as *const EventsCtx as *mut c_void, size: Some(events_size), get: Some(events_get) };
        let out_events = clap_output_events { ctx: std::ptr::null_mut(), try_push: Some(events_push) };
        for b in self.out_data.iter_mut() {
            b[..n].fill(0.0);
        }
        if effect {
            // The first input port gets the track's audio (mono: both sides).
            match self.shared.in_channels.first() {
                Some(&c) if c >= 2 => {
                    self.in_data[0][..n].copy_from_slice(left);
                    self.in_data[1][..n].copy_from_slice(right);
                }
                Some(_) => {
                    for i in 0..n {
                        self.in_data[0][i] = 0.5 * (left[i] + right[i]);
                    }
                }
                None => return,
            }
        }
        let process = clap_process {
            steady_time: self.steady,
            frames_count: n as u32,
            transport: std::ptr::null(),
            audio_inputs: self.in_bufs.as_ptr(),
            audio_outputs: self.out_bufs.as_mut_ptr(),
            audio_inputs_count: self.in_bufs.len() as u32,
            audio_outputs_count: self.out_bufs.len() as u32,
            in_events: &in_events,
            out_events: &out_events,
        };
        let Some(f) = process_fn else { return };
        // SAFETY: audio thread; every pointer in `process` is valid for the call.
        let status = unsafe { f(plugin_ptr, &process) };
        self.steady += n as i64;
        if status == CLAP_PROCESS_ERROR || self.out_bufs.is_empty() {
            return;
        }
        let ch = self.out_bufs[0].channel_count as usize;
        let (a, b) = (0, if ch >= 2 { 1 } else { 0 });
        for i in 0..n {
            let (x, y) = (self.out_data[a][i], self.out_data[b][i]);
            if x.is_finite() && y.is_finite() {
                if effect {
                    left[i] = x;
                    right[i] = y;
                } else {
                    left[i] += x;
                    right[i] += y;
                }
            }
        }
    }
}

impl AudioProcessor for ClapProcessor {
    fn process(&mut self, events: &[PluginEvent], left: &mut [f32], right: &mut [f32]) {
        if self.failed {
            return;
        }
        let total = left.len().min(right.len());
        let mut done = 0;
        while done < total {
            let n = (total - done).min(self.max);
            let last = done + n >= total;
            self.run(events, done, &mut left[done..done + n], &mut right[done..done + n], last, false);
            done += n;
        }
    }

    fn process_effect(&mut self, left: &mut [f32], right: &mut [f32]) {
        if self.failed || self.shared.in_channels.is_empty() {
            return;
        }
        let total = left.len().min(right.len());
        let mut done = 0;
        while done < total {
            let n = (total - done).min(self.max);
            self.run(&[], done, &mut left[done..done + n], &mut right[done..done + n], done + n >= total, true);
            done += n;
        }
    }

    fn suspend(&mut self) {
        if self.started {
            if let Some(f) = self.shared.plugin().stop_processing {
                // SAFETY: audio thread, processing was started.
                unsafe { f(self.shared.plugin) };
            }
            self.started = false;
        }
    }
}
