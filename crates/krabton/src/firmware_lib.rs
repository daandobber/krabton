//! A Virus TI firmware out of the installer it came in, for computers the
//! installer does not run on (Linux) or where it was never installed:
//! Access's installers (.exe, .msi, .dmg, .pkg) and the archives they come
//! in are unpacked with 7-Zip (or msitools, innoextract or bsdtar, whichever
//! is there), the archives inside them too, and the firmware (found by what
//! is in it, whatever it is called) is kept in Krabton's Firmware folder,
//! where Hyper finds it.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Receiver};

/// Archives inside archives: this many deep, this many unpacked at most.
const MAX_DEPTH: u32 = 4;
const MAX_UNPACKS: u32 = 60;

/// Krabton's own folder for firmware (Hyper looks in it).
pub fn firmware_dir() -> PathBuf {
    crate::piano_lib::library_dir().parent().map(Path::to_path_buf).unwrap_or_default().join("Firmware")
}

/// Whether a file is something to unpack: an installer or an archive, by
/// its first bytes (a disk image by its name), and big enough to hold a
/// firmware.
pub fn is_container(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else { return false };
    if !meta.is_file() || meta.len() < 256 * 1024 {
        return false;
    }
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if matches!(ext.as_str(), "dmg" | "iso" | "img" | "hfs") {
        return true;
    }
    let mut head = [0u8; 8];
    if std::fs::File::open(path).and_then(|mut f| f.read_exact(&mut head)).is_err() {
        return false;
    }
    const MAGIC: &[&[u8]] = &[
        b"MZ",                                // a Windows installer
        b"MSCF",                              // a cabinet
        &[0xD0, 0xCF, 0x11, 0xE0],            // an .msi
        b"PK\x03\x04",                        // a zip
        &[b'7', b'z', 0xBC, 0xAF],            // a 7z
        &[0x1F, 0x8B],                        // gzip
        b"xar!",                              // a macOS .pkg
        b"BZh",                               // bzip2
        &[0xFD, b'7', b'z', b'X', b'Z', 0x00], // xz
        b"Rar!",                              // a rar
        b"0707",                              // cpio (a .pkg's payload)
    ];
    MAGIC.iter().any(|m| head.starts_with(m))
}

/// Installers in a folder (a few levels down) that look like Access's for
/// the Virus: archives with "virus" or "access" in their name.
pub fn installers_in(dir: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                if depth < 2 {
                    walk(&p, depth + 1, out);
                }
                continue;
            }
            let name = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
            if (name.contains("virus") || name.contains("access")) && is_container(&p) {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, 0, &mut out);
    out
}

/// 7-Zip, under whichever name it is here.
fn seven_zip() -> Option<PathBuf> {
    let mut names: Vec<PathBuf> = ["7z", "7zz", "7za"].into_iter().map(PathBuf::from).collect();
    if cfg!(windows) {
        for var in ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"] {
            if let Some(d) = std::env::var_os(var) {
                names.push(PathBuf::from(d).join("7-Zip").join("7z.exe"));
            }
        }
    }
    names.into_iter().find(|n| quiet(Command::new(n).arg("i")).is_some())
}

/// Runs a command with nothing in and nothing out; whether it ran at all.
fn quiet(cmd: &mut Command) -> Option<bool> {
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().ok().map(|s| s.success())
}

fn has_files(dir: &Path) -> bool {
    std::fs::read_dir(dir).map(|mut rd| rd.next().is_some()).unwrap_or(false)
}

/// Unpacks `file` into `dest` with the first tool that can; whether
/// anything came out.
fn unpack(file: &Path, dest: &Path, seven: Option<&Path>) -> bool {
    if std::fs::create_dir_all(dest).is_err() {
        return false;
    }
    if let Some(z) = seven {
        quiet(Command::new(z).arg("x").arg("-y").arg("-bd").arg(format!("-o{}", dest.display())).arg(file));
        if has_files(dest) {
            return true;
        }
    }
    let ext = file.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if ext == "msi" {
        quiet(Command::new("msiextract").arg("-C").arg(dest).arg(file));
    } else if ext == "exe" {
        quiet(Command::new("innoextract").arg("-s").arg("-d").arg(dest).arg(file));
    }
    if !has_files(dest) {
        quiet(Command::new("bsdtar").arg("-xf").arg(file).arg("-C").arg(dest));
    }
    has_files(dest)
}

/// Every file under `dir`.
fn files_in(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for p in rd.flatten().map(|e| e.path()) {
        if p.is_dir() {
            files_in(&p, out);
        } else {
            out.push(p);
        }
    }
}

