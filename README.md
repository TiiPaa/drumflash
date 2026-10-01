# Flash Drum

**A free drum machine plugin (VST3, Windows).** Flash Drum started as a sidekick for a Roland TR-606 and grew into a full instrument: a 64-step sequencer driving 14 modular lanes, each one able to host any of 25 instruments — synthesized, sampled, modelled or hybrid.

**[Download the latest version](https://github.com/TiiPaa/flashdrum/releases)** · Beta — feedback and bug reports welcome in [Issues](https://github.com/TiiPaa/flashdrum/issues).

## Sound engines

- **Synthesized** — kicks, snares, hats, toms, claps, cymbals, an 808-style kick, percussion, and SDrex, a metallic snare.
- **Sampled** — a real TR-606, multisampled: every hit picks a different layer, like the hardware.
- **Modelled** — six 606 voices ported from analogcode's open-source models.
- **Hybrid** — Rift slices long textures; One-Shot plays your own samples: drop a WAV file on the grid.

## Sequencer

- 64 steps on 4 pages, a different length per lane for polyrhythms, swing and humanize.
- Per-step sound locks, probability, conditions (1:2, 1:4, not first…), ratchets and microtiming.
- Fused cells: one long note made of pulses, morphing from one sound to another.
- Pattern generators (Euclidean, Markov, probabilistic, genre templates), a 16-pattern bank and a song mode.

## In your DAW

- 14 stereo outputs plus the main mix, choke groups, 16 MIDI-learnable macros.
- Drag your pattern into the DAW as a MIDI clip.
- Presets for instruments, patterns, kits and songs.

## Install

Requirements: Windows 10 or 11 (64-bit) and a VST3 host. Tested in Studio One and REAPER.

1. Download `FlashDrum-Setup-<version>.exe` from the [releases page](https://github.com/TiiPaa/flashdrum/releases) and run it.
2. Windows may show *"Windows protected your PC"*: the installer is not code-signed yet. Click **More info**, then **Run anyway**.
3. Accept the administrator prompt: the plugin goes to `C:\Program Files\Common Files\VST3`, where every DAW looks for VST3 plugins.
4. Rescan your plugins in your DAW.

To uninstall: Windows **Settings → Apps → Installed apps → Flash Drum → Uninstall**.

What changed in each version: [RELEASE-NOTES.md](RELEASE-NOTES.md).

## Build from source

Flash Drum is written in Rust with [nih-plug](https://github.com/robbert-vdh/nih-plug) and [egui](https://github.com/emilk/egui). The toolchain is pinned by `drum-pattern-vst/rust-toolchain.toml`.

```powershell
cd drum-pattern-vst
cargo test                # unit and integration tests
.\build.ps1               # release build + VST3 bundle in build\drum-pattern-vst.vst3
.\build.ps1 -Install      # same, then installs into the system VST3 folder (close your DAW first)
```

More details in [drum-pattern-vst/README.md](drum-pattern-vst/README.md).

## License

Flash Drum is free software under the [GNU General Public License v3](LICENSE). It builds on third-party components listed in [THIRD-PARTY.txt](THIRD-PARTY.txt); their full license texts are in [THIRD-PARTY-LICENSES.txt](THIRD-PARTY-LICENSES.txt).
