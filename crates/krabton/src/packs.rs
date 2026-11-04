//! Everything Krabton can fetch, in one list: the grand piano, the orchestra
//! and the soloists, the drum kits, the SoundFonts, the voices, the Princess
//! Sounds and the helper programs. Each still installs its own way (see the
//! `*_lib` modules); here they stand in a queue, one at a time, for the Packs
//! window, the first start and the installer's choice.

use crate::app::App;
use krabton_engine::dsp::real_drums;
use krabton_engine::orchestra::{self, INSTRUMENTS};
use krabton_engine::princess_sounds as ps;
use std::collections::{BTreeSet, HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    Piano,
    Instrument(usize),
    Drums(usize),
    Sf2(usize),
    Voice(usize),
    Princess,
    Ffmpeg,
    YtDlp,
}

/// The groups, in the order they are shown. The Windows installer offers
/// them by these ids (see `packaging/windows/krabton.iss`).
pub const GROUPS: [(&str, &str); 7] = [
    ("piano", "Grand Piano"),
    ("orchestra", "Orchestra"),
    ("soloists", "Soloists and Band"),
    ("drums", "Drum Kits"),
    ("soundfonts", "SoundFonts"),
    ("voices", "Voices"),
    ("tools", "Sounds and Tools"),
];

pub struct Pack {
    pub kind: Kind,
    pub group: &'static str,
    pub name: String,
    pub about: String,
    /// About how big its download is; 0: nothing to download.
    pub mb: u32,
}

/// Every pack on this computer's list.
pub fn all() -> Vec<Pack> {
    let mut v = vec![Pack { kind: Kind::Piano, group: "piano", name: "Salamander Grand Piano".into(), about: krabton_engine::piano::library::CREDIT.into(), mb: 750 }];
    for (i, d) in INSTRUMENTS.iter().enumerate() {
        let group = if d.src.folder == orchestra::FOLDER { "orchestra" } else { "soloists" };
        v.push(Pack { kind: Kind::Instrument(i), group, name: d.name.into(), about: d.src.short.into(), mb: d.mb });
    }
    for (i, p) in real_drums::PACKS.iter().enumerate() {
        let kits: Vec<&str> = p.kits.iter().map(|k| k.0).collect();
        v.push(Pack { kind: Kind::Drums(i), group: "drums", name: kits.join(", "), about: p.credit.into(), mb: p.mb });
    }
    for (i, o) in crate::sf2_lib::OFFERS.iter().enumerate() {
        v.push(Pack { kind: Kind::Sf2(i), group: "soundfonts", name: o.name.into(), about: o.about.into(), mb: (o.bytes / 1_000_000) as u32 });
    }
    for (i, o) in crate::singer_lib::OFFERS.iter().enumerate() {
        v.push(Pack { kind: Kind::Voice(i), group: "voices", name: o.name.into(), about: format!("{}. {}", o.about, o.terms), mb: (o.bytes / 1_000_000) as u32 });
    }
    v.push(Pack { kind: Kind::Princess, group: "tools", name: "Princess Sounds".into(), about: "Bells, plucks, chords and 808s in the Analog Princess style, made on this computer".into(), mb: 0 });
    if let Some((_, bytes)) = crate::video::ffmpeg::download() {
        v.push(Pack { kind: Kind::Ffmpeg, group: "tools", name: "ffmpeg".into(), about: "Plays films in the Video and opens MP3s and other audio files".into(), mb: (bytes / 1_000_000) as u32 });
    }
    if crate::video::ytdl::download().is_some() {
        v.push(Pack { kind: Kind::YtDlp, group: "tools", name: "yt-dlp".into(), about: "Fetches films from links for the Video".into(), mb: 20 });
    }
    v
}

/// The packs the installer's choice names: group ids by commas.
pub fn parse_choice(text: &str, packs: &[Pack]) -> Vec<Kind> {
    let groups: Vec<&str> = text.split([',', '\n', '\r']).map(str::trim).filter(|g| !g.is_empty()).collect();
    packs.iter().filter(|p| groups.contains(&p.group)).map(|p| p.kind).collect()
}

