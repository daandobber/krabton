//! CLAP plugin hosting for Krabton: scanning, loading, audio processing,
//! state and plugin editor windows.
//!
//! Threading follows the CLAP rules: create a [`ClapHost`] and instantiate
//! plugins on the UI (main) thread; hand [`PluginInstance::processor`] to the
//! audio engine; give the processor back to the UI thread to drop it.

mod host;
mod instance;
mod library;
#[cfg(windows)]
mod win32;

pub use instance::{ClapProcessor, PluginInstance};
pub use library::{default_search_paths, find_clap_files, PluginInfo, PluginLibrary};

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Owns the loaded plugin libraries.
pub struct ClapHost {
    libraries: HashMap<PathBuf, Arc<PluginLibrary>>,
}

impl Default for ClapHost {
    fn default() -> Self {
        Self::new()
    }
}

impl ClapHost {
    /// Must be called on the main (UI) thread.
    pub fn new() -> Self {
        host::set_main_thread();
        Self { libraries: HashMap::new() }
    }

    pub fn library(&mut self, path: &Path) -> Result<Arc<PluginLibrary>, String> {
        if let Some(l) = self.libraries.get(path) {
            return Ok(l.clone());
        }
        let lib = Arc::new(PluginLibrary::load(path)?);
        self.libraries.insert(path.to_path_buf(), lib.clone());
        Ok(lib)
    }

    /// Lists every plugin in the `.clap` files below `folders`.
    pub fn scan(&mut self, folders: &[PathBuf]) -> (Vec<PluginInfo>, Vec<String>) {
        let mut infos = Vec::new();
        let mut errors = Vec::new();
        for file in find_clap_files(folders) {
            match self.library(&file) {
                Ok(lib) => infos.extend(lib.plugins()),
                Err(e) => errors.push(e),
            }
        }
        infos.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        (infos, errors)
    }

    /// Creates, initialises and activates a plugin.
    pub fn instantiate(&mut self, path: &Path, plugin_id: &str, sample_rate: f64, max_frames: u32) -> Result<PluginInstance, String> {
        let lib = self.library(path)?;
        PluginInstance::new(lib, plugin_id, sample_rate, max_frames)
    }
}
