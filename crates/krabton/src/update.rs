//! Telling that a new Krabton is out, and putting it in place. The check asks
//! GitHub for the latest release with the system's `curl`; updating fetches
//! the file made for the way this copy was installed, in the background, and
//! on the user's word replaces the program and starts it again. Only files of
//! this project's own releases are taken, over HTTPS.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const LATEST: &str = "https://api.github.com/repos/daandobber/krabton/releases/latest";
const PAGES: &str = "https://github.com/daandobber/krabton/";
const FILES: &str = "https://github.com/daandobber/krabton/releases/download/";
/// Everything past this in a release's text is about downloading by hand.
const NOTES_END: &str = "## Download";

/// This copy's version.
pub fn current() -> String {
    if cfg!(debug_assertions) {
        if let Ok(v) = std::env::var("KRABTON_UPDATE_AS") {
            return v;
        }
    }
    env!("CARGO_PKG_VERSION").to_string()
}

/// Is version `a` newer than `b`? (Numbers by dots; an odd one is never newer.)
pub fn newer(a: &str, b: &str) -> bool {
    let parts = |v: &str| v.trim_start_matches('v').split('.').map(|p| p.parse::<u32>()).collect::<Result<Vec<_>, _>>();
    match (parts(a), parts(b)) {
        (Ok(a), Ok(b)) => a > b,
        _ => false,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub bytes: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub version: String,
    /// What is new, as far as it says.
    pub notes: String,
    pub page: String,
    pub assets: Vec<Asset>,
}

/// Reads GitHub's answer. Files from elsewhere are left out.
pub fn parse_release(json: &str) -> Result<Release, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("GitHub's answer: {e}"))?;
    let version = v.get("tag_name").and_then(|t| t.as_str()).ok_or("GitHub's answer has no version")?.trim_start_matches('v').to_string();
    let body = v.get("body").and_then(|b| b.as_str()).unwrap_or("").replace('\r', "");
    // Older releases only tell how to download.
    let notes = if body.starts_with("Download the file") { "" } else { body.split(NOTES_END).next().unwrap_or("") };
    let notes = notes.replace("## What's new", "").trim().to_string();
    let page = v.get("html_url").and_then(|p| p.as_str()).filter(|p| p.starts_with(PAGES)).unwrap_or("https://github.com/daandobber/krabton/releases/latest").to_string();
    let assets = v
        .get("assets")
        .and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|f| Some(Asset { name: f.get("name")?.as_str()?.to_string(), url: f.get("browser_download_url")?.as_str()?.to_string(), bytes: f.get("size")?.as_u64()? }))
                .filter(|f| f.url.starts_with(FILES))
                .collect()
        })
        .unwrap_or_default();
    Ok(Release { version, notes, page, assets })
}

#[derive(Clone, Debug, PartialEq)]
pub enum Check {
    Idle,
    Checking,
    UpToDate,
    Found(Release),
    Failed(String),
}

/// Asks GitHub in the background.
pub fn start_check() -> Arc<Mutex<Check>> {
    let state = Arc::new(Mutex::new(Check::Checking));
    let s = state.clone();
    std::thread::spawn(move || {
        let answer = ask();
        if let Ok(mut st) = s.lock() {
            *st = answer;
        }
    });
    state
}

fn ask() -> Check {
    let out = crate::piano_lib::curl().args(["-sSfL", "--proto", "=https", "--max-time", "20", "-H", "Accept: application/vnd.github+json", "-A", "Krabton", LATEST]).output();
    let out = match out {
        Ok(o) if o.status.success() => o,
        Ok(_) => return Check::Failed("GitHub did not answer; are you online?".into()),
        Err(e) => return Check::Failed(format!("curl could not run ({e})")),
    };
    match parse_release(&String::from_utf8_lossy(&out.stdout)) {
        Ok(r) if newer(&r.version, &current()) => Check::Found(r),
        Ok(_) => Check::UpToDate,
        Err(e) => Check::Failed(e),
    }
}

/// How this copy got here, so which file replaces it.
#[derive(Clone, Debug, PartialEq)]
pub enum How {
    /// Windows, installed: the new installer runs without questions.
    Setup,
    /// Windows, from the zip: this program file.
    PortableZip(PathBuf),
    AppImage(PathBuf),
    /// Linux, from the tar.gz: this program file.
    Tarball(PathBuf),
    /// macOS: the Krabton.app.
    MacApp(PathBuf),
    /// A package manager, a folder Krabton may not write to, or a developer's build.
    ByHand,
}

