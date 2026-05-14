# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project intent

The active product is the **Rust VST3 plugin in `drum-pattern-vst/`**. The files `index.html` and `index.js` at the repo root are a legacy browser PoC kept as a *functional reference* for instrument mapping, presets, sequencing behavior and MIDI export expectations — they are not the shipping product. Do not invest in the web PoC unless a task is explicitly about it.

Authoritative product docs: `PROJECT_BRIEF.md` (scope/V1), `BACKLOG_VST.md` (priorities), `AGENTS.md` (operating rules), `drum-pattern-vst/README.md` (real plugin state).

## Build / test (Windows + PowerShell)

```powershell
# Full build, bundle regen, install to C:\Program Files\Common Files\VST3
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
.\build.ps1 -Install     # add -Debug for debug build
```

`build.ps1` injects a `DRUM_PATTERN_BUILD_ID` env var (timestamp) that ends up displayed in the plugin UI. It builds the `cdylib` to `target/release/drum_pattern_vst.dll`, copies it into the `build/drum-pattern-vst.vst3/Contents/x86_64-win/` bundle, and (with `-Install`) deploys to the system VST3 folder.

Quick checks (run from `drum-pattern-vst/`):

```powershell
cargo check
cargo test                              # full test suite
cargo test persistent_pattern           # single test by name
cargo run --bin test_standalone         # headless audio engine harness
```

There is no lint config beyond `cargo`’s default warnings.

## Architecture — read these before editing

The plugin is a single `nih-plug` VST3 with an internal step sequencer, modular drum synthesis, and an `egui` UI. Layers:

- `src/lib.rs` — plugin entry: declares `DrumFlashParams` (every persisted/automatable param), `AUDIO_IO_LAYOUTS` (Main Mix + 10 stereo aux outs per `DrumVoice`), wires `transport` → `Sequencer::sync_to_host` (detects seeks via 4-beat-circle diff), and runs the sample loop that calls `Sequencer::process_sample` → `DrumSynthesizer::trigger` → mixes into main + aux buffers and emits `NoteOn/NoteOff` on MIDI channel 10 (index 9).
- `src/sequencer/` — `Sequencer` holds one master `beat_position` (0..4 = 1 bar = 16 steps). Per-track state (`TrackState`) supplies `track_length`, `push_pull_ms`, `humanize_amount`. Humanize affects **velocity only**, not timing (avoids double triggers). Groove/swing is applied to the master grid (`src/groove.rs`). `SharedPattern` is the lock-free `Arc<…>` grid mutated by the UI and read by the audio thread.
- `src/synthesis/` — one DSP file per voice (`kick.rs`, `snare.rs`, `hihat.rs`, `open_hihat.rs`, `tom.rs`, `clap.rs`, `ride.rs`, `cymbal.rs`) plus `dsp.rs` primitives. `special_params.rs` and `algos_for/specials_for` describe per-voice algorithm variants exposed to the UI; `DrumVoice::COUNT = 10`. Voice settings come from `SoundSettingsState` (versioned, polled per sample-block in `process()`).
- `src/generator/` — four pattern generators (`probabilistic`, `markov`, `euclidean`, `classic`) driven by `Style` + density/variation params. Output writes back into `SharedPattern`.
- `src/ui.rs` + `src/ui/` — `egui` editor.
- `src/midi_export.rs`, `src/sound_settings.rs`, `src/groove.rs` — auxiliary modules referenced from `lib.rs`.
- `src/bin/test_standalone.rs` — pulls modules via `#[path]` to exercise the engine without `nih-plug`.

### Vendored nih-plug — do not unvendor

`Cargo.toml` resolves `nih_plug` and `nih_plug_egui` to `vendor/nih-plug/`. That copy carries patches required for Studio One multi-out and state save/restore parity. **Replacing it with the crates.io version will silently break multi-out and state restore.** Specifically the local fork:
- routes audio/event bus → root unit via `get_unit_by_bus()`
- accepts progressive output activation in `set_bus_arrangements()`
- allows `num_ins == 0` with a null input audio pointer
- ignores non-activated outputs during buffer validation
- links event/MIDI input to main audio out via `getRoutingInfo()`
- saves/restores plugin state on the `IEditController` side in addition to `IComponent`

### Pattern persistence + legacy migration

The grid is persisted in the VST3 state field **`pattern-v1`** (see `PATTERN_STATE_FIELD` in `lib.rs`), serialized directly from `SharedPattern`. Older builds stored 16 hidden `IntParam`s named `st01`…`st16`; `DrumFlashVst::filter_state` migrates those to `pattern-v1` on load and is covered by `legacy_step_params_migrate_to_persistent_pattern_field`. Don’t reintroduce `stNN` params or rename `pattern-v1` — it’s the contract that keeps existing Studio One sessions loading.

### Frozen VST3 identity

`VST3_CLASS_ID = *b"DrumFlashPlugin1"` is **deliberately stable** to preserve compatibility with saved DAW projects. Do not change it for the V1 line.

## Real-time constraints (audio thread)

Enforced by code review, not by tooling. In `process()` and everything it calls:

- no allocation
- no blocking locks (use atomics / lock-free; e.g. `AtomicU32`, `AtomicBool`, `SharedPattern`)
- no panic — no `unwrap()` on host-supplied data, prefer `Option` handling like the `transport.tempo` fallback in `lib.rs`
- preallocate and reuse buffers

## Documentation discipline

From `AGENTS.md`: do not describe a feature as implemented unless it’s visible in the current Rust code path. Older markdown files (`AGENT_VST_CONVERSION.md`, `VST_CONVERSION_PLAN.md`, `TEST_PLUGIN.md`, `MISE_A_JOUR.md`) may be stale — treat them as history, not as truth about current state. If build/test results conflict with docs, trust the command output and update the doc.
