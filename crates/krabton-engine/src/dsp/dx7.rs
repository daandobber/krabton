//! DX7-compatible voice data: the 155-byte VCED layout, 32-voice bank
//! (VMEM) packing, sysex import/export, the 32 algorithms and Krabton's own
//! factory voices. Written from the public DX7 documentation.

/// Bytes per operator in the unpacked (VCED) layout.
pub const OP_BYTES: usize = 21;
/// Size of an unpacked voice.
pub const VOICE_BYTES: usize = 155;

// Per-operator VCED offsets.
pub const R1: usize = 0;
pub const L1: usize = 4;
pub const BP: usize = 8;
pub const LD: usize = 9;
pub const RD: usize = 10;
pub const LC: usize = 11;
pub const RC: usize = 12;
pub const RS: usize = 13;
pub const AMS: usize = 14;
pub const KVS: usize = 15;
pub const OL: usize = 16;
pub const PM: usize = 17;
pub const PC: usize = 18;
pub const PF: usize = 19;
pub const PD: usize = 20;

// Global VCED offsets.
pub const PR1: usize = 126;
pub const PL1: usize = 130;
pub const ALG: usize = 134;
pub const FB: usize = 135;
pub const OKS: usize = 136;
pub const LFS: usize = 137;
pub const LFD: usize = 138;
pub const LPMD: usize = 139;
pub const LAMD: usize = 140;
pub const LFKS: usize = 141;
pub const LFW: usize = 142;
pub const LPMS: usize = 143;
pub const TRNP: usize = 144;
pub const NAME: usize = 145;

const OP_MAX: [u8; OP_BYTES] = [99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 3, 3, 7, 3, 7, 99, 1, 31, 99, 14];

/// How the six operators are wired.
#[derive(Clone, Copy, Debug)]
pub struct Algorithm {
    /// For each operator (index 0 = OP1), a bitmask of the operators that modulate it.
    pub mods: [u8; 6],
    /// Bitmask of carriers (operators that reach the output).
    pub carriers: u8,
    /// Feedback: the output of `fb_from` is fed into the phase of `fb_to`.
    pub fb_from: usize,
    pub fb_to: usize,
}

const fn b(ops: &[usize]) -> u8 {
    let mut m = 0u8;
    let mut i = 0;
    while i < ops.len() {
        m |= 1 << (ops[i] - 1);
        i += 1;
    }
    m
}

/// `mods[op-1]` lists modulators of each op (1-based), carriers 1-based, feedback (from, to) 1-based.
const fn alg(mods: [&[usize]; 6], carriers: &[usize], fb: (usize, usize)) -> Algorithm {
    Algorithm {
        mods: [b(mods[0]), b(mods[1]), b(mods[2]), b(mods[3]), b(mods[4]), b(mods[5])],
        carriers: b(carriers),
        fb_from: fb.0 - 1,
        fb_to: fb.1 - 1,
    }
}