/// A pack's place on this disk, as Krabton keeps it.
fn place(kind: Kind) -> Vec<PathBuf> {
    match kind {
        Kind::Piano => vec![crate::piano_lib::library_dir()],
        Kind::Instrument(i) => vec![crate::orchestra_lib::dir(i)],
        Kind::Drums(i) => vec![real_drums::dir(&real_drums::PACKS[i])],
        Kind::Sf2(i) => {
            let o = &crate::sf2_lib::OFFERS[i];
            let dir = crate::sf2_lib::dir();
            vec![dir.join(o.file), dir.join(format!("{}-LICENSE.txt", o.file.trim_end_matches(".sf2")))]
        }
        Kind::Voice(i) => vec![crate::singer_lib::dir().join(crate::singer_lib::OFFERS[i].folder)],
        Kind::Princess => vec![ps::dir()],
        Kind::Ffmpeg => vec![krabton_engine::ffmpeg::library_dir()],
        Kind::YtDlp => vec![crate::video::ytdl::tool_dir()],
    }
}

/// Is it on this computer? (The helper programs may be the system's own.)
fn here(kind: Kind) -> bool {
    match kind {
        Kind::Piano => krabton_engine::piano::library::installed(&crate::piano_lib::library_dir()),
        Kind::Instrument(i) => crate::orchestra_lib::installed(i),
        Kind::Drums(i) => real_drums::installed(&real_drums::PACKS[i]),
        Kind::Sf2(i) => crate::sf2_lib::installed(&crate::sf2_lib::OFFERS[i]).is_some(),
        Kind::Voice(i) => crate::singer_lib::installed(&crate::singer_lib::OFFERS[i]).is_some(),
        Kind::Princess => ps::installed(&ps::dir()),
        Kind::Ffmpeg => krabton_engine::ffmpeg::find().is_some(),
        Kind::YtDlp => crate::video::ytdl::find().is_some(),
    }
}

fn disk_size(path: &Path) -> u64 {
    let Ok(meta) = std::fs::metadata(path) else { return 0 };
    if !meta.is_dir() {
        return meta.len();
    }
    std::fs::read_dir(path).map(|d| d.flatten().map(|e| disk_size(&e.path())).sum()).unwrap_or(0)
}

/// What is on this disk, pack by pack: (here, bytes Krabton keeps for it).
pub type Survey = HashMap<Kind, (bool, u64)>;

/// Looks in the background (a helper program has to be started to see that it works).
fn start_survey() -> Arc<Mutex<Option<Survey>>> {
    let out = Arc::new(Mutex::new(None));
    let o = out.clone();
    std::thread::spawn(move || {
        let survey: Survey = all().iter().map(|p| (p.kind, (here(p.kind), place(p.kind).iter().map(|d| disk_size(d)).sum()))).collect();
        if let Ok(mut o) = o.lock() {
            *o = Some(survey);
        }
    });
    out
}

/// How a job goes.
pub enum Progress {
    Busy(f32, String),
    Done,
    Failed(String),
}

/// The helper programs have no place in the App to wait in: they wait here.
enum Tool {
    Ffmpeg(Arc<Mutex<crate::video::ffmpeg::Fetch>>),
    YtDlp(Arc<Mutex<Option<Result<PathBuf, String>>>>),
}

/// The queue and what the window shows.
#[derive(Default)]
pub struct Packs {
    pub open: bool,
    /// The first start: the window says why it came.
    pub welcome: bool,
    pub picked: BTreeSet<Kind>,
    pub queue: VecDeque<Kind>,
    pub running: Option<Kind>,
    tool: Option<Tool>,
    /// What this round of the queue finished.
    finished: usize,
    pub failed: HashMap<Kind, String>,
    /// "Are you sure?" for removing this one.
    pub removing: Option<Kind>,
    pub survey: Survey,
    surveying: Option<Arc<Mutex<Option<Survey>>>>,
    started: bool,
}

impl Packs {
    pub fn left(&self) -> usize {
        self.queue.len() + self.running.is_some() as usize
    }

    pub fn is_here(&self, kind: Kind) -> bool {
        self.survey.get(&kind).is_some_and(|s| s.0)
    }

    /// Has the first look at the disk come in yet?
    pub fn surveyed(&self) -> bool {
        !self.survey.is_empty()
    }

