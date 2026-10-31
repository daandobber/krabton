//! Films from a link (YouTube and the many sites yt-dlp knows): yt-dlp
//! fetches the film into Documents/Krabton/Library/Links and the Video
//! plays it from there. Krabton does not ship yt-dlp; it uses the one it
//! finds (next to itself, in its Library, or on the PATH) or fetches the
//! single program from yt-dlp's own releases the first time it is needed.
//! Films are asked for at 720p at most, as MP4, one file: with ffmpeg
//! picture and sound are joined, without it the best single file is taken.

use krabton_engine::ffmpeg;
use std::ffi::OsString;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The program's file name on this system.
pub fn exe_name() -> &'static str {
    if cfg!(windows) {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    }
}

/// Krabton's own place for yt-dlp.
pub fn tool_dir() -> PathBuf {
    ffmpeg::library_dir().with_file_name("yt-dlp")
}

/// Where films from links go.
pub fn films_dir() -> PathBuf {
    ffmpeg::library_dir().with_file_name("Links")
}

/// The places to look, in order (the PATH last, as a bare name).
fn candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(p) = std::env::var_os("KRABTON_YTDLP") {
        out.push(PathBuf::from(p));
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
        out.push(dir.join(exe_name()));
    }
    out.push(tool_dir().join(exe_name()));
    if cfg!(target_os = "macos") {
        out.push("/opt/homebrew/bin/yt-dlp".into());
        out.push("/usr/local/bin/yt-dlp".into());
    } else if cfg!(unix) {
        out.push("/usr/bin/yt-dlp".into());
        out.push("/usr/local/bin/yt-dlp".into());
    }
    out.push(PathBuf::from(exe_name()));
    out
}

/// Whether yt-dlp runs from there.
fn works(exe: &Path) -> bool {
    if exe.components().count() > 1 && !exe.is_file() {
        return false;
    }
    ffmpeg::command(exe).arg("--version").stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().is_ok_and(|s| s.success())
}

/// yt-dlp on this computer, if there is one.
pub fn find() -> Option<PathBuf> {
    candidates().into_iter().find(|c| works(c))
}

/// Where the single yt-dlp program for this computer is published.
pub fn download() -> Option<&'static str> {
    if cfg!(windows) {
        Some("https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe")
    } else if cfg!(target_os = "macos") {
        Some("https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos")
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        Some("https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux")
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        Some("https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux_aarch64")
    } else {
        None
    }
}

/// A JavaScript runtime yt-dlp can use for YouTube (without one it sees
/// fewer of the formats): Deno is used by default, Node or Bun when asked.
pub fn js_runtime() -> Option<&'static str> {
    let runs = |name: &str| ffmpeg::command(Path::new(name)).arg("--version").stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().is_ok_and(|s| s.success());
    ["deno", "node", "bun"].into_iter().find(|r| runs(r))
}

/// Whether text looks like a link to fetch.
pub fn is_link(text: &str) -> bool {
    let t = text.trim();
    (t.starts_with("https://") || t.starts_with("http://")) && t.len() > 10 && !t.contains(char::is_whitespace)
}

/// yt-dlp's arguments for a link: one film (not its playlist), 720p at
/// most, as MP4 where there is one, into `dir` named after its title,
/// saying how far it is on lines of its own and the file it made at the
/// end. Without ffmpeg to join picture and sound, the picture alone will do.
pub fn args(url: &str, dir: &Path, ffmpeg: Option<&Path>, js: Option<&str>) -> Vec<OsString> {
    let mut a: Vec<OsString> = Vec::new();
    let mut push = |s: &str| a.push(s.into());
    push("--no-playlist");
    push("--no-simulate");
    push("--newline");
    push("--progress");
    push("--restrict-filenames");
    push("--no-mtime");
    push("-f");
    if ffmpeg.is_some() {
        push("bv*[height<=720][ext=mp4]+ba[ext=m4a]/bv*[height<=720]+ba/b[height<=720]/bv*[height<=720]/b/bv*");
        push("--merge-output-format");
        push("mp4");
    } else {
        push("b[height<=720][ext=mp4]/bv*[height<=720][ext=mp4]/b[height<=720]/bv*[height<=720]/b/bv*");
    }
    if let Some(r) = js.filter(|r| *r != "deno") {
        push("--js-runtimes");
        push(r);
    }
    push("--print");
    push("after_move:FILE:%(filepath)s");
    push("-o");
    a.push(dir.join("%(title).80B [%(id)s].%(ext)s").into_os_string());
    if let Some(f) = ffmpeg {
        a.push("--ffmpeg-location".into());
        a.push(f.as_os_str().to_owned());
    }
    a.push("--".into());
    a.push(url.trim().into());
    a
}

