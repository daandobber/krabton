//! Runs a patch: the modules in signal order (cables back up the chain
//! arrive a sample late, as feedback), once per voice.

use super::dsp::{SampleCtx, State};
use super::{ModuleKind, Patch, SPECS, VOICES, VOLUME};
use crate::dsp::db_to_gain;

pub const MAX_VOICES: usize = 8;
const MAX_PORTS: usize = 8;

struct Voice {
    states: Vec<State>,
    outs: Vec<f32>,
    note: u8,
    gate: bool,
    vel: f32,
    trig: bool,
    active: bool,
    age: u64,
    quiet: u32,
}

pub struct ModularState {
    sr: f32,
    structure: Option<u64>,
    kinds: Vec<(u32, ModuleKind)>,
    order: Vec<usize>,
    /// Per module and input: the (module, output) patched into it.
    routes: Vec<[Option<(usize, usize)>; MAX_PORTS]>,
    connected: Vec<u32>,
    offsets: Vec<usize>,
    outs_len: usize,
    voices: Vec<Voice>,
    held: Vec<(u8, f32)>,
    counter: u64,
    bpm: f32,
    beat: f64,
    playing: bool,
}

impl ModularState {
    pub fn new(sr: f32) -> Self {
        Self {
            sr,
            structure: None,
            kinds: Vec::new(),
            order: Vec::new(),
            routes: Vec::new(),
            connected: Vec::new(),
            offsets: Vec::new(),
            outs_len: 0,
            voices: (0..MAX_VOICES)
                .map(|_| Voice { states: Vec::new(), outs: Vec::new(), note: 60, gate: false, vel: 0.0, trig: false, active: false, age: 0, quiet: 0 })
                .collect(),
            held: Vec::with_capacity(32),
            counter: 0,
            bpm: 120.0,
            beat: 0.0,
            playing: false,
        }
    }

    /// The song position at the start of the next block.
    pub fn set_transport(&mut self, bpm: f32, beat: f64, playing: bool) {
        self.bpm = bpm;
        self.beat = beat;
        self.playing = playing;
    }

    /// Rebuilds the graph after modules or cables changed; modules that stay
    /// keep their state.
    fn rebuild(&mut self, patch: &Patch) {
        let n = patch.modules.len();
        let index = |id: u32| patch.modules.iter().position(|m| m.id == id);
        self.routes = vec![[None; MAX_PORTS]; n];
        self.connected = vec![0; n];
        for c in &patch.cables {
            if let (Some(a), Some(b)) = (index(c.from), index(c.to)) {
                if (c.input as usize) < MAX_PORTS {
                    self.routes[b][c.input as usize] = Some((a, c.out as usize));
                    self.connected[b] |= 1 << c.input;
                }
            }
        }
        // Signal order: a module after the ones that feed it (Kahn); what is
        // left in a loop goes in rack order.
        let mut indeg = vec![0usize; n];
        let mut feeds: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (b, r) in self.routes.iter().enumerate() {
            let mut srcs: Vec<usize> = r.iter().flatten().map(|&(a, _)| a).filter(|&a| a != b).collect();
            srcs.sort_unstable();
            srcs.dedup();
            for a in srcs {
                indeg[b] += 1;
                feeds[a].push(b);
            }
        }
        let mut order = Vec::with_capacity(n);
        let mut ready: Vec<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
        let mut done = vec![false; n];
        while order.len() < n {
            if ready.is_empty() {
                // A feedback loop: start it at the first module not yet placed.
                if let Some(i) = (0..n).find(|&i| !done[i]) {
                    ready.push(i);
                    indeg[i] = 0;
                }
            }
            ready.sort_unstable_by(|a, b| b.cmp(a));
            let Some(i) = ready.pop() else { break };
            if done[i] {
                continue;
            }
            done[i] = true;
            order.push(i);
            for &b in &feeds[i] {
                if !done[b] && indeg[b] > 0 {
                    indeg[b] -= 1;
                    if indeg[b] == 0 {
                        ready.push(b);
                    }
                }
            }
        }
        self.order = order;
        self.offsets.clear();
        let mut off = 0;
        for m in &patch.modules {
            self.offsets.push(off);
            off += m.kind.info().outputs.len();
        }
        self.outs_len = off;
        let new_kinds: Vec<(u32, ModuleKind)> = patch.modules.iter().map(|m| (m.id, m.kind)).collect();
        for (vi, v) in self.voices.iter_mut().enumerate() {
            let mut old: Vec<Option<State>> = std::mem::take(&mut v.states).into_iter().map(Some).collect();
            v.states = new_kinds
                .iter()
                .map(|&(id, kind)| {
                    match self.kinds.iter().position(|&k| k == (id, kind)) {
                        Some(j) if j < old.len() => old[j].take().unwrap_or_else(|| State::new(kind, self.sr, id * 131 + vi as u32)),
                        _ => State::new(kind, self.sr, id * 131 + vi as u32),
                    }
                })
                .collect();
            v.outs = vec![0.0; off];
        }
        self.kinds = new_kinds;
        self.structure = Some(patch.structure());
    }

