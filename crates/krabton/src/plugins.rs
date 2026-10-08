//! CLAP plugin instruments: scanning, one live instance per placed plugin,
//! and their state.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use krabton_clap::{ClapHost, PluginInfo, PluginInstance};
use krabton_engine::engine::MAX_BLOCK;
use krabton_engine::plugin::AudioProcessor;
use krabton_engine::{Command, Instrument, PluginInstrument, Project};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub enum PluginStatus {
    Loaded { has_editor: bool, editor_open: bool },
    Failed(String),
    Loading,
}

pub struct PluginManager {
    host: ClapHost,
    pub available: Vec<PluginInfo>,
    pub scan_errors: Vec<String>,
    pub extra_folders: Vec<PathBuf>,
    pub scanned: bool,
    instances: HashMap<u64, PluginInstance>,
    failed: HashMap<u64, String>,
}

impl PluginManager {
    /// Must be created on the UI thread.
    pub fn new() -> Self {
        Self {
            host: ClapHost::new(),
            available: Vec::new(),
            scan_errors: Vec::new(),
            extra_folders: Vec::new(),
            scanned: false,
            instances: HashMap::new(),
            failed: HashMap::new(),
        }
    }

    pub fn folders(&self) -> Vec<PathBuf> {
        let mut v = krabton_clap::default_search_paths();
        v.extend(self.extra_folders.iter().cloned());
        v
    }

    pub fn scan(&mut self) {
        let folders = self.folders();
        let (infos, errors) = self.host.scan(&folders);
        self.available = infos;
        self.scan_errors = errors;
        self.scanned = true;
    }

    pub fn add_folder(&mut self, dir: PathBuf) {
        if !self.extra_folders.contains(&dir) {
            self.extra_folders.push(dir);
        }
        self.scan();
    }

    pub fn instruments(&self) -> impl Iterator<Item = &PluginInfo> {
        self.available.iter().filter(|p| p.is_instrument())
    }

    pub fn effect_count(&self) -> usize {
        self.available.iter().filter(|p| p.is_effect() && !p.is_instrument()).count()
    }

    pub fn make_instrument(info: &PluginInfo, instance: u64) -> PluginInstrument {
        PluginInstrument {
            instance,
            format: "clap".into(),
            plugin_id: info.id.clone(),
            name: info.name.clone(),
            vendor: info.vendor.clone(),
            path: info.path.clone(),
            state: None,
        }
    }

    fn create(&mut self, p: &PluginInstrument, sample_rate: f64, state: Option<&[u8]>) -> Result<PluginInstance, String> {
        let inst = self.host.instantiate(&p.path, &p.plugin_id, sample_rate, MAX_BLOCK as u32)?;
        let decoded;
        let state = match state {
            Some(s) => Some(s),
            None => match &p.state {
                Some(s) => {
                    decoded = B64.decode(s.as_bytes()).map_err(|e| e.to_string())?;
                    Some(decoded.as_slice())
                }
                None => None,
            },
        };
        if let Some(s) = state {
            if !inst.load_state(s) {
                eprintln!("{}: could not restore its state", p.name);
            }
        }
        Ok(inst)
    }

    /// Creates instances for plugins that appeared in the project (new
    /// tracks, loaded sets, undo) and drops the ones that disappeared.
    /// Returns commands for the engine and error messages for the user.
    pub fn sync(&mut self, project: &Project, sample_rate: f64) -> (Vec<Command>, Vec<String>) {
        let mut wanted: HashMap<u64, &PluginInstrument> = HashMap::new();
        for t in &project.tracks {
            if let Some(Instrument::Plugin(p)) = &t.instrument {
                wanted.insert(p.instance, p);
            }
        }
        self.instances.retain(|id, _| wanted.contains_key(id));
        self.failed.retain(|id, _| wanted.contains_key(id));
        let mut cmds = Vec::new();
        let mut msgs = Vec::new();
        for (id, p) in wanted {
            if self.instances.contains_key(&id) || self.failed.contains_key(&id) {
                continue;
            }
            match self.create(p, sample_rate, None) {
                Ok(inst) => {
                    cmds.push(Command::AttachProcessor { instance: id, processor: inst.processor() });
                    if inst.has_editor() {
                        msgs.push(format!("{} is ready. Open its editor in the device view.", p.name));
                    }
                    self.instances.insert(id, inst);
                }
                Err(e) => {
                    msgs.push(format!("Plugin failed to load: {}: {e}", p.name));
                    self.failed.insert(id, e);
                }
            }
        }
        (cmds, msgs)
    }

    /// Main-thread housekeeping. Returns true if a plugin changed its state.
    pub fn idle(&mut self) -> bool {
        let mut dirty = false;
        for inst in self.instances.values_mut() {
            dirty |= inst.idle();
        }
        dirty
    }

    pub fn status(&self, instance: u64) -> PluginStatus {
        if let Some(i) = self.instances.get(&instance) {
            PluginStatus::Loaded { has_editor: i.has_editor(), editor_open: i.editor_open() }
        } else if let Some(e) = self.failed.get(&instance) {
            PluginStatus::Failed(e.clone())
        } else {
            PluginStatus::Loading
        }
    }

    pub fn toggle_editor(&mut self, instance: u64, title: &str) -> Result<(), String> {
        let inst = self.instances.get_mut(&instance).ok_or("the plugin is not loaded")?;
        if inst.editor_open() {
            inst.close_editor();
            Ok(())
        } else {
            inst.open_editor(title)
        }
    }

    /// Writes every plugin's current state into the project, so it gets
    /// saved (or copied, when duplicating a track).
    pub fn store_states(&self, project: &mut Project) {
        for t in &mut project.tracks {
            if let Some(Instrument::Plugin(p)) = &mut t.instrument {
                if let Some(bytes) = self.instances.get(&p.instance).and_then(|i| i.save_state()) {
                    let encoded: Arc<str> = Arc::from(B64.encode(bytes));
                    if p.state.as_deref() != Some(&*encoded) {
                        p.state = Some(encoded);
                    }
                }
            }
        }
    }

    /// Separate instances, at the export sample rate and with the current
    /// state, for an offline render. Keep the returned instances alive (on
    /// this thread) until the render thread has given the processors back.
    pub fn export_instances(&mut self, project: &Project, sample_rate: f64) -> (Vec<PluginInstance>, Vec<(u64, Box<dyn AudioProcessor>)>, Vec<String>) {
        let mut keep = Vec::new();
        let mut procs = Vec::new();
        let mut errors = Vec::new();
        for t in &project.tracks {
            let Some(Instrument::Plugin(p)) = &t.instrument else { continue };
            let state = self.instances.get(&p.instance).and_then(|i| i.save_state());
            match self.create(p, sample_rate, state.as_deref()) {
                Ok(inst) => {
                    procs.push((p.instance, inst.processor()));
                    keep.push(inst);
                }
                Err(e) => errors.push(format!("{}: {e}", p.name)),
            }
        }
        (keep, procs, errors)
    }
}
