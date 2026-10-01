# Flash Drum — developer notes

The plugin's source: a VST3 drum machine in Rust, built on a patched copy of [nih-plug](https://github.com/robbert-vdh/nih-plug) (`vendor/nih-plug/`, patches listed in `vendor/nih-plug/FLASH-DRUM-PATCHES.md`) and [egui](https://github.com/emilk/egui) through a patched `egui-baseview` (`vendor/egui-baseview/FLASH-DRUM-PATCHES.md`). Product overview and downloads: the [main README](../README.md).

## Build and test (Windows, PowerShell)

The Rust toolchain is pinned by `rust-toolchain.toml`. Run everything from this folder.

```powershell
cargo check                      # fast type check
cargo test                       # unit and integration tests
cargo run --bin test_standalone  # headless harness: runs the engine without a host
.\build.ps1                      # release build + bundle in build\drum-pattern-vst.vst3
.\build.ps1 -Install             # tests, build, then atomic install into
                                 # C:\Program Files\Common Files\VST3 (close the DAW first)
.\build.ps1 -Installer           # also compiles the Inno Setup installer into ..\dist\
```

- Studio One (and most DAWs) lock the plugin file while it is loaded: `-Install` detects the lock and exits without touching the installed copy.
- The installer needs Inno Setup 6 (`winget install JRSoftware.InnoSetup`); its script is `installer/flash-drum.iss`.
- Before a release, regenerate the third-party license texts from `Cargo.lock`: `python ..\tools\gen_third_party_licenses.py`.

## Layout

| Path | What it holds |
|---|---|
| `src/lib.rs` | Plugin entry: parameters, persisted state, the audio callback `process()` |
| `src/sequencer/`, `src/groove.rs` | 64-step sequencer, microtiming, swing |
| `src/synthesis/` | One file per voice, DSP primitives (`dsp.rs`), sample banks |
| `src/instrument_registry.rs` | The instruments: names, parameters, defaults |
| `src/ui.rs`, `src/ui/` | The egui editor |
| `src/plock.rs` | Per-step parameter locks |
| `src/generator/` | Pattern generators |
| `src/presets.rs` | Instrument / pattern / grid / song presets |
| `installer/` | Windows installer script |

The code is cross-platform by design (Windows-only parts are behind `#[cfg(target_os = "windows")]`); only Windows is built and released for now.
