//! Ready-made patches.

use super::{ModuleKind as K, Patch, SPECS, VOICES, VOLUME};

pub fn preset_names() -> &'static [&'static str] {
    &["Init", "Acid Bass", "Fat Lead", "West Coast", "FM Bell", "Drone", "Generative", "Kick", "Poly Pluck", "Drum Machine", "Macro Voice", "Texture Pad", "Pluck Melody", "Struck Resonator", "Empty Rack"]
}

/// A patch being built: modules go into the first free place.
struct B {
    patch: Patch,
}

impl B {
    fn new() -> Self {
        Self { patch: Patch::default() }
    }

    fn add(&mut self, kind: K, row: u16) -> u32 {
        self.patch.add(kind, row)
    }

    fn set(&mut self, id: u32, pairs: &[(usize, f32)]) {
        if let Some(m) = self.patch.modules.iter_mut().find(|m| m.id == id) {
            for &(i, v) in pairs {
                if let Some(x) = m.params.get_mut(i) {
                    *x = v;
                }
            }
        }
    }

    fn wire(&mut self, from: u32, out: u8, to: u32, input: u8) {
        self.patch.connect(from, out, to, input);
    }
}

/// The instrument parameters and the patch of a preset.
pub fn preset(name: &str) -> (Vec<f32>, Patch) {
    let mut params = crate::params::defaults(SPECS);
    let mut b = B::new();
    // Jack numbers, by module (see the module catalog).
    const V_OCT: u8 = 0;
    const GATE: u8 = 1;
    const VEL: u8 = 2;
    const TRIG: u8 = 3;
    let (sine, tri, saw, square) = (0u8, 1u8, 2u8, 3u8);
    match name {
        "Empty Rack" => {
            let midi = b.add(K::Midi, 0);
            let out = b.add(K::Out, 0);
            let _ = (midi, out);
        }
        "Acid Bass" => {
            let midi = b.add(K::Midi, 0);
            let vco = b.add(K::Vco, 0);
            let ladder = b.add(K::Ladder, 0);
            let env = b.add(K::Ad, 0);
            let amp_env = b.add(K::Adsr, 0);
            let vca = b.add(K::Vca, 0);
            let out = b.add(K::Out, 0);
            b.set(midi, &[(0, 0.06), (1, -1.0)]);
            b.set(ladder, &[(0, 220.0), (1, 0.78), (2, 0.55), (3, 0.5)]);
            b.set(env, &[(1, 0.22), (2, 0.8)]);
            b.set(amp_env, &[(0, 0.002), (1, 0.2), (2, 0.8), (3, 0.06)]);
            b.wire(midi, V_OCT, vco, 0);
            b.wire(vco, saw, ladder, 0);
            b.wire(midi, TRIG, env, 0);
            b.wire(env, 0, ladder, 1);
            b.wire(midi, GATE, amp_env, 0);
            b.wire(ladder, 0, vca, 0);
            b.wire(amp_env, 0, vca, 1);
            b.wire(vca, 0, out, 0);
        }
        "Fat Lead" => {
            let midi = b.add(K::Midi, 0);
            let vco1 = b.add(K::Vco, 0);
            let vco2 = b.add(K::Vco, 0);
            let lfo = b.add(K::Lfo, 0);
            let mix = b.add(K::Mixer, 0);
            let vcf = b.add(K::Ladder, 0);
            let env = b.add(K::Adsr, 0);
            let vca = b.add(K::Vca, 0);
            let delay = b.add(K::Delay, 1);
            let out = b.add(K::Out, 1);
            b.set(midi, &[(0, 0.04)]);
            b.set(vco2, &[(2, 9.0), (5, 0.4)]);
            b.set(lfo, &[(0, 0.4)]);
            b.set(vcf, &[(0, 1800.0), (1, 0.3), (2, 0.25)]);
            b.set(env, &[(0, 0.01), (1, 0.5), (2, 0.8), (3, 0.3)]);
            b.set(delay, &[(0, 375.0), (1, 0.35), (3, 0.25)]);
            b.wire(midi, V_OCT, vco1, 0);
            b.wire(midi, V_OCT, vco2, 0);
            b.wire(lfo, sine, vco2, 2);
            b.wire(vco1, saw, mix, 0);
            b.wire(vco2, square, mix, 1);
            b.wire(mix, 0, vcf, 0);
            b.wire(env, 0, vcf, 1);
            b.wire(midi, GATE, env, 0);
            b.wire(vcf, 0, vca, 0);
            b.wire(env, 0, vca, 1);
            b.wire(vca, 0, delay, 0);
            b.wire(delay, 0, out, 0);
        }
        "West Coast" => {
            let midi = b.add(K::Midi, 0);
            let vco = b.add(K::Vco, 0);
            let folder = b.add(K::Folder, 0);
            let env = b.add(K::Ad, 0);
            let vcf = b.add(K::Vcf, 0);
            let vca = b.add(K::Vca, 0);
            let rev = b.add(K::Reverb, 0);
            let out = b.add(K::Out, 0);
            b.set(folder, &[(0, 0.2), (2, 0.9)]);
            b.set(env, &[(1, 0.45), (2, 0.85)]);
            b.set(vcf, &[(0, 300.0), (1, 0.2), (2, 0.6)]);
            b.set(rev, &[(2, 0.25)]);
            b.wire(midi, V_OCT, vco, 0);
            b.wire(vco, tri, folder, 0);
            b.wire(midi, TRIG, env, 0);
            b.wire(env, 0, folder, 1);
            b.wire(folder, 0, vcf, 0);
            // Low-pass gate: the envelope opens filter and amp together.
            b.wire(env, 0, vcf, 1);
            b.wire(vcf, 0, vca, 0);
            b.wire(env, 0, vca, 1);
            b.wire(vca, 0, rev, 0);
            b.wire(rev, 0, out, 0);
            b.wire(rev, 1, out, 1);
        }
        "FM Bell" => {
            let midi = b.add(K::Midi, 0);
            let modu = b.add(K::Vco, 0);
            let car = b.add(K::Vco, 0);
            let env = b.add(K::Ad, 0);
            let vca = b.add(K::Vca, 0);
            let rev = b.add(K::Reverb, 0);
            let out = b.add(K::Out, 0);
            // The modulator about three times the carrier: bell partials.
            b.set(modu, &[(0, 1.0), (1, 7.0), (2, 17.0)]);
            b.set(car, &[(4, 0.7)]);
            b.set(env, &[(1, 2.5), (2, 0.9)]);
            b.set(rev, &[(0, 0.7), (1, 0.7), (2, 0.35)]);
            b.wire(midi, V_OCT, modu, 0);
            b.wire(midi, V_OCT, car, 0);
            b.wire(modu, sine, car, 1);
            b.wire(midi, TRIG, env, 0);
            b.wire(car, sine, vca, 0);
            b.wire(env, 0, vca, 1);
            b.wire(vca, 0, rev, 0);
            b.wire(rev, 0, out, 0);
            b.wire(rev, 1, out, 1);
            params[VOICES] = 6.0;
        }
        "Drone" => {
            let vco1 = b.add(K::Vco, 0);
            let vco2 = b.add(K::Vco, 0);
            let lfo = b.add(K::Lfo, 0);
            let noise = b.add(K::Noise, 0);
            let mix = b.add(K::Mixer, 0);
            let vcf = b.add(K::Vcf, 0);
            let vca = b.add(K::Vca, 1);
            let rev = b.add(K::Reverb, 1);
            let out = b.add(K::Out, 1);
            b.set(vco1, &[(0, -2.0)]);
            b.set(vco2, &[(0, -1.0), (1, 7.0), (2, 6.0)]);
            b.set(lfo, &[(0, 0.07)]);
            b.set(noise, &[(0, 0.2)]);
            b.set(vcf, &[(0, 600.0), (1, 0.55), (2, 0.5)]);
            b.set(vca, &[(0, 0.7)]);
            b.set(rev, &[(0, 0.9), (1, 0.85), (2, 0.5)]);
            b.wire(vco1, saw, mix, 0);
            b.wire(vco2, square, mix, 1);
            b.wire(lfo, tri, vcf, 1);
            b.wire(noise, 2, vco2, 2);
            b.wire(mix, 0, vcf, 0);
            b.wire(vcf, 0, vca, 0);
            b.wire(vca, 0, rev, 0);
            b.wire(rev, 0, out, 0);
            b.wire(rev, 1, out, 1);
            params[VOLUME] = -6.0;
        }
        "Generative" => {
            let clock = b.add(K::Clock, 0);
            let seq = b.add(K::Sequencer, 0);
            let quant = b.add(K::Quantizer, 0);
            let chance = b.add(K::Chance, 0);
            let vco = b.add(K::Vco, 0);
            let env = b.add(K::Ad, 0);
            let vcf = b.add(K::Vcf, 1);
            let lfo = b.add(K::Lfo, 1);
            let vca = b.add(K::Vca, 1);
            let delay = b.add(K::Delay, 1);
            let rev = b.add(K::Reverb, 1);
            let out = b.add(K::Out, 1);
            b.set(quant, &[(0, 5.0)]);
            b.set(chance, &[(0, 0.7)]);
            b.set(env, &[(1, 0.35)]);
            b.set(vcf, &[(0, 900.0), (1, 0.45), (2, 0.4)]);
            b.set(lfo, &[(0, 0.13)]);
            b.set(delay, &[(0, 375.0), (1, 0.5), (3, 0.4)]);
            b.set(rev, &[(0, 0.8), (1, 0.7), (2, 0.35)]);
            b.wire(clock, 0, seq, 0);
            b.wire(clock, 3, seq, 1);
            b.wire(seq, 0, quant, 0);
            b.wire(quant, 0, vco, 0);
            b.wire(clock, 0, chance, 0);
            b.wire(chance, 0, env, 0);
            b.wire(vco, saw, vcf, 0);
            b.wire(lfo, sine, vcf, 1);
            b.wire(vcf, 0, vca, 0);
            b.wire(env, 0, vca, 1);
            b.wire(vca, 0, delay, 0);
            b.wire(delay, 0, rev, 0);
            b.wire(rev, 0, out, 0);
            b.wire(rev, 1, out, 1);
        }
        "Kick" => {
            let midi = b.add(K::Midi, 0);
            let pitch_env = b.add(K::Ad, 0);
            let amount = b.add(K::Atten, 0);
            let vco = b.add(K::Vco, 0);
            let amp_env = b.add(K::Ad, 0);
            let vca = b.add(K::Vca, 0);
            let out = b.add(K::Out, 0);
            b.set(pitch_env, &[(1, 0.06), (2, 1.0)]);
            b.set(amount, &[(0, 0.35)]);
            b.set(vco, &[(0, -2.0)]);
            b.set(amp_env, &[(1, 0.5), (2, 0.6)]);
            b.wire(midi, TRIG, pitch_env, 0);
            b.wire(pitch_env, 0, amount, 0);
            b.wire(amount, 0, vco, 0);
            b.wire(midi, TRIG, amp_env, 0);
            b.wire(vco, sine, vca, 0);
            b.wire(amp_env, 0, vca, 1);
            b.wire(vca, 0, out, 0);
            let _ = VEL;
        }
        "Drum Machine" => {
            let clock = b.add(K::Clock, 0);
            let kick_seq = b.add(K::TrigSeq, 0);
            let snare_seq = b.add(K::TrigSeq, 0);
            let hats = b.add(K::Euclid, 0);
            let accent = b.add(K::RandomGen, 0);
            let kick = b.add(K::BassDrum, 1);
            let snare = b.add(K::SnareDrum, 1);
            let hat = b.add(K::HiHat, 1);
            let mix = b.add(K::StereoMixer, 1);
            let out = b.add(K::Out, 1);
            // Kick on 1, 5, 9 and 13 (the default), snare on 5 and 13.
            let mut snare_steps = [0.0f32; 16];
            snare_steps[4] = 1.0;
            snare_steps[12] = 1.0;
            let pairs: Vec<(usize, f32)> = snare_steps.iter().copied().enumerate().collect();
            b.set(snare_seq, &pairs);
            b.set(hats, &[(0, 16.0), (1, 11.0)]);
            b.set(accent, &[(0, 8.0), (3, 5.0)]);
            b.set(kick, &[(2, 0.55), (3, 0.5)]);
            b.set(hat, &[(2, 0.2)]);
            b.set(mix, &[(0, 1.0), (1, 0.8), (2, 0.5), (4, 0.0), (5, 0.1), (6, 0.3)]);
            b.wire(clock, 0, kick_seq, 0);
            b.wire(clock, 3, kick_seq, 1);
            b.wire(clock, 0, snare_seq, 0);
            b.wire(clock, 3, snare_seq, 1);
            b.wire(clock, 0, hats, 0);
            b.wire(clock, 3, hats, 1);
            b.wire(kick_seq, 1, kick, 0);
            b.wire(snare_seq, 1, snare, 0);
            b.wire(hats, 0, hat, 0);
            b.wire(accent, 2, hat, 1);
            b.wire(kick, 0, mix, 0);
            b.wire(snare, 0, mix, 1);
            b.wire(hat, 0, mix, 2);
            b.wire(mix, 0, out, 0);
            b.wire(mix, 1, out, 1);
        }
        "Macro Voice" => {
            let midi = b.add(K::Midi, 0);
            let osc = b.add(K::MacroVco, 0);
            let rev = b.add(K::Reverb, 0);
            let out = b.add(K::Out, 0);
            // The chord model, struck by the notes through its own low-pass gate.
            b.set(osc, &[(0, 14.0), (2, 0.4), (3, 0.5), (4, 0.4), (5, 0.6)]);
            b.set(rev, &[(2, 0.3)]);
            b.wire(midi, V_OCT, osc, 0);
            b.wire(midi, GATE, osc, 1);
            b.wire(osc, 0, rev, 0);
            b.wire(rev, 0, out, 0);
            b.wire(rev, 1, out, 1);
            params[VOICES] = 4.0;
        }
        "Texture Pad" => {
            let midi = b.add(K::Midi, 0);
            let osc = b.add(K::ShapeVco, 0);
            let env = b.add(K::Adsr, 0);
            let vca = b.add(K::Vca, 0);
            let tex = b.add(K::Texture, 1);
            let lfo = b.add(K::Lfo, 1);
            let out = b.add(K::Out, 1);
            b.set(osc, &[(0, 14.0), (2, 0.4), (3, 0.6)]);
            b.set(env, &[(0, 0.6), (2, 0.9), (3, 2.5)]);
            b.set(tex, &[(2, 0.7), (4, 0.75), (6, 0.6), (9, 0.5)]);
            b.set(lfo, &[(0, 0.1)]);
            b.wire(midi, V_OCT, osc, 0);
            b.wire(osc, 0, vca, 0);
            b.wire(midi, GATE, env, 0);
            b.wire(env, 0, vca, 1);
            b.wire(vca, 0, tex, 0);
            b.wire(lfo, sine, tex, 4);
            b.wire(tex, 0, out, 0);
            b.wire(tex, 1, out, 1);
        }
        "Pluck Melody" => {
            let clock = b.add(K::Clock, 0);
            let reg = b.add(K::ShiftReg, 0);
            let quant = b.add(K::Quantizer, 0);
            let pluck = b.add(K::Pluck, 0);
            let burst = b.add(K::Burst, 0);
            let chance = b.add(K::Chance, 0);
            let merge = b.add(K::Logic, 0);
            let delay = b.add(K::Delay, 1);
            let rev = b.add(K::Reverb, 1);
            let out = b.add(K::Out, 1);
            b.set(clock, &[(0, 1.0)]);
            b.set(reg, &[(0, 0.08), (2, 2.0)]);
            b.set(quant, &[(0, 5.0)]);
            b.set(burst, &[(0, 3.0), (1, 16.0)]);
            b.set(chance, &[(0, 0.85)]);
            b.set(delay, &[(0, 500.0), (1, 0.45), (3, 0.35)]);
            b.set(rev, &[(0, 0.7), (2, 0.3)]);
            b.wire(clock, 0, reg, 0);
            b.wire(reg, 0, quant, 0);
            b.wire(quant, 0, pluck, 0);
            b.wire(clock, 0, chance, 0);
            b.wire(chance, 1, burst, 0);
            // Plain hits and bursts together (the larger of the two).
            b.wire(chance, 0, merge, 0);
            b.wire(burst, 0, merge, 1);
            b.wire(merge, 0, pluck, 1);
            b.wire(pluck, 0, delay, 0);
            b.wire(delay, 0, rev, 0);
            b.wire(rev, 0, out, 0);
            b.wire(rev, 1, out, 1);
        }
        "Struck Resonator" => {
            let midi = b.add(K::Midi, 0);
            let res = b.add(K::Resonator, 0);
            let lfo = b.add(K::Lfo, 0);
            let out = b.add(K::Out, 0);
            b.set(res, &[(1, 60.0), (2, 0.6), (3, 0.55), (4, 0.7)]);
            b.set(lfo, &[(0, 0.2)]);
            b.wire(midi, V_OCT, res, 1);
            b.wire(midi, TRIG, res, 2);
            b.wire(lfo, tri, res, 6);
            b.wire(res, 0, out, 0);
            b.wire(res, 1, out, 1);
            params[VOICES] = 4.0;
        }
        "Poly Pluck" => {
            let midi = b.add(K::Midi, 0);
            let vco = b.add(K::Vco, 0);
            let vcf = b.add(K::Vcf, 0);
            let env = b.add(K::Adsr, 0);
            let vca = b.add(K::Vca, 0);
            let out = b.add(K::Out, 0);
            b.set(vco, &[(3, 0.3)]);
            b.set(vcf, &[(0, 400.0), (1, 0.35), (2, 0.55)]);
            b.set(env, &[(0, 0.002), (1, 0.35), (2, 0.0), (3, 0.4)]);
            b.wire(midi, V_OCT, vco, 0);
            b.wire(vco, square, vcf, 0);
            b.wire(midi, GATE, env, 0);
            b.wire(env, 0, vcf, 1);
            b.wire(vcf, 0, vca, 0);
            b.wire(env, 0, vca, 1);
            b.wire(vca, 0, out, 0);
            params[VOICES] = 6.0;
        }
        _ => {
            // Init: the classic subtractive voice.
            let midi = b.add(K::Midi, 0);
            let vco = b.add(K::Vco, 0);
            let vcf = b.add(K::Vcf, 0);
            let env = b.add(K::Adsr, 0);
            let vca = b.add(K::Vca, 0);
            let out = b.add(K::Out, 0);
            b.set(vcf, &[(2, 0.4)]);
            b.wire(midi, V_OCT, vco, 0);
            b.wire(vco, saw, vcf, 0);
            b.wire(midi, GATE, env, 0);
            b.wire(env, 0, vcf, 1);
            b.wire(vcf, 0, vca, 0);
            b.wire(env, 0, vca, 1);
            b.wire(vca, 0, out, 0);
        }
    }
    (params, b.patch)
}
