# Krabton

**An open-source, Live-inspired DAW written in Rust.** Session view clip launching, an arrangement timeline, built-in synths, drums, a sampler and effects, all in one native binary with no plugins required.

![Krabton session view](docs/screenshot.png)

> Krabton is a hobby project inspired by the workflow of Ableton Live. It is not affiliated with or endorsed by Ableton.

## Features

**Session view**
- Clip launcher grid with scenes, per-track stop buttons and *Stop All Clips*
- Global launch quantization (none, 1/16 up to 4 bars), blinking queued clips, progress bars
- Drag clips between slots (Ctrl+drag copies), right-click menus, rename, recolor, duplicate
- Mixer under the grid: sends, pan, volume fader with peak meters, track activator, solo, arm
- Two return tracks (A Reverb, B Delay) out of the box, and a master track

**Arrangement view**
- One lane per track with mini note/waveform previews
- Move clips and resize them from either edge (clips loop when stretched), with grid snapping (Alt = free)
- Loop brace, click-to-locate ruler, follow playhead, Ctrl+wheel zoom
- *Scenes to Arrangement* turns a session jam into a song
- Session clips override the arrangement per track, with *Back to Arrangement* like Live

**Editing and recording**
- Piano roll with draw mode, marquee selection, move/resize, Ctrl+drag duplicate, velocity lane, quantize, transpose, x2 / /2
- Drum mode: the piano roll folds to the pads of the drum rack
- Computer MIDI keyboard (A W S E D F T G Y H U J K ..., Z/X octave, C/V velocity) and hardware MIDI input
- Session record: arm a track, press record (F9), play; notes are overdubbed into the looping clip with optional 1/16 quantization
- Undo/redo for every edit

**Key, scales and MIDI tools** (Live 12 style)
- A project-wide key and scale (18 scales: major/minor modes, harmonic and melodic minor, pentatonics, blues, whole tone, In Sen, Hirajoshi, ...) shown in the transport bar
- Scale mode highlights the scale in the piano roll (root brighter); *Fold* shows only the notes of the scale
- *Convert Project* moves every note of every melodic clip, in the session and the arrangement, to the same scale degree in a new key (A minor's fifth becomes C major's fifth); drum tracks are left alone and Ctrl+Z undoes it
- Scale-aware tools above the piano roll: Snap to scale, move by scale steps, diatonic Chords and 7ths, Arpeggiate (up/down/up-down/random), Strum, Legato, Humanize, and *Seed* to generate a melody in the scale
- Play in key: snap played notes (keyboard and MIDI) to the scale, or let the home row A S D F ... play consecutive scale steps so you can't hit a wrong note

**MIDI effects** (in front of any instrument, including FM and CLAP plugins)
- *Arpeggiator*: Up, Down, Up/Down, Down/Up, Converge, Diverge, Random, As Played and Chord styles; tempo-synced rate (1/32 to 1 bar, triplets), gate, octave range, swing and hold
- *Chord*: one key plays a chord: diatonic triads or sevenths in the project key, major, minor, maj7, min7, dom7, sus2/4, power, octaves, add9 or custom intervals, with inversions
- *Note Length*, *Pitch* (semitones or scale steps), *Scale* (snap or filter, project or custom scale), *Random* (optionally in key), *Velocity* (curve, range, randomize, fixed) and *Note Echo* (tempo-synced MIDI delay with feedback and pitch shift)
- Chain as many as you like; clips, the arrangement, the computer keyboard and MIDI input all go through them, sample-accurately. Recording keeps the notes you played

