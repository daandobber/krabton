//! Breaks to cut up: free drum loops downloaded when picked (with the
//! system's `curl`) into Documents/Krabton/Library/Breaks, each with its
//! length in bars so Breaks cuts it right at once. Most are CC0 on
//! Freesound (their 128 kbit/s previews, which need no account) or in Sonic
//! Pi; a few are cut from old records, and say so.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Where a break comes from, and so what may be done with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    /// Made by those who uploaded them to Freesound, given away as CC0.
    Free,
    /// Sonic Pi's loops, CC0 (from Freesound too).
    SonicPi,
    /// Uploaded as CC0, but cut from a record: the recording is its makers'.
    Records,
}

impl Group {
    pub const ALL: [Group; 3] = [Group::Free, Group::SonicPi, Group::Records];

    pub fn name(self) -> &'static str {
        match self {
            Group::Free => "Free breaks (CC0)",
            Group::SonicPi => "Sonic Pi loops (CC0)",
            Group::Records => "From records",
        }
    }
}

pub struct LibBreak {
    pub name: &'static str,
    /// The file it is kept in, in `dir()`.
    pub file: &'static str,
    pub url: &'static str,
    /// Its length in bars (0: Breaks works it out) and its tempo.
    pub bars: u8,
    pub bpm: f32,
    pub by: &'static str,
    /// Where it was found.
    pub page: &'static str,
    pub group: Group,
}

macro_rules! fs {
    ($name:expr, $dir:expr, $id:expr, $uid:expr, $user:expr, $bars:expr, $bpm:expr, $group:expr) => {
        LibBreak {
            name: $name,
            file: concat!("freesound-", $id, ".mp3"),
            url: concat!("https://cdn.freesound.org/previews/", $dir, "/", $id, "_", $uid, "-hq.mp3"),
            bars: $bars,
            bpm: $bpm,
            by: $user,
            page: concat!("https://freesound.org/people/", $user, "/sounds/", $id, "/"),
            group: $group,
        }
    };
}

macro_rules! sp {
    ($name:expr, $file:expr, $bars:expr, $bpm:expr, $by:expr, $page:expr, $group:expr) => {
        LibBreak { name: $name, file: concat!("sonicpi-", $file, ".flac"), url: concat!("https://raw.githubusercontent.com/sonic-pi-net/sonic-pi/a0e3699a82a6ae236a759559eae842131d05a485/etc/samples/", $file, ".flac"), bars: $bars, bpm: $bpm, by: $by, page: $page, group: $group }
    };
}

pub static BREAKS: &[LibBreak] = &[
    fs!("Venetian", 669, 669926, 9839964, "harrisonlace", 8, 175.0, Group::Free),
    fs!("Slow DnB", 65, 65770, 894560, "dahovv", 8, 175.0, Group::Free),
    fs!("IDM", 494, 494914, 8139179, "lakepunch", 8, 170.0, Group::Free),
    fs!("Crunchy", 629, 629052, 11532701, "deadrobotmusic", 8, 128.0, Group::Free),
    fs!("Old School Rave", 183, 183441, 236950, "Alastair_Pursloe", 8, 135.0, Group::Free),
    fs!("Distorted", 816, 816134, 16924119, "nyfeukgia", 4, 144.0, Group::Free),
    fs!("Break 180", 87, 87154, 872714, "Blackie666", 4, 180.0, Group::Free),
    fs!("Classic 180", 626, 626471, 8130761, "nolit47", 4, 180.0, Group::Free),
    fs!("Tortoise vs. Hare", 268, 268110, 2514850, "NenadSimic", 4, 160.0, Group::Free),
    fs!("Nuttah", 677, 677705, 14211176, "nintendontmusic", 4, 170.0, Group::Free),
    fs!("Wey", 240, 240099, 997039, "Bronxio", 4, 90.0, Group::Free),
    fs!("DnB 174", 613, 613927, 9855733, "crash_358", 2, 174.0, Group::Free),
    fs!("Live DnB", 731, 731436, 15766305, "MarkAllentheProducer", 2, 160.0, Group::Free),
    fs!("Mean", 531, 531835, 3069580, "El_Bardo", 2, 170.0, Group::Free),
    fs!("Broke", 531, 531826, 3069580, "El_Bardo", 2, 170.0, Group::Free),
    fs!("Breakbeat 170", 631, 631870, 13826696, "DJ3WB", 2, 170.0, Group::Free),
    fs!("Jungle Thought", 239, 239078, 997039, "Bronxio", 2, 100.0, Group::Free),
    fs!("Funkbeat", 172, 172647, 33044, "Snapper4298", 2, 100.0, Group::Free),
    fs!("Hip-Hop 95", 678, 678304, 14784311, "msx2plus", 1, 95.0, Group::Free),
    fs!("Jack Inside", 649, 649045, 8705222, "Liquid_Tribal", 1, 140.0, Group::Free),
    fs!("Pseudo-Think", 540, 540737, 11045610, "vekoN", 1, 113.0, Group::Free),
    sp!("Breakbeat", "loop_breakbeat", 1, 126.0, "Snapper4298", "https://freesound.org/people/Snapper4298/sounds/160354/", Group::SonicPi),
    sp!("Compus", "loop_compus", 0, 148.0, "godspine", "https://freesound.org/people/godspine/sounds/212288/", Group::SonicPi),
    sp!("Garzul", "loop_garzul", 4, 120.0, "Garzul", "https://freesound.org/people/Garzul/sounds/213524/", Group::SonicPi),
    sp!("Mika", "loop_mika", 4, 120.0, "mika55", "https://freesound.org/people/mika55/sounds/188551/", Group::SonicPi),
    sp!("Safari", "loop_safari", 4, 120.0, "nemaavla", "https://freesound.org/people/nemaavla/sounds/318759/", Group::SonicPi),
    sp!("Tabla", "loop_tabla", 4, 90.0, "lezaarth", "https://freesound.org/people/lezaarth/sounds/242591/", Group::SonicPi),
    fs!("Amen 170", 263, 263872, 3095604, "Kevcio", 4, 170.0, Group::Records),
    fs!("Amen Breakcore 190", 713, 713695, 14667038, "shallowed", 8, 190.0, Group::Records),
    fs!("Think", 654, 654667, 14036999, "GregQuisha", 1, 140.0, Group::Records),
    sp!("Amen", "loop_amen", 1, 137.0, "Dolfeus", "https://freesound.org/people/Dolfeus/sounds/43389/", Group::Records),
    sp!("Amen, four bars", "loop_amen_full", 4, 140.0, "gowers", "https://freesound.org/people/gowers/sounds/202537/", Group::Records),
];