    pub fn resurvey(&mut self) {
        if self.surveying.is_none() {
            self.surveying = Some(start_survey());
        }
    }
}

/// What the Windows installer left beside the program, and when.
fn installer_choice() -> Option<(String, u64)> {
    let path = std::env::current_exe().ok()?.with_file_name("packs-choice.txt");
    let when = std::fs::metadata(&path).ok()?.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
    Some((std::fs::read_to_string(&path).ok()?, when))
}

/// Anything fetched before, so this is not somebody's first time?
fn anything_here(packs: &[Pack]) -> bool {
    packs.iter().any(|p| !matches!(p.kind, Kind::Ffmpeg | Kind::YtDlp) && here(p.kind))
}

impl App {
    /// The packs on their way, one at a time; once a frame.
    pub(crate) fn poll_packs(&mut self) {
        if !self.packs.started {
            self.packs.started = true;
            self.packs_first_start();
        }
        if let Some(found) = self.packs.surveying.as_ref().and_then(|s| s.lock().ok().and_then(|mut s| s.take())) {
            self.packs.surveying = None;
            // The queue skips what turned out to be here.
            self.packs.queue.retain(|k| !found.get(k).is_some_and(|s| s.0));
            self.packs.picked.retain(|k| !found.get(k).is_some_and(|s| s.0));
            self.packs.survey = found;
        }
        if let Some(kind) = self.packs.running {
            match self.pack_progress(kind) {
                Progress::Busy(..) => return,
                Progress::Done => {
                    let bytes = place(kind).iter().map(|d| disk_size(d)).sum();
                    self.packs.survey.insert(kind, (true, bytes));
                    self.packs.failed.remove(&kind);
                }
                Progress::Failed(e) => {
                    self.packs.failed.insert(kind, e);
                }
            }
            self.packs.running = None;
            self.packs.tool = None;
            self.packs.finished += 1;
            if self.packs.queue.is_empty() {
                let (done, failed) = (self.packs.finished, self.packs.failed.len());
                self.packs.finished = 0;
                self.packs.resurvey();
                self.toast(match failed {
                    0 => format!("Packs: {done} here"),
                    _ => format!("Packs: {failed} did not come; the Packs window has them"),
                });
            }
        }
        // While the first look is under way the queue waits: it may take some out.
        if self.packs.surveying.is_none() {
            if let Some(kind) = self.packs.queue.pop_front() {
                self.packs.running = Some(kind);
                self.start_pack(kind);
            }
        }
    }

    fn packs_first_start(&mut self) {
        if std::env::var_os("KRABTON_TEST_WINDOW").is_some() {
            return;
        }
        let packs = all();
        if let Some((text, when)) = installer_choice() {
            if self.prefs.packs_choice_seen != when {
                self.prefs.packs_choice_seen = when;
                self.prefs.packs_asked = true;
                self.prefs.save();
                let kinds = parse_choice(&text, &packs);
                if !kinds.is_empty() {
                    self.toast("Fetching the packs you chose; Krabton menu, Packs shows how far");
                    self.queue_packs(kinds);
                }
                return;
            }
        }
        if !self.prefs.packs_asked {
            self.prefs.packs_asked = true;
            self.prefs.save();
            if !anything_here(&packs) {
                self.packs.open = true;
                self.packs.welcome = true;
                self.packs.resurvey();
            }
        }
    }

    /// Adds packs to the queue (not those here, or on their way).
    pub(crate) fn queue_packs(&mut self, kinds: impl IntoIterator<Item = Kind>) {
        for k in kinds {
            if !self.packs.queue.contains(&k) && self.packs.running != Some(k) && !self.packs.is_here(k) {
                self.packs.failed.remove(&k);
                self.packs.queue.push_back(k);
                self.packs.picked.remove(&k);
            }
        }
        // Not looked yet: the look takes out what is here.
        if !self.packs.surveyed() {
            self.packs.resurvey();
        }
    }