    pub fn note_on(&mut self, p: &[f32], note: u8, vel: f32) {
        let poly = Self::poly(p);
        self.counter += 1;
        if poly == 1 {
            // Mono: the newest note plays; held notes come back legato.
            self.held.retain(|h| h.0 != note);
            if self.held.len() < self.held.capacity() {
                self.held.push((note, vel));
            }
            let v = &mut self.voices[0];
            v.note = note;
            v.vel = vel;
            v.gate = true;
            v.trig = true;
            v.active = true;
            v.quiet = 0;
            return;
        }
        let pool = &mut self.voices[..poly];
        let pick = pool
            .iter()
            .position(|v| v.active && v.gate && v.note == note)
            .or_else(|| pool.iter().position(|v| !v.active))
            .or_else(|| pool.iter().enumerate().filter(|(_, v)| !v.gate).min_by_key(|(_, v)| v.age).map(|(i, _)| i))
            .unwrap_or_else(|| pool.iter().enumerate().min_by_key(|(_, v)| v.age).map(|(i, _)| i).unwrap_or(0));
        let v = &mut pool[pick];
        v.note = note;
        v.vel = vel;
        v.gate = true;
        v.trig = true;
        v.active = true;
        v.quiet = 0;
        v.age = self.counter;
    }

    pub fn note_off(&mut self, p: &[f32], note: u8) {
        if Self::poly(p) == 1 {
            self.held.retain(|h| h.0 != note);
            let v = &mut self.voices[0];
            match self.held.last() {
                Some(&(n, vel)) if v.note == note => {
                    v.note = n;
                    v.vel = vel;
                }
                None => v.gate = false,
                _ => {}
            }
            return;
        }
        for v in self.voices.iter_mut().filter(|v| v.gate && v.note == note) {
            v.gate = false;
        }
    }

    pub fn all_off(&mut self) {
        self.held.clear();
        for v in self.voices.iter_mut() {
            v.gate = false;
        }
    }

    fn poly(p: &[f32]) -> usize {
        (p.get(VOICES).copied().unwrap_or(1.0).round() as usize).clamp(1, MAX_VOICES)
    }

    pub fn render(&mut self, p: &[f32], patch: &Patch, l: &mut [f32], r: &mut [f32]) {
        if p.len() < SPECS.len() {
            return;
        }
        if self.structure != Some(patch.structure()) || self.kinds.len() != patch.modules.len() {
            self.rebuild(patch);
        }
        let n = l.len().min(r.len());
        let poly = Self::poly(p);
        let gain = db_to_gain(p[VOLUME]) / 5.0;
        let mut macros = [0.0f32; 8];
        macros.copy_from_slice(&p[..8]);
        let beats_per_sample = self.bpm.max(1.0) as f64 / 60.0 / self.sr as f64;
        let quiet_limit = self.sr as u32;
        for (vi, v) in self.voices.iter_mut().enumerate() {
            // Mono: the one voice always runs (LFOs, clocks and drones too).
            if vi >= poly || (poly > 1 && !v.active) {
                continue;
            }
            if poly == 1 {
                v.active = true;
            }
            let mut ctx = SampleCtx { sr: self.sr, bpm: self.bpm, beat: self.beat, playing: self.playing, pitch: (v.note as f32 - 60.0) / 12.0, gate: v.gate, vel: v.vel, trig: false, macros };
            for s in 0..n {
                ctx.beat = self.beat + s as f64 * beats_per_sample;
                ctx.trig = std::mem::take(&mut v.trig);
                let (mut out_l, mut out_r) = (0.0f32, 0.0f32);
                for &mi in &self.order {
                    let module = &patch.modules[mi];
                    let kind = module.kind;
                    let info = kind.info();
                    let mut inputs = [0.0f32; MAX_PORTS];
                    for (j, route) in self.routes[mi].iter().enumerate().take(info.inputs.len()) {
                        if let Some((src, o)) = route {
                            inputs[j] = v.outs[self.offsets[*src] + o];
                        }
                    }
                    if kind == ModuleKind::Out {
                        let level = module.params.first().copied().unwrap_or(0.8);
                        let left = inputs[0];
                        let right = if self.connected[mi] & 2 != 0 { inputs[1] } else { left };
                        out_l += left * level;
                        out_r += right * level;
                        continue;
                    }
                    let off = self.offsets[mi];
                    let outs = &mut v.outs[off..off + info.outputs.len()];
                    v.states[mi].tick(kind, &module.params, &inputs[..info.inputs.len().max(1)], self.connected[mi], outs, &ctx);
                }
                let (yl, yr) = (out_l * gain, out_r * gain);
                let (yl, yr) = if yl.is_finite() && yr.is_finite() { (yl, yr) } else { (0.0, 0.0) };
                l[s] += yl;
                r[s] += yr;
                if poly > 1 {
                    if !v.gate && yl.abs().max(yr.abs()) < 1e-4 {
                        v.quiet += 1;
                    } else {
                        v.quiet = 0;
                    }
                }
            }
            if poly > 1 && v.quiet > quiet_limit {
                v.active = false;
            }
        }
        self.beat += n as f64 * beats_per_sample;
    }
}