**Sequencers** (drop them on any MIDI track, in front of any instrument)
- *Step*: a classic 16-step sequencer with a scale note, velocity, length and ratchets (1-4 hits) per step; forward, backward, ping-pong or random direction, swing
- *Acid*: a TB-303 style bass line: note, octave, accent, slide and tie per step, swing, notes kept in key, optional key transpose (hold a key to move the pattern). Slides overlap the notes so a mono synth with glide really slides. *Generate* writes a new line
- *Drum Grid*: an x0x drum machine: 8 lanes by 16 steps with accents, swing and per-lane notes (GM by default), plus classic patterns (four on the floor, breakbeat, electro, boom bap, half time)
- *Progression*: up to 8 chords in the project key (roman numerals, triads, 7ths, sus, power, add9, 9th, inversions) with rhythms from held chords to stabs, Charleston and arpeggios, plus well-known progressions
- And the generative ones:
  - *Polymeter*: a Kria-style step sequencer with five lanes (trigger, scale note, octave, length, probability), each with its own loop length and clock division, so patterns drift against each other
  - *Euclid*: four euclidean rhythm rings (steps, pulses, rotation, note), drawn as circles. Made for drums
  - *Turing Machine*: a looping shift register that slowly mutates into new melodies; lock it when you like what you hear
  - *Life*: Conway's Game of Life on a 16x8 board, played column by column. Draw the seed, watch it evolve
  - *Bounce*: bouncing balls that play a note on every floor hit, speeding up as they lose energy
  - *Drunk*: a random walk through the scale
- They follow the transport, play in the project key and feed the rest of the chain: put an Arpeggiator or Chord after a Turing Machine. Stopping or bypassing one never leaves notes hanging

