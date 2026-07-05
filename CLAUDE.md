# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

**Flash Drum** — a VST3 drum sequencer plugin written in Rust with `nih-plug` + `egui`. 64-step sequencer (4 pages × 16), 13 modular synthesis voices, 13 stereo aux outs + a main mix. The active product lives entirely in `drum-pattern-vst/`. The web files (`index.html`, `index.js`, `archive/web-poc/`) are a legacy PoC kept only as a behavioral reference — do not treat them as the target.

## Authoritative docs — read before editing

- **`AGENTS.md`** (repo root) — the canonical architecture + invariants guide. Read it before any non-trivial change. The sections below are a summary; `AGENTS.md` is the source of truth.
- **`ADDING_AN_INSTRUMENT.md`** — the exact 12-step procedure for adding a synthesis voice. The instrument system is data-driven from `src/instrument_registry.rs`; follow this guide rather than improvising.
- **`TODO.md`** — active priorities and known issues. **`CHANGELOG.md`** — build history.
- **`docs/design/UI-REDESIGN-HANDOFF.md`** — ⚠️ **read before touching `src/ui/`**: the active UI redesign's scope, design-fidelity rules, and pitfalls already hit (egui has no blur → no glow halos; a flex slider eats the inline ADSR graph's space; `build.ps1 -Install` must run **plainly** with Studio One **closed**). The rendered mockup in `design-pack/Flash_Drum_design_11062026/` is the pixel source of truth.
- Treat other markdown (especially under `docs/historique/`) as potentially stale unless updated alongside code.

## Build / test / run

All commands run from `drum-pattern-vst/` (PowerShell on Windows):

```powershell
.\build.ps1 -Install          # release build → bundle → install to C:\Program Files\Common Files\VST3
.\build.ps1                    # build + bundle only, no system install
.\build.ps1 -Debug             # debug build
cargo check                    # fast type-check
cargo test                     # unit + integration tests
cargo test <name>              # run a single test by substring match
cargo run --bin test_standalone  # headless harness — exercises the engine without nih-plug
```

`build.ps1` stamps `DRUM_PATTERN_BUILD_ID` (a timestamp) into the env so it shows in the plugin UI, builds the `cdylib` to `target/release/drum_pattern_vst.dll`, copies it (plus the `drum-pattern-midi-drag-helper.exe`) into `build/drum-pattern-vst.vst3/Contents/x86_64-win/`, then optionally deploys.

**Studio One locks the VST3 DLL** — it must be fully closed before `-Install`, or the copy fails. There is no lint config beyond `cargo`'s default warnings.

## Deployment rule

After completing a task that changes plugin behavior, build and install the VST3 (`build.ps1 -Install`) and update `CHANGELOG.md` with the build ID. Don't mark a task done until it builds and installs cleanly.

**Every installed build MUST be announced to the user with a numbered "À tester dans Studio One" checklist** — the exact manipulations, expected results, and regressions to watch for, covering what changed in that build. The full requirements and format live in `AGENTS.md` → "Deployment Rule → Manual test instructions" (AGENTS.md is the source of truth; read it).

## Architecture (layers)

- **`src/lib.rs`** — plugin entry. Declares `DrumFlashParams` (every persisted/automatable param), `AUDIO_IO_LAYOUTS` (Main Mix + `AUX_OUT_COUNT` stereo auxes), the sample loop, and host-transport sync. `process()` calls `Sequencer::process_sample` → `DrumSynthesizer::trigger` → mixes into main + aux buffers, emitting NoteOn/NoteOff on MIDI channel 10.
- **`src/sequencer/`** — one master `beat_position`; per-track `TrackState` (length, push/pull, humanize). `SharedPattern` is the lock-free `Arc<…>` grid mutated by UI, read by audio. Humanize affects **velocity only**, never timing. Groove/swing in `src/groove.rs`.
- **`src/synthesis/`** — one DSP file per voice + `dsp.rs` primitives. Voice config flows UI → atomics (`SoundSettingsState`, versioned) → audio thread polls per block → `set_settings()` on each voice. `instrument_registry.rs` is the **single source of truth** for the 13 instruments (names, MIDI notes, params, defaults). Typed per-voice settings structs live in `src/synthesis/settings/`.
- **`src/generator/`** — four pattern generators (probabilistic, markov, euclidean, classic) that write back into `SharedPattern`.
- **`src/ui.rs` + `src/ui/`** — `egui` editor. The Sound Panel and plock menu are **data-driven** from the registry — adding params there usually needs no UI edits. Avoid hardcoding behavior by instrument index; loop over `instrument_registry` instead.
- **`src/plock.rs`** — per-step sound overrides ("parameter locks"), stored in lock-free atomic structures (18 fields × 13 instruments × 16 steps).

## Invariants that will silently break things

- **Vendored `nih-plug` (`vendor/nih-plug/`) is patched** for Studio One multi-out and state save/restore parity. `Cargo.toml` points at it on purpose. Replacing it with the crates.io version silently breaks multi-out and state restore. Do not unvendor.
- **Real-time audio thread** (`process()` and everything it calls): no allocation, no blocking locks (use atomics / `SharedPattern`), no panic / `unwrap()` on host data, preallocate and reuse buffers. Enforced by review, not tooling.
- **Anti-click**: `trigger()` must not reset oscillator phase, filter state, or reseed noise — continuity prevents retrigger clicks. Use `DecayReleaseEnvelope::trigger_at_peak()` (not `value = 1.0`). Route freq/cutoff changes through `OnePoleSmoother`. Never recreate envelopes in `set_settings()` — use their setters (recreating resets internal state and cuts the sound mid-slider-drag).
- **Persistence contract**: the grid is stored in VST3 state field `pattern-v1`. Legacy `st01…st16` params are migrated on load. Don't rename `pattern-v1` or reintroduce `stNN` params — it keeps old Studio One sessions loading.
- **`VST3_CLASS_ID = *b"DrumFlashPlugin1"`** is frozen for V1 to preserve saved-project compatibility. Do not change it.
- When adding a voice, **`DrumVoice::COUNT`, `AUX_OUT_COUNT`, and the registry must stay in sync**.
- **Per-slot instances (ST-7, 2026-07-05)**: special params and the Hz/Notes mode live **per slot** in `SoundSettingsState` (`special[32]` + `freq_mode`), seeded from the registry defaults — new special params only need their `SpecialParamDef` in the registry plus `set_special_param()` in the voice. The legacy per-voice nih-plug params and `special_param()` are a **migration source only** (`needs_param_seed`) — never read them elsewhere. Everything keyed by lane (settings, plocks, algo, lane-length locks) is indexed by **slot**; only registry/schema lookups use the voice index derived from the slot's kind.

## Workflow rule: "next" / "on continue"

When the user signals they want to move to the next task ("next", "on continue", "qu'est-ce qu'on fait maintenant", or similar): **do not start coding.** Read `TODO.md`, present a curated list of unchecked tasks organized by priority, and wait for the user to explicitly choose before implementing.
