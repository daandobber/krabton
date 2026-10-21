//! UTAU voicebanks for the Singer to download: free Japanese VCV voices
//! whose terms let them be passed on, fetched with the system's `curl` from
//! their GitHub mirrors (by oxygen-dioxide, at a fixed commit) and unpacked
//! into Documents/Krabton/Library/Voicebanks, each with its own readme and
//! terms as it came.

use krabton_engine::singer::VoicebankRef;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

/// A voicebank Krabton can fetch.
pub struct Offer {
    pub name: &'static str,
    /// Its folder in the library.
    pub folder: &'static str,
    pub url: &'static str,
    /// About how big the download is, to show and to follow it.
    pub bytes: u64,
    pub about: &'static str,
    /// Its terms, in short (the whole of them comes with it).
    pub terms: &'static str,
    /// Where it comes from.
    pub page: &'static str,
}

pub const OFFERS: [Offer; 4] = [
    Offer {
        name: "Dorodoro β",
        folder: "Dorodoro",
        url: "https://github.com/oxygen-dioxide/dorodoro-ja-vcv/archive/a726f3e3a26874ad37ecfb31bc191d337480b38b.zip",
        bytes: 61_537_380,
        about: "Dorodoro β (どろどろβ) by Orcinus-Library, voiced by Cerenothoa: Japanese VCV, one pitch (G3)",
        terms: "Use it any way you like, commercially too; when passing it on, give its official page and keep its readme.txt as it is",
        page: "https://chongzhen.wikidot.com/files:8940236",
    },
    Offer {
        name: "Misa Mochi",
        folder: "Misa Mochi",
        url: "https://github.com/oxygen-dioxide/misa-mochi-ja/archive/e20c1aa0dd8fe8d4f1a76bbda0d204fa603662c4.zip",
        bytes: 63_000_000,
        about: "Misa Mochi by KrankerApfel, voiced by MISAO (oto by Text_To_Death): Japanese VCV, one pitch (B3)",
        terms: "CC BY-NC-SA: credit her, not for commercial use, share what you change alike",
        page: "https://github.com/KrankerApfel/Misa-Mochi",
    },
    Offer {
        name: "Gakuto Deshimaru χ",
        folder: "Gakuto Deshimaru",
        url: "https://github.com/oxygen-dioxide/gakuto-deshimaru-chi-ja/archive/563ac8bd0ab78a8ead80f3345f58875e526b5789.zip",
        bytes: 74_000_000,
        about: "Gakuto Deshimaru χ (学人デシマル χ) by UtaUtaUtau: a man's voice, Japanese VCV, four pitches (A2 to C4) with power",
        terms: "Name the voicebank when you publish; commercial use only with the author's approval; no sexual, political or religious content",
        page: "https://utautautau.neocities.org/utaus/deshimaru.html",
    },
    Offer {
        name: "Milk",
        folder: "Milk",
        url: "https://github.com/oxygen-dioxide/milk-ja/archive/b69310d792e2c3cc692c942f29129bf90a02898e.zip",
        bytes: 268_000_000,
        about: "Milk by Xepheris: Japanese VCV, six pitches (F3 to C5) with falsetto and breaths",
        terms: "Name the voicebank when you publish; commercial use only with the author's approval; no political content",
        page: "https://xepheris.wixsite.com/milk",
    },
];

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Downloading { bytes: u64 },
    Unpacking,
    /// Here and read.
    Ready(VoicebankRef),
    Failed(String),
}

pub type Install = Arc<Mutex<State>>;

/// Where the voicebanks go: Documents/Krabton/Library/Voicebanks.
pub fn dir() -> PathBuf {
    crate::piano_lib::library_dir().parent().map(Path::to_path_buf).unwrap_or_default().join("Voicebanks")
}

/// The offer's folder, when it is here.
pub fn installed(offer: &Offer) -> Option<PathBuf> {
    let path = dir().join(offer.folder);
    path.join("oto.ini").is_file().then_some(path)
}

