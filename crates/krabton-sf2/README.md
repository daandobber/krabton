# krabton-sf2

The synthesizer behind Krabton's SF2 Player: [rustysynth](https://github.com/sinshu/rustysynth)
1.3.7 by Nobuaki Tanaka (MIT, see [LICENSE](LICENSE)), a Rust port of his MeltySynth, with
Krabton's changes:

- `Shape` and `Synthesizer::set_shape`: per channel, changes to every voice on top of what
  the bank's preset sets: the volume envelope's times and sustain, the filter's cutoff and
  resonance, a filter envelope of its own, velocity to cutoff, vibrato rate, depth and delay,
  tremolo, filter wobble and the modulation LFO's rate, fine tuning, stereo width and where
  the sample starts. Most of them follow the knob while a note sounds.

The MIDI file player of the original is left as it was. Two of the original's tests read sample
files from its repository, which its crate does not ship, so they are left out.