pub static ALGORITHMS: [Algorithm; 32] = [
    alg([&[2], &[], &[4], &[5], &[6], &[]], &[1, 3], (6, 6)),
    alg([&[2], &[], &[4], &[5], &[6], &[]], &[1, 3], (2, 2)),
    alg([&[2], &[3], &[], &[5], &[6], &[]], &[1, 4], (6, 6)),
    alg([&[2], &[3], &[], &[5], &[6], &[]], &[1, 4], (4, 6)),
    alg([&[2], &[], &[4], &[], &[6], &[]], &[1, 3, 5], (6, 6)),
    alg([&[2], &[], &[4], &[], &[6], &[]], &[1, 3, 5], (5, 6)),
    alg([&[2], &[], &[4, 5], &[], &[6], &[]], &[1, 3], (6, 6)),
    alg([&[2], &[], &[4, 5], &[], &[6], &[]], &[1, 3], (4, 4)),
    alg([&[2], &[], &[4, 5], &[], &[6], &[]], &[1, 3], (2, 2)),
    alg([&[2], &[3], &[], &[5, 6], &[], &[]], &[1, 4], (3, 3)),
    alg([&[2], &[3], &[], &[5, 6], &[], &[]], &[1, 4], (6, 6)),
    alg([&[2], &[], &[4, 5, 6], &[], &[], &[]], &[1, 3], (2, 2)),
    alg([&[2], &[], &[4, 5, 6], &[], &[], &[]], &[1, 3], (6, 6)),
    alg([&[2], &[], &[4], &[5, 6], &[], &[]], &[1, 3], (6, 6)),
    alg([&[2], &[], &[4], &[5, 6], &[], &[]], &[1, 3], (2, 2)),
    alg([&[2, 3, 5], &[], &[4], &[], &[6], &[]], &[1], (6, 6)),
    alg([&[2, 3, 5], &[], &[4], &[], &[6], &[]], &[1], (2, 2)),
    alg([&[2, 3, 4], &[], &[], &[5], &[6], &[]], &[1], (3, 3)),
    alg([&[2], &[3], &[], &[6], &[6], &[]], &[1, 4, 5], (6, 6)),
    alg([&[3], &[3], &[], &[5, 6], &[], &[]], &[1, 2, 4], (3, 3)),
    alg([&[3], &[3], &[], &[6], &[6], &[]], &[1, 2, 4, 5], (3, 3)),
    alg([&[2], &[], &[6], &[6], &[6], &[]], &[1, 3, 4, 5], (6, 6)),
    alg([&[], &[3], &[], &[6], &[6], &[]], &[1, 2, 4, 5], (6, 6)),
    alg([&[], &[], &[6], &[6], &[6], &[]], &[1, 2, 3, 4, 5], (6, 6)),
    alg([&[], &[], &[], &[6], &[6], &[]], &[1, 2, 3, 4, 5], (6, 6)),
    alg([&[], &[3], &[], &[5, 6], &[], &[]], &[1, 2, 4], (6, 6)),
    alg([&[], &[3], &[], &[5, 6], &[], &[]], &[1, 2, 4], (3, 3)),
    alg([&[2], &[], &[4], &[5], &[], &[]], &[1, 3, 6], (5, 5)),
    alg([&[], &[], &[4], &[], &[6], &[]], &[1, 2, 3, 5], (6, 6)),
    alg([&[], &[], &[4], &[5], &[], &[]], &[1, 2, 3, 6], (5, 5)),
    alg([&[], &[], &[], &[], &[6], &[]], &[1, 2, 3, 4, 5], (6, 6)),
    alg([&[], &[], &[], &[], &[], &[]], &[1, 2, 3, 4, 5, 6], (6, 6)),
];

/// An unpacked DX7 voice (VCED order: OP6 first, then globals and name).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dx7Voice(pub [u8; VOICE_BYTES]);

impl Default for Dx7Voice {
    fn default() -> Self {
        init_voice()
    }
}

