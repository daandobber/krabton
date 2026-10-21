//! Drummer: a drummer who plays styles. A groove of two bars in fourteen
//! lanes (kick, snares, hats, ride, crash, three toms, tambourine, cowbell)
//! with ghost notes, accents and rolls; pick a style (rock, funk, house,
//! boom bap, bossa nova, jazz...) and edit it. On top of the groove it plays
//! like a drummer: Complexity adds ghost notes, sixteenth hats and kick
//! pickups, Energy sets the dynamics (and moves to the side stick when low,
//! to the ride when high), a fill ends every few bars (tom runs, snare
//! builds, rolls) with a crash after it, and Humanize loosens the timing and
//! the dynamics the way hands do.
//!
//! It plays the notes of the kit on the track: a lane whose drum the kit
//! does not have takes the closest one it has (a ride becomes a closed hat,
//! a floor tom the tom).

use super::SeqState;
use crate::midi_fx::{MidiCtx, MidiEvent};
use crate::params::{ParamSpec, Unit};

pub const BARS: usize = 0;
pub const COMPLEXITY: usize = 1;
pub const ENERGY: usize = 2;
pub const FILLS: usize = 3;
pub const FILL_LENGTH: usize = 4;
pub const CRASH: usize = 5;
pub const SWING: usize = 6;
pub const SWING_ON: usize = 7;
pub const HUMANIZE: usize = 8;
pub const VELOCITY: usize = 9;
/// The step: sixteenths, or thirty-seconds for double time, eighths for half time...
pub const RATE: usize = 12;

pub const BAR_CHOICES: &[&str] = &["A", "A B", "A A A B"];
pub const FILL_CHOICES: &[&str] = &["Off", "2 Bars", "4 Bars", "8 Bars"];
pub const FILL_LENGTHS: &[&str] = &["1 Beat", "2 Beats", "1 Bar"];
pub const SWING_UNITS: &[&str] = &["16ths", "8ths"];