    fn start_pack(&mut self, kind: Kind) {
        match kind {
            Kind::Piano => self.start_piano_install(),
            Kind::Instrument(i) => self.start_orchestra_install(i),
            Kind::Drums(i) => {
                if let Some(kit) = real_drums::PACKS[i].kits.first() {
                    self.start_drum_install(kit.0);
                }
            }
            Kind::Sf2(i) => self.start_sf2_install(i),
            Kind::Voice(i) => self.start_singer_install(i, None),
            Kind::Princess => self.princess_sounds = Some(crate::princess_lib::start()),
            Kind::Ffmpeg => self.packs.tool = Some(Tool::Ffmpeg(crate::video::ffmpeg::start_fetch())),
            Kind::YtDlp => self.packs.tool = Some(Tool::YtDlp(crate::video::ytdl::start_tool())),
        }
    }

    /// How far a pack is. Each install leaves its place in the App when it
    /// is done (its own poll opened it for the engines), or says it failed.
    pub(crate) fn pack_progress(&self, kind: Kind) -> Progress {
        use crate::video::ffmpeg::Fetch;
        use crate::{drum_lib as d, orchestra_lib as o, piano_lib as p, princess_lib as q, sf2_lib as s, singer_lib as v};
        let of = |done: usize, total: usize| done as f32 / total.max(1) as f32;
        let failed = |e: &str| Progress::Failed(if e.is_empty() { "It did not come".to_string() } else { e.to_string() });
        let mb = |bytes: u64, of: u64| Progress::Busy((bytes as f32 / of.max(1) as f32).min(0.99), format!("{} of {} MB", bytes / 1_000_000, of / 1_000_000));
        match kind {
            Kind::Piano => match self.piano_install.as_ref().and_then(|s| s.lock().ok().map(|s| s.clone())) {
                None | Some(p::InstallState::Ready) => Progress::Done,
                Some(p::InstallState::Listing) => Progress::Busy(0.0, "Asking GitHub".into()),
                Some(p::InstallState::Downloading { bytes, total_bytes, .. }) => mb(bytes, total_bytes),
                Some(p::InstallState::Building { done, total }) => Progress::Busy(of(done, total), "Preparing".into()),
                Some(p::InstallState::Failed(e)) => failed(&e),
            },
            Kind::Instrument(i) => match self.orchestra_installs.get(&i).and_then(|s| s.lock().ok().map(|s| s.clone())) {
                None | Some(o::State::Ready) => Progress::Done,
                Some(o::State::Downloading { done, total }) => Progress::Busy(of(done, total), format!("{done} of {total} files")),
                Some(o::State::Building { done, total }) => Progress::Busy(of(done, total), "Preparing".into()),
                Some(o::State::Failed(e)) => failed(&e),
            },
            Kind::Drums(i) => match self.drum_installs.get(real_drums::PACKS[i].folder).and_then(|s| s.lock().ok().map(|s| s.clone())) {
                None | Some(d::State::Ready) => Progress::Done,
                Some(d::State::Downloading { done, total }) => Progress::Busy(of(done, total), format!("{done} of {total} files")),
                Some(d::State::Preparing) => Progress::Busy(1.0, "Preparing".into()),
                Some(d::State::Failed(e)) => failed(&e),
            },
            Kind::Sf2(i) => match self.sf2_installs.get(&i).and_then(|s| s.lock().ok().map(|s| s.clone())) {
                None | Some(s::State::Ready(_)) => Progress::Done,
                Some(s::State::Downloading { bytes }) => mb(bytes, s::OFFERS[i].bytes),
                Some(s::State::Failed(e)) => failed(&e),
            },
            Kind::Voice(i) => match self.singer_installs.get(&i).and_then(|(s, _)| s.lock().ok().map(|s| s.clone())) {
                None | Some(v::State::Ready(_)) => Progress::Done,
                Some(v::State::Downloading { bytes }) => mb(bytes, v::OFFERS[i].bytes),
                Some(v::State::Unpacking) => Progress::Busy(1.0, "Unpacking".into()),
                Some(v::State::Failed(e)) => failed(&e),
            },
            Kind::Princess => match self.princess_sounds.as_ref().and_then(|s| s.lock().ok().map(|s| s.clone())) {
                None | Some(q::State::Ready) => Progress::Done,
                Some(q::State::Making { done, total }) => Progress::Busy(of(done, total), format!("Making {done} of {total}")),
                Some(q::State::Failed(e)) => failed(&e),
            },
            Kind::Ffmpeg | Kind::YtDlp => match &self.packs.tool {
                Some(Tool::Ffmpeg(s)) => match s.lock().map(|s| s.clone()) {
                    Ok(Fetch::Ready(_)) => Progress::Done,
                    Ok(Fetch::Downloading { bytes }) => mb(bytes, crate::video::ffmpeg::download().map_or(1, |d| d.1)),
                    Ok(Fetch::Unpacking) => Progress::Busy(1.0, "Unpacking".into()),
                    Ok(Fetch::Failed(e)) => failed(&e),
                    Err(_) => failed(""),
                },
                Some(Tool::YtDlp(s)) => match s.lock().map(|s| s.clone()) {
                    Ok(None) => Progress::Busy(0.5, "Downloading".into()),
                    Ok(Some(Ok(_))) => Progress::Done,
                    Ok(Some(Err(e))) => failed(&e),
                    Err(_) => failed(""),
                },
                None => Progress::Done,
            },
        }
    }