/// Where they are kept: Documents/Krabton/Library/Breaks.
pub fn dir() -> PathBuf {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from).unwrap_or_default();
    home.join("Documents").join("Krabton").join("Library").join("Breaks")
}

impl LibBreak {
    pub fn path(&self) -> PathBuf {
        dir().join(self.file)
    }

    pub fn have(&self) -> bool {
        self.path().is_file()
    }

    /// Who made it and what may be done with it (its hover text).
    pub fn credit(&self) -> String {
        let tempo = if self.bars > 0 { format!("{} bars at {:.0}", self.bars, self.bpm) } else { format!("about {:.0} BPM", self.bpm) };
        match self.group {
            Group::Free => format!("{tempo}. By {} on Freesound, CC0 (free for anything): {}", self.by, self.page),
            Group::SonicPi => format!("{tempo}. From Sonic Pi's samples, by {} on Freesound, CC0 (free for anything): {}", self.by, self.page),
            Group::Records => format!("{tempo}. Uploaded to Freesound as CC0 by {}, but cut from an old record: the recording belongs to its makers, so check before you release anything made with it. {}", self.by, self.page),
        }
    }

    /// Its length as Breaks' Bars setting (0: Auto).
    pub fn bars_setting(&self) -> f32 {
        match self.bars {
            1 => 1.0,
            2 => 2.0,
            4 => 3.0,
            8 => 4.0,
            _ => 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Downloading,
    Ready,
    Failed(String),
}

pub type Fetch = Arc<Mutex<State>>;

/// Downloads break `k` in the background (at once Ready if it is here already).
pub fn start(k: usize) -> Fetch {
    let state: Fetch = Arc::new(Mutex::new(State::Downloading));
    let Some(b) = BREAKS.get(k) else {
        return Arc::new(Mutex::new(State::Failed("no such break".into())));
    };
    if b.have() {
        return Arc::new(Mutex::new(State::Ready));
    }
    let s = state.clone();
    std::thread::spawn(move || {
        let result = std::fs::create_dir_all(dir()).map_err(|e| format!("{}: {e}", dir().display())).and_then(|_| crate::piano_lib::fetch(b.url, &b.path()));
        let _ = std::fs::write(dir().join("README.txt"), readme());
        if let Ok(mut st) = s.lock() {
            *st = match result {
                Ok(()) => State::Ready,
                Err(e) => State::Failed(e),
            };
        }
    });
    state
}

/// Who made each, beside the files.
fn readme() -> String {
    let mut s = String::from("Breaks for Krabton's Breaks instrument, downloaded when picked.\n\n");
    for b in BREAKS {
        s += &format!("{}  ({}): {}\n", b.file, b.name, b.credit());
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_break_has_a_file_of_its_own_and_a_place_it_came_from() {
        let mut files = std::collections::HashSet::new();
        for b in BREAKS {
            assert!(files.insert(b.file), "{} twice", b.file);
            assert!(b.url.starts_with("https://") && b.page.starts_with("https://"));
            assert!([0, 1, 2, 4, 8].contains(&b.bars), "{}", b.name);
            assert!(!b.credit().is_empty());
        }
        assert_eq!(BREAKS[0].url, "https://cdn.freesound.org/previews/669/669926_9839964-hq.mp3");
        assert_eq!(BREAKS[0].page, "https://freesound.org/people/harrisonlace/sounds/669926/");
        assert!(BREAKS.iter().any(|b| b.group == Group::Records && b.credit().contains("check before you release")));
    }
}