fn writable(dir: &Path) -> bool {
    let probe = dir.join(".krabton-update-probe");
    let ok = std::fs::write(&probe, b"").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

pub fn how() -> How {
    let Ok(exe) = std::env::current_exe() else { return How::ByHand };
    if cfg!(debug_assertions) {
        // Trying it out: a developer's build pretends to be a portable copy.
        return if std::env::var_os("KRABTON_UPDATE_AS").is_some() { How::PortableZip(exe) } else { How::ByHand };
    }
    let Some(dir) = exe.parent().map(Path::to_path_buf) else { return How::ByHand };
    if cfg!(windows) {
        if dir.join("unins000.exe").is_file() {
            How::Setup
        } else if writable(&dir) {
            How::PortableZip(exe)
        } else {
            How::ByHand
        }
    } else if cfg!(target_os = "macos") {
        // .../Krabton.app/Contents/MacOS/krabton, not on the disk image itself.
        match exe.ancestors().nth(3).map(Path::to_path_buf) {
            Some(app) if app.extension().is_some_and(|e| e == "app") && !app.starts_with("/Volumes") && app.parent().is_some_and(writable) => How::MacApp(app),
            _ => How::ByHand,
        }
    } else if let Some(image) = std::env::var_os("APPIMAGE").map(PathBuf::from) {
        if image.parent().is_some_and(writable) {
            How::AppImage(image)
        } else {
            How::ByHand
        }
    } else if writable(&dir) && !dir.starts_with("/usr") {
        How::Tarball(exe)
    } else {
        How::ByHand
    }
}

/// The name of the release's file for this copy (see the release workflow).
pub fn asset_name(how: &How, version: &str, arch: &str) -> Option<String> {
    Some(match how {
        How::Setup => format!("Krabton-Setup-{version}.exe"),
        How::PortableZip(_) => format!("Krabton-{version}-windows-portable.zip"),
        How::AppImage(_) => format!("Krabton-{version}-{arch}.AppImage"),
        How::Tarball(_) => format!("krabton-{version}-linux-{arch}.tar.gz"),
        How::MacApp(_) => format!("Krabton-{version}-macOS.dmg"),
        How::ByHand => return None,
    })
}

/// The file in this release for this copy, if there is one.
pub fn asset_for<'a>(release: &'a Release, how: &How) -> Option<&'a Asset> {
    let name = asset_name(how, &release.version, std::env::consts::ARCH)?;
    release.assets.iter().find(|a| a.name == name)
}

#[derive(Clone, Debug, PartialEq)]
pub enum Fetch {
    Getting { bytes: u64, total: u64 },
    Ready(PathBuf),
    Failed(String),
}

/// Where the new file waits: Documents/Krabton/Updates.
pub fn dir() -> PathBuf {
    crate::piano_lib::library_dir().ancestors().nth(2).map(Path::to_path_buf).unwrap_or_default().join("Updates")
}

/// Fetches the file in the background.
pub fn start_fetch(asset: Asset) -> Arc<Mutex<Fetch>> {
    let state = Arc::new(Mutex::new(Fetch::Getting { bytes: 0, total: asset.bytes }));
    let s = state.clone();
    std::thread::spawn(move || {
        let result = fetch(&asset, &s);
        if let Ok(mut st) = s.lock() {
            *st = match result {
                Ok(p) => Fetch::Ready(p),
                Err(e) => Fetch::Failed(e),
            };
        }
    });
    state
}