pub static SPECS: &[ParamSpec] = &[
    ParamSpec::choice("Bars", "Groove", BAR_CHOICES, 1),
    ParamSpec::lin("Complexity", "Groove", 0.0, 1.0, 0.3, Unit::Percent),
    ParamSpec::lin("Energy", "Groove", 0.0, 1.0, 0.55, Unit::Percent),
    ParamSpec::choice("Fill Every", "Fills", FILL_CHOICES, 2),
    ParamSpec::choice("Fill Length", "Fills", FILL_LENGTHS, 0),
    ParamSpec::toggle("Crash", "Fills", true),
    ParamSpec::lin("Swing", "Feel", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::choice("Swing On", "Feel", SWING_UNITS, 0),
    ParamSpec::lin("Humanize", "Feel", 0.0, 1.0, 0.35, Unit::Percent),
    ParamSpec::lin("Velocity", "Feel", 0.05, 1.0, 0.85, Unit::Percent),
    ParamSpec::toggle("Transpose", "Keys", false),
    ParamSpec::toggle("Hold", "Keys", false),
    ParamSpec::choice("Rate", "Groove", super::RATES, 2),
    ParamSpec::lin("Vel Low", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel High", "Dynamics", 0.0, 1.0, 1.0, Unit::Percent),
    ParamSpec::lin("Vel Random", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
    ParamSpec::lin("Vel Accent", "Dynamics", 0.0, 1.0, 0.0, Unit::Percent),
];

// ---------------------------------------------------------------- lanes

pub const LANES: usize = 14;
pub const KICK: usize = 0;
pub const SNARE: usize = 1;
pub const STICK: usize = 2;
pub const CLAP: usize = 3;
pub const HAT: usize = 4;
pub const PEDAL: usize = 5;
pub const OPEN: usize = 6;
pub const RIDE: usize = 7;
pub const CRASH_LANE: usize = 8;
pub const TOM_HI: usize = 9;
pub const TOM_MID: usize = 10;
pub const TOM_FLOOR: usize = 11;
pub const TAMB: usize = 12;
pub const BELL: usize = 13;

pub const LANE_NAMES: [&str; LANES] = ["Kick", "Snare", "Side Stick", "Clap", "Closed Hat", "Pedal Hat", "Open Hat", "Ride", "Crash", "High Tom", "Mid Tom", "Floor Tom", "Tambourine", "Cowbell"];

/// The drums each lane would like (General MIDI notes), best first, so any
/// kit plays sensibly.
pub const LANE_NOTES: [&[u8]; LANES] = [
    &[36, 35],
    &[38, 40, 39],
    &[37, 38],
    &[39, 38, 40],
    &[42, 44, 46],
    &[44, 42],
    &[46, 42],
    &[51, 59, 53, 42],
    &[49, 57, 55, 52, 46],
    &[50, 48, 47, 45],
    &[47, 48, 45, 50],
    &[43, 41, 45],
    &[54, 69, 70, 82, 44, 42],
    &[56, 37],
];

/// The note a lane plays in a kit (`kit`: a mask of the notes it has, 0
/// when unknown: then General MIDI).
pub fn lane_note(lane: usize, kit: u128) -> Option<u8> {
    let c = LANE_NOTES[lane.min(LANES - 1)];
    if kit == 0 {
        return Some(c[0]);
    }
    c.iter().copied().find(|n| kit >> n & 1 == 1)
}

// ---------------------------------------------------------------- cells

pub const OFF: u8 = 0;
pub const GHOST: u8 = 1;
pub const HIT: u8 = 2;
pub const ACCENT: u8 = 3;
/// Two thirty-seconds.
pub const ROLL: u8 = 4;

const MAGIC: [u8; 2] = *b"D1";
pub const STEPS: usize = 16;
pub const DATA_BYTES: usize = 4 + 2 * STEPS * LANES;

fn at(bar: usize, step: usize, lane: usize) -> usize {
    4 + ((bar & 1) * STEPS + step % STEPS) * LANES + lane.min(LANES - 1)
}

pub fn cell(data: &[u8], bar: usize, step: usize, lane: usize) -> u8 {
    if data.len() < DATA_BYTES || data[..2] != MAGIC {
        return OFF;
    }
    data[at(bar, step, lane)].min(ROLL)
}

pub fn set_cell(data: &mut Vec<u8>, bar: usize, step: usize, lane: usize, v: u8) {
    if data.len() < DATA_BYTES || data[..2] != MAGIC {
        *data = empty();
    }
    data[at(bar, step, lane)] = v.min(ROLL);
}

pub fn is_current(data: &[u8]) -> bool {
    data.len() >= DATA_BYTES && data[..2] == MAGIC
}

/// The groove is no longer a style's (it was cleared).
pub fn forget_style(data: &mut [u8]) {
    if is_current(data) {
        data[2] = u8::MAX;
    }
}

/// Bar B becomes bar A with a couple of the changes a drummer makes.
pub fn vary(data: &mut Vec<u8>, seed: u32) {
    if !is_current(data) {
        *data = empty();
    }
    let mut s = seed | 1;
    let mut rnd = move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        (s >> 8) as usize
    };
    for st in 0..STEPS {
        for l in 0..LANES {
            data[at(1, st, l)] = data[at(0, st, l)];
        }
    }
    let first = rnd() % 5;
    let second = (first + 1 + rnd() % 4) % 5;
    for change in [first, second] {
        let free = |d: &[u8], st: usize, l: usize| d[at(1, st, l)] == OFF;
        match change {
            // A kick pickup.
            0 => {
                let st = [14, 10, 7, 11][rnd() % 4];
                if free(data, st, KICK) && free(data, st, SNARE) {
                    data[at(1, st, KICK)] = HIT;
                }
            }
            // The hats open on the last "and".
            1 => {
                if !free(data, 14, HAT) {
                    data[at(1, 14, HAT)] = OFF;
                    data[at(1, 14, OPEN)] = HIT;
                }
            }
            // A ghost note before the backbeat.
            2 => {
                let st = [3, 11, 9, 15][rnd() % 4];
                if free(data, st, SNARE) {
                    data[at(1, st, SNARE)] = GHOST;
                }
            }
            // A snare pickup into the next bar.
            3 => {
                if free(data, 15, SNARE) {
                    data[at(1, 15, SNARE)] = HIT;
                }
            }
            // One kick less (never the one).
            _ => {
                let kicks: Vec<usize> = (1..STEPS).filter(|&st| !free(data, st, KICK)).collect();
                if !kicks.is_empty() {
                    let st = kicks[rnd() % kicks.len()];
                    data[at(1, st, KICK)] = OFF;
                }
            }
        }
    }
}

/// The style the groove came from (for the editor).
pub fn style_of(data: &[u8]) -> Option<usize> {
    data.get(2).map(|s| *s as usize).filter(|s| *s < STYLES.len()).filter(|_| data.len() >= DATA_BYTES)
}

fn empty() -> Vec<u8> {
    let mut d = vec![0u8; DATA_BYTES];
    d[..2].copy_from_slice(&MAGIC);
    d[2] = u8::MAX;
    d
}

// ---------------------------------------------------------------- styles

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Flavor {
    /// Fills on the toms and the snare.
    Kit,
    /// Snare and clap builds, rolls.
    Electronic,
    /// Soft, sparse: snare and toms with ghosts under the ride.
    Jazz,
}

pub struct Style {
    pub name: &'static str,
    pub group: &'static str,
    pub swing: f32,
    /// Swing on the eighths (shuffle) instead of the sixteenths.
    pub eighths: bool,
    pub flavor: Flavor,
    /// Lane letter and two bars of steps: `.` off, `g` ghost, `x` hit,
    /// `X` accent, `r` roll. K kick, S snare, T side stick, C clap, H
    /// closed hat, P pedal hat, O open hat, R ride, Y crash, 1 2 3 toms
    /// (high, mid, floor), B tambourine, W cowbell.
    pub rows: &'static [(char, &'static str)],
}

pub const STYLE_GROUPS: &[&str] = &["Rock & Pop", "Funk & Soul", "Hip Hop", "Electronic", "World", "Jazz"];

