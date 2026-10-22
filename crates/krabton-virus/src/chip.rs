//! A bare DSP56300 for Krabton's own firmware: the emulator's core with its
//! memory and JIT, no Virus, no peripherals. Krabton is its host: it writes
//! the program and the values the program reads into the chip's memory, runs
//! it until it is where it waits, and reads back what it wrote.
//!
//! [`assemble`] turns firmware source into program words with the
//! emulator's own assembler, an instruction at a time, around which it does
//! what an assembler's first and second pass do: labels (`name:`), constants
//! (`name equ $20`) and comments (`;`).

use std::collections::HashMap;
use std::ffi::{c_char, c_int, CString};
use std::ptr::NonNull;

#[repr(C)]
struct KdChip {
    _p: [u8; 0],
}

extern "C" {
    fn kd_create(mem_size: u32, err: *mut c_char, err_len: usize) -> *mut KdChip;
    fn kd_destroy(c: *mut KdChip);
    fn kd_assemble(text: *const c_char, words: *mut u32, err: *mut c_char, err_len: usize) -> c_int;
    fn kd_disassemble(a: u32, b: u32, pc: u32, out: *mut c_char, len: usize) -> c_int;
    fn kd_write(c: *mut KdChip, area: c_int, addr: u32, words: *const u32, count: usize);
    fn kd_read(c: *mut KdChip, area: c_int, addr: u32, out: *mut u32, count: usize);
    fn kd_run(c: *mut KdChip, pc: u32, until: u32, max_steps: u64) -> i64;
    fn kd_cycles(c: *const KdChip) -> u64;
}

fn text(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

/// The chip's memories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Area {
    P,
    X,
    Y,
}

impl Area {
    fn code(self) -> c_int {
        match self {
            Area::P => 0,
            Area::X => 1,
            Area::Y => 2,
        }
    }
}

/// A DSP56300 of its own.
pub struct Chip(NonNull<KdChip>);

// Only ever used by one thread at a time (it is moved, not shared).
unsafe impl Send for Chip {}

impl Chip {
    /// A chip with `mem_size` words of P, X and Y memory each.
    pub fn new(mem_size: u32) -> Result<Chip, String> {
        let mut err = [0u8; 256];
        let p = unsafe { kd_create(mem_size, err.as_mut_ptr() as *mut c_char, err.len()) };
        NonNull::new(p).map(Chip).ok_or_else(|| text(&err))
    }

    pub fn write(&mut self, area: Area, addr: u32, words: &[u32]) {
        unsafe { kd_write(self.0.as_ptr(), area.code(), addr, words.as_ptr(), words.len()) }
    }

    pub fn read(&mut self, area: Area, addr: u32, out: &mut [u32]) {
        unsafe { kd_read(self.0.as_ptr(), area.code(), addr, out.as_mut_ptr(), out.len()) }
    }

    pub fn read_one(&mut self, area: Area, addr: u32) -> u32 {
        let mut w = [0u32; 1];
        self.read(area, addr, &mut w);
        w[0]
    }

    /// A program's words into P memory where it starts.
    pub fn load(&mut self, program: &Program) {
        self.write(Area::P, program.origin, &program.words);
    }

    /// Runs from `pc` until the chip is at `until`; how many steps it took,
    /// or None if it was not there after `max_steps`.
    pub fn run(&mut self, pc: u32, until: u32, max_steps: u64) -> Option<u64> {
        let n = unsafe { kd_run(self.0.as_ptr(), pc, until, max_steps) };
        (n >= 0).then_some(n as u64)
    }

    /// Cycles the chip has run so far.
    pub fn cycles(&self) -> u64 {
        unsafe { kd_cycles(self.0.as_ptr()) }
    }
}

impl Drop for Chip {
    fn drop(&mut self) {
        unsafe { kd_destroy(self.0.as_ptr()) }
    }
}

/// One instruction, by the emulator's assembler: its one or two words.
pub fn assemble_one(line: &str) -> Result<Vec<u32>, String> {
    let c = CString::new(line).map_err(|_| "a zero byte in the line".to_string())?;
    let mut words = [0u32; 2];
    let mut err = [0u8; 128];
    let n = unsafe { kd_assemble(c.as_ptr(), words.as_mut_ptr(), err.as_mut_ptr() as *mut c_char, err.len()) };
    if n <= 0 {
        return Err(text(&err));
    }
    Ok(words[..n as usize].to_vec())
}