impl Dx7Voice {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        let arr: [u8; VOICE_BYTES] = data.get(..VOICE_BYTES)?.try_into().ok()?;
        let mut v = Self(arr);
        v.sanitize();
        Some(v)
    }

    /// Byte offset of an operator parameter; `op` 0 = OP1 ... 5 = OP6.
    pub fn op_offset(op: usize, param: usize) -> usize {
        (5 - op.min(5)) * OP_BYTES + param
    }

    pub fn op(&self, op: usize, param: usize) -> u8 {
        self.0[Self::op_offset(op, param)]
    }

    pub fn set_op(&mut self, op: usize, param: usize, value: u8) {
        self.0[Self::op_offset(op, param)] = value.min(OP_MAX[param]);
    }

    pub fn get(&self, index: usize) -> u8 {
        self.0[index]
    }

    pub fn set(&mut self, index: usize, value: u8) {
        self.0[index] = value;
        self.sanitize();
    }

    pub fn algorithm(&self) -> usize {
        (self.0[ALG] as usize).min(31)
    }

    pub fn name(&self) -> String {
        self.0[NAME..NAME + 10].iter().map(|&c| if (32..127).contains(&c) { c as char } else { ' ' }).collect::<String>().trim_end().to_string()
    }

    pub fn set_name(&mut self, name: &str) {
        let bytes: Vec<u8> = name.bytes().filter(|c| (32..127).contains(c)).chain(std::iter::repeat(b' ')).take(10).collect();
        self.0[NAME..NAME + 10].copy_from_slice(&bytes);
    }

    /// Clamps every parameter into its legal range.
    pub fn sanitize(&mut self) {
        for op in 0..6 {
            for p in 0..OP_BYTES {
                let i = op * OP_BYTES + p;
                self.0[i] = self.0[i].min(OP_MAX[p]);
            }
        }
        let max: [(usize, u8); 13] = [
            (ALG, 31),
            (FB, 7),
            (OKS, 1),
            (LFS, 99),
            (LFD, 99),
            (LPMD, 99),
            (LAMD, 99),
            (LFKS, 1),
            (LFW, 5),
            (LPMS, 7),
            (TRNP, 48),
            (PR1, 99),
            (PL1, 99),
        ];
        for (i, m) in max {
            self.0[i] = self.0[i].min(m);
        }
        for i in 0..4 {
            self.0[PR1 + i] = self.0[PR1 + i].min(99);
            self.0[PL1 + i] = self.0[PL1 + i].min(99);
        }
        for i in NAME..NAME + 10 {
            if !(32..127).contains(&self.0[i]) {
                self.0[i] = b' ';
            }
        }
    }

    /// Packs into the 128-byte bank (VMEM) format.
    pub fn pack(&self) -> [u8; 128] {
        let v = &self.0;
        let mut out = [0u8; 128];
        for k in 0..6 {
            let s = k * OP_BYTES;
            let d = k * 17;
            out[d..d + 11].copy_from_slice(&v[s..s + 11]);
            out[d + 11] = (v[s + RC] << 2) | v[s + LC];
            out[d + 12] = (v[s + PD] << 3) | v[s + RS];
            out[d + 13] = (v[s + KVS] << 2) | v[s + AMS];
            out[d + 14] = v[s + OL];
            out[d + 15] = (v[s + PC] << 1) | v[s + PM];
            out[d + 16] = v[s + PF];
        }
        out[102..110].copy_from_slice(&v[PR1..PR1 + 8]);
        out[110] = v[ALG];
        out[111] = (v[OKS] << 3) | v[FB];
        out[112] = v[LFS];
        out[113] = v[LFD];
        out[114] = v[LPMD];
        out[115] = v[LAMD];
        out[116] = (v[LPMS] << 4) | (v[LFW] << 1) | v[LFKS];
        out[117] = v[TRNP];
        out[118..128].copy_from_slice(&v[NAME..NAME + 10]);
        out
    }

    /// Unpacks a 128-byte bank (VMEM) voice.
    pub fn unpack(p: &[u8]) -> Self {
        let mut v = [0u8; VOICE_BYTES];
        for k in 0..6 {
            let s = k * 17;
            let d = k * OP_BYTES;
            v[d..d + 11].copy_from_slice(&p[s..s + 11]);
            v[d + LC] = p[s + 11] & 3;
            v[d + RC] = (p[s + 11] >> 2) & 3;
            v[d + RS] = p[s + 12] & 7;
            v[d + PD] = (p[s + 12] >> 3) & 15;
            v[d + AMS] = p[s + 13] & 3;
            v[d + KVS] = (p[s + 13] >> 2) & 7;
            v[d + OL] = p[s + 14];
            v[d + PM] = p[s + 15] & 1;
            v[d + PC] = (p[s + 15] >> 1) & 31;
            v[d + PF] = p[s + 16];
        }
        v[PR1..PR1 + 8].copy_from_slice(&p[102..110]);
        v[ALG] = p[110] & 31;
        v[FB] = p[111] & 7;
        v[OKS] = (p[111] >> 3) & 1;
        v[LFS] = p[112];
        v[LFD] = p[113];
        v[LPMD] = p[114];
        v[LAMD] = p[115];
        v[LFKS] = p[116] & 1;
        v[LFW] = (p[116] >> 1) & 7;
        v[LPMS] = (p[116] >> 4) & 7;
        v[TRNP] = p[117];
        v[NAME..NAME + 10].copy_from_slice(&p[118..128]);
        let mut voice = Self(v);
        voice.sanitize();
        voice
    }
}

fn checksum(data: &[u8]) -> u8 {
    let sum: u32 = data.iter().map(|&b| b as u32).sum();
    ((128 - (sum & 127)) & 127) as u8
}

/// Reads a DX7 sysex file: a 32-voice bank, a single voice, or raw dumps
/// without sysex headers. Checksum errors are tolerated (many files online
/// have them).
pub fn parse_sysex(data: &[u8]) -> Result<Vec<Dx7Voice>, String> {
    // Bank: F0 43 0n 09 20 00 <4096> cs F7
    if data.len() >= 4102 && data[0] == 0xF0 && data[1] == 0x43 && data[3] == 9 {
        let body = &data[6..6 + 4096];
        return Ok(body.chunks(128).map(Dx7Voice::unpack).collect());
    }
    // Single voice: F0 43 0n 00 01 1B <155> cs F7
    if data.len() >= 161 && data[0] == 0xF0 && data[1] == 0x43 && data[3] == 0 {
        return Dx7Voice::from_slice(&data[6..6 + VOICE_BYTES]).map(|v| vec![v]).ok_or_else(|| "truncated voice".into());
    }
    if data.len() == 4096 {
        return Ok(data.chunks(128).map(Dx7Voice::unpack).collect());
    }
    if data.len() == VOICE_BYTES || data.len() == VOICE_BYTES + 1 {
        return Dx7Voice::from_slice(data).map(|v| vec![v]).ok_or_else(|| "truncated voice".into());
    }
    // Some files contain several banks or extra bytes: look for a bank header.
    if let Some(pos) = data.windows(6).position(|w| w[0] == 0xF0 && w[1] == 0x43 && w[3] == 9 && w[4] == 0x20) {
        if data.len() >= pos + 6 + 4096 {
            let body = &data[pos + 6..pos + 6 + 4096];
            return Ok(body.chunks(128).map(Dx7Voice::unpack).collect());
        }
    }
    Err(format!("not a DX7 voice or bank ({} bytes)", data.len()))
}