pub static STYLES: &[Style] = &[
    Style { name: "Rock", group: "Rock & Pop", swing: 0.0, eighths: false, flavor: Flavor::Kit, rows: &[
        ('K', "x.......x.x.....x.......x.x...x."),
        ('S', "....X.......X.......X.......X..."),
        ('H', "x.x.x.x.x.x.x.x.x.x.x.x.x.x.x..."),
        ('O', "..............................x."),
    ]},
    Style { name: "Pop", group: "Rock & Pop", swing: 0.0, eighths: false, flavor: Flavor::Kit, rows: &[
        ('K', "x.......x.....x.x.......x.x....."),
        ('S', "....X.......X.......X.......X..."),
        ('H', "XgxgXgxgXgxgXgxgXgxgXgxgXgxgXgxg"),
        ('B', "....x.......x.......x.......x..."),
    ]},
    Style { name: "Half Time", group: "Rock & Pop", swing: 0.0, eighths: false, flavor: Flavor::Kit, rows: &[
        ('K', "x.........x.....x.....x...x....."),
        ('S', "........X...............X......g"),
        ('H', "x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x."),
    ]},
    Style { name: "Punk", group: "Rock & Pop", swing: 0.0, eighths: false, flavor: Flavor::Kit, rows: &[
        ('K', "x.x...x.x.x...x.x.x...x.x.x...x."),
        ('S', "....X.......X.......X.......X..."),
        ('H', "X.x.X.x.X.x.X.x.X.x.X.x.X.x.X.x."),
    ]},
    Style { name: "Metal", group: "Rock & Pop", swing: 0.0, eighths: false, flavor: Flavor::Kit, rows: &[
        ('K', "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"),
        ('S', "....X.......X.......X.......X..."),
        ('R', "X.x.X.x.X.x.X.x.X.x.X.x.X.x.X.x."),
        ('Y', "X...............X..............."),
    ]},
    Style { name: "Shuffle", group: "Rock & Pop", swing: 1.0, eighths: true, flavor: Flavor::Kit, rows: &[
        ('K', "x.......x.....x.x.......x......."),
        ('S', "....X.......X.......X.......X..."),
        ('H', "x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x."),
    ]},
    Style { name: "Synth Pop", group: "Rock & Pop", swing: 0.0, eighths: false, flavor: Flavor::Electronic, rows: &[
        ('K', "x...x...x...x...x...x...x...x..."),
        ('S', "....X.......X.......X.......X..."),
        ('C', "....x.......x.......x.......x..."),
        ('H', "xgxgxgxgxgxgxgxgxgxgxgxgxgxgxgxg"),
    ]},
    Style { name: "Funk", group: "Funk & Soul", swing: 0.15, eighths: false, flavor: Flavor::Kit, rows: &[
        ('K', "x.x.......x..x..x.x....x..x....."),
        ('S', "....X..g.g..X..g....X..g.g..X.g."),
        ('H', "XgxgXgxgXgxgXgxgXgxgXgxgXgxgXg.."),
        ('O', "..............................x."),
    ]},
    Style { name: "Disco", group: "Funk & Soul", swing: 0.0, eighths: false, flavor: Flavor::Kit, rows: &[
        ('K', "x...x...x...x...x...x...x...x..."),
        ('S', "....X.......X.......X.......X..."),
        ('H', "x.g.x.g.x.g.x.g.x.g.x.g.x.g.x.g."),
        ('O', "..x...x...x...x...x...x...x...x."),
    ]},
    Style { name: "Motown", group: "Funk & Soul", swing: 0.1, eighths: false, flavor: Flavor::Kit, rows: &[
        ('K', "x.....x.x.......x.....x.x...x..."),
        ('S', "....X.......X.......X.......X..."),
        ('H', "x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x."),
        ('B', "....X.......X.......X.......X..."),
    ]},
    Style { name: "Boom Bap", group: "Hip Hop", swing: 0.45, eighths: false, flavor: Flavor::Kit, rows: &[
        ('K', "x......x..x.....x.....x...x....."),
        ('S', "....X.......X.......X.......X..."),
        ('H', "x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x."),
        ('O', "..............x................."),
    ]},
    Style { name: "Lo-Fi", group: "Hip Hop", swing: 0.55, eighths: false, flavor: Flavor::Kit, rows: &[
        ('K', "x......x..x.....x.....x...x...g."),
        ('S', "....X.......X.......X.......X..."),
        ('T', ".........g.............g........"),
        ('H', "xgx.xgx.xgx.xgx.xgx.xgx.xgx.xg.."),
    ]},
    Style { name: "Trap", group: "Hip Hop", swing: 0.0, eighths: false, flavor: Flavor::Electronic, rows: &[
        ('K', "x......x..x.....x.....x.x......."),
        ('C', "........X...............X......."),
        ('H', "x.x.x.x.xrx.x.rxx.x.x.r.x.xrrrrr"),
        ('O', "..............................x."),
    ]},
    Style { name: "House", group: "Electronic", swing: 0.1, eighths: false, flavor: Flavor::Electronic, rows: &[
        ('K', "x...x...x...x...x...x...x...x..."),
        ('C', "....X.......X.......X.......X..."),
        ('H', "xg.gxg.gxg.gxg.gxg.gxg.gxg.gxg.g"),
        ('O', "..x...x...x...x...x...x...x...x."),
    ]},
    Style { name: "Techno", group: "Electronic", swing: 0.0, eighths: false, flavor: Flavor::Electronic, rows: &[
        ('K', "x...x...x...x...x...x...x...x..."),
        ('C', "....x.......x.......x.......x..."),
        ('T', ".......x......x........x......x."),
        ('H', "xgXgxgXgxgXgxgXgxgXgxgXgxgXgxgXg"),
    ]},
    Style { name: "Electro", group: "Electronic", swing: 0.0, eighths: false, flavor: Flavor::Electronic, rows: &[
        ('K', "x.....x...x.....x.....x.x..x...."),
        ('C', "....X.......X.......X.......X..."),
        ('H', "x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x."),
        ('W', "......x.......x.......x.......x."),
    ]},
    Style { name: "Drum & Bass", group: "Electronic", swing: 0.0, eighths: false, flavor: Flavor::Electronic, rows: &[
        ('K', "x.........x.....x.........x...x."),
        ('S', "....X..g....X.......X..g....X.g."),
        ('H', "x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x."),
    ]},
    Style { name: "Breakbeat", group: "Electronic", swing: 0.1, eighths: false, flavor: Flavor::Kit, rows: &[
        ('K', "x.x.......xx....x.x.......x....."),
        ('S', "....X..g.g..X..g....X..g.g....X."),
        ('R', "x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x."),
    ]},
    Style { name: "2-Step", group: "Electronic", swing: 0.35, eighths: false, flavor: Flavor::Electronic, rows: &[
        ('K', "x......x..x.....x.........x....."),
        ('S', "....X.......X.......X.......X..."),
        ('T', "...x......x.......x......x......"),
        ('H', "..x...x...x...x...x...x...x...x."),
    ]},
    Style { name: "Dubstep", group: "Electronic", swing: 0.15, eighths: false, flavor: Flavor::Electronic, rows: &[
        ('K', "x......x..........x...x........."),
        ('S', "........X...............X......."),
        ('C', "........x...............x......."),
        ('H', "x.x.x.x.x.x.x.x.x.x.x.x.x.x.xrx."),
    ]},
    Style { name: "Reggae", group: "World", swing: 0.4, eighths: true, flavor: Flavor::Kit, rows: &[
        ('K', "........X...............X......."),
        ('T', "........x...............x......."),
        ('H', "x.x.x.x.x.x.x.x.x.x.x.x.x.xxx.x."),
    ]},
    Style { name: "Bossa Nova", group: "World", swing: 0.0, eighths: false, flavor: Flavor::Jazz, rows: &[
        ('K', "x..xx..xx..xx..xx..xx..xx..xx..x"),
        ('T', "x..x..x...x..x....x..x...x..x..."),
        ('H', "x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x."),
    ]},
    Style { name: "Samba", group: "World", swing: 0.1, eighths: false, flavor: Flavor::Kit, rows: &[
        ('K', "x..xx..xx..xx..xx..xx..xx..xx..x"),
        ('S', "g.g..x.g.g..x.g.g.g..x.g.g..x.g."),
        ('H', "XggxXggxXggxXggxXggxXggxXggxXggx"),
        ('B', "xgxgxgxgxgxgxgxgxgxgxgxgxgxgxgxg"),
    ]},
    Style { name: "Afrobeat", group: "World", swing: 0.1, eighths: false, flavor: Flavor::Kit, rows: &[
        ('K', "x.....x...x..x..x.....x...x..x.."),
        ('S', "....X.g..g..X.......X.g..g..X..."),
        ('H', "x.xxx.xxx.xxx.xxx.xxx.xxx.xxx.xx"),
        ('W', "x.x.xx.x.x.xx.x.x.x.xx.x.x.xx.x."),
    ]},
    Style { name: "Reggaeton", group: "World", swing: 0.0, eighths: false, flavor: Flavor::Electronic, rows: &[
        ('K', "x...x...x...x...x...x...x...x..."),
        ('S', "...X..x....X..x....X..x....X..x."),
        ('H', "x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x."),
    ]},
    Style { name: "Jazz Swing", group: "Jazz", swing: 1.0, eighths: true, flavor: Flavor::Jazz, rows: &[
        ('K', "g...g...g...g...g...g...g...g..."),
        ('S', "......g...........g.........g.g."),
        ('P', "....x.......x.......x.......x..."),
        ('R', "X...x.x.X...x.x.X...x.x.X...x.x."),
    ]},
    Style { name: "Jazz Ballad", group: "Jazz", swing: 0.8, eighths: true, flavor: Flavor::Jazz, rows: &[
        ('K', "g.......g.......g.......g......."),
        ('T', "............g...............g..."),
        ('P', "....x.......x.......x.......x..."),
        ('R', "x...g.g.x...g.g.x...g.g.x...g.g."),
    ]},
];