/// Words back to text, at `pc`: the text and how many words it took.
pub fn disassemble(a: u32, b: u32, pc: u32) -> (String, usize) {
    let mut out = [0u8; 256];
    let n = unsafe { kd_disassemble(a, b, pc, out.as_mut_ptr() as *mut c_char, out.len()) };
    (text(&out), n.max(0) as usize)
}

/// Firmware assembled: its words from `origin` on, and where its labels are.
#[derive(Clone, Debug)]
pub struct Program {
    pub origin: u32,
    pub words: Vec<u32>,
    pub labels: HashMap<String, u32>,
}

impl Program {
    /// Where label `name` is (it must be there).
    pub fn at(&self, name: &str) -> u32 {
        *self.labels.get(&name.to_lowercase()).unwrap_or_else(|| panic!("no label {name}"))
    }
}

fn number(s: &str) -> Option<u32> {
    let s = s.trim();
    if let Some(h) = s.strip_prefix('$') {
        u32::from_str_radix(h, 16).ok()
    } else {
        s.parse().ok()
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

/// The names in `text` that are symbols, each replaced by its value (as
/// `$hex`); unknown names that look like symbols (they hold an underscore or
/// start with a letter and are not the assembler's own words) by `unknown`.
fn substitute(text: &str, symbols: &HashMap<String, u32>, unknown: Option<u32>) -> Result<String, String> {
    let mut out = String::with_capacity(text.len() + 8);
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        // A name begins where no name or number did.
        let starts = is_ident_start(c) && (i == 0 || !(chars[i - 1].is_ascii_alphanumeric() || chars[i - 1] == '_' || chars[i - 1] == '$'));
        if starts {
            let mut j = i;
            while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            let name: String = chars[i..j].iter().collect::<String>().to_lowercase();
            // A memory address past the short ones ($00-$3f) is written as a long one.
            let long = |out: &String, v: u32| v > 0x3f && (out.ends_with("x:") || out.ends_with("y:"));
            match symbols.get(&name) {
                Some(&v) if long(&out, v) => out.push_str(&format!(">${v:x}")),
                Some(v) => out.push_str(&format!("${v:x}")),
                None if name.contains('_') => match unknown {
                    Some(v) => out.push_str(&format!("${v:x}")),
                    None => return Err(format!("unknown symbol {name}")),
                },
                None => out.push_str(&name),
            }
            i = j;
        } else {
            out.push(c);
            i += 1;
        }
    }
    Ok(out)
}

/// Assembles firmware source placed from `origin`: an instruction a line,
/// `label:` before it (or alone), `name equ value`, `;` comments. Labels and
/// constants are written with an underscore in them (`loop_voice`, `X_PHASE`)
/// so they cannot be taken for registers.
pub fn assemble(source: &str, origin: u32) -> Result<Program, String> {
    struct Line {
        no: usize,
        text: String,
        at: u32,
        size: usize,
    }
    let mut symbols: HashMap<String, u32> = HashMap::new();
    let mut lines: Vec<Line> = Vec::new();
    let mut pc = origin;
    // First pass: constants, labels, and how long each instruction is.
    for (no, raw) in source.lines().enumerate() {
        let mut l = raw.split(';').next().unwrap_or("").trim().to_string();
        if l.is_empty() {
            continue;
        }
        let lower = l.to_lowercase();
        let parts: Vec<&str> = lower.split_whitespace().collect();
        if parts.len() == 3 && parts[1] == "equ" {
            let v = number(parts[2]).or_else(|| symbols.get(parts[2]).copied()).ok_or_else(|| format!("line {}: {} is not a number", no + 1, parts[2]))?;
            symbols.insert(parts[0].to_string(), v);
            continue;
        }
        if let Some(colon) = l.find(':') {
            let head = &l[..colon];
            if !head.is_empty() && head.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && is_ident_start(head.chars().next().unwrap_or(' ')) {
                let name = head.to_lowercase();
                if symbols.insert(name.clone(), pc).is_some() {
                    return Err(format!("line {}: {name} is there twice", no + 1));
                }
                l = l[colon + 1..].trim().to_string();
                if l.is_empty() {
                    continue;
                }
            }
        }
        let probe = substitute(&l, &symbols, Some(0x100)).map_err(|e| format!("line {}: {e}", no + 1))?;
        let words = assemble_one(&probe).map_err(|e| format!("line {} ({l}): {e}", no + 1))?;
        lines.push(Line { no, text: l, at: pc, size: words.len() });
        pc += words.len() as u32;
    }
    // Second pass: every name known now.
    let mut words = Vec::with_capacity((pc - origin) as usize);
    for line in &lines {
        let t = substitute(&line.text, &symbols, None).map_err(|e| format!("line {}: {e}", line.no + 1))?;
        let w = assemble_one(&t).map_err(|e| format!("line {} ({}): {e}", line.no + 1, line.text))?;
        if w.len() != line.size {
            return Err(format!("line {} ({}): its length changed between the passes", line.no + 1, line.text));
        }
        // What it became must be what was written: the assembler turns a
        // pair of registers a multiply cannot take into another instruction
        // (mpy x0,y0 into mpysu), so read it back.
        let (back, _) = disassemble(w[0], w.get(1).copied().unwrap_or(0), line.at);
        let said = t.split_whitespace().next().unwrap_or("").to_lowercase();
        let got = back.split_whitespace().next().unwrap_or("").to_lowercase();
        if said != got {
            return Err(format!("line {} ({}): became {back}", line.no + 1, line.text));
        }
        debug_assert_eq!(origin + words.len() as u32, line.at);
        words.extend(w);
    }
    Ok(Program { origin, words, labels: symbols })
}

/// A fraction (-1..1) as the chip's 24-bit word.
pub fn frac(v: f32) -> u32 {
    let x = (v.clamp(-1.0, 0.999_999_9) * 8_388_608.0).round() as i32;
    (x.clamp(-8_388_608, 8_388_607) as u32) & 0xFF_FFFF
}

/// The chip's 24-bit word as a fraction (-1..1).
pub fn to_frac(w: u32) -> f32 {
    (((w & 0xFF_FFFF) << 8) as i32 >> 8) as f32 / 8_388_608.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_assembler_and_disassembler_agree() {
        for line in ["move x0,x:$10", "mpy x0,y0,a", "add x0,a", "rts", "nop"] {
            let w = assemble_one(line).unwrap_or_else(|e| panic!("{line}: {e}"));
            let (back, n) = disassemble(w[0], w.get(1).copied().unwrap_or(0), 0x100);
            assert_eq!(n, w.len(), "{line} -> {back}");
            assert_eq!(assemble_one(&back).unwrap(), w, "{line} -> {back}");
        }
        assert!(assemble_one("flurb a,b").is_err());
    }

    #[test]
    fn a_program_runs_on_the_chip_and_its_memory_is_read_back() {
        let src = "
X_IN equ $10
X_OUT equ $11
start:
        move x:X_IN,x0
        clr a
        do #5,loop_end      ; five times
        add x0,a
loop_end:
        move a1,x:X_OUT
idle_loop:
        jmp idle_loop
";
        let prog = assemble(src, 0x100).unwrap();
        let mut chip = Chip::new(0x8000).unwrap();
        chip.load(&prog);
        chip.write(Area::X, 0x10, &[frac(0.1)]);
        let steps = chip.run(prog.at("start"), prog.at("idle_loop"), 10_000);
        assert!(steps.is_some(), "it got to the end");
        let out = to_frac(chip.read_one(Area::X, 0x11));
        assert!((out - 0.5).abs() < 1e-5, "five times 0.1: {out}");
        assert!(chip.cycles() > 5);
    }

    #[test]
    fn labels_constants_and_mistakes() {
        let p = assemble("A_B equ $3\nfirst:\n nop\nsecond: nop\n move x:A_B,x0\n", 0x40).unwrap();
        assert_eq!((p.at("first"), p.at("second")), (0x40, 0x41));
        assert_eq!(p.words.len(), 3);
        assert!(assemble("jmp no_where", 0).unwrap_err().contains("no_where"));
        assert!(assemble("twice:\ntwice: nop", 0).is_err());
        assert!(assemble("mpy x0,y0,a", 0).unwrap_err().contains("mpysu"), "a pair a multiply cannot take");
        assert!(assemble("mpy y0,x0,a", 0).is_ok());
        let far = assemble("X_FAR equ $40\nX_NEAR equ $3f\n move x:X_FAR,x0\n move x:X_NEAR,x0\n move y0,y:X_FAR", 0).unwrap();
        assert_eq!(far.words.len(), 5, "a long address takes two words, a short one one");
        assert!((to_frac(frac(-0.25)) + 0.25).abs() < 1e-6 && to_frac(frac(1.5)) > 0.99);
    }
}