/// The firmware in an installer (or an archive), unpacked as deep as it
/// takes, kept in `keep_in` (named after the installer). A firmware kept
/// there before, the same to the byte, is that one.
pub fn firmware_in(installer: &Path, keep_in: &Path) -> Result<PathBuf, String> {
    let name = installer.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    if krabton_engine::hyper::looks_like_ti_firmware(installer) {
        return keep(installer, installer, keep_in);
    }
    let seven = seven_zip();
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let work = std::env::temp_dir().join(format!("krabton-firmware-{}-{stamp}", std::process::id()));
    let mut queue = vec![(installer.to_path_buf(), 0u32)];
    let mut unpacks = 0;
    let mut opened = false;
    let mut found = None;
    while let Some((file, depth)) = queue.pop() {
        unpacks += 1;
        if unpacks > MAX_UNPACKS {
            break;
        }
        let dest = work.join(unpacks.to_string());
        if !unpack(&file, &dest, seven.as_deref()) {
            continue;
        }
        opened = true;
        let mut files = Vec::new();
        files_in(&dest, &mut files);
        files.sort();
        if let Some(fw) = files.iter().find(|p| krabton_engine::hyper::looks_like_ti_firmware(p)) {
            found = Some(fw.clone());
            break;
        }
        if depth < MAX_DEPTH {
            // Bigger ones last, so taken first: the firmware is in a big one.
            let mut inner: Vec<(u64, PathBuf)> = files.into_iter().filter(|p| is_container(p)).map(|p| (std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0), p)).collect();
            inner.sort();
            queue.extend(inner.into_iter().map(|(_, p)| (p, depth + 1)));
        }
    }
    let result = match found {
        Some(fw) => keep(&fw, installer, keep_in),
        None if !opened && seven.is_none() => Err(format!("Krabton unpacks {name} with 7-Zip, and 7-Zip is not on this computer: install it (Linux: the 7zip or p7zip-full package; Windows: 7-zip.org) and open it again")),
        None if !opened => Err(format!("{name} could not be unpacked")),
        None => Err(format!("No Virus TI firmware in {name} (a file of 6 to 9 MB that starts with FORM)")),
    };
    let _ = std::fs::remove_dir_all(&work);
    result
}

/// The firmware copied into `keep_in`, unless the same one is there.
fn keep(fw: &Path, installer: &Path, keep_in: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(keep_in).map_err(|e| format!("{}: {e}", keep_in.display()))?;
    let bytes = std::fs::read(fw).map_err(|e| format!("{}: {e}", fw.display()))?;
    let stem = installer.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Virus TI".into());
    let mut target = keep_in.join(format!("{stem} firmware.bin"));
    let mut k = 2;
    while target.exists() {
        if std::fs::read(&target).is_ok_and(|b| b == bytes) {
            return Ok(target);
        }
        target = keep_in.join(format!("{stem} firmware {k}.bin"));
        k += 1;
    }
    std::fs::write(&target, &bytes).map_err(|e| format!("{}: {e}", target.display()))?;
    Ok(target)
}

/// [`firmware_in`] on a thread of its own, into Krabton's Firmware folder.
pub fn start(installer: PathBuf) -> Receiver<Result<PathBuf, String>> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let _ = tx.send(firmware_in(&installer, &firmware_dir()));
    });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("krabton-fwtest-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Something shaped like a TI firmware (Access's container, 6.5 MB).
    fn fake_firmware(at: &Path) {
        // Noise, as compressed code is: it does not pack small.
        let mut b = vec![0u8; 6_500_000];
        let mut r = 0x2545_F491_4F6C_DD1Du64;
        for x in b.iter_mut() {
            r ^= r << 13;
            r ^= r >> 7;
            r ^= r << 17;
            *x = (r >> 32) as u8;
        }
        b[..4].copy_from_slice(b"FORM");
        std::fs::write(at, b).unwrap();
    }

    #[test]
    fn the_firmware_comes_out_of_archives_in_archives() {
        let Some(z) = seven_zip() else {
            eprintln!("no 7-Zip here: skipped");
            return;
        };
        let d = scratch("nested");
        // As an installer would: a firmware with a name of its own, in a cabinet-ish zip, in a 7z.
        fake_firmware(&d.join("fil8A2C41D0"));
        assert!(quiet(Command::new(&z).arg("a").arg("-bd").arg("-mx=1").arg(d.join("Data1.zip")).arg(d.join("fil8A2C41D0"))).is_some());
        std::fs::write(d.join("readme.txt"), "not it").unwrap();
        assert!(quiet(Command::new(&z).arg("a").arg("-bd").arg("-mx=1").arg(d.join("Virus TI Setup.7z")).arg(d.join("Data1.zip")).arg(d.join("readme.txt"))).is_some());
        assert!(is_container(&d.join("Virus TI Setup.7z")));
        assert_eq!(installers_in(&d), vec![d.join("Virus TI Setup.7z")]);
        let keep_in = d.join("Firmware");
        let got = firmware_in(&d.join("Virus TI Setup.7z"), &keep_in).unwrap();
        assert_eq!(got, keep_in.join("Virus TI Setup firmware.bin"));
        assert!(krabton_engine::hyper::looks_like_ti_firmware(&got));
        // Again: the same one, not a second copy.
        assert_eq!(firmware_in(&d.join("Virus TI Setup.7z"), &keep_in).unwrap(), got);
        assert_eq!(std::fs::read_dir(&keep_in).unwrap().count(), 1);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn without_a_firmware_it_says_so() {
        let d = scratch("empty");
        std::fs::write(d.join("small.zip"), b"PK\x03\x04 too small").unwrap();
        assert!(!is_container(&d.join("small.zip")), "too small to hold a firmware");
        let mut big = b"MZ".to_vec();
        big.resize(300_000, 7);
        std::fs::write(d.join("access_tool.exe"), &big).unwrap();
        assert!(is_container(&d.join("access_tool.exe")));
        let err = firmware_in(&d.join("access_tool.exe"), &d.join("Firmware")).unwrap_err();
        assert!(err.contains("access_tool.exe"), "{err}");
        assert!(!d.join("Firmware").exists() || std::fs::read_dir(d.join("Firmware")).unwrap().count() == 0);
        let _ = std::fs::remove_dir_all(&d);
    }
}