fn lane_of(c: char) -> Option<usize> {
    Some(match c {
        'K' => KICK,
        'S' => SNARE,
        'T' => STICK,
        'C' => CLAP,
        'H' => HAT,
        'P' => PEDAL,
        'O' => OPEN,
        'R' => RIDE,
        'Y' => CRASH_LANE,
        '1' => TOM_HI,
        '2' => TOM_MID,
        '3' => TOM_FLOOR,
        'B' => TAMB,
        'W' => BELL,
        _ => return None,
    })
}

/// The groove of style `s`.
pub fn style_data(s: usize) -> Vec<u8> {
    let mut d = empty();
    let Some(style) = STYLES.get(s) else { return d };
    d[2] = s as u8;
    for &(c, steps) in style.rows {
        let Some(lane) = lane_of(c) else { continue };
        for (i, ch) in steps.chars().filter(|c| *c != '|').take(2 * STEPS).enumerate() {
            let v = match ch {
                'g' => GHOST,
                'x' => HIT,
                'X' => ACCENT,
                'r' => ROLL,
                _ => OFF,
            };
            d[at(i / STEPS, i % STEPS, lane)] = v;
        }
    }
    d
}

pub fn drummer_default() -> Vec<u8> {
    style_data(0)
}

fn flavor_of(data: &[u8]) -> Flavor {
    style_of(data).map(|s| STYLES[s].flavor).unwrap_or(Flavor::Kit)
}

