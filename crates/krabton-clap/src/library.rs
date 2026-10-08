//! Loading CLAP binaries and listing the plugins inside them.

use clap_sys::entry::clap_plugin_entry;
use clap_sys::factory::plugin_factory::{clap_plugin_factory, CLAP_PLUGIN_FACTORY_ID};
use clap_sys::version::clap_version_is_compatible;
use std::ffi::{c_char, CStr, CString};
use std::mem::ManuallyDrop;
use std::path::{Path, PathBuf};

/// What a plugin says about itself.
#[derive(Clone, Debug, PartialEq)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub version: String,
    pub description: String,
    pub path: PathBuf,
    pub features: Vec<String>,
}

impl PluginInfo {
    pub fn is_instrument(&self) -> bool {
        self.features.iter().any(|f| f == "instrument")
    }

    pub fn is_effect(&self) -> bool {
        self.features.iter().any(|f| f == "audio-effect")
    }
}

pub(crate) unsafe fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}

/// A loaded `.clap` binary. Libraries are never unloaded: plugins and their
/// threads may outlive any single instance.
pub struct PluginLibrary {
    _lib: ManuallyDrop<libloading::Library>,
    pub(crate) factory: *const clap_plugin_factory,
    pub path: PathBuf,
}

// SAFETY: the entry and factory are immutable tables that CLAP requires to be
// usable from any thread.
unsafe impl Send for PluginLibrary {}
unsafe impl Sync for PluginLibrary {}

impl PluginLibrary {
    pub fn load(path: &Path) -> Result<Self, String> {
        let binary = binary_path(path);
        let err = |e: &dyn std::fmt::Display| format!("{}: {e}", path.display());
        // SAFETY: loading a plugin runs its initialisers; that is the point of hosting.
        let lib = unsafe { libloading::Library::new(&binary) }.map_err(|e| err(&e))?;
        // SAFETY: `clap_entry` is a data symbol holding a `clap_plugin_entry`.
        let entry: *const clap_plugin_entry = unsafe {
            let sym: libloading::Symbol<*const clap_plugin_entry> = lib.get(b"clap_entry\0").map_err(|e| err(&e))?;
            *sym
        };
        if entry.is_null() {
            return Err(err(&"null clap_entry"));
        }
        // SAFETY: checked non-null above.
        let e = unsafe { &*entry };
        if !clap_version_is_compatible(e.clap_version) {
            return Err(err(&"incompatible CLAP version"));
        }
        let cpath = CString::new(path.to_string_lossy().as_bytes()).map_err(|e| err(&e))?;
        let init = e.init.ok_or_else(|| err(&"missing entry init"))?;
        // SAFETY: called once per library with a valid C string.
        if !unsafe { init(cpath.as_ptr()) } {
            return Err(err(&"plugin entry init failed"));
        }
        let get_factory = e.get_factory.ok_or_else(|| err(&"missing get_factory"))?;
        // SAFETY: valid factory id string.
        let factory = unsafe { get_factory(CLAP_PLUGIN_FACTORY_ID.as_ptr()) } as *const clap_plugin_factory;
        if factory.is_null() {
            return Err(err(&"no plugin factory"));
        }
        Ok(Self { _lib: ManuallyDrop::new(lib), factory, path: path.to_path_buf() })
    }

    pub fn plugins(&self) -> Vec<PluginInfo> {
        // SAFETY: the factory pointer is valid for the life of the library.
        unsafe {
            let f = &*self.factory;
            let count = f.get_plugin_count.map(|c| c(self.factory)).unwrap_or(0);
            let Some(get) = f.get_plugin_descriptor else { return Vec::new() };
            (0..count)
                .filter_map(|i| {
                    let d = get(self.factory, i);
                    if d.is_null() {
                        return None;
                    }
                    let d = &*d;
                    let mut features = Vec::new();
                    if !d.features.is_null() {
                        let mut k = 0;
                        loop {
                            let p = *d.features.add(k);
                            if p.is_null() || k > 64 {
                                break;
                            }
                            features.push(cstr(p));
                            k += 1;
                        }
                    }
                    Some(PluginInfo {
                        id: cstr(d.id),
                        name: cstr(d.name),
                        vendor: cstr(d.vendor),
                        version: cstr(d.version),
                        description: cstr(d.description),
                        path: self.path.clone(),
                        features,
                    })
                })
                .collect()
        }
    }
}

/// On macOS a `.clap` is a bundle directory; elsewhere it is the binary itself.
fn binary_path(path: &Path) -> PathBuf {
    if path.is_dir() {
        if let Some(stem) = path.file_stem() {
            let mac = path.join("Contents").join("MacOS").join(stem);
            if mac.exists() {
                return mac;
            }
        }
    }
    path.to_path_buf()
}

/// The standard CLAP locations for this platform, plus `CLAP_PATH`.
pub fn default_search_paths() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(p) = std::env::var_os("CLAP_PATH") {
        v.extend(std::env::split_paths(&p));
    }
    #[cfg(windows)]
    {
        if let Some(c) = std::env::var_os("COMMONPROGRAMFILES") {
            v.push(PathBuf::from(c).join("CLAP"));
        }
        if let Some(l) = std::env::var_os("LOCALAPPDATA") {
            v.push(PathBuf::from(l).join("Programs").join("Common").join("CLAP"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        v.push(PathBuf::from("/Library/Audio/Plug-Ins/CLAP"));
        if let Some(h) = std::env::var_os("HOME") {
            v.push(PathBuf::from(h).join("Library/Audio/Plug-Ins/CLAP"));
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(h) = std::env::var_os("HOME") {
            v.push(PathBuf::from(h).join(".clap"));
        }
        v.push(PathBuf::from("/usr/lib/clap"));
        v.push(PathBuf::from("/usr/local/lib/clap"));
    }
    v
}

/// All `.clap` files/bundles below the given folders.
pub fn find_clap_files(folders: &[PathBuf]) -> Vec<PathBuf> {
    fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
        if depth > 6 {
            return;
        }
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            let is_clap = p.extension().map(|x| x.eq_ignore_ascii_case("clap")).unwrap_or(false);
            if is_clap {
                out.push(p);
            } else if p.is_dir() {
                walk(&p, depth + 1, out);
            }
        }
    }
    let mut out = Vec::new();
    for f in folders {
        if f.extension().map(|x| x.eq_ignore_ascii_case("clap")).unwrap_or(false) {
            out.push(f.clone());
        } else {
            walk(f, 0, &mut out);
        }
    }
    out.sort();
    out.dedup();
    out
}
