# Krabton

**An open-source, Live-inspired DAW written in Rust.** Session view clip launching, an arrangement timeline, built-in synths, drums, a sampler and effects, all in one native binary with no plugins required.

![Krabton session view](docs/screenshot.png)

> Krabton is a hobby project inspired by the workflow of Ableton Live. It is not affiliated with or endorsed by Ableton.

## Install

No programming needed: download the file for your system from the [Releases page](https://github.com/daandobber/krabton/releases).

- **Windows**: `Krabton-Setup-....exe`. Run it and click through (it does not need administrator rights). Krabton appears in the Start menu, and double-clicking a `.krab` set opens it. Windows may warn about an unknown publisher the first time: click *More info*, then *Run anyway*. Prefer no installer? Unzip the portable zip and start `krabton.exe`.
- **Linux**: `Krabton-....AppImage`. Make it executable (right-click > Properties > Allow executing, or `chmod +x Krabton-*.AppImage`) and double-click it. Or unpack the `tar.gz` and run `./install.sh`, which puts Krabton in your applications menu (`./install.sh --uninstall` removes it).
- **Arch Linux**: build the package from source with makepkg:
  ```sh
  git clone https://github.com/daandobber/krabton.git
  cd krabton/packaging/arch && makepkg -si
  ```
- **macOS**: `Krabton-....dmg`. Open it and drag Krabton to Applications. It is not signed by Apple, so the first time right-click Krabton and choose *Open*.

For sound on Linux see [Building](#building) below: Krabton talks to PulseAudio/PipeWire directly, or to ALSA.

## Features

**Session view**
- Clip launcher grid with scenes, per-track stop buttons and *Stop All Clips*
- Global launch quantization (none, 1/16 up to 4 bars), blinking queued clips, progress bars
- Drag clips between slots (Ctrl+drag copies), right-click menus, rename, recolor, duplicate
- Mixer under the grid: sends, pan, volume fader with peak meters, track activator, solo, arm
- Two return tracks (A Reverb, B Delay) out of the box, and a master track
- Stop keeps the launched clips, so Play starts them again, as in Live; *Stop All Clips* or a track's stop button clears them
- New tracks are named after their instrument (Analog, Elements, FM, ...), not after the preset; a name you type stays

**Your layout**
- Drag any divider to make things bigger or smaller: the browser's edge, the top of the detail view (devices and clips), the right edge of a track header (track width), the line above the mixer (mixer height), the arrangement's header edge and the bottom of a lane (lane height), the clip view's left column, and the right edge of the big device displays (Lens EQ, Lens Comp, Lens Limiter, Oscilloscope, the sample editor). Dividers light up under the pointer; small dots show where they are
- *Options > Size of everything* (or Ctrl +, Ctrl -, Ctrl 0) makes the whole interface bigger or smaller
- Sizes and zoom are remembered between sessions

**Browser** (laid out like Live's)
- A sidebar with *Collections* (Favorites and six colors), *Categories* (All, Sounds, Drums, Instruments, Audio Effects, MIDI Effects, Plug-ins, Clips, Samples) and *Places* (Music, the current project's folder and folders you add)
- A *Name* list where devices open up into their presets (Analog, Braids, Drums, Elements, FM with its banks, Plaits, Sampler, Synth) and folders on disk into their samples and DX7 banks
- *Sounds* sorts every preset by the kind of sound (Bass, Keys, Lead, Pad, Strings, Mallets & Bells, ...), *Drums* has the kits and the drum hits, *Clips* the clips of this project to copy elsewhere
- Search finds words anywhere in a name or its folders and opens the folders with results; *All* searches everything
- Double-click or Enter loads an item onto the selected track, or drag it onto a track, slot or the arrangement. A device folder loads the device
- Arrow keys browse (Left/Right close and open folders), 1-7 put the selected item in a collection, 0 takes it out of all of them; right-click for the same
- A preview strip shows the selected sample's waveform and plays it (Preview on/off)
- Collections, Places, plug-in folders, FM banks and the grid rotation are kept between sessions in `prefs.json` (`%APPDATA%\Krabton`, `~/Library/Application Support/Krabton` or `~/.config/krabton`)

**Arrangement view**
- One lane per track with mini note/waveform previews
- Move clips and resize them from either edge (clips loop when stretched), with grid snapping (Alt = free)
- Loop brace, click-to-locate ruler, follow playhead, Ctrl+wheel zoom
- *Scenes to Arrangement* turns a session jam into a song
- An **arranger track** (as in Studio One): song sections (Intro, Verse, Chorus...) above the ruler. Double-click to add one; drag a section to move it with all its music (the song closes up behind it and makes room where it lands), Ctrl+drag to copy it, drag its edge to resize; right-click to rename, loop it, duplicate it, recolour it or delete it with or without its music. Clips across a section's edge are cut in two
- Session clips override the arrangement per track, with *Back to Arrangement* like Live

**Editing and recording**
- Piano roll with draw mode, marquee selection, move/resize, Ctrl+drag duplicate, velocity lane, quantize, transpose, x2 / /2, and a *Preview* switch to hear notes while placing and moving them (off by default; the keys on the left always play)
- Drum mode: the piano roll folds to the pads of the drum rack
- Computer MIDI keyboard (A W S E D F T G Y H U J K ..., Z/X octave, C/V velocity) and hardware MIDI input
- Session record: arm a track, press record (F9), play; notes are overdubbed into the looping clip with optional 1/16 quantization
- Undo/redo for every edit, and a **history** window (Edit > History, Ctrl+H, as in Reaper and Bitwig) that names every step ("Edit notes in Pads", "Bass: Lens EQ", "Add section Chorus") and goes back or forward to any of them with a click
- **Note operators** (as in Bitwig) for the selected notes: a *chance* to play each time, *velocity spread*, *repeats* (ratchets) over the note's length, and *recurrence* (play only on chosen loops of a cycle of 2 to 8). The randomness is repeatable, so an export sounds like playback; the piano roll dims notes with a chance, divides repeated ones and labels them
- **Ghost notes**: the piano roll shows the notes of the other melodic tracks that play alongside (the same scene, or overlapping arrangement clips) faintly in their colours (toggle *Ghosts*)

**Audio recording and routing** (Live's In/Out section)
- Audio tracks record their input: arm the track and click a slot's record button (it starts and stops on the launch quantization grid, then the new clip plays on), or press Record (F9) to record every armed audio track into the session, or onto the arrangement from the play position in Arrangement view
- An In/Out section under the clip slots of the session view, laid out like Live's: *Audio From* with a second chooser for the input channel or tap point (and an input meter), *Monitor* In/Auto/Off, and *Audio To*. Show or hide it with the I/O button next to Master or Ctrl+Alt+I; when space runs short the mixer turns compact
- *Audio From*: *Ext. In* (channel 1, 2, ... or the pair 1/2, 3/4, ...), another track taken *Pre FX*, *Post FX* or *Post Mixer*, *Resampling* (the master output) or *No Input*. So track 1 can be sent into track 2 and recorded there as audio
- *MIDI From* on MIDI tracks: All Ins, Computer Keyboard, the MIDI device or No Input, and All Channels or one channel; with Monitor In a track always plays what you play, with Auto when it is armed (or selected, if nothing is armed)
- *Monitor*: In (always hear the input), Auto (while armed and no clip plays) or Off (record without hearing it)
- *Audio To*: the master or another audio track, which then plays it through its own effects and mixer (a bus)
- Right-click a track header and choose *Record into new audio track* for an armed audio track that listens to it, ready to record
- Tracks are processed so that every track comes after the tracks it listens to, which makes track-to-track routing sample exact; routing that would loop back is not offered. Resampling hears the master one block late and is never monitored, so it cannot feed back
- Recordings are WAV files (24-bit) in a *Recorded* folder next to the saved set, or in Documents/Krabton/Recorded for an unsaved one. The audio interface input opens when a track that listens to it is armed or monitoring, at the output's sample rate; input and output on different clocks are kept in step with a small buffer

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
- *Strum* (chords played note by note, up, down, alternating or random, with a velocity tilt), *Note Repeat* (held notes roll at 1/32 to 1/4, on the grid, with gate and decay), *Humanize* (loose timing, velocity and swing on the 8ths or 16ths), *Note Filter* (by chance, every Nth note, key range and velocity range), *Mono* (one note at a time: last, lowest or highest, with legato) and *Quantize* (late notes moved onto the grid)
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
  - *Bounce*: up to 8 bouncing balls that play a note on every floor hit, speeding up as they lose energy. Each ball has its own note and drop height (from a 16th to two bars: drag them in the editor), hits can snap to a grid, and balls that come to rest drop again on the next beat or bar
  - *Drunk*: a random walk through the scale
  - *Orbits*: up to 8 planets circling at their own speeds; each pass plays its note. Polyrhythms (4 against 3 against 5...) that always meet again on the downbeat
  - *Chaos*: the logistic map as a melody. One knob takes it from a steady note through cycles of 2, 4 and 8 into chaos, drawn as the bifurcation diagram you drag along
  - *Automaton*: a one-dimensional cellular automaton (rules 30, 90, 110 and all 256 others); click the rule's pictures to change it, watch the generations scroll
  - *Ant*: Langton's ant on an 8x8 board (up to three ants), playing the cells it walks over
  - *Pong*: balls bouncing in a box; every wall has its note
  - *Markov*: a melody from chances: repeat, step, leap or rest, leaning up or down, with accents
  - *Rain*: drops at random times and pitches, in drizzles and swelling storms
  - *Words*: type text and it plays: each letter a note (vowels accented), or the whole text as Morse code
  - *Lorenz*: the Lorenz attractor's butterfly, its wings as the melody and rests below a line you drag
- They follow the transport, play in the project key and feed the rest of the chain: put an Arpeggiator or Chord after a Turing Machine. Stopping or bypassing one never leaves notes hanging

**Modulation matrix** (something Live doesn't have)
- Project-wide modulators: *LFO* (7 shapes, free or tempo-synced), *Envelope* (ADSR triggered by a track's notes, after its MIDI effects, so sequencers can fire it), *Follower* (follows a track's loudness: sidechain ducking in two clicks), *Steps* (a drawn step sequence), *Macro* (one knob for many), *Notes* (velocity, pitch, gate or a random value per note) and the modifiers *Math* (add, multiply, min, max, compare two inputs) and *Shaper* (gain, offset, curve, quantize, slew, invert, rectify)
- Connect anything to anything: instrument, audio effect and MIDI effect parameters on any track, return or the master, plus volume, pan and sends. Modulators can modulate each other (an envelope that speeds up an LFO, an LFO multiplied by a follower, ...)
- Three ways to connect: drag a modulator chip from the device view onto a knob, press *Map* and click or drag knobs (drag sets the amount), or click a cell in the matrix (rows are sources, columns are targets; drag to set the amount, right-click for invert and more)
- Modulated knobs show a ring per connection in the modulator's color and a dot that moves with the live value; the Mod tab has a scope per modulator
- Runs inside the audio engine at control rate (every 64 samples, about 1.3 ms), so it is also in *Export Audio*

**monome grid and arc** (native serialosc support, and grids straight over USB serial)
- Plug in a grid and Krabton finds it: through serialosc, or, for grids serialosc does not see (newer and DIY grids that are plain USB serial devices), directly on their serial port with monome's serial protocol. The Grid tab shows which grid is connected and where, and *Rescan* looks again Three modes: *Session* (launch clips and scenes, stop tracks), *Keys* (an in-key isomorphic keyboard, rows a fourth apart) and *Sequencer* (edit the selected track's sequencer on the grid: draw Life seeds, edit Polymeter lanes, set euclidean pulses, ...)
- An arc controls the first four parameters of the selected track's first MIDI device, or volume, pan and sends; with the Mod tab open it plays the selected modulator
- No grid? The *Grid* tab in the detail view has an on-screen grid that runs the same logic with the mouse
- *Rotation* turns the grid (0 or 180 degrees, and 90 or 270 for square grids) when its cable points the other way; keys and LEDs follow
- Varibright LEDs, 8x8, 16x8 and 16x16 grids. Tested against a simulated serialosc and a DIY 16x8 grid over USB serial

**Instruments**
- **Analog**: a classic analog polysynth, see below
- **Modular**: a eurorack-style modular synth in its own rack window (*Open Rack*). Modules sit in 104 HP rows and are patched by dragging cables from jack to jack (drag an input's cable to move it, right-click a jack to unplug it, drag a module by its name to move it). Real eurorack voltages: 1 V/oct (0 V is C4), 10 V gates, ±5 V audio and LFOs, 0-10 V envelopes. 48 modules:
  - *Sources*: VCO (sine, triangle, saw, square, PWM, linear FM, hard sync), Macro VCO (24 synthesis models: virtual analog, FM, formants, chords, speech, strings, modal, drums; with its low-pass gate), Shape VCO (47 digital shapes), Resonator (modal, sympathetic and inharmonic strings, FM voice, chords; struck or fed audio), Pluck (Karplus-Strong string), Noise (white, pink, drift)
  - *Drums*: Bass Drum, Snare Drum, Hi-Hat
  - *Filters & Amps*: VCF (multimode state-variable), Ladder, Formant (vowel filter), VCA, Quad VCA, Folder (wavefolder), Ring Mod, Meta Mod (crossfade, fold, analog and digital ring, XOR, compare, morphing)
  - *Envelopes & LFOs*: ADSR, AD, Function (rise and fall with a curve, triggered or cycling), LFO
  - *Sequencing*: Clock (in time with the song, swing, divisions, bar reset), Sequencer (8 steps), Trigger Seq (16 steps), Euclid, Random (random gates and voltages with bias, spread, steps and deja vu), Shift Register (a looping, slowly changing random sequence), Burst (ratchets), Switch (sequential), Quantizer, Divider, Chance (Bernoulli gate)
  - *Utilities*: S&H, Mixer, Stereo Mixer (level and pan), Atten (attenuverter and offset), Slew, Transpose, Logic (min, max, rectify, compare), Follower (envelope follower), Pulse (trigger to gate, with delay)
  - *Effects & I/O*: Delay, Reverb, Texture (granular: grains, time stretch, looping delay, spectral), Crusher, MIDI-CV (your notes as pitch, gate, velocity and trigger), Macros (eight knobs you can modulate and automate from Krabton) and Audio Out
  - Mono (legato, with clocks and drones running free) or up to 8 voices, each running its own copy of the patch. Patches: Init, Acid Bass, Fat Lead, West Coast, FM Bell, Drone, Generative, Kick, Poly Pluck, Drum Machine (plays by itself), Macro Voice, Texture Pad, Pluck Melody, Struck Resonator
- **Organ**: a tonewheel organ with nine drawbars (the top wheels fold back), single-triggered percussion on the second or third harmonic (fast or slow, soft or normal), key click, the scanner vibrato and chorus (V1-V3, C1-C3), a tube-style drive and a rotating speaker (slow, fast, with inertia). Registrations from Jazz Trio, Green Onions and Gospel to Whiter Shade, Reggae Bubble and Cathedral
- **Winds**, **Strings**, **Mallets** and **Voice**: physical models from the Synthesis ToolKit, each with a model choice and that model's own controls (named after it), polyphonic. Winds: flute, clarinet, saxophone, brass, blown bottle, blow hole and whistle (breath pressure, reed, noise, vibrato). Strings: bowed (bow pressure, position and speed), plucked, mandolin (body, pluck position, detuning), sitar and stiff string. Mallets: marimba, vibraphone, agogo, two woods, reso, beats, two fixed, clump, and the banded waveguides: uniform and tuned bar, glass harmonica and Tibetan bowl. Voice: a formant singer with 32 vowels and consonants
- **Rings**: the Mutable Instruments Rings resonator, played with notes: modal resonator, sympathetic strings, inharmonic string, FM voice, chord strings and a string with reverb, 1, 2 or 4 voices, Structure, Brightness, Damping and Position, and the hidden Disastrous Peace string synth with its formant, chorus, ensemble and reverb effects. Presets from Marimba, Kalimba and Glass Bowl to Sitar, FM Bells and Peace Pad
- **Shakers**: Perry Cook's PhISEM shakers played with notes in the piano roll: maraca, cabasa, sekere, tambourine, sleigh bells, bamboo chimes, sandpaper, a can, crunch, rocks and gravel. The key tunes the resonances (Key Track, Transpose); a note shakes once, a number of times at a rate, or for as long as it is held (Hold), with Humanize, Swell (shakes that come in slowly) and Fade (each shake softer or louder). Objects, Length, Grain, Ring and Scatter shape the shaker itself; a second shaker can be layered on top; Brightness, Low Cut, Drive and a stereo Spread that throws each collision somewhere else; Damp stops a released note. Presets from Maraca Roll and Tambourine Roll to Rain Stick, Bell Tree, Popcorn, Footsteps, Gravel Walk and Shaker Pad
- **Synth**: two PolyBLEP oscillators, sub, noise, unison detune, resonant multimode filter with envelope and key tracking, drive, two ADSRs, LFO, mono/legato with glide. Presets: Fat Bass, Acid, Sub Bass, Soft Pad, Pluck, Lead, Keys
- **Drum instruments**: each is an 8-pad instrument on General MIDI notes with its own voices and kits (presets). Every pad has level, tune, decay, tone and pan plus the knobs of its voice, choke groups (a closed hat stops the open one), and can play a WAV or FLAC sample instead
  - **808**, **909** and **CR-78**: voices after the classic drum machines, built from how their circuits work (no samples). 808: kick, snare, tom, conga, rim, clap, claves, maracas, cowbell, cymbal, hats; kits 808 Classic, Boom, Electro, Latin. 909: swept kick with its attack, snare with snappy, tom, rim, clap, hats; kits 909 Classic, House, Techno, Toms. CR-78: kick, snare, hat, cymbal, bongo, guiro, tambourine, metal; kits CR-78 Classic, Disco, Bossa. Velocity works as accent
  - **606**: the 606's tighter versions of the 808 circuits: kick, snare, low and high tom, six-square hats and cymbal; kits 606 Classic and 606 Acid House
  - **FM Drums**: two-operator FM drums with an FM knob on every pad: kick, snare, tom, hat, bell, zap, perc and noise; kits FM Kit, FM Industrial and FM Glitch
  - **Mutable Drums**: the drum models of Mutable Instruments Plaits: an analog and a synthetic kick, an analog and a synthetic snare, a metallic and a ring-modulated hi-hat, with their knobs (Punch, Dirt, FM, Snappy, Noise); kits Mutable Analog, Synthetic, Industrial
  - **Drum Rack**: any of these voices side by side, Krabton's own Classic drums and a bending electronic tom, the shakers (one shake per hit, the number of objects as a knob; kits Shaker Kit, Hand Percussion, Found Sounds), and samples. *From folder...* turns a folder of drum samples into a kit: kick, snare, clap, rim, hats and toms go to their pads by name, the rest fill the free pads
- **Sampler**: one sample across the keyboard, or a multi-sampled instrument
  - Zones: samples over ranges of keys and velocities, each with its root key, tuning, level, pan, start and end. A zone map shows them over keys and velocities; click one to edit it
  - Load several WAV or FLAC files (or a folder) and they are laid out by the notes in their names (`Piano C4 mf.wav`), with velocity layers from `pp`...`ff` or `v1`, `v2`; files without notes go one per key from C1
  - Plays SFZ instruments (regions, groups, key and velocity ranges, tuning, volume, pan, offsets, loops, amp envelope, `#define` and `#include`; release triggers and extra round robins are left out), so the many free SFZ libraries work. SFZ files show up in the browser
  - *Slice* cuts a loop at its hits, one slice per key from C1, and *One-Shot* mode plays every note to the end (for drums)
  - A sample editor: drag Start (S), End (E) and the loop points on the waveform (Shift for fine moves; loop points snap to zero crossings), Ctrl+wheel to zoom. Loop off, forward (with a crossfade) or back and forth. Start, End and the loop are also knobs, so they can be modulated, and they apply to every zone (a slice keeps its own start and end)
  - ADSR, filter and velocity

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

**Plugins (CLAP)**
- Host any CLAP instrument, such as [Dexed](https://asb2m10.github.io/dexed/) (DX7 FM), [Surge XT](https://surge-synthesizer.github.io/) or [Odin 2](https://thewavewarden.com/odin2), with its own editor window
- And CLAP audio effects (LSP, Dragonfly Reverb, Airwindows, ChowDSP, ...) in the device chain of any track, return or the master, next to the built-in effects
- Plugins are scanned at startup from the standard CLAP folders: `~/.clap`, `/usr/lib/clap` and `/usr/local/lib/clap` on Linux, `Common Files/CLAP` on Windows, `Library/Audio/Plug-Ins/CLAP` on macOS, plus `CLAP_PATH`; add more folders in the browser. The browser lists them by vendor under *Plug-ins* (effects also under *Audio Effects > Plug-ins*), and files that would not load with the reason why
- Drag a plugin onto a track (or double-click it), open its editor from the device view, play and record it like the built-in instruments
- The plugin's patch and settings are stored in the `.krab` set, and plugins are included in *Export Audio*
- Krabton does not bundle these plugins: install them yourself (they keep their own licenses, mostly GPL-3)
- Supported so far: CLAP instruments with note input and stereo (or mono) audio effects; editors are embedded on Windows, other platforms use the plugin's floating window when available. Sidechains, latency compensation and VST3 are on the roadmap

**Audio effects**, in folders by kind as in Live:
- *Delay & Loop*: Delay (tempo-synced, ping-pong), Pitch Delay (echoes pitched in semitone steps, -24 to +24: Climb makes each repeat one more step up or down, so +12 rises an octave per echo; Fixed shifts every echo by the same interval; sync, feedback, tone, ping-pong), Echo (tape echo with wow, flutter, saturation and runaway feedback), Beat Repeat (stutters and rolls in time with the song: interval, offset, gate, grid with variation, chance, decay, pitch decay; mix, insert or gate) and Clouds (the Mutable Instruments Clouds granular processor: granular, time stretch, looping delay and spectral modes, freeze, four buffer qualities, spread, feedback and reverb)
- *Distortion*: Saturator (soft/hard/fold/tube), Redux (bit crusher), Vinyl (crackle, pops, hiss, rumble, wow and wear)
- *Dynamics*: Compressor (parallel mix), Gate (threshold, return, attack, hold, release, floor), Transient (attack and sustain shaping), Multiband (three bands of upward and downward compression, over the top), Lens Comp, Lens Limiter
- *EQ & Filters*: EQ Three (with kills), Auto Filter (tempo-synced LFO), Auto Wah (envelope filter), Lens EQ
- *Modulation*: Chorus, Phaser (2 to 12 stages, feedback, stereo), Flanger, Auto Pan (tremolo to auto-pan, five LFO shapes, sync), Frequency Shifter (shift or ring, wide), Rotary (horn and drum, slow/fast with inertia)
- *Pitch*: Shifter (pitch shifter, two octaves either way, feedback, stereo detune)
- *Reverb & Resonance*: Reverb, Plate (Dattorro's plate: dense, long, modulated, with freeze), Hall (an eight-line feedback delay network: decays from 0.3 s to a minute, separate low decay, damping, modulation, freeze), Shimmer (the hall with a pitch shifter in its loop: every pass an octave, a fifth or any interval up or down), Spring (a spring tank: chirp, boing and drips), Resonators (five tuned combs on a note and a chord), Rings (the Rings resonator excited by the track's audio, tuned to a note)
- *Utilities*: Utility, Oscilloscope
- *Airwindows*: fifteen effects by Chris Johnson: Galactic, Verbity2 and Chamber (reverbs), ToTape6 and IronOxide5 (tape), PurestDrive, Density2, Tube2 and Mackity (saturation), Pressure4 and ButterComp2 (compression), Air3 and Capacitor2 (tone), DeRez2 (lo-fi) and TapeDelay2
- LFOs sync to the song, and Beat Repeat follows the transport

**Lens EQ, Lens Comp, Lens Limiter** (in the style of the big visual mixing plug-ins)
- *Lens EQ*: twelve bands on a big interactive curve over a real-time spectrum analyzer (before and/or after the EQ, with the usual 4.5 dB/oct tilt). Double-click to add a band (cuts near the edges), drag a dot for frequency and gain (Shift for fine), scroll for Q, Alt+click to bypass, double-click a dot to remove it, right-click for shape, slope, stereo and dynamics. Shapes: bell, low/high shelf, tilt, low/high cut from 6 to 96 dB/oct with resonance, notch and band pass. Every band can work on stereo, left, right, mid or side, and can be *dynamic*: its gain moves by up to its range as the level in its own frequency region crosses the threshold, and the display shows where it is right now. The filters are matched to their analog shapes, so a bell at 15 kHz keeps its shape up to Nyquist instead of being squeezed, and the drawn curve is computed from the same filters the audio runs through
- *Lens Comp*: eight styles (Clean, Classic feedback, Opto with program-dependent release, Vocal, Mastering, Bus, Punch, Pumping), threshold, ratio, knee, range, attack, release with auto release, lookahead, hold, sidechain high-pass, stereo link, mix and auto gain; a scrolling display of input, output and gain reduction with the threshold, a transfer curve with the current level, and a gain reduction meter
- *Lens Limiter*: lookahead brickwall limiting whose gain ramps down before each peak, true-peak detection (4x), eight release styles (Transparent, Punchy, Dynamic, Aggressive, Allround, Bus, Safe, Modern), channel link and unity-gain monitoring; an EBU R128 loudness meter (momentary, short-term and gated integrated LUFS, true peak) next to a scrolling level and gain reduction display
- Krabton is not affiliated with FabFilter; these are its own implementations

**See the sound**
- The **Oscilloscope** device goes anywhere in a chain (tracks, returns, master) and passes audio through. *Wave* locks to the pitch (period by a normalized square difference function, window started where the fundamental rises through zero, so even complex waves stand still), *Time* is a classic triggered time base (rising, falling or free, 1 ms to 1 s), *XY* draws left against right (Lissajous figures), *Vector* draws mid up and side across, and *Spectrum* shows the level per frequency (20 Hz to 20 kHz, with the loudest peak and its note). Mid, stereo, left or right; auto gain or a fixed gain; phosphor-style afterglow; freeze; a readout of frequency, note, cents and level. Its look is yours: colour (green, amber, ice, pink, white), line width, fill, grid, the phase where the locked wave starts, and its width (drag the right edge of the screen)
- The audio thread writes into lock-free buffers that the UI reads, so watching costs the audio nothing

**Audio clips**: drop WAV files on audio tracks. They are beat-locked in Re-Pitch mode, so they follow the project tempo. The original tempo is guessed automatically.

**Files**: projects are saved as readable JSON (`.krab`). *Export Audio* renders the arrangement or a scene to a 24-bit WAV faster than real time.

## Building

To run it from source you need a recent stable Rust toolchain ([rustup.rs](https://rustup.rs)).

```sh
git clone https://github.com/daandobber/krabton
cd krabton
cargo run --release
```

Platform notes:
- **Windows**: works out of the box (needs the MSVC build tools that rustup asks for).
- **macOS**: works out of the box.
- **Linux**: Krabton plays straight to PulseAudio or PipeWire (through pipewire-pulse, which desktop installs have), loading the PulseAudio client library when it starts, so `cargo run` just plays. Without a sound server it uses ALSA. MIDI goes through ALSA. Build packages:
  - Arch: `sudo pacman -S --needed base-devel rustup alsa-lib libpulse libxkbcommon mesa`, then `rustup default stable`
  - Debian/Ubuntu: `sudo apt install build-essential libasound2-dev libpulse0 libxkbcommon-dev libgl1-mesa-dev`
  - Fedora: `sudo dnf install alsa-lib-devel pulseaudio-libs libxkbcommon-devel mesa-libGL-devel`
  - A monome grid on a serial port needs permission to open it: `sudo usermod -aG uucp $USER` on Arch (`dialout` on Debian/Ubuntu and Fedora), then log out and in

**No sound on Linux?** The status bar (bottom right) shows where Krabton plays: `PulseAudio / PipeWire` when it reached the sound server, an ALSA device name otherwise, or that it found nothing (*Options > Audio output* then says why).
- Krabton needs the PulseAudio client library (`libpulse`) and a running server: PulseAudio, or PipeWire with `pipewire-pulse` (check with `pactl info`)
- Without them it opens ALSA's `default` device, which only reaches the sound server through its ALSA plugin (`pipewire-alsa` or `pulseaudio-alsa`); otherwise ALSA tries the sound card itself, which the server already holds
- Pick a device in *Options > Audio output* (takes effect after a restart), or start with `KRABTON_AUDIO_DEVICE=pipewire cargo run --release`

Run the engine tests (no audio device or GUI needed):

```sh
cargo test -p krabton-engine -p krabton-monome -p krabton-braids -p krabton-elements
```

### Making the installers

The [Release workflow](.github/workflows/release.yml) builds them on GitHub for every version tag (`git tag v0.2.0 && git push origin v0.2.0`) and publishes them as a release; run it by hand under *Actions* to just get the files. To make them yourself:

- **Windows**: `cargo build --release -p krabton`, then compile [`packaging/windows/krabton.iss`](packaging/windows/krabton.iss) with [Inno Setup 6](https://jrsoftware.org/isinfo.php) (`iscc /DMyAppVersion=0.2.0 packaging\windows\krabton.iss`); the installer lands in `dist\`.
- **Linux**: see the AppImage and tarball steps in the workflow; [`packaging/linux`](packaging/linux) has the desktop entry and `install.sh`.
- **macOS**: see the workflow: a universal binary in `Krabton.app` with [`packaging/macos/Info.plist`](packaging/macos/Info.plist), in a disk image.

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
| F9 | Record (session, or the arrangement in Arrangement view) |
| Enter | Launch the selected scene (with the browser focused: load the selected item) |
| Ctrl+F | Search in the browser (Down goes from the search field to the results) |
| Ctrl+Alt+B | Show or hide the browser |
| Ctrl+H | Undo history |
| Ctrl +, Ctrl -, Ctrl 0 | Make everything bigger, smaller, or normal size |
| Arrow keys, Enter, 1-7, 0 in the browser | Browse, open and close folders, load; put the selected item in a collection, take it out of all of them |
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
| Ctrl+Alt+I | Show or hide the In/Out section |
| Ctrl+R | Export audio |

## Architecture

The workspace has six crates:

- [`crates/krabton-engine`](crates/krabton-engine): the project model, DSP and the real-time engine. It has no GUI or audio-device dependencies, so it is fully unit tested and can render offline.
- [`crates/krabton-clap`](crates/krabton-clap): CLAP plugin hosting (loading, audio processing, state, editor windows).
- [`crates/krabton-monome`](crates/krabton-monome): a small OSC codec and serialosc client for monome grids and arcs, plus monome's serial protocol for grids on a USB serial port (with [serialport](https://github.com/serialport/serialport-rs)).
- [`crates/krabton-braids`](crates/krabton-braids): the Rust port of the Braids macro-oscillator (fixed-point DSP, no dependencies).
- [`crates/krabton-elements`](crates/krabton-elements): the Rust port of the Elements modal voice (float DSP, no dependencies).
- [`crates/krabton-rings`](crates/krabton-rings), [`crates/krabton-clouds`](crates/krabton-clouds): Rust ports of the Rings resonator and the Clouds granular processor (no dependencies; they resample to and from the host rate).
- [`crates/krabton-airwindows`](crates/krabton-airwindows): Rust ports of fifteen Airwindows effects.
- [`crates/krabton-stk`](crates/krabton-stk): Rust ports of physical models from the Synthesis ToolKit.
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

For an effect plugin, `fx_probe` runs a test signal through it and then puts it on the master of the demo set:

```sh
cargo run -p krabton-clap --example fx_probe -- /usr/lib/clap/lsp-plugins.clap
```

`swap_fx` is a tiny CLAP effect for testing the host (it swaps left and right at half level): `cargo build -p krabton-clap --example swap_fx` builds `target/debug/examples/swap_fx.dll` (`libswap_fx.so` on Linux); rename it to `.clap` or pass it to `fx_probe` as it is.

## Debugging helpers

- `KRABTON_NO_AUDIO=1` runs the engine on a silent real-time thread instead of the audio device.
- `KRABTON_AUDIO_DEVICE=name` plays on that output device (as listed in *Options > Audio output*); if it does not open, Krabton falls back to the default and the others.
- `KRABTON_SCRIPT="30:scene=1;60:arrangement;90:devices;120:convert=0,0"` runs UI actions at the given frame numbers (see `run_script` in `app.rs`). Handy for screenshots and manual testing.
- `KRABTON_SERIALOSC_PORT=12102` talks to a serialosc on another port, for example a simulated one.
- Test windows (`KRABTON_TEST_WINDOW`, or a custom serialosc port) never open grids on serial ports, so they cannot take over a real grid; `KRABTON_SERIAL_GRID=1` allows it. `cargo run -p krabton-monome --example serial_probe` lists the serial ports that look like grids and asks them for their id and size, without lighting anything.
- Test windows keep their settings in memory; `KRABTON_PREFS=path/to/prefs.json` uses another settings file.
- Script actions `routetrack=i` (a new audio track listening to track `i`) and `recslot=t,s` (start or stop recording track `t` into slot `s`) drive audio recording.
- Script action `fx=n` adds effect number `n` (in the order of the effect list) to the selected chain, for example `fx=9` for an Oscilloscope.
- Script actions `click=x,y`, `dclick=x,y`, `rclick=x,y`, `drag=x1,y1,x2,y2` (in UI points), `press=ctrl+f` and `type=text` feed input to the Krabton window only, so UI flows can be replayed without touching the desktop. `quit` closes the window cleanly (a connected grid goes dark).

## Roadmap

Ideas that would make Krabton even more Live-like (contributions welcome):

- Automation lanes and clip envelopes
- Effect plugins and VST3 hosting, plugin parameter automation
- Time-stretching warp modes (Beats, Complex)
- Groove pool, follow actions, MIDI output, MIDI mapping of controls
- More file formats for samples (AIFF, MP3, OGG), and streaming large sample libraries from disk instead of loading them into memory

## Credits

- Braids is a port of the Mutable Instruments Braids DSP code by Emilie Gillet (MIT, <https://github.com/pichenettes/eurorack>), with the original copyright and license kept in [`crates/krabton-braids/LICENSE`](crates/krabton-braids/LICENSE).
- Elements is a port of the Mutable Instruments Elements DSP code, lookup tables and exciter samples by Emilie Gillet (MIT, <https://github.com/pichenettes/eurorack>), with the original copyright and license kept in [`crates/krabton-elements/LICENSE`](crates/krabton-elements/LICENSE).
- Plaits runs the DSP code of Mutable Instruments Plaits by Emilie Gillet, in the Rust port `mi-plaits-dsp` by Oliver Rockstedt. Both are MIT licensed; their copyright and license notices travel with the crate. Krabton is not affiliated with Mutable Instruments.
- The Mutable voices of Drums use the drum models of Plaits from the same `mi-plaits-dsp` crate.
- The Machine voices of Drums are original implementations after the published workings of the drum machines' circuits; no samples are used. 808, 909 and CR-78 refer to the instruments they imitate; Krabton is not affiliated with their makers.
- FLAC files are decoded with [claxon](https://github.com/ruuda/claxon) (Apache-2.0). SFZ is an open format for sampled instruments (<https://sfzformat.com>).
- Shakers (and the shakers of the Drum Rack) are an implementation of Perry Cook's PhISEM (Physically Informed Stochastic Event Modeling, Computer Music Journal 21:3, 1997), with instrument settings after the Shakers of his Synthesis ToolKit (STK).
- Analog, Synth and FM are original implementations; FM is DX7-style synthesis and reads standard DX7 sysex files.
- Winds, Strings, Mallets and Voice are ports of instruments from the Synthesis ToolKit in C++ (STK) by Perry R. Cook and Gary P. Scavone (MIT-style license, <https://github.com/thestk/stk>; BandedWG by Georg Essl), including its rawwave samples for the mandolin body and the marimba strike; the notice is in [`crates/krabton-stk`](crates/krabton-stk/src/lib.rs).
- Rings is a port of the Mutable Instruments Rings DSP code by Emilie Gillet (MIT, <https://github.com/pichenettes/eurorack>), with the original copyright and license kept in [`crates/krabton-rings/LICENSE`](crates/krabton-rings/LICENSE). Clouds is a port of the Mutable Instruments Clouds DSP code by the same author (MIT), see [`crates/krabton-clouds/LICENSE`](crates/krabton-clouds/LICENSE). Krabton is not affiliated with Mutable Instruments.
- The Airwindows effects are ports of Chris Johnson's Airwindows plugins (MIT, <https://github.com/airwindows/airwindows>); the notice is in [`crates/krabton-airwindows`](crates/krabton-airwindows/src/lib.rs).
- Modular and its modules are original implementations, after the way eurorack modules work, except Macro VCO, Shape VCO, Resonator, Texture and the three drums, which run the Plaits, Braids, Rings, Clouds and Plaits drum code of Mutable Instruments (MIT, see above).
- Plate follows Jon Dattorro's "Effect Design, Part 1" (J. Audio Eng. Soc., 1997). Organ, the 606 and FM drums, Hall, Shimmer, Spring, Pitch Delay, Phaser, Flanger, Auto Pan, Frequency Shifter (with Olli Niemitalo's Hilbert transform coefficients), Rotary, Gate, Transient, Multiband, Echo, Beat Repeat, Shifter, Auto Wah, Resonators, Vinyl and the MIDI effects are original implementations.

## License

MIT, see [LICENSE](LICENSE).