// ---------------------------------------------------------------- playing

/// What the drummer remembers between steps.
#[derive(Clone, Copy, Default)]
pub struct DrummerState {
    /// The fill being played: its bar and which kind.
    fill_bar: i64,
    fill_kind: u8,
}

impl SeqState {
    /// Runs on a 1/16 grid.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn drummer(&mut self, k: i64, p: &[f32], data: &[u8], at: u64, step_frames: f64, out: &mut Vec<MidiEvent>, ctx: &MidiCtx) {
        let bar_no = k.div_euclid(STEPS as i64);
        let step = k.rem_euclid(STEPS as i64) as usize;
        let bar = match p[BARS].round() as i32 {
            0 => 0,
            1 => bar_no.rem_euclid(2) as usize,
            _ => usize::from(bar_no.rem_euclid(4) == 3),
        };
        let complexity = p[COMPLEXITY].clamp(0.0, 1.0);
        let energy = p[ENERGY].clamp(0.0, 1.0);
        let human = p[HUMANIZE].clamp(0.0, 1.0);
        let flavor = flavor_of(data);
        let every = [0i64, 2, 4, 8][(p[FILLS].round().max(0.0) as usize).min(3)];
        let fill_len = [4usize, 8, 16][(p[FILL_LENGTH].round().max(0.0) as usize).min(2)];
        let in_fill = every > 0 && bar_no.rem_euclid(every) == every - 1 && step >= STEPS - fill_len;
        let after_fill = every > 0 && bar_no > 0 && bar_no.rem_euclid(every) == 0 && step == 0;
        if in_fill && self.drum.fill_bar != bar_no {
            self.drum.fill_bar = bar_no;
            self.drum.fill_kind = (self.unit() * 4.0) as u8;
        }

        // What each lane plays this step: (lane, level), groove first.
        let mut hits: [u8; LANES] = std::array::from_fn(|l| cell(data, bar, step, l));
        // Energy: low moves the backbeat to the side stick, high the hats to the ride.
        if energy < 0.2 && flavor != Flavor::Electronic && hits[STICK] == OFF && hits[SNARE] >= HIT {
            hits[STICK] = hits[SNARE];
            hits[SNARE] = OFF;
        }
        if energy > 0.8 && flavor == Flavor::Kit && hits[RIDE] == OFF && hits[HAT] != OFF {
            hits[RIDE] = hits[HAT];
            hits[HAT] = OFF;
        }
        // Complexity: what a drummer adds on the way.
        let odd = step % 2 == 1;
        let snare_near = |s: usize| [s.wrapping_sub(1), s, s + 1].iter().any(|&j| j < STEPS && cell(data, bar, j, SNARE) >= HIT);
        if flavor != Flavor::Electronic && odd && hits[SNARE] == OFF && !snare_near(step) && self.unit() < 0.3 * complexity {
            hits[SNARE] = GHOST;
        }
        let hats_on_eighths = (0..STEPS).step_by(2).all(|j| cell(data, bar, j, HAT) != OFF) && (1..STEPS).step_by(2).all(|j| cell(data, bar, j, HAT) == OFF);
        if hats_on_eighths && odd && hits[HAT] == OFF && self.unit() < 0.55 * complexity {
            hits[HAT] = GHOST;
        }
        if matches!(step, 7 | 10 | 14) && hits[KICK] == OFF && hits[SNARE] < HIT && self.unit() < 0.22 * complexity {
            hits[KICK] = if step == 14 { HIT } else { GHOST };
        }
        let last_of_phrase = every == 0 || bar_no.rem_euclid(every) == every - 1;
        if step == 14 && last_of_phrase && hits[HAT] != OFF && hits[OPEN] == OFF && self.unit() < 0.5 * complexity {
            hits[OPEN] = HIT;
            hits[HAT] = OFF;
        }
        // The fill takes over the hands (and keeps the kick on the beats).
        if in_fill {
            let j = step - (STEPS - fill_len);
            let fill = fill_hits(flavor, self.drum.fill_kind, j, fill_len);
            for l in [SNARE, STICK, CLAP, HAT, OPEN, RIDE, TOM_HI, TOM_MID, TOM_FLOOR, TAMB] {
                hits[l] = OFF;
            }
            for (l, v) in fill.into_iter().flatten() {
                hits[l] = v;
            }
            if step % 4 != 0 && flavor != Flavor::Electronic {
                hits[KICK] = OFF;
            }
        }
        // A crash after the fill (the hand that plays it leaves the hats).
        if after_fill && p[CRASH] >= 0.5 {
            hits[CRASH_LANE] = ACCENT;
            hits[KICK] = hits[KICK].max(HIT);
            hits[HAT] = OFF;
            hits[RIDE] = OFF;
        }