/// A film's name from its file: "Me_at_the_zoo [jNQXAC9IVRw]" is "Me at the zoo".
pub fn title_of(path: &Path) -> String {
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let title = match stem.rfind(" [") {
        Some(i) if stem.ends_with(']') => &stem[..i],
        _ => stem.as_str(),
    };
    title.replace('_', " ").trim().to_string()
}

/// What a line of yt-dlp's says: how far (0..1), or the file it made.
#[derive(Clone, Debug, PartialEq)]
pub enum Said {
    Progress(f32),
    File(PathBuf),
}

pub fn parse_line(line: &str) -> Option<Said> {
    let l = line.trim();
    if let Some(f) = l.strip_prefix("FILE:") {
        return (!f.trim().is_empty()).then(|| Said::File(PathBuf::from(f.trim())));
    }
    let rest = l.strip_prefix("[download]")?.trim_start();
    let pct = rest.split('%').next()?.trim();
    pct.parse::<f32>().ok().map(|p| Said::Progress((p / 100.0).clamp(0.0, 1.0)))
}

/// How fetching a film from a link goes.
#[derive(Clone, Debug, PartialEq)]
pub enum Grab {
    /// yt-dlp itself is being fetched first.
    Tool,
    /// The film: how far (0..1; a film with sound comes in two parts).
    Film(f32),
    Ready(PathBuf),
    Failed(String),
}

/// Starts fetching the film behind `url` in the background.
pub fn start(url: String) -> Arc<Mutex<Grab>> {
    let state = Arc::new(Mutex::new(Grab::Film(0.0)));
    let s = state.clone();
    std::thread::spawn(move || {
        let result = grab(&url, &s);
        if let Ok(mut st) = s.lock() {
            *st = match result {
                Ok(p) => Grab::Ready(p),
                Err(e) => Grab::Failed(e),
            };
        }
    });
    state
}

fn set(state: &Arc<Mutex<Grab>>, g: Grab) {
    if let Ok(mut s) = state.lock() {
        *s = g;
    }
}

fn grab(url: &str, state: &Arc<Mutex<Grab>>) -> Result<PathBuf, String> {
    let exe = match find() {
        Some(e) => e,
        None => {
            set(state, Grab::Tool);
            fetch_tool()?
        }
    };
    set(state, Grab::Film(0.0));
    let dir = films_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let ff = ffmpeg::find();
    let mut child = ffmpeg::command(&exe)
        .args(args(url, &dir, ff.as_deref(), js_runtime()))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("yt-dlp did not start: {e}"))?;
    // What it says goes wrong (read aside, so neither pipe fills up).
    let err = child.stderr.take().map(|e| {
        std::thread::spawn(move || {
            BufReader::new(e).lines().map_while(Result::ok).filter(|l| l.contains("ERROR")).last()
        })
    });
    let mut file = None;
    // Picture and sound come one after the other: the second part counts from half way.
    let (mut part, mut last) = (0u32, 0.0f32);
    if let Some(out) = child.stdout.take() {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            match parse_line(&line) {
                Some(Said::Progress(p)) => {
                    if p + 0.2 < last {
                        part += 1;
                    }
                    last = p;
                    let whole = if ff.is_some() { ((part.min(1) as f32 + p) / 2.0).min(1.0) } else { p };
                    set(state, Grab::Film(whole));
                }
                Some(Said::File(f)) => file = Some(f),
                None => {}
            }
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let why = err.and_then(|t| t.join().ok()).flatten();
    match file.filter(|f| f.is_file()) {
        Some(f) if status.success() => Ok(f),
        _ => Err(match why {
            Some(w) => format!("The link could not be fetched: {}", w.trim_start_matches("ERROR:").trim()),
            None => "The link could not be fetched".into(),
        }),
    }
}

