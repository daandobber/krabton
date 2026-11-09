# krabton-chimera

The engine of CHIMERA, a physical-modelling voice synthesizer by Jameson Hodge
(<https://github.com/jamesondh/chimera-vocal-synth>), MIT licensed (see `LICENSE`),
as kept in the community fork Chimera Lab (<https://github.com/Franzlazarus/chimera-lab>,
crate `crates/chimera`, commit `5ca6345d49f7a1ce7013465045834d42dac880a7`). Only the
engine (`src/`) is here, unchanged; its tests and golden files are left out. The
fork's VST3 adapter (GPL-3.0) is not used: Krabton's Airway drives the engine itself
(`krabton-engine/src/dsp/airway.rs`).
