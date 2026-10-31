//! Where the `ffmpeg` program is, for whatever Krabton reads through it
//! (the Video's films; sound files Krabton cannot read itself). Krabton
//! does not ship it: it is looked for in `KRABTON_FFMPEG`, next to Krabton,
//! in Krabton's library (Documents/Krabton/Library/ffmpeg, where the Video
//! can fetch it), in the usual places of macOS and Linux, and on the PATH.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The program's name on this system.
pub fn exe_name() -> &'static str {
    if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    }
}

/// Where Krabton keeps the ffmpeg it fetched: Documents/Krabton/Library/ffmpeg.
pub fn library_dir() -> PathBuf {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from).unwrap_or_default();
    home.join("Documents").join("Krabton").join("Library").join("ffmpeg")
}

/// The places to look, in order (the PATH last, as a bare name).
fn candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(p) = std::env::var_os("KRABTON_FFMPEG") {
        out.push(PathBuf::from(p));
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
        out.push(dir.join(exe_name()));
    }
    // Fetched by Krabton: somewhere in its folder (a zip may keep it in bin/).
    let lib = library_dir();
    out.push(lib.join(exe_name()));
    out.push(lib.join("bin").join(exe_name()));
    // Programs started from the Dock or a desktop menu do not get the shell's PATH.
    if cfg!(target_os = "macos") {
        out.push("/opt/homebrew/bin/ffmpeg".into());
        out.push("/usr/local/bin/ffmpeg".into());
        out.push("/opt/local/bin/ffmpeg".into());
    } else if cfg!(unix) {
        out.push("/usr/bin/ffmpeg".into());
        out.push("/usr/local/bin/ffmpeg".into());
        out.push("/snap/bin/ffmpeg".into());
    }
    out.push(PathBuf::from(exe_name()));
    out
}

/// A command for the program at `exe`: no console window, nothing on its input.
pub fn command(exe: &Path) -> std::process::Command {
    let mut cmd = std::process::Command::new(exe);
    cmd.stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // No console window popping up.
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

/// Whether `exe` runs and is ffmpeg.
pub fn works(exe: &Path) -> bool {
    // A full path must be a file; a bare name is looked up on the PATH.
    if exe.components().count() > 1 && !exe.is_file() {
        return false;
    }
    command(exe).arg("-version").stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).output().map(|o| o.status.success() && o.stdout.starts_with(b"ffmpeg")).unwrap_or(false)
}

static FOUND: Mutex<Option<Option<PathBuf>>> = Mutex::new(None);

/// The ffmpeg to use, if there is one (looked for once; `forget` looks again).
pub fn find() -> Option<PathBuf> {
    let mut found = FOUND.lock().ok()?;
    if found.is_none() {
        *found = Some(candidates().into_iter().find(|c| works(c)));
    }
    found.clone().flatten()
}

/// Looks again next time (after fetching it, or installing it).
pub fn forget() {
    if let Ok(mut f) = FOUND.lock() {
        *f = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_is_looked_for_in_the_library_and_on_the_path_last() {
        let c = candidates();
        assert!(c.contains(&library_dir().join(exe_name())));
        assert_eq!(c.last(), Some(&PathBuf::from(exe_name())));
        assert!(library_dir().ends_with("Krabton/Library/ffmpeg"));
        // A path that is not there is not ffmpeg.
        assert!(!works(&library_dir().join("nothing here").join(exe_name())));
    }
}