fn fetch(asset: &Asset, state: &Arc<Mutex<Fetch>>) -> Result<PathBuf, String> {
    if !asset.url.starts_with(FILES) {
        return Err("that file is not from Krabton's releases".into());
    }
    let dir = dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let to = dir.join(&asset.name);
    if std::fs::metadata(&to).is_ok_and(|m| m.len() == asset.bytes) {
        return Ok(to);
    }
    let part = dir.join(format!("{}.part", asset.name));
    let mut child = crate::piano_lib::curl()
        .args(["-sSfL", "--proto", "=https", "--retry", "3", "--retry-delay", "2", "-o"])
        .arg(&part)
        .arg(&asset.url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("curl could not run ({e})"))?;
    let status = loop {
        if let Some(st) = child.try_wait().map_err(|e| e.to_string())? {
            break st;
        }
        let bytes = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
        if let Ok(mut s) = state.lock() {
            *s = Fetch::Getting { bytes, total: asset.bytes };
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    };
    let arrived = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
    if !status.success() || arrived != asset.bytes {
        let _ = std::fs::remove_file(&part);
        return Err("the download did not arrive whole; try again".into());
    }
    std::fs::rename(&part, &to).map_err(|e| e.to_string())?;
    Ok(to)
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    path.with_file_name(format!("{name}{suffix}"))
}

/// The new program in place of `exe`: beside it first, then swapped.
/// (A running program can be renamed on every system.)
fn swap_program(new: &Path, exe: &Path) -> Result<(), String> {
    let beside = with_suffix(exe, ".new");
    let old = with_suffix(exe, ".old");
    std::fs::copy(new, &beside).map_err(|e| format!("{}: {e}", beside.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&beside, std::fs::Permissions::from_mode(0o755));
    }
    let _ = std::fs::remove_file(&old);
    std::fs::rename(exe, &old).map_err(|e| format!("{}: {e}", exe.display()))?;
    if let Err(e) = std::fs::rename(&beside, exe) {
        let _ = std::fs::rename(&old, exe);
        return Err(format!("{}: {e}", exe.display()));
    }
    Ok(())
}

/// What is left to do after `apply`.
#[derive(Debug, PartialEq)]
pub enum Then {
    /// Start this program, then quit.
    Start(PathBuf),
    /// macOS opens an app, not a program.
    OpenApp(PathBuf),
    /// An installer starts the new Krabton itself: just quit.
    Quit,
}

/// Puts the fetched file in place. An error leaves the old copy as it was.
pub fn apply(how: &How, file: &Path) -> Result<Then, String> {
    match how {
        How::Setup => {
            // Asks for rights itself when Krabton was installed for everyone,
            // and starts the new Krabton when it is done (see krabton.iss).
            std::process::Command::new(file).args(["/SILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/CLOSEAPPLICATIONS"]).spawn().map_err(|e| format!("the installer could not start: {e}"))?;
            Ok(Then::Quit)
        }
        How::PortableZip(exe) => {
            let out = dir().join("unpacked");
            let _ = std::fs::remove_dir_all(&out);
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            crate::singer_lib::unzip_some(file, &out, |name| name.replace('\\', "/").ends_with("/krabton.exe"))?;
            let new = find_file(&out, "krabton.exe").ok_or("the zip holds no krabton.exe")?;
            swap_program(&new, exe)?;
            let _ = std::fs::remove_dir_all(&out);
            Ok(Then::Start(exe.clone()))
        }
        How::AppImage(image) => {
            swap_program(file, image)?;
            Ok(Then::Start(image.clone()))
        }
        How::Tarball(exe) => {
            let out = dir().join("unpacked");
            let _ = std::fs::remove_dir_all(&out);
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            let ok = std::process::Command::new("tar").arg("xzf").arg(file).arg("-C").arg(&out).status().is_ok_and(|s| s.success());
            if !ok {
                return Err("the download could not be unpacked".into());
            }
            let new = find_file(&out, "krabton").ok_or("the download holds no krabton program")?;
            swap_program(&new, exe)?;
            let _ = std::fs::remove_dir_all(&out);
            Ok(Then::Start(exe.clone()))
        }
        How::MacApp(app) => {
            let mount = dir().join("mount");
            std::fs::create_dir_all(&mount).map_err(|e| e.to_string())?;
            let ran = |cmd: &mut std::process::Command| cmd.status().is_ok_and(|s| s.success());
            if !ran(std::process::Command::new("hdiutil").args(["attach", "-nobrowse", "-readonly", "-mountpoint"]).arg(&mount).arg(file)) {
                return Err("the disk image could not be opened".into());
            }
            let new = with_suffix(app, ".new");
            let old = with_suffix(app, ".old");
            let _ = std::fs::remove_dir_all(&new);
            let copied = ran(std::process::Command::new("ditto").arg(mount.join("Krabton.app")).arg(&new));
            ran(std::process::Command::new("hdiutil").arg("detach").arg(&mount));
            if !copied {
                let _ = std::fs::remove_dir_all(&new);
                return Err("the new Krabton could not be copied".into());
            }
            let _ = std::fs::remove_dir_all(&old);
            std::fs::rename(app, &old).map_err(|e| format!("{}: {e}", app.display()))?;
            if let Err(e) = std::fs::rename(&new, app) {
                let _ = std::fs::rename(&old, app);
                return Err(format!("{}: {e}", app.display()));
            }
            Ok(Then::OpenApp(app.clone()))
        }
        How::ByHand => Err("this copy cannot replace itself".into()),
    }
}

fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let p = e.path();
        if p.is_dir() {
            if let Some(f) = find_file(&p, name) {
                return Some(f);
            }
        } else if p.file_name().is_some_and(|n| n == name) {
            return Some(p);
        }
    }
    None
}