**Modulation matrix** (something Live doesn't have)
- Project-wide modulators: *LFO* (7 shapes, free or tempo-synced), *Envelope* (ADSR triggered by a track's notes, after its MIDI effects, so sequencers can fire it), *Follower* (follows a track's loudness: sidechain ducking in two clicks), *Steps* (a drawn step sequence), *Macro* (one knob for many), *Notes* (velocity, pitch, gate or a random value per note) and the modifiers *Math* (add, multiply, min, max, compare two inputs) and *Shaper* (gain, offset, curve, quantize, slew, invert, rectify)
- Connect anything to anything: instrument, audio effect and MIDI effect parameters on any track, return or the master, plus volume, pan and sends. Modulators can modulate each other (an envelope that speeds up an LFO, an LFO multiplied by a follower, ...)
- Three ways to connect: drag a modulator chip from the device view onto a knob, press *Map* and click or drag knobs (drag sets the amount), or click a cell in the matrix (rows are sources, columns are targets; drag to set the amount, right-click for invert and more)
- Modulated knobs show a ring per connection in the modulator's color and a dot that moves with the live value; the Mod tab has a scope per modulator
- Runs inside the audio engine at control rate (every 64 samples, about 1.3 ms), so it is also in *Export Audio*

**monome grid and arc** (native serialosc support, no extra software besides serialosc)
- Plug in a grid and Krabton finds it. Three modes: *Session* (launch clips and scenes, stop tracks), *Keys* (an in-key isomorphic keyboard, rows a fourth apart) and *Sequencer* (edit the selected track's sequencer on the grid: draw Life seeds, edit Polymeter lanes, set euclidean pulses, ...)
- An arc controls the first four parameters of the selected track's first MIDI device, or volume, pan and sends; with the Mod tab open it plays the selected modulator
- No grid? The *Grid* tab in the detail view has an on-screen grid that runs the same logic with the mouse
- Varibright LEDs, 8x8, 16x8 and 16x16 grids. Tested against a simulated serialosc; reports from real hardware are welcome

**Instruments**
- **Analog**: a classic analog polysynth, see below
- **Synth**: two PolyBLEP oscillators, sub, noise, unison detune, resonant multimode filter with envelope and key tracking, drive, two ADSRs, LFO, mono/legato with glide. Presets: Fat Bass, Acid, Sub Bass, Soft Pad, Pluck, Lead, Keys
- **Drums**: an 8-pad synthesized drum rack (kick, snare, hats with choke, clap, tom, rim, cowbell) on General MIDI notes, with 808/909/Lo-Fi kits. Every pad can also play a WAV sample
- **Sampler**: plays any WAV chromatically, with ADSR, filter, start offset and loop

**Analog: a classic analog polysynth**
- Two oscillators (saw, pulse, triangle, sine) with octave switches, hard sync, cross modulation, pulse-width modulation and a filter-envelope sweep of oscillator 2 (the classic sync lead), plus a sub oscillator and noise
- A nonlinear 24 dB ladder filter that saturates and self-oscillates, or a 12 dB state-variable filter (low-pass, band-pass, high-pass), with drive, envelope, key tracking and velocity
- RC-style envelopes whose attack overshoots like a capacitor circuit, one LFO (triangle, sine, saw, square, sample & hold) with delay to pitch, filter and pulse width
- Up to 8 voices, unison up to 4 per note (16 voice slots), mono legato with glide, and a VINTAGE control: slowly drifting oscillators and a little spread of filter, tuning and envelope times per voice
- A bucket-brigade-style stereo chorus with the classic I, II and I+II modes
- Runs at twice the project rate with band-limited oscillators (PolyBLEP steps and BLAMP corners, also at sync resets) and a half-band decimator
- 24 presets: Moog-style, sub, squelch and wobble basses, Juno pads and strings, sweep and warm pads, Prophet brass, rave stab, unison saws, keys, pluck, clav, organ, bell, sync/mono/square leads, laser zap, an analog kick, a noise hit and a wonky tape keys sound

**Plaits: the Plaits engine, polyphonic**
- The 24 synthesis models of [Mutable Instruments Plaits](https://pichenettes.github.io/mutable-instruments-documentation/modules/plaits/), built in through the Rust port [mi-plaits-dsp](https://github.com/sourcebox/mi-plaits-dsp-rs) (both MIT): *VA + Filter* (a classic analog voice with a resonant 2/4-pole filter and sub oscillator), *Virtual Analog* (two waveforms with hard sync), *Phase Distortion*, three banks of *6-op FM*, *Wave Terrain*, *String Machine*, *Chiptune*, *Waveshaper*, *FM*, *Formant*, *Additive*, *Wavetable*, *Chords*, *Speech*, *Swarm*, *Noise*, *Particles*, *Inharmonic String*, *Modal Resonator* and analog-style *Bass Drum*, *Snare* and *Hi-Hat*
- Krabton makes it a playable instrument: up to 8 voices with stereo spread, mono legato with glide, an ADSR per voice or the original low-pass gate ("pluck"), and Plaits' own decay envelope routed to timbre, morph and pitch
- The HARMONICS, TIMBRE and MORPH knobs are labelled with what they do in the selected model (resonance, cutoff, waveform, ...); every parameter can be modulated from the matrix
- 22 presets, from Analog Poly, Analog Brass, Analog Bass and an Acid Line to DX Piano, String Machine, Talking Synth and a Plaits drum kit

**Braids: the Braids macro-oscillator, polyphonic**
- A complete Rust port of the DSP of Mutable Instruments Braids (MIT), in the `krabton-braids` crate: all 47 shapes, from CSaw, Morph, Sync, Fold and triple oscillators to ring mod, saw swarm, digital filters, VOSIM, vowels, additive harmonics, three FM flavours, plucked/bowed/blown/fluted physical models, bell, drum, kick, cymbal and snare, wavetables, wave maps, paraphonic chords, filtered/clocked/granular/particle noise and QPSK
- Runs at 96 kHz like the module and is decimated to the project rate, in tune at 44.1 and 48 kHz
- Up to 8 voices, mono legato with glide, ADSR, the module's AD envelope routed to TIMBRE, COLOR and pitch, analog-style VCO drift, and the BITS and RATE lo-fi settings
- Shapes are picked by family with the module's four-letter codes; TIMBRE and COLOR are labelled with what they do in each shape. 32 presets

**Elements: modal synthesis, polyphonic**
- A Rust port of the DSP of [Mutable Instruments Elements](https://pichenettes.github.io/mutable-instruments-documentation/modules/elements/) (MIT), in the `krabton-elements` crate: bow, blow and strike exciters (the blow exciter with its granular noise player and tube, the strike exciter morphing from samples to mallet, plectrum and particles) driving a resonator, followed by the module's reverb
- Three resonator models like the module: *Modal* (52 band-pass modes, from strings and bars to plates and bells), *String* (a Karplus-Strong string with dispersion, down to a buzzing sitar bridge) and *Strings* (five sympathetic strings tuned to a chord picked by GEOMETRY)
- Runs at 32 kHz like the module, resampled to the project rate with a windowed-sinc interpolator, in tune at 44.1 and 48 kHz
- Up to 8 voices share the reverb (the module plays one); notes open the gate, velocity sets the strength (accent). Contour, Space with freeze, the stereo pickup LFO ("Shimmer") and the exciter signature are all on the panel
- 26 presets: marimba, vibraphone, glockenspiel, kalimba, steel drum, bells, gong, wind chimes, rain stick, drums, plucked and bowed strings, harp, sitar, strummed and bowed chords, blown pipe, breath pad and an ambient drone

**FM: a MiniDexed-style DX7 engine, built in**
- 6-operator FM with all 32 DX7 algorithms, operator feedback, 4-stage envelopes with rate and level scaling, velocity sensitivity, LFO with delay and pitch envelope
- Loads DX7 `.syx` banks and single voices, so the thousands of banks made for the DX7, Dexed and MiniDexed work (point *Folder...* at MiniDexed's `sysex/voice` folder); voices are stored inside your set
- MiniDexed-style tone generator controls per track: unison (up to 4 voices) with detune and stereo spread, resonant filter, brightness/attack/release macros, transpose, mono with glide
- Device view with voice and bank browser, an algorithm diagram, an operator table (level, ratio, fine, detune, fixed frequency, velocity, envelope) and single-voice `.syx` export
- Ships with Krabton's own 16 voices (E-piano, basses, brass, bells, organ, pads, ...); the original Yamaha ROM banks are not included
- An original implementation written from the published DX7 behaviour, not a port of the GPL Dexed/MiniDexed code, so it stays MIT

**Plugin instruments (CLAP)**
- Host any CLAP instrument, such as [Dexed](https://asb2m10.github.io/dexed/) (DX7 FM), [Surge XT](https://surge-synthesizer.github.io/) or [Odin 2](https://thewavewarden.com/odin2), with its own editor window
- Plugins are scanned from the standard CLAP folders (and `CLAP_PATH`); add more folders in the browser
- Drag a plugin onto a track (or double-click it), open its editor from the device view, play and record it like the built-in instruments
- The plugin's patch and settings are stored in the `.krab` set, and plugins are included in *Export Audio*
- Krabton does not bundle these plugins: install them yourself (they keep their own licenses, mostly GPL-3)
- Supported so far: CLAP instruments with note input; editors are embedded on Windows, other platforms use the plugin's floating window when available. Effect plugins and VST3 are on the roadmap

**Audio effects**: EQ Three (with kills), Auto Filter (tempo-synced LFO), Compressor (parallel mix), Saturator (soft/hard/fold/tube), Delay (tempo-synced, ping-pong), Reverb, Chorus, Redux (bit crusher), Utility, Oscilloscope

**See the wave**
- Every instrument panel shows the live wave of that instrument, locked to its pitch so it stands still while you turn knobs: a saw opening up through the filter, sync sweeps, the bow on an Elements string. Click it for 1, 2 or 4 cycles; the note it hears is shown below the wave, and the last wave stays on screen after the note ends
- The **Oscilloscope** device goes anywhere in a chain (tracks, returns, master) and passes audio through. *Wave* locks to the pitch (period by a normalized square difference function, window started where the fundamental rises through zero, so even complex waves stand still), *Time* is a classic triggered time base (rising, falling or free, 1 ms to 1 s), *XY* draws left against right (Lissajous figures) and *Vector* draws mid up and side across. Mid, stereo, left or right; auto gain or a fixed gain; phosphor-style afterglow; freeze; a readout of frequency, note, cents and level
- The audio thread writes into lock-free buffers that the UI reads; a track's wave is only captured while its panel is on screen

**Audio clips**: drop WAV files on audio tracks. They are beat-locked in Re-Pitch mode, so they follow the project tempo. The original tempo is guessed automatically.

**Files**: projects are saved as readable JSON (`.krab`). *Export Audio* renders the arrangement or a scene to a 24-bit WAV faster than real time.

## Building

You need a recent stable Rust toolchain ([rustup.rs](https://rustup.rs)).

```sh
git clone https://github.com/daandobber/krabton
cd krabton
cargo run --release
```

Platform notes:
- **Windows**: works out of the box (needs the MSVC build tools that rustup asks for).
- **macOS**: works out of the box.
- **Linux**: install the ALSA and X11/Wayland development packages first, for example on Debian/Ubuntu:
  `sudo apt install libasound2-dev libudev-dev libxkbcommon-dev libgl1-mesa-dev`

Run the engine tests (no audio device or GUI needed):

```sh
cargo test -p krabton-engine -p krabton-monome -p krabton-braids -p krabton-elements
```

## Keyboard shortcuts

| Key | Action |
| --- | --- |
| Space | Play / stop (stop twice returns to the start) |
| Tab | Switch Session / Arrangement |
| Shift+Tab | Switch Clip / Device view (the Mod and Grid tabs are next to them) |
| Esc | Leave modulation map mode |
| A W S E D F T G Y H U J K O L P | Play notes on the armed (or selected) MIDI track |
| Z / X, C / V | Octave down/up, velocity down/up |
| M | Computer MIDI keyboard on/off |
| F9 | Session record |
| Enter | Launch the selected scene |
| B | Draw mode in the piano roll |
| Click / right-click the key in the transport | Key & Scale dialog / scale mode on-off |
| Ctrl+U, Ctrl+A | Quantize notes, select all notes |
| Ctrl+C / Ctrl+X / Ctrl+V / Ctrl+D | Copy, cut, paste, duplicate the selected notes (in the clip view) or clip. Clips paste where you look: into the selected slot in the session, at the insert position on the selected track in the arrangement. Notes paste at the insert marker (click an empty spot) and repeated pastes line up |
| Del | Delete the selected notes or clip |
| Arrow keys | Notes: Left/Right move by the grid (Alt: finer), Up/Down transpose (Shift: octave, in a folded scale: scale steps), Shift+Left/Right length. Arrangement clip: Left/Right move, Up/Down to another track. Session: move the slot selection |
| Drag a clip onto the Arrangement/Session button (or press Tab while dragging) | Switch views mid-drag: session clips drop on arrangement lanes, arrangement clips drop on session slots (as a copy) |
| Ctrl+drag an arrangement clip | Leave a copy behind; drag up or down to move it to another track |
| Ctrl+Z, Ctrl+Y | Undo, redo |
| Ctrl+S, Ctrl+Shift+S, Ctrl+O, Ctrl+N | Save, save as, open, new |
| Ctrl+T, Ctrl+Shift+T, Ctrl+I | New audio track, MIDI track, scene |
| Ctrl+R | Export audio |

## Architecture

The workspace has six crates:

- [`crates/krabton-engine`](crates/krabton-engine): the project model, DSP and the real-time engine. It has no GUI or audio-device dependencies, so it is fully unit tested and can render offline.
- [`crates/krabton-clap`](crates/krabton-clap): CLAP plugin hosting (loading, audio processing, state, editor windows).
- [`crates/krabton-monome`](crates/krabton-monome): a small OSC codec and serialosc client for monome grids and arcs (no dependencies).
- [`crates/krabton-braids`](crates/krabton-braids): the Rust port of the Braids macro-oscillator (fixed-point DSP, no dependencies).
- [`crates/krabton-elements`](crates/krabton-elements): the Rust port of the Elements modal voice (float DSP, no dependencies).
- [`crates/krabton`](crates/krabton): the application, built with [egui](https://github.com/emilk/egui)/eframe, [cpal](https://github.com/RustAudio/cpal) for audio output and [midir](https://github.com/Boddlnagg/midir) for MIDI input.

How the threads talk:

- The UI owns the `Project` and edits it freely. Whenever it changes, an immutable `Arc<Project>` snapshot is sent to the audio thread over a lock-free channel.
- The audio thread keeps its own runtime state (voices, filter and delay memories, playing clips), keyed by track and device ids, so it survives snapshot swaps and reordering.
- Old snapshots and removed runtime state are handed back through a *garbage* channel, so freeing memory never happens on the audio thread.
- Transport position, playing clips and meter peaks flow back to the UI through atomics.
- Audio is processed in blocks of at most 256 frames, split exactly at loop points and launch-quantization boundaries, with sample-accurate note events inside each block.

## Listening to the instruments without the GUI

```sh
cargo run --release -p krabton-engine --example fm_voices -- out_dir            # factory voices
cargo run --release -p krabton-engine --example fm_voices -- out_dir bank.syx   # any DX7 bank
```

Renders a short chord phrase per voice to WAV files and prints their levels. The same for the presets of Analog, Plaits, Braids and Elements:

```sh
cargo run --release -p krabton-engine --example analog_presets -- out_dir
cargo run --release -p krabton-engine --example macro_presets -- out_dir
cargo run --release -p krabton-engine --example braids_presets -- out_dir
cargo run --release -p krabton-engine --example elements_presets -- out_dir
```

## Testing a plugin without the GUI

```sh
cargo run -p krabton-clap --example clap_probe -- "C:/Program Files/Common Files/CLAP/Dexed.clap"
cargo run -p krabton-clap --example engine_probe -- "C:/Program Files/Common Files/CLAP/Dexed.clap"
```

The first plays a chord straight into the plugin and checks its state save/restore; the second runs it through the whole engine (live notes, a launched clip and an offline render).

## Debugging helpers

- `KRABTON_NO_AUDIO=1` runs the engine on a silent real-time thread instead of the audio device.
- `KRABTON_SCRIPT="30:scene=1;60:arrangement;90:devices;120:convert=0,0"` runs UI actions at the given frame numbers (see `run_script` in `app.rs`). Handy for screenshots and manual testing.
- `KRABTON_SERIALOSC_PORT=12102` talks to a serialosc on another port, for example a simulated one.
- Script action `fx=n` adds effect number `n` (in the order of the effect list) to the selected chain, for example `fx=9` for an Oscilloscope.
- Script actions `click=x,y` and `drag=x1,y1,x2,y2` (in UI points) feed pointer events to the Krabton window only, so UI flows can be replayed without touching the desktop.

## Roadmap

Ideas that would make Krabton even more Live-like (contributions welcome):

- Audio input recording and arrangement recording
- Automation lanes and clip envelopes
- Effect plugins and VST3 hosting, plugin parameter automation
- Time-stretching warp modes (Beats, Complex)
- Groove pool, follow actions, MIDI output, MIDI mapping of controls
- More file formats for samples (FLAC, MP3, OGG)

## Credits

- Braids is a port of the Mutable Instruments Braids DSP code by Emilie Gillet (MIT, <https://github.com/pichenettes/eurorack>), with the original copyright and license kept in [`crates/krabton-braids/LICENSE`](crates/krabton-braids/LICENSE).
- Elements is a port of the Mutable Instruments Elements DSP code, lookup tables and exciter samples by Emilie Gillet (MIT, <https://github.com/pichenettes/eurorack>), with the original copyright and license kept in [`crates/krabton-elements/LICENSE`](crates/krabton-elements/LICENSE).
- Plaits runs the DSP code of Mutable Instruments Plaits by Emilie Gillet, in the Rust port `mi-plaits-dsp` by Oliver Rockstedt. Both are MIT licensed; their copyright and license notices travel with the crate. Krabton is not affiliated with Mutable Instruments.
- Analog, Synth and FM are original implementations; FM is DX7-style synthesis and reads standard DX7 sysex files.

## License

MIT, see [LICENSE](LICENSE).