#[cfg(test)]
mod tests {
    use super::super::{presets, ModuleKind};
    use std::f32::consts::TAU;
    use super::*;

    fn play(name: &str, secs: f32) -> (Vec<f32>, Vec<f32>) {
        let (params, patch) = presets::preset(name);
        let mut m = ModularState::new(48000.0);
        m.set_transport(120.0, 0.0, true);
        m.note_on(&params, 48, 0.9);
        let n = (secs * 48000.0) as usize;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        for (k, (cl, cr)) in l.chunks_mut(256).zip(r.chunks_mut(256)).enumerate() {
            if k == 100 {
                m.note_off(&params, 48);
            }
            m.render(&params, &patch, cl, cr);
        }
        (l, r)
    }

    #[test]
    fn every_preset_makes_sound() {
        for name in presets::preset_names().iter().filter(|n| **n != "Empty Rack") {
            let (l, r) = play(name, 2.0);
            let peak = l.iter().chain(&r).fold(0.0f32, |m, x| m.max(x.abs()));
            eprintln!("{name:<18} peak {peak:.3}");
            assert!(l.iter().chain(&r).all(|x| x.is_finite()), "{name}");
            assert!(peak > 0.02 && peak < 3.0, "{name}: {peak}");
        }
    }

    #[test]
    fn the_init_patch_plays_the_note() {
        let (params, mut patch) = presets::preset("Init");
        // Open the filter to hear the saw's pitch plainly.
        for m in patch.modules.iter_mut().filter(|m| m.kind == ModuleKind::Vcf) {
            m.params[0] = 18000.0;
        }
        let mut m = ModularState::new(48000.0);
        m.note_on(&params, 69, 0.9);
        let (mut l, mut r) = (vec![0.0; 24000], vec![0.0; 24000]);
        m.render(&params, &patch, &mut l, &mut r);
        let ups = l[4800..].windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
        // 440 Hz over 0.4 s.
        assert!((ups as i32 - 176).abs() <= 3, "{ups}");
    }

    #[test]
    fn a_feedback_loop_does_not_hang_or_blow_up() {
        let (params, mut patch) = presets::preset("Init");
        let vco = patch.modules.iter().find(|m| m.kind == ModuleKind::Vco).unwrap().id;
        let vcf = patch.modules.iter().find(|m| m.kind == ModuleKind::Vcf).unwrap().id;
        // The filter's output back into the oscillator's FM, and itself.
        patch.connect(vcf, 0, vco, 1);
        patch.connect(vcf, 1, vcf, 0);
        let mut m = ModularState::new(48000.0);
        m.note_on(&params, 60, 1.0);
        let (mut l, mut r) = (vec![0.0; 9600], vec![0.0; 9600]);
        m.render(&params, &patch, &mut l, &mut r);
        assert!(l.iter().all(|x| x.is_finite()));
    }

    #[test]
    fn every_module_stays_sane_with_anything_patched_in() {
        use super::super::dsp::{SampleCtx, State};
        let sr = 48000.0;
        for kind in ModuleKind::ALL {
            let info = kind.info();
            let params = crate::params::defaults(info.params);
            let mut st = State::new(kind, sr, 7);
            let mut outs = vec![0.0f32; info.outputs.len()];
            let mut peak = 0.0f32;
            let mut ctx = SampleCtx { sr, bpm: 120.0, playing: true, pitch: 0.0, gate: true, vel: 0.8, trig: true, macros: [0.5; 8], ..Default::default() };
            for s in 0..48000 {
                ctx.beat = s as f64 / 24000.0;
                // A 4 Hz gate on even inputs, a 220 Hz tone on odd ones.
                let gate = if (s / 6000) % 2 == 0 { 10.0 } else { 0.0 };
                let tone = 5.0 * (s as f32 * 220.0 * TAU / sr).sin();
                let inputs: Vec<f32> = (0..info.inputs.len().max(1)).map(|j| if j % 2 == 0 { gate } else { tone }).collect();
                st.tick(kind, &params, &inputs, u32::MAX, &mut outs, &ctx);
                ctx.trig = false;
                for o in &outs {
                    assert!(o.is_finite(), "{}", info.name);
                    peak = peak.max(o.abs());
                }
            }
            assert!(peak < 60.0, "{}: {peak}", info.name);
        }
    }

    #[test]
    fn the_drum_machine_plays_by_itself() {
        let (params, patch) = presets::preset("Drum Machine");
        let mut m = ModularState::new(48000.0);
        m.set_transport(120.0, 0.0, true);
        let (mut l, mut r) = (vec![0.0; 96000], vec![0.0; 96000]);
        for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            m.render(&params, &patch, cl, cr);
        }
        // A kick on every beat at 120 BPM.
        let beats: Vec<f32> = (0..4).map(|b| l[b * 24000..b * 24000 + 2400].iter().fold(0.0f32, |m, x| m.max(x.abs()))).collect();
        assert!(beats.iter().all(|&b| b > 0.05), "{beats:?}");
    }

}