        // Play them.
        let swing = p[SWING].clamp(0.0, 1.0) as f64;
        let delay = if p[SWING_ON] >= 0.5 {
            // Shuffle: the off-beat eighth moves towards the last triplet.
            match step % 4 {
                2 => swing * 2.0 / 3.0,
                1 | 3 => swing / 3.0,
                _ => 0.0,
            }
        } else if odd {
            swing * 0.5
        } else {
            0.0
        };
        let base = at as f64 + delay * step_frames;
        let accent_of_step = if step % 4 == 0 { 1.0 } else if step % 2 == 0 { 0.92 } else { 0.85 };
        let mut shown = 0u64;
        for (lane, &v) in hits.iter().enumerate() {
            if v == OFF {
                continue;
            }
            let Some(note) = lane_note(lane, ctx.drum_notes) else { continue };
            shown |= 1 << lane;
            let level = match v {
                GHOST => 0.32,
                ACCENT => 1.0,
                _ => 0.74,
            };
            // Energy widens the dynamics; a side stick that falls back on the snare is played softly.
            let fallback = if note != LANE_NOTES[lane][0] && lane == STICK { 0.55 } else { 1.0 };
            let dyn_ = if v == GHOST { 1.0 } else { 0.7 + 0.45 * energy };
            let vel = p[VELOCITY] * level * dyn_ * fallback * (accent_of_step + (1.0 - accent_of_step) * (lane == KICK || lane == SNARE) as u8 as f32);
            let vel = vel * (1.0 + 0.16 * human * (self.unit() * 2.0 - 1.0));
            // Hands drift more than feet; the backbeat sits a touch late.
            let spread = match lane {
                KICK => 0.004,
                SNARE | CLAP | STICK => 0.006,
                _ => 0.008,
            };
            let late = if matches!(lane, SNARE | CLAP) && v >= HIT { 0.003 * human as f64 } else { 0.0 };
            let jitter = human as f64 * spread * ctx.sr as f64 * (self.unit() as f64 + self.unit() as f64 - 1.0) + late * ctx.sr as f64;
            let when = (base + jitter).max(at as f64) as u64;
            let len = (step_frames * 0.5).max(16.0);
            self.play(ctx, when, note as i32, vel.clamp(0.03, 1.0), len, out);
            if v == ROLL {
                // The second thirty-second, a little softer.
                let half = (step_frames * 0.5) as u64;
                self.play(ctx, when + half, note as i32, (vel * 0.8).clamp(0.03, 1.0), len * 0.5, out);
            }
        }
        self.visual[0] = step as u64 | (bar as u64) << 8 | (in_fill as u64) << 9 | (bar_no.max(0) as u64 & 0xffff) << 16;
        self.visual[1] = shown;
    }
}