/// A single-voice sysex dump (163 bytes).
pub fn single_voice_sysex(v: &Dx7Voice) -> Vec<u8> {
    let mut out = vec![0xF0, 0x43, 0x00, 0x00, 0x01, 0x1B];
    out.extend_from_slice(&v.0);
    out.push(checksum(&v.0));
    out.push(0xF7);
    out
}

/// A 32-voice bank sysex dump (4104 bytes). Missing voices are INIT VOICE.
pub fn bank_sysex(voices: &[Dx7Voice]) -> Vec<u8> {
    let mut body = Vec::with_capacity(4096);
    for i in 0..32 {
        let v = voices.get(i).cloned().unwrap_or_default();
        body.extend_from_slice(&v.pack());
    }
    let mut out = vec![0xF0, 0x43, 0x00, 0x09, 0x20, 0x00];
    out.extend_from_slice(&body);
    out.push(checksum(&body));
    out.push(0xF7);
    out
}

/// One operator of a voice we design ourselves.
#[derive(Clone, Copy)]
struct OpDef {
    r: [u8; 4],
    l: [u8; 4],
    ol: u8,
    coarse: u8,
    fine: u8,
    detune: u8,
    fixed: bool,
    kvs: u8,
    rs: u8,
    ams: u8,
    /// Keyboard level scaling: breakpoint, left/right depth, left/right curve.
    ks: (u8, u8, u8, u8, u8),
}

const fn op(r: [u8; 4], l: [u8; 4], ol: u8, coarse: u8) -> OpDef {
    OpDef { r, l, ol, coarse, fine: 0, detune: 7, fixed: false, kvs: 2, rs: 2, ams: 0, ks: (39, 0, 0, 0, 0) }
}

impl OpDef {
    const fn fine(mut self, f: u8) -> Self {
        self.fine = f;
        self
    }
    const fn det(mut self, d: u8) -> Self {
        self.detune = d;
        self
    }
    const fn vel(mut self, k: u8) -> Self {
        self.kvs = k;
        self
    }
    const fn rate_scale(mut self, r: u8) -> Self {
        self.rs = r;
        self
    }
    const fn am(mut self, a: u8) -> Self {
        self.ams = a;
        self
    }
    const fn fixed(mut self, coarse: u8, fine: u8) -> Self {
        self.fixed = true;
        self.coarse = coarse;
        self.fine = fine;
        self
    }
    /// Fade the operator out above the breakpoint (keeps high notes clean).
    const fn soft_top(mut self, bp: u8, depth: u8) -> Self {
        self.ks = (bp, 0, depth, 0, 0);
        self
    }
}

const OFF: OpDef = op([99, 99, 99, 99], [99, 99, 99, 0], 0, 1);

struct VoiceDef {
    name: &'static str,
    alg: u8,
    fb: u8,
    ops: [OpDef; 6],
    lfo: (u8, u8, u8, u8, u8, u8),
    pitch_env: ([u8; 4], [u8; 4]),
    transpose: u8,
}

fn build(d: &VoiceDef) -> Dx7Voice {
    let mut v = [0u8; VOICE_BYTES];
    for (k, o) in d.ops.iter().enumerate() {
        let base = (5 - k) * OP_BYTES;
        v[base..base + 4].copy_from_slice(&o.r);
        v[base + 4..base + 8].copy_from_slice(&o.l);
        v[base + BP] = o.ks.0;
        v[base + LD] = o.ks.1;
        v[base + RD] = o.ks.2;
        v[base + LC] = o.ks.3;
        v[base + RC] = o.ks.4;
        v[base + RS] = o.rs;
        v[base + AMS] = o.ams;
        v[base + KVS] = o.kvs;
        v[base + OL] = o.ol;
        v[base + PM] = o.fixed as u8;
        v[base + PC] = o.coarse;
        v[base + PF] = o.fine;
        v[base + PD] = o.detune;
    }
    v[PR1..PR1 + 4].copy_from_slice(&d.pitch_env.0);
    v[PL1..PL1 + 4].copy_from_slice(&d.pitch_env.1);
    v[ALG] = d.alg - 1;
    v[FB] = d.fb;
    v[OKS] = 1;
    let (speed, delay, pmd, amd, wave, pms) = d.lfo;
    v[LFS] = speed;
    v[LFD] = delay;
    v[LPMD] = pmd;
    v[LAMD] = amd;
    v[LFKS] = 1;
    v[LFW] = wave;
    v[LPMS] = pms;
    v[TRNP] = d.transpose;
    let mut voice = Dx7Voice(v);
    voice.set_name(d.name);
    voice.sanitize();
    voice
}

