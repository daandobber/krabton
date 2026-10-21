# What is in `vendor/`, and where it comes from

The Virus emulation of [gearmulator](https://github.com/dsp56300/gearmulator) by
The Usual Suspects (the code behind their OsTIrus and Osirus plugins), without
its plugin and user-interface parts. Copied as it is (no changes), the tests and
build files left out; Krabton's own glue is in `cpp/` and `src/`.

| Folder | From | Version | License |
|---|---|---|---|
| `gearmulator/baseLib`, `gearmulator/synthLib`, `gearmulator/virusLib` | gearmulator, `source/framework/…` and `source/axel/virusLib` | commit `b8e1a28d4efaa3764d875eb3118d881dba672a70` (2026-10-20) | GPL-3.0 (`gearmulator/LICENSE.md`) |
| `dsp56300/dsp56kBase`, `dsp56300/dsp56kEmu` | [dsp56300](https://github.com/dsp56300/dsp56300), the Motorola DSP56300 emulator | commit `bf222a32c2bc19df3200f7a02a84e2dd2008f9f3` (gearmulator's submodule) | GPL-3.0 (`dsp56300/LICENSE.md`) |
| `asmjit/asmjit` | [asmjit](https://github.com/dsp56300/asmjit) (The Usual Suspects' fork), the emulator's JIT | commit `3577608cab0bc509f856ebf6e41b2f9d9f71acc4` (dsp56300's submodule) | Zlib (`asmjit/LICENSE.md`) |
| `libresample` | libresample by Dominic Mazzoni, as gearmulator carries it (`source/3rdparty/libresample`) | the same gearmulator commit | BSD or LGPL (`libresample/LICENSE-*.txt`) |

Because GPL-3.0 code is linked in, the Krabton program as a whole is distributed
under the GPL-3.0; Krabton's own source stays under its MIT license, which the
GPL-3.0 allows.

**No firmware is included.** The emulation runs the original firmware of a Virus
(a Virus TI's, or a Virus A, B or C operating system), which is copyrighted by its
maker and cannot be passed on: every player brings their own.

To update: copy the same folders from newer commits (the script that did this
copies `*.cpp`, `*.h`, `*.inl`, `*.c` and `*.hpp` and leaves out `*_test.cpp`),
and change the commits above.