/// Sweeps up after an update: the old program and the downloaded files.
fn clean_up(how: &How) {
    let _ = std::fs::remove_dir_all(dir());
    match how {
        How::PortableZip(p) | How::AppImage(p) | How::Tarball(p) => {
            let _ = std::fs::remove_file(with_suffix(p, ".old"));
        }
        How::MacApp(app) => {
            let _ = std::fs::remove_dir_all(with_suffix(app, ".old"));
        }
        How::Setup | How::ByHand => {}
    }
}

/// Opens a page of the project in the browser.
pub fn open_page(url: &str) {
    if url.starts_with(PAGES) {
        let _ = std::process::Command::new(opener()).arg(url).spawn();
    }
}

/// The system's program that opens pages and folders.
pub fn opener() -> &'static str {
    if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    }
}

/// The updater's state, for the status bar and the window.
pub struct Updater {
    pub open: bool,
    pub check: Arc<Mutex<Check>>,
    pub fetch: Option<Arc<Mutex<Fetch>>>,
    pub how: How,
    /// Putting it in place failed: why.
    pub trouble: Option<String>,
    told: bool,
    started: bool,
}

impl Default for Updater {
    fn default() -> Self {
        Self { open: false, check: Arc::new(Mutex::new(Check::Idle)), fetch: None, how: How::ByHand, trouble: None, told: false, started: false }
    }
}

impl Updater {
    /// The release waiting, when there is one.
    pub fn found(&self) -> Option<Release> {
        match self.check.lock().ok()?.clone() {
            Check::Found(r) => Some(r),
            _ => None,
        }
    }

    pub fn check_now(&mut self) {
        self.told = false;
        self.check = start_check();
    }
}

impl crate::app::App {
    /// Looks for a new version once per start, and says so once.
    pub(crate) fn poll_update(&mut self) {
        if !self.update.started {
            self.update.started = true;
            self.update.how = how();
            if std::env::var_os("KRABTON_TEST_WINDOW").is_none() {
                clean_up(&self.update.how);
                if !self.prefs.update_check_off {
                    self.update.check_now();
                }
            }
            self.reopen_after_update();
        }
        if !self.update.told {
            if let Some(r) = self.update.found() {
                self.update.told = true;
                if self.prefs.update_skip.as_deref() != Some(r.version.as_str()) {
                    self.toast(format!("Krabton {} is out: the Update button below has it", r.version));
                }
            }
        }
    }

    /// The set that was open when the update restarted Krabton.
    fn reopen_after_update(&mut self) {
        if let Some(path) = self.prefs.reopen_after_update.take() {
            self.prefs.save();
            if self.file_path.is_none() && path.is_file() {
                self.open_path(path);
                self.toast(format!("Updated to {}", current()));
            }
        }
    }