/// Step `j` of a fill of `n` steps: up to two (lane, level) hits.
fn fill_hits(flavor: Flavor, kind: u8, j: usize, n: usize) -> [Option<(usize, u8)>; 2] {
    let t = j as f32 / n.max(1) as f32;
    let rising = if t < 0.34 { HIT } else { ACCENT };
    match flavor {
        Flavor::Electronic => match kind % 2 {
            // A snare (or clap) build: eighths, then sixteenths, then rolls.
            0 => {
                let v = if t > 0.75 { ROLL } else if t > 0.4 || j % 2 == 0 { rising } else { OFF };
                [Some((SNARE, v)), (j + 1 == n).then_some((CLAP, ACCENT))]
            }
            _ => [Some((CLAP, if j % 2 == 0 || t > 0.5 { rising } else { GHOST })), (t > 0.5).then_some((SNARE, HIT))],
        },
        Flavor::Jazz => {
            // Sparse and soft: a snare comment, a tom answer.
            let v = match (j % 3, kind % 2) {
                (0, _) => Some((SNARE, if j + 1 >= n { HIT } else { GHOST })),
                (2, 0) => Some((TOM_MID, GHOST)),
                (1, 1) => Some((TOM_FLOOR, GHOST)),
                _ => None,
            };
            [v, None]
        }
        Flavor::Kit => match kind % 4 {
            // Down the toms: snare, high, mid, floor.
            0 => {
                let drum = [SNARE, TOM_HI, TOM_MID, TOM_FLOOR][((t * 4.0) as usize).min(3)];
                [Some((drum, rising)), None]
            }
            // The snare, crescendo, with a tom at the end.
            1 => {
                let drum = if j + 2 >= n { TOM_FLOOR } else { SNARE };
                [Some((drum, if j % 2 == 0 { rising } else { HIT.min(rising) })), None]
            }
            // Snare and toms in pairs, the kick under the last one.
            2 => {
                let drum = [SNARE, SNARE, TOM_HI, TOM_HI, TOM_MID, TOM_MID, TOM_FLOOR, TOM_FLOOR][(j * 8 / n.max(1)).min(7)];
                [Some((drum, rising)), (j + 1 == n).then_some((KICK, ACCENT))]
            }
            // Syncopated: snare and floor tom on the eighths, the high tom between.
            _ => {
                let drum = if j % 4 == 0 { SNARE } else if j % 4 == 2 { TOM_FLOOR } else { TOM_HI };
                let v = if j % 2 == 1 { GHOST } else { rising };
                [Some((drum, v)), None]
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::{Scale, ScaleKind};

    fn ctx(now: u64, kit: u128) -> MidiCtx {
        MidiCtx { now, len: 256, samples_per_beat: 24000.0, scale: Scale::new(0, ScaleKind::Major), scale_on: true, beat: now as f64 / 24000.0, playing: true, sr: 48000.0, drum_notes: kit, lead: 0.0 }
    }

    /// The notes played over `bars` bars: (frame, note, velocity).
    fn run(p: &[f32], data: &[u8], kit: u128, bars: u64) -> Vec<(u64, u8, f32)> {
        let mut s = SeqState::new();
        let mut notes = Vec::new();
        let mut now = 0;
        while now < bars * 4 * 24000 {
            let mut out = Vec::new();
            s.process(crate::midi_fx::MidiFxKind::Drummer, p, data, &mut out, &ctx(now, kit));
            notes.extend(out.iter().filter(|e| e.on).map(|e| (now + e.offset as u64, e.pitch, e.vel)));
            now += 256;
        }
        notes
    }

    /// Within a frame (step times are floored to frames).
    fn near(a: u64, b: u64) -> bool {
        a.abs_diff(b) <= 1
    }

    fn plain() -> Vec<f32> {
        let mut p = crate::params::defaults(SPECS);
        p[COMPLEXITY] = 0.0;
        p[HUMANIZE] = 0.0;
        p[FILLS] = 0.0;
        p
    }

    #[test]
    fn every_style_reads_and_has_a_kick_or_a_ride() {
        for (i, s) in STYLES.iter().enumerate() {
            for &(c, steps) in s.rows {
                assert!(lane_of(c).is_some(), "{}: lane {c}", s.name);
                assert_eq!(steps.chars().count(), 32, "{}: lane {c}", s.name);
                assert!(steps.chars().all(|ch| ".gxXr".contains(ch)), "{}: lane {c}", s.name);
            }
            assert!(STYLE_GROUPS.contains(&s.group), "{}", s.name);
            let d = style_data(i);
            assert_eq!(style_of(&d), Some(i));
            assert!((0..STEPS).any(|j| cell(&d, 0, j, KICK) != OFF || cell(&d, 0, j, RIDE) != OFF), "{}", s.name);
        }
    }

    #[test]
    fn the_rate_plays_double_and_half_time() {
        let mut p = plain();
        let hats = |p: &[f32]| run(p, &style_data(0), 0, 2).iter().filter(|n| n.1 == 42).count();
        let normal = hats(&p);
        // Twice the groove in the same time, or half of it (an open hat here and there).
        p[RATE] = 0.0; // 1/32
        assert!(hats(&p).abs_diff(normal * 2) <= 2, "{} {normal}", hats(&p));
        p[RATE] = 4.0; // 1/8
        assert!(hats(&p).abs_diff(normal / 2) <= 1, "{} {normal}", hats(&p));
        // Older sets without the rate play sixteenths, and Transpose stays where it was.
        assert_eq!(crate::sequencers::keys_params(crate::midi_fx::MidiFxKind::Drummer), (10, 11));
    }

    #[test]
    fn rock_plays_its_groove_on_the_grid() {
        let notes = run(&plain(), &style_data(0), 0, 2);
        let step = 6000u64;
        // Kick on 1, snare on 2, hats on the eighths.
        assert!(notes.iter().any(|n| n.0 == 0 && n.1 == 36));
        assert!(notes.iter().any(|n| near(n.0, 4 * step) && n.1 == 38));
        assert_eq!(notes.iter().filter(|n| n.1 == 42 && n.0 < 16 * step).count(), 8);
        // Accents are louder than plain hits.
        let snare = notes.iter().find(|n| n.1 == 38).unwrap().2;
        let hat = notes.iter().find(|n| n.1 == 42).unwrap().2;
        assert!(snare > hat, "{snare} {hat}");
    }

    #[test]
    fn a_fill_ends_the_phrase_and_a_crash_follows() {
        let mut p = plain();
        p[FILLS] = 1.0; // every 2 bars
        p[FILL_LENGTH] = 1.0; // 2 beats
        let notes = run(&p, &style_data(0), 0, 3);
        let step = 6000u64;
        // The second half of bar 2 is a fill: toms or snare, no hats.
        let fill: Vec<_> = notes.iter().filter(|n| n.0 >= 24 * step && n.0 < 32 * step).collect();
        assert!(fill.iter().any(|n| [38, 50, 47, 43].contains(&n.1)), "{fill:?}");
        assert!(!fill.iter().any(|n| n.1 == 42));
        // Bar 3 opens with a crash and the kick.
        assert!(notes.iter().any(|n| near(n.0, 32 * step) && n.1 == 49));
        assert!(notes.iter().any(|n| near(n.0, 32 * step) && n.1 == 36));
    }

    #[test]
    fn it_plays_the_kit_it_has() {
        // A kit of eight: no ride, no crash, one tom.
        let kit: u128 = [36u8, 37, 38, 39, 42, 45, 46, 56].iter().fold(0, |m, n| m | 1u128 << n);
        let mut p = plain();
        p[FILLS] = 1.0;
        p[FILL_LENGTH] = 2.0;
        let metal = STYLES.iter().position(|s| s.name == "Metal").unwrap();
        let notes = run(&p, &style_data(metal), kit, 3);
        assert!(notes.iter().all(|n| kit >> n.1 & 1 == 1), "only notes of the kit");
        // The ride falls back to the closed hat, the crash to the open hat, toms to the tom.
        assert!(notes.iter().any(|n| n.1 == 42) && notes.iter().any(|n| n.1 == 46) && notes.iter().any(|n| n.1 == 45));
    }

    #[test]
    fn humanize_and_swing_move_the_notes() {
        let mut p = plain();
        p[SWING] = 1.0;
        let notes = run(&p, &style_data(0), 0, 1);
        // The off-beat sixteenths are not used by Rock; the off-beat eighths stay put with 16th swing.
        assert!(notes.iter().filter(|n| n.1 == 42).all(|n| near(n.0, (n.0 + 6000) / 12000 * 12000)));
        p[SWING_ON] = 1.0;
        let notes = run(&p, &style_data(0), 0, 1);
        // Shuffle: the "and" moves two thirds of a sixteenth later.
        assert!(notes.iter().any(|n| n.1 == 42 && near(n.0, 12000 + 4000)));
        let mut p = plain();
        p[HUMANIZE] = 1.0;
        let notes = run(&p, &style_data(0), 0, 2);
        assert!(notes.iter().filter(|n| n.1 == 42).any(|n| !near(n.0, (n.0 + 3000) / 6000 * 6000)));
    }

    #[test]
    fn complexity_adds_ghost_notes_and_energy_moves_the_backbeat() {
        let mut p = plain();
        p[COMPLEXITY] = 1.0;
        let busy = run(&p, &style_data(0), 0, 8);
        let calm = run(&plain(), &style_data(0), 0, 8);
        assert!(busy.len() > calm.len() + 10, "{} vs {}", busy.len(), calm.len());
        let mut p = plain();
        p[ENERGY] = 0.0;
        let soft = run(&p, &style_data(0), 0, 1);
        assert!(soft.iter().any(|n| n.1 == 37) && !soft.iter().any(|n| n.1 == 38));
        p[ENERGY] = 1.0;
        let loud = run(&p, &style_data(0), 0, 1);
        assert!(loud.iter().any(|n| n.1 == 51) && !loud.iter().any(|n| n.1 == 42));
    }

    #[test]
    fn vary_keeps_bar_a_and_changes_bar_b_a_little() {
        let mut d = style_data(0);
        let a: Vec<u8> = (0..STEPS).flat_map(|s| (0..LANES).map(move |l| (s, l))).map(|(s, l)| cell(&d, 0, s, l)).collect();
        for seed in 1..20 {
            vary(&mut d, seed);
            let a2: Vec<u8> = (0..STEPS).flat_map(|s| (0..LANES).map(move |l| (s, l))).map(|(s, l)| cell(&d, 0, s, l)).collect();
            assert_eq!(a, a2);
            let diff = (0..STEPS).flat_map(|s| (0..LANES).map(move |l| (s, l))).filter(|&(s, l)| cell(&d, 0, s, l) != cell(&d, 1, s, l)).count();
            assert!(diff <= 4, "{diff}");
        }
    }

    #[test]
    fn rolls_play_two_thirty_seconds() {
        let trap = STYLES.iter().position(|s| s.name == "Trap").unwrap();
        let notes = run(&plain(), &style_data(trap), 0, 1);
        // Step 9 of bar A is a roll: hats at 9 and 9.5 sixteenths.
        assert!(notes.iter().any(|n| n.1 == 42 && near(n.0, 9 * 6000)) && notes.iter().any(|n| n.1 == 42 && near(n.0, 9 * 6000 + 3000)));
    }
}