/// Every offered voicebank that is here.
pub fn all_installed() -> Vec<PathBuf> {
    OFFERS.iter().filter_map(installed).collect()
}

/// Starts fetching an offer in the background; the state tells how far it is.
pub fn start(i: usize) -> Install {
    let state: Install = Arc::new(Mutex::new(State::Downloading { bytes: 0 }));
    let s = state.clone();
    std::thread::spawn(move || {
        let result = fetch(&OFFERS[i], &s);
        if let Ok(mut st) = s.lock() {
            *st = match result {
                Ok(bank) => State::Ready(bank),
                Err(e) => State::Failed(e),
            };
        }
    });
    state
}

fn fetch(offer: &Offer, state: &Install) -> Result<VoicebankRef, String> {
    let dir = dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let zip = dir.join(format!("{}.zip.part", offer.folder));
    let mut child = crate::piano_lib::curl()
        .args(["-sSfL", "--retry", "3", "--retry-delay", "2", "-o"])
        .arg(&zip)
        .arg(offer.url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("curl could not run ({e}); it comes with Windows 10 and later, macOS and most Linux systems"))?;
    // How far it is: the size of what has arrived.
    let status = loop {
        if let Some(st) = child.try_wait().map_err(|e| e.to_string())? {
            break st;
        }
        let bytes = std::fs::metadata(&zip).map(|m| m.len()).unwrap_or(0);
        if let Ok(mut s) = state.lock() {
            *s = State::Downloading { bytes };
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    };
    if !status.success() {
        let _ = std::fs::remove_file(&zip);
        return Err(format!("downloading {} failed", offer.name));
    }
    if let Ok(mut s) = state.lock() {
        *s = State::Unpacking;
    }
    // Unpacked beside its place, then moved in whole.
    let to = dir.join(offer.folder);
    let part = dir.join(format!("{}.part", offer.folder));
    let _ = std::fs::remove_dir_all(&part);
    let unpacked = unzip(&zip, &part);
    let _ = std::fs::remove_file(&zip);
    if let Err(e) = unpacked {
        let _ = std::fs::remove_dir_all(&part);
        return Err(format!("{} arrived but could not be unpacked: {e}", offer.name));
    }
    let _ = std::fs::remove_dir_all(&to);
    std::fs::rename(&part, &to).map_err(|e| e.to_string())?;
    let _ = std::fs::write(
        to.join("Krabton-SOURCE.txt"),
        format!("{}\n{}\n\nTerms in short: {}.\nThe whole of its terms are in its own readme and license files here.\n\nFrom {}\nOfficial page: {}\n", offer.name, offer.about, offer.terms, offer.url, offer.page),
    );
    VoicebankRef::load(&to).map_err(|e| format!("{} arrived but is not a voicebank: {e}", offer.name))
}

/// Unpacks a zip into `to`, without the one folder everything sits in (a
/// GitHub archive's): stored and deflated files, names in UTF-8 or Shift-JIS,
/// nothing outside `to`.
pub fn unzip(zip: &Path, to: &Path) -> Result<usize, String> {
    let mut f = std::fs::File::open(zip).map_err(|e| e.to_string())?;
    let len = f.metadata().map_err(|e| e.to_string())?.len();
    // The end of the central directory, in the last 64 KiB.
    let tail_len = len.min(65_557);
    f.seek(SeekFrom::Start(len - tail_len)).map_err(|e| e.to_string())?;
    let mut tail = vec![0u8; tail_len as usize];
    f.read_exact(&mut tail).map_err(|e| e.to_string())?;
    let at = (0..tail.len().saturating_sub(21)).rev().find(|&i| tail[i..i + 4] == [0x50, 0x4b, 0x05, 0x06]).ok_or("not a zip file")?;
    let eocd = &tail[at..];
    let u16_at = |b: &[u8], o: usize| u16::from_le_bytes([b[o], b[o + 1]]) as u64;
    let u32_at = |b: &[u8], o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) as u64;
    let u64_at = |b: &[u8], o: usize| u64::from_le_bytes(b[o..o + 8].try_into().unwrap_or([0; 8]));
    let (mut count, mut cd_size, mut cd_at) = (u16_at(eocd, 10), u32_at(eocd, 12), u32_at(eocd, 16));
    // ZIP64: its own end record, which the locator just before points at.
    if (count == 0xFFFF || cd_size == 0xFFFF_FFFF || cd_at == 0xFFFF_FFFF) && at >= 20 && tail[at - 20..at - 16] == [0x50, 0x4b, 0x06, 0x07] {
        let z_at = u64_at(&tail, at - 12);
        let mut z = [0u8; 56];
        f.seek(SeekFrom::Start(z_at)).map_err(|e| e.to_string())?;
        f.read_exact(&mut z).map_err(|e| e.to_string())?;
        (count, cd_size, cd_at) = (u64_at(&z, 32), u64_at(&z, 40), u64_at(&z, 48));
    }
    if cd_size > len {
        return Err("broken zip file".into());
    }
    let mut cd = vec![0u8; cd_size as usize];
    f.seek(SeekFrom::Start(cd_at)).map_err(|e| e.to_string())?;
    f.read_exact(&mut cd).map_err(|e| e.to_string())?;
    struct Entry {
        name: String,
        method: u64,
        size: u64,
        packed: u64,
        local: u64,
    }
    let mut entries = Vec::new();
    let mut p = 0usize;
    for _ in 0..count {
        if p + 46 > cd.len() || cd[p..p + 4] != [0x50, 0x4b, 0x01, 0x02] {
            return Err("broken zip directory".into());
        }
        let flags = u16_at(&cd, p + 8);
        let method = u16_at(&cd, p + 10);
        let (mut packed, mut size) = (u32_at(&cd, p + 20), u32_at(&cd, p + 24));
        let (n, e, c) = (u16_at(&cd, p + 28) as usize, u16_at(&cd, p + 30) as usize, u16_at(&cd, p + 32) as usize);
        let mut local = u32_at(&cd, p + 42);
        let raw = cd.get(p + 46..p + 46 + n).ok_or("broken zip directory")?;
        let name = if flags & 0x800 != 0 { String::from_utf8_lossy(raw).into_owned() } else { krabton_engine::singer::voicebank::decode(raw) };
        // ZIP64 sizes and offset, in the order of those that overflowed.
        let mut x = p + 46 + n;
        let x_end = (x + e).min(cd.len());
        while x + 4 <= x_end {
            let (id, l) = (u16_at(&cd, x), u16_at(&cd, x + 2) as usize);
            if id == 1 {
                let mut o = x + 4;
                for v in [&mut size, &mut packed, &mut local] {
                    if *v == 0xFFFF_FFFF && o + 8 <= x_end {
                        *v = u64_at(&cd, o);
                        o += 8;
                    }
                }
            }
            x += 4 + l;
        }
        entries.push(Entry { name, method, size, packed, local });
        p += 46 + n + e + c;
    }
    // The one folder everything sits in, left out.
    let top = entries.first().and_then(|e| e.name.split('/').next()).map(str::to_string).unwrap_or_default();
    let plain = matches!(Path::new(&top).components().collect::<Vec<_>>().as_slice(), [Component::Normal(_)]);
    let strip = plain && entries.iter().all(|e| e.name.starts_with(&format!("{top}/")));
    std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
    let mut files = 0;
    for en in &entries {
        let name = if strip { &en.name[top.len() + 1..] } else { en.name.as_str() };
        if name.is_empty() {
            continue;
        }
        // Nothing may land outside `to`.
        let rel = Path::new(name);
        if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
            return Err(format!("a file outside the folder: {name}"));
        }
        let out = to.join(rel);
        if name.ends_with('/') {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            continue;
        }
        let mut head = [0u8; 30];
        f.seek(SeekFrom::Start(en.local)).map_err(|e| e.to_string())?;
        f.read_exact(&mut head).map_err(|e| e.to_string())?;
        if head[..4] != [0x50, 0x4b, 0x03, 0x04] {
            return Err(format!("broken zip entry: {name}"));
        }
        let skip = u16_at(&head, 26) + u16_at(&head, 28);
        f.seek(SeekFrom::Current(skip as i64)).map_err(|e| e.to_string())?;
        if en.packed > len {
            return Err(format!("broken zip entry: {name}"));
        }
        let mut packed = vec![0u8; en.packed as usize];
        f.read_exact(&mut packed).map_err(|e| e.to_string())?;
        let data = match en.method {
            0 => packed,
            8 => miniz_oxide::inflate::decompress_to_vec_with_limit(&packed, en.size as usize).map_err(|e| format!("{name}: {e:?}"))?,
            m => return Err(format!("{name}: packed a way Krabton does not read ({m})")),
        };
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&out, data).map_err(|e| format!("{}: {e}", out.display()))?;
        files += 1;
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A zip as GitHub makes them: one folder around everything, a stored
    /// file and a deflated one with a Japanese name.
    fn zip_of(files: &[(&str, &[u8], bool)]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut cd = Vec::new();
        for (name, data, deflate) in files {
            let packed = if *deflate { miniz_oxide::deflate::compress_to_vec(data, 6) } else { data.to_vec() };
            let local = out.len() as u32;
            let method: u16 = if *deflate { 8 } else { 0 };
            out.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04, 20, 0]);
            out.extend_from_slice(&0x800u16.to_le_bytes());
            out.extend_from_slice(&method.to_le_bytes());
            out.extend_from_slice(&[0; 8]);
            out.extend_from_slice(&(packed.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(&packed);
            cd.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02, 20, 0, 20, 0]);
            cd.extend_from_slice(&0x800u16.to_le_bytes());
            cd.extend_from_slice(&method.to_le_bytes());
            cd.extend_from_slice(&[0; 8]);
            cd.extend_from_slice(&(packed.len() as u32).to_le_bytes());
            cd.extend_from_slice(&(data.len() as u32).to_le_bytes());
            cd.extend_from_slice(&(name.len() as u16).to_le_bytes());
            cd.extend_from_slice(&[0; 12]);
            cd.extend_from_slice(&local.to_le_bytes());
            cd.extend_from_slice(name.as_bytes());
        }
        let cd_at = out.len() as u32;
        out.extend_from_slice(&cd);
        out.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0]);
        out.extend_from_slice(&(files.len() as u16).to_le_bytes());
        out.extend_from_slice(&(files.len() as u16).to_le_bytes());
        out.extend_from_slice(&(cd.len() as u32).to_le_bytes());
        out.extend_from_slice(&cd_at.to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        out
    }

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("krabton-unzip-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_voicebank_zip_unpacks_into_its_folder() {
        let d = scratch("ok");
        let wav: Vec<u8> = (0..5000u32).map(|i| (i * 7 % 251) as u8).collect();
        let zip = zip_of(&[("bank-abc/", b"", false), ("bank-abc/oto.ini", b"a.wav=- \x82\xa0,0,0,0,0,0", false), ("bank-abc/A3/_あい.wav", &wav, true)]);
        std::fs::write(d.join("v.zip"), zip).unwrap();
        let n = unzip(&d.join("v.zip"), &d.join("bank")).unwrap();
        assert_eq!(n, 2);
        assert!(d.join("bank/oto.ini").is_file(), "without the folder around it");
        assert_eq!(std::fs::read(d.join("bank/A3/_あい.wav")).unwrap(), wav);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn nothing_lands_outside_the_folder() {
        let d = scratch("evil");
        for files in [&[("../evil.txt", &b"x"[..], false)][..], &[("bank/ok.txt", &b"x"[..], false), ("bank/../../evil.txt", &b"x"[..], false)][..]] {
            std::fs::write(d.join("e.zip"), zip_of(files)).unwrap();
            assert!(unzip(&d.join("e.zip"), &d.join("in/bank")).is_err());
            assert!(!d.join("evil.txt").exists() && !d.join("in/evil.txt").exists());
        }
        let _ = std::fs::remove_dir_all(&d);
    }
}