    /// Takes a pack off this disk. It stays playing where it is loaded; a
    /// file still in use says so, and goes after a restart.
    pub(crate) fn remove_pack(&mut self, kind: Kind) {
        match kind {
            Kind::Piano => krabton_engine::piano::library::set_shared(None),
            Kind::Instrument(i) => orchestra::set_library(i, None),
            Kind::Drums(i) => {
                self.packs_here.remove(real_drums::PACKS[i].folder);
            }
            _ => {}
        }
        let mut trouble = None;
        for path in place(kind) {
            let gone = if path.is_dir() {
                std::fs::remove_dir_all(&path)
            } else if path.exists() {
                std::fs::remove_file(&path)
            } else {
                Ok(())
            };
            if let Err(e) = gone {
                trouble = Some(format!("{}: {e}", path.display()));
            }
        }
        match kind {
            Kind::Sf2(i) => {
                let path = crate::sf2_lib::dir().join(crate::sf2_lib::OFFERS[i].file);
                self.prefs.sf2_banks.retain(|p| *p != path);
                if self.prefs.gm_bank.as_ref() == Some(&path) {
                    self.prefs.gm_bank = None;
                }
                self.sf2_open.retain(|b| b.path.as_ref() != Some(&path));
                self.prefs.save();
            }
            Kind::Voice(i) => {
                let path = crate::singer_lib::dir().join(crate::singer_lib::OFFERS[i].folder);
                self.prefs.singer_banks.retain(|p| *p != path);
                self.prefs.save();
            }
            Kind::Ffmpeg => krabton_engine::ffmpeg::forget(),
            _ => {}
        }
        match trouble {
            Some(e) => self.toast(format!("In use, so it could not all go; try again after a restart ({e})")),
            None => self.toast("Removed from this disk"),
        }
        self.packs.resurvey();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_group_has_packs_and_every_pack_a_group() {
        let packs = all();
        for (id, _) in GROUPS {
            assert!(packs.iter().any(|p| p.group == id), "{id}");
        }
        assert!(packs.iter().all(|p| GROUPS.iter().any(|g| g.0 == p.group)));
        let kinds: BTreeSet<Kind> = packs.iter().map(|p| p.kind).collect();
        assert_eq!(kinds.len(), packs.len());
    }

    #[test]
    fn the_installers_choice_names_groups() {
        let packs = all();
        let kinds = parse_choice("piano,voices\r\n", &packs);
        assert!(kinds.contains(&Kind::Piano) && kinds.contains(&Kind::Voice(0)));
        assert!(!kinds.contains(&Kind::Sf2(0)));
        assert!(parse_choice("", &packs).is_empty());
        assert!(parse_choice("nonsense", &packs).is_empty());
    }

    #[test]
    fn the_installer_offers_every_group_and_no_other() {
        let script = include_str!("../../../packaging/windows/krabton.iss");
        let mut offered = Vec::new();
        for line in script.lines() {
            if let Some(rest) = line.trim().strip_prefix("AddPack('") {
                offered.push(rest.split('\'').next().unwrap_or("").to_string());
            }
        }
        let groups: Vec<String> = GROUPS.iter().map(|g| g.0.to_string()).collect();
        assert_eq!(offered, groups);
    }
}