/// Fetches yt-dlp into Krabton's Library (a single program).
fn fetch_tool() -> Result<PathBuf, String> {
    let url = download().ok_or("there is no yt-dlp to fetch for this computer; install it yourself (pip install yt-dlp)")?;
    let dir = tool_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let part = dir.join(format!("{}.part", exe_name()));
    let status = crate::piano_lib::curl()
        .args(["-sSfL", "--retry", "3", "--retry-delay", "2", "-o"])
        .arg(&part)
        .arg(url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| format!("curl could not run ({e})"))?;
    if !status.success() {
        let _ = std::fs::remove_file(&part);
        return Err("downloading yt-dlp failed".into());
    }
    let exe = dir.join(exe_name());
    std::fs::rename(&part, &exe).map_err(|e| format!("{}: {e}", exe.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755));
    }
    if !works(&exe) {
        return Err("the yt-dlp that came does not run on this computer".into());
    }
    Ok(exe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_are_told_from_other_text() {
        assert!(is_link("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
        assert!(is_link("  https://youtu.be/abc123def  "));
        assert!(!is_link("youtube.com/watch"));
        assert!(!is_link("https://a b"));
        assert!(!is_link(""));
    }

    #[test]
    fn yt_dlp_is_asked_for_one_film_of_720p() {
        let s = |a: Vec<OsString>| a.into_iter().map(|x| x.to_string_lossy().into_owned()).collect::<Vec<_>>();
        let with = s(args("https://youtu.be/x", Path::new("films"), Some(Path::new("ff/ffmpeg")), Some("node")));
        assert!(with.contains(&"--no-playlist".into()));
        assert!(with.windows(2).any(|w| w[0] == "--merge-output-format" && w[1] == "mp4"));
        assert!(with.windows(2).any(|w| w[0] == "--ffmpeg-location" && w[1].ends_with("ffmpeg")));
        assert!(with.iter().any(|a| a.contains("height<=720")));
        // The link last, after "--" (so a link can never be read as an option).
        assert_eq!(&with[with.len() - 2..], ["--", "https://youtu.be/x"]);
        assert!(with.windows(2).any(|w| w[0] == "--js-runtimes" && w[1] == "node"));
        let without = s(args("https://youtu.be/x", Path::new("films"), None, Some("deno")));
        assert!(!without.iter().any(|a| a == "--merge-output-format" || a == "--ffmpeg-location" || a == "--js-runtimes"));
        assert!(without.iter().any(|a| a.starts_with("b[height<=720][ext=mp4]")));
    }

    #[test]
    fn a_film_is_named_by_its_title() {
        assert_eq!(title_of(Path::new("Links/Me_at_the_zoo [jNQXAC9IVRw].mp4")), "Me at the zoo");
        assert_eq!(title_of(Path::new("plain.mp4")), "plain");
    }

    #[test]
    fn its_lines_say_how_far_and_what_it_made() {
        assert_eq!(parse_line("[download]  42.5% of   12.34MiB at    2.00MiB/s ETA 00:04"), Some(Said::Progress(0.425)));
        assert_eq!(parse_line("[download] 100% of 12.34MiB in 00:00:06"), Some(Said::Progress(1.0)));
        assert_eq!(parse_line("[download] Destination: films/a.mp4"), None);
        assert_eq!(parse_line("FILE:C:\\Users\\x\\Links\\Song [abc].mp4"), Some(Said::File(PathBuf::from("C:\\Users\\x\\Links\\Song [abc].mp4"))));
        assert_eq!(parse_line("[youtube] abc: Downloading webpage"), None);
    }
}