    /// Puts the fetched file in place and restarts.
    pub(crate) fn restart_into_update(&mut self, file: &Path) {
        match apply(&self.update.how, file) {
            Ok(then) => {
                // A trial in a test window stops here: the swap is what it checks.
                if std::env::var_os("KRABTON_TEST_WINDOW").is_some() {
                    self.toast("Update put in place (test window: not restarting)");
                    return;
                }
                self.prefs.reopen_after_update = self.file_path.clone();
                self.prefs.save();
                match then {
                    Then::Start(p) => {
                        if let Err(e) = std::process::Command::new(&p).spawn() {
                            self.update.trouble = Some(format!("It is in place, but it could not start ({e}): start Krabton yourself"));
                            return;
                        }
                    }
                    Then::OpenApp(app) => {
                        let _ = std::process::Command::new("open").arg("-n").arg(&app).spawn();
                    }
                    Then::Quit => {}
                }
                self.safety.closing = true;
                self.quit_requested = true;
            }
            Err(e) => self.update.trouble = Some(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ANSWER: &str = r###"{
        "tag_name": "v1.2.3",
        "html_url": "https://github.com/daandobber/krabton/releases/tag/v1.2.3",
        "body": "## What's new\r\n- Fire\r\n- Packs\r\n\r\n## Download\r\nDownload the file for your system",
        "assets": [
            {"name": "Krabton-Setup-1.2.3.exe", "size": 10, "browser_download_url": "https://github.com/daandobber/krabton/releases/download/v1.2.3/Krabton-Setup-1.2.3.exe"},
            {"name": "Krabton-1.2.3-x86_64.AppImage", "size": 20, "browser_download_url": "https://github.com/daandobber/krabton/releases/download/v1.2.3/Krabton-1.2.3-x86_64.AppImage"},
            {"name": "evil.exe", "size": 1, "browser_download_url": "https://example.org/evil.exe"}
        ]
    }"###;

    #[test]
    fn versions_compare_as_numbers() {
        assert!(newer("0.96.0", "0.95.9"));
        assert!(newer("v0.100.0", "0.99.0"));
        assert!(newer("1.0.0", "0.99.99"));
        assert!(newer("0.96.1", "0.96"));
        assert!(!newer("0.95.0", "0.95.0"));
        assert!(!newer("0.94.0", "0.95.0"));
        assert!(!newer("nonsense", "0.95.0"));
        assert!(!newer("0.96.0-rc1", "0.95.0"));
    }

    #[test]
    fn a_release_is_read_and_only_its_own_files_kept() {
        let r = parse_release(ANSWER).unwrap();
        assert_eq!(r.version, "1.2.3");
        assert_eq!(r.notes, "- Fire\n- Packs");
        assert_eq!(r.assets.len(), 2);
        assert!(r.assets.iter().all(|a| a.url.starts_with(FILES)));
        assert!(parse_release("not json").is_err());
        assert!(parse_release("{}").is_err());
        // The releases before this tell only how to download.
        let old = parse_release(r#"{"tag_name":"v0.95.0","body":"Download the file for your system:\n\n- **Windows**"}"#).unwrap();
        assert_eq!(old.notes, "");
    }

    #[test]
    fn each_way_of_installing_takes_its_own_file() {
        let name = |h: &How, arch| asset_name(h, "1.2.3", arch);
        let p = PathBuf::from("x");
        assert_eq!(name(&How::Setup, "x86_64").unwrap(), "Krabton-Setup-1.2.3.exe");
        assert_eq!(name(&How::PortableZip(p.clone()), "x86_64").unwrap(), "Krabton-1.2.3-windows-portable.zip");
        assert_eq!(name(&How::AppImage(p.clone()), "aarch64").unwrap(), "Krabton-1.2.3-aarch64.AppImage");
        assert_eq!(name(&How::Tarball(p.clone()), "x86_64").unwrap(), "krabton-1.2.3-linux-x86_64.tar.gz");
        assert_eq!(name(&How::MacApp(p), "aarch64").unwrap(), "Krabton-1.2.3-macOS.dmg");
        assert_eq!(name(&How::ByHand, "x86_64"), None);
        // The workflow makes these names.
        let wf = include_str!("../../../.github/workflows/release.yml");
        for piece in ["Krabton-$VERSION-x86_64.AppImage", "Krabton-$VERSION-aarch64.AppImage", "krabton-$VERSION-linux-x86_64.tar.gz", "krabton-$VERSION-linux-aarch64.tar.gz", "Krabton-$VERSION-macOS.dmg", "Krabton-$env:VERSION-windows-portable.zip"] {
            assert!(wf.contains(piece), "{piece}");
        }
        assert!(include_str!("../../../packaging/windows/krabton.iss").contains("OutputBaseFilename=Krabton-Setup-{#MyAppVersion}"));
    }

    #[test]
    fn the_swap_leaves_the_new_program_and_keeps_the_old_for_later() {
        let dir = std::env::temp_dir().join(format!("krabton-swap-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (exe, new) = (dir.join("program"), dir.join("fresh"));
        std::fs::write(&exe, b"old").unwrap();
        std::fs::write(&new, b"new").unwrap();
        swap_program(&new, &exe).unwrap();
        assert_eq!(std::fs::read(&exe).unwrap(), b"new");
        assert_eq!(std::fs::read(dir.join("program.old")).unwrap(), b"old");
        assert!(!dir.join("program.new").exists());
        // Again: the old one before is replaced.
        swap_program(&new, &exe).unwrap();
        assert_eq!(std::fs::read(dir.join("program.old")).unwrap(), b"new");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