const NO_LFO: (u8, u8, u8, u8, u8, u8) = (35, 0, 0, 0, 0, 3);
const VIBRATO: (u8, u8, u8, u8, u8, u8) = (34, 33, 6, 0, 4, 3);
const FLAT_PITCH: ([u8; 4], [u8; 4]) = ([99, 99, 99, 99], [50, 50, 50, 50]);

/// The initial voice: one sine carrier.
pub fn init_voice() -> Dx7Voice {
    build(&VoiceDef {
        name: "INIT VOICE",
        alg: 1,
        fb: 0,
        ops: [op([99, 99, 99, 99], [99, 99, 99, 0], 99, 1), OFF, OFF, OFF, OFF, OFF],
        lfo: NO_LFO,
        pitch_env: FLAT_PITCH,
        transpose: 24,
    })
}

/// Krabton's own voices (operator lists are OP1..OP6).
pub fn factory_bank() -> Vec<Dx7Voice> {
    let defs = [
        VoiceDef {
            name: "KRAB EPNO",
            alg: 5,
            fb: 6,
            ops: [
                op([96, 25, 25, 67], [99, 75, 0, 0], 99, 1).rate_scale(3),
                op([95, 50, 35, 78], [99, 75, 0, 0], 78, 1).vel(6).rate_scale(3),
                op([95, 20, 20, 50], [99, 95, 0, 0], 97, 1).det(6).rate_scale(3),
                op([95, 42, 25, 60], [99, 55, 0, 0], 72, 14).vel(7).rate_scale(4).soft_top(60, 40),
                op([95, 20, 20, 50], [99, 95, 0, 0], 86, 1).det(8).rate_scale(3),
                op([95, 29, 20, 50], [99, 95, 0, 0], 70, 1).vel(4).rate_scale(3),
            ],
            lfo: (34, 33, 0, 0, 4, 2),
            pitch_env: FLAT_PITCH,
            transpose: 24,
        },
        VoiceDef {
            name: "FAT FMBASS",
            alg: 1,
            fb: 5,
            ops: [
                op([99, 40, 30, 70], [99, 88, 75, 0], 99, 0),
                op([99, 60, 30, 70], [99, 68, 50, 0], 90, 0).vel(5),
                op([99, 45, 30, 70], [99, 80, 60, 0], 78, 1).det(9),
                op([99, 70, 40, 70], [99, 45, 0, 0], 82, 3).vel(6),
                op([99, 55, 30, 70], [99, 70, 50, 0], 75, 1),
                op([99, 50, 30, 70], [99, 70, 60, 0], 72, 1),
            ],
            lfo: NO_LFO,
            pitch_env: FLAT_PITCH,
            transpose: 24,
        },
        VoiceDef {
            name: "BRASS SECT",
            alg: 22,
            fb: 7,
            ops: [
                op([72, 76, 99, 71], [99, 88, 96, 0], 98, 1),
                op([62, 51, 29, 71], [82, 95, 96, 0], 90, 1).vel(4),
                op([72, 76, 99, 71], [99, 88, 96, 0], 95, 1).det(5),
                op([72, 76, 99, 71], [99, 88, 96, 0], 95, 1).det(9),
                op([72, 76, 99, 71], [99, 88, 96, 0], 90, 1).det(7),
                op([77, 36, 41, 71], [99, 98, 98, 0], 90, 1).vel(4),
            ],
            lfo: VIBRATO,
            pitch_env: ([84, 95, 95, 60], [49, 50, 50, 50]),
            transpose: 24,
        },
        VoiceDef {
            name: "TUBE BELLS",
            alg: 5,
            fb: 3,
            ops: [
                op([95, 33, 71, 25], [99, 0, 32, 0], 99, 1).rate_scale(3),
                op([98, 12, 71, 28], [99, 0, 32, 0], 80, 3).fine(50).vel(3),
                op([95, 33, 71, 25], [99, 0, 32, 0], 92, 2).det(9).rate_scale(3),
                op([98, 12, 71, 28], [99, 0, 32, 0], 74, 7).vel(3),
                op([95, 33, 71, 25], [99, 0, 32, 0], 80, 4).det(5).rate_scale(3),
                op([98, 12, 71, 28], [99, 0, 32, 0], 66, 11).vel(3),
            ],
            lfo: NO_LFO,
            pitch_env: FLAT_PITCH,
            transpose: 24,
        },
        VoiceDef {
            name: "MARIMBA",
            alg: 7,
            fb: 0,
            ops: [
                op([99, 52, 30, 60], [99, 0, 0, 0], 99, 1).rate_scale(4),
                op([99, 75, 30, 60], [99, 0, 0, 0], 82, 4).vel(5),
                op([99, 60, 30, 60], [99, 0, 0, 0], 86, 1).rate_scale(4),
                op([99, 80, 30, 60], [99, 0, 0, 0], 75, 10).vel(5),
                op([99, 80, 30, 60], [99, 0, 0, 0], 65, 3),
                op([99, 85, 30, 60], [99, 0, 0, 0], 55, 1),
            ],
            lfo: NO_LFO,
            pitch_env: FLAT_PITCH,
            transpose: 24,
        },
        VoiceDef {
            name: "DRAWBARS",
            alg: 32,
            fb: 0,
            ops: [
                op([99, 99, 99, 90], [99, 99, 99, 0], 93, 0).vel(0),
                op([99, 99, 99, 90], [99, 99, 99, 0], 99, 1).vel(0),
                op([99, 99, 99, 90], [99, 99, 99, 0], 90, 2).vel(0),
                op([99, 99, 99, 90], [99, 99, 99, 0], 82, 3).vel(0),
                op([99, 99, 99, 90], [99, 99, 99, 0], 76, 4).vel(0),
                op([99, 99, 99, 90], [99, 99, 99, 0], 70, 6).vel(0),
            ],
            lfo: (68, 0, 4, 0, 4, 2),
            pitch_env: FLAT_PITCH,
            transpose: 24,
        },
        VoiceDef {
            name: "FM STRINGS",
            alg: 2,
            fb: 5,
            ops: [
                op([45, 25, 20, 40], [99, 92, 88, 0], 99, 1).vel(1),
                op([50, 30, 20, 40], [99, 90, 85, 0], 80, 1).det(10),
                op([42, 25, 20, 42], [99, 92, 88, 0], 97, 1).det(4).vel(1),
                op([50, 30, 20, 40], [99, 90, 85, 0], 78, 1).det(11),
                op([55, 30, 20, 40], [99, 90, 85, 0], 70, 3),
                op([55, 30, 20, 40], [99, 90, 85, 0], 66, 1),
            ],
            lfo: (30, 40, 5, 0, 4, 3),
            pitch_env: FLAT_PITCH,
            transpose: 24,
        },
        VoiceDef {
            name: "SAW LEAD",
            alg: 18,
            fb: 6,
            ops: [
                op([99, 50, 40, 60], [99, 92, 88, 0], 99, 1),
                op([99, 50, 40, 60], [99, 90, 85, 0], 88, 1).det(9),
                op([99, 50, 40, 60], [99, 90, 85, 0], 84, 1),
                op([99, 50, 40, 60], [99, 85, 80, 0], 80, 2),
                op([99, 50, 40, 60], [99, 85, 80, 0], 70, 3),
                op([99, 50, 40, 60], [99, 85, 80, 0], 72, 1),
            ],
            lfo: VIBRATO,
            pitch_env: FLAT_PITCH,
            transpose: 24,
        },
        VoiceDef {
            name: "GLASS PAD",
            alg: 5,
            fb: 4,
            ops: [
                op([48, 28, 25, 34], [99, 92, 88, 0], 97, 1).vel(1),
                op([45, 25, 20, 34], [99, 85, 80, 0], 76, 2).vel(2),
                op([46, 28, 25, 34], [99, 92, 88, 0], 92, 1).det(10).vel(1),
                op([42, 22, 20, 34], [99, 80, 75, 0], 70, 5).am(1),
                op([50, 28, 25, 34], [99, 92, 88, 0], 85, 2).det(4).vel(1),
                op([45, 25, 20, 34], [99, 85, 80, 0], 66, 1),
            ],
            lfo: (24, 20, 3, 20, 4, 2),
            pitch_env: FLAT_PITCH,
            transpose: 24,
        },
        VoiceDef {
            name: "CLAVINET",
            alg: 3,
            fb: 6,
            ops: [
                op([99, 72, 50, 80], [99, 60, 0, 0], 99, 1).rate_scale(4),
                op([99, 80, 50, 80], [99, 45, 0, 0], 90, 3).vel(5),
                op([99, 85, 50, 80], [99, 40, 0, 0], 80, 5).vel(4),
                op([99, 70, 50, 80], [99, 55, 0, 0], 88, 1).det(9).rate_scale(4),
                op([99, 80, 50, 80], [99, 45, 0, 0], 85, 1).vel(5),
                op([99, 82, 50, 80], [99, 40, 0, 0], 74, 2),
            ],
            lfo: NO_LFO,
            pitch_env: FLAT_PITCH,
            transpose: 24,
        },
        VoiceDef {
            name: "BREATHY FL",
            alg: 16,
            fb: 7,
            ops: [
                op([60, 40, 40, 60], [99, 95, 90, 0], 99, 1).vel(1),
                op([65, 40, 40, 60], [99, 90, 85, 0], 74, 1),
                op([65, 40, 40, 60], [99, 85, 80, 0], 60, 2),
                op([70, 40, 40, 60], [99, 80, 75, 0], 55, 1),
                op([65, 40, 40, 60], [99, 90, 85, 0], 65, 1),
                op([70, 40, 40, 60], [99, 85, 80, 0], 60, 1),
            ],
            lfo: (36, 45, 6, 0, 4, 3),
            pitch_env: ([90, 99, 99, 99], [48, 50, 50, 50]),
            transpose: 24,
        },
        VoiceDef {
            name: "HARP PLUCK",
            alg: 5,
            fb: 3,
            ops: [
                op([99, 35, 25, 50], [99, 0, 0, 0], 99, 1).rate_scale(4),
                op([99, 55, 25, 50], [99, 0, 0, 0], 78, 2).vel(5),
                op([99, 38, 25, 50], [99, 0, 0, 0], 90, 1).det(9).rate_scale(4),
                op([99, 62, 25, 50], [99, 0, 0, 0], 70, 4).vel(6),
                op([99, 40, 25, 50], [99, 0, 0, 0], 76, 2).rate_scale(4),
                op([99, 70, 25, 50], [99, 0, 0, 0], 64, 1),
            ],
            lfo: NO_LFO,
            pitch_env: FLAT_PITCH,
            transpose: 24,
        },
        VoiceDef {
            name: "CHIME KEYS",
            alg: 5,
            fb: 2,
            ops: [
                op([97, 30, 30, 45], [99, 60, 0, 0], 99, 1).rate_scale(3),
                op([98, 50, 30, 50], [99, 40, 0, 0], 80, 7).vel(6),
                op([97, 30, 30, 45], [99, 60, 0, 0], 88, 2).det(8).rate_scale(3),
                op([98, 55, 30, 50], [99, 30, 0, 0], 74, 9).vel(6),
                op([97, 28, 30, 45], [99, 60, 0, 0], 70, 4).rate_scale(3),
                op([98, 60, 30, 50], [99, 30, 0, 0], 68, 1),
            ],
            lfo: NO_LFO,
            pitch_env: FLAT_PITCH,
            transpose: 24,
        },
        VoiceDef {
            name: "SYNC BASS",
            alg: 16,
            fb: 7,
            ops: [
                op([99, 55, 40, 75], [99, 85, 70, 0], 99, 0),
                op([99, 70, 40, 75], [99, 60, 30, 0], 90, 1).vel(5),
                op([99, 75, 40, 75], [99, 55, 20, 0], 84, 2).vel(4),
                op([99, 80, 40, 75], [99, 40, 0, 0], 76, 3),
                op([99, 70, 40, 75], [99, 60, 40, 0], 86, 1),
                op([99, 60, 40, 75], [99, 75, 60, 0], 80, 1),
            ],
            lfo: NO_LFO,
            pitch_env: FLAT_PITCH,
            transpose: 24,
        },
        VoiceDef {
            name: "DIGI VOX",
            alg: 15,
            fb: 4,
            ops: [
                op([55, 35, 30, 50], [99, 92, 88, 0], 99, 1).vel(1),
                op([60, 35, 30, 50], [99, 85, 80, 0], 78, 1).det(10),
                op([55, 35, 30, 50], [99, 92, 88, 0], 90, 3).vel(1),
                op([60, 35, 30, 50], [99, 85, 80, 0], 74, 5),
                op([60, 35, 30, 50], [99, 85, 80, 0], 70, 7),
                op([60, 35, 30, 50], [99, 85, 80, 0], 66, 2),
            ],
            lfo: (40, 40, 8, 0, 4, 3),
            pitch_env: FLAT_PITCH,
            transpose: 24,
        },
        VoiceDef {
            name: "TOY PIANO",
            alg: 5,
            fb: 5,
            ops: [
                op([99, 45, 30, 55], [99, 30, 0, 0], 99, 1).rate_scale(4),
                op([99, 65, 30, 60], [99, 20, 0, 0], 80, 5).fine(0).vel(5),
                op([99, 48, 30, 55], [99, 30, 0, 0], 86, 3).rate_scale(4),
                op([99, 70, 30, 60], [99, 20, 0, 0], 72, 13).vel(5),
                op([99, 50, 30, 55], [99, 30, 0, 0], 50, 1).fixed(2, 60),
                op([99, 75, 30, 60], [99, 20, 0, 0], 55, 1),
            ],
            lfo: NO_LFO,
            pitch_env: FLAT_PITCH,
            transpose: 24,
        },
    ];
    let mut bank: Vec<Dx7Voice> = defs.iter().map(build).collect();
    bank.push(init_voice());
    bank
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn algorithms_are_well_formed() {
        for (i, a) in ALGORITHMS.iter().enumerate() {
            assert!(a.carriers != 0, "alg {}", i + 1);
            for op in 0..6 {
                // Modulators always have a higher operator number.
                assert_eq!(a.mods[op] & ((1u8 << (op + 1)) - 1), 0, "alg {} op {}", i + 1, op + 1);
                // An operator is either a carrier or modulates something.
                let modulates = a.mods.iter().any(|m| m & (1 << op) != 0);
                assert!(modulates || a.carriers & (1 << op) != 0, "alg {} op {} is unused", i + 1, op + 1);
            }
        }
        assert_eq!(ALGORITHMS[31].carriers, 0b111111);
        assert_eq!(ALGORITHMS[0].carriers, 0b000101);
    }

    #[test]
    fn pack_unpack_roundtrip() {
        for v in factory_bank() {
            let packed = v.pack();
            assert_eq!(Dx7Voice::unpack(&packed), v, "{}", v.name());
        }
    }

    #[test]
    fn sysex_roundtrip() {
        let bank = factory_bank();
        let syx = bank_sysex(&bank);
        assert_eq!(syx.len(), 4104);
        let back = parse_sysex(&syx).unwrap();
        assert_eq!(back.len(), 32);
        assert_eq!(back[0], bank[0]);
        assert_eq!(back[0].name(), "KRAB EPNO");
        let one = single_voice_sysex(&bank[3]);
        assert_eq!(one.len(), 163);
        assert_eq!(parse_sysex(&one).unwrap()[0], bank[3]);
        assert!(parse_sysex(&[1, 2, 3]).is_err());
    }

    #[test]
    fn unpack_reads_dx7_bitfields() {
        // Hand-built packed voice; OP6 is stored first.
        let mut p = [0u8; 128];
        p[11] = (2 << 2) | 1; // RC=2, LC=1
        p[12] = (9 << 3) | 5; // detune 9, rate scaling 5
        p[13] = (6 << 2) | 3; // KVS 6, AMS 3
        p[14] = 87; // output level
        p[15] = (14 << 1) | 1; // coarse 14, fixed
        p[16] = 33; // fine
        p[110] = 21; // algorithm 22
        p[111] = (1 << 3) | 6; // key sync, feedback 6
        p[116] = (5 << 4) | (3 << 1) | 1; // PMS 5, wave 3, LFO sync
        p[117] = 24;
        p[118..128].copy_from_slice(b"TEST VOICE");
        let v = Dx7Voice::unpack(&p);
        let op6 = 5;
        assert_eq!((v.op(op6, LC), v.op(op6, RC)), (1, 2));
        assert_eq!((v.op(op6, PD), v.op(op6, RS)), (9, 5));
        assert_eq!((v.op(op6, KVS), v.op(op6, AMS)), (6, 3));
        assert_eq!((v.op(op6, OL), v.op(op6, PC), v.op(op6, PM), v.op(op6, PF)), (87, 14, 1, 33));
        assert_eq!((v.algorithm(), v.get(FB), v.get(OKS)), (21, 6, 1));
        assert_eq!((v.get(LPMS), v.get(LFW), v.get(LFKS)), (5, 3, 1));
        assert_eq!(v.name(), "TEST VOICE");
        assert_eq!(v.pack(), p);
    }

    #[test]
    fn names_and_sanitize() {
        let mut v = Dx7Voice([255; VOICE_BYTES]);
        v.sanitize();
        assert_eq!(v.algorithm(), 31);
        assert_eq!(v.op(0, OL), 99);
        v.set_name("hello world, too long");
        assert_eq!(v.name(), "hello worl");
    }
}
