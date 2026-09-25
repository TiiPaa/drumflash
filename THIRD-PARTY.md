# Third-party components — Flash Drum

Flash Drum is distributed under **GPL-3.0-only** (see `LICENSE`). This file
lists the third-party components it embeds and their licenses. The VST3
export path (`vst3-sys`, GPLv3) requires the shipped binary to be GPL — it is.

## Rust frameworks (compiled in)

| Component | Source | License |
|---|---|---|
| **nih-plug** (VST3 plugin framework) | `vendor/nih-plug/`, patched fork of [robbert-vdh/nih-plug](https://github.com/robbert-vdh/nih-plug) | **ISC** (see `vendor/nih-plug/LICENSE`) |
| **vst3-sys** (VST3 COM bindings) | git dependency, fork `robbert-vdh/vst3-sys` | **GPL-3.0** |
| **baseview** (windowing) | git dependency, [RustAudio/baseview](https://github.com/RustAudio/baseview) | **MIT** |
| **egui-baseview** (egui↔baseview adapter) | `vendor/egui-baseview/`, patched fork (see `FLASH-DRUM-PATCHES.md`) | **MIT** (see `vendor/egui-baseview/LICENSE`) |
| **egui** (immediate-mode UI) | crates.io | **MIT OR Apache-2.0** |
| **clap-sys** (CLAP bindings, unused export) | git dependency, [micahrj/clap-sys](https://github.com/micahrj/clap-sys) | **MIT OR Apache-2.0** |

## Ported DSP code

- **`src/synthesis/ac606/`** — the six AC606 voices are ported from
  analogcode's *606-Inspired-Synth-Drums*, **MIT License**, Copyright (c)
  2026 Matthew Fecher. Full text: `src/synthesis/ac606/LICENSE-MIT.txt`.

## Fonts (embedded in the binary)

- **IBM Plex Sans / IBM Plex Mono** — Copyright © IBM Corp.,
  **SIL Open Font License 1.1**. Source: `assets/fonts/IBMPlexSans.zip`
  (official IBM release, includes the OFL text).

## Audio assets

- **`assets/textures/*.wav`** (Rift textures) — **original work**, generated
  by `tools/gen_textures.py` (fixed seed, committed in this repository).
- **`assets/bd606.wav`, `sd606.wav`, `ch606.wav`, `oh606.wav`** (TR-606
  multisamples) — **original work**: recorded by the author from his own
  Roland TR-606 hardware unit (confirmed 2026-09-25).
- **UI images** (`assets/pads/`, `assets/keycaps/`, `assets/logotype.png`) —
  original work by the author.

## Helper executable

- **`drum-pattern-midi-drag-helper.exe`** — built from this same repository
  (`src/bin/midi_drag_helper.rs`), same GPL-3.0-only license.
