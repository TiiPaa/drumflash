# CLAUDE.md

**This file is the canonical reference for ALL AI agents working on this repo** —
Claude Code, Codex, Kimi, and any other assistant. `AGENTS.md` and other
per-tool entry files redirect here. If something here disagrees with older
markdown (e.g. `docs/historique/`), this file wins; if this file disagrees with
the actual Rust code path, trust the code and update this file.

## What this is

**Flash Drum** — a VST3 drum sequencer plugin written in Rust with `nih-plug` + `egui`.

- **64-step sequencer** (4 pages × 16), master `beat_position`.
- **14 modular slots** (`MAX_TRACKS = 14` / `INSTRUMENT_COUNT = 14`): each slot hosts one selectable instrument **kind** (`TrackInstrumentKind`, 15 kinds).
- **17 synthesis voices** (`DrumVoice::COUNT = 17`): the 13 original voices — Kick, Snare, HiHat, OpenHiHat, Tom1, Tom2, Tom3, Clap, Ride, Cymbal, Snare606, BassDrum808, Perc1 — plus 3 TR-606 **sample-based** voices (BD6smp/SD6smp/CH6smp) and **Buzz** (tonal-percussion + adjustable noise + fast amplitude gate/retrigger).
- **14 stereo aux outs + a main mix** (`AUX_OUT_COUNT = 14`, one per slot).

The active product lives entirely in `drum-pattern-vst/`. The web files (`index.html`, `index.js`, `archive/web-poc/`) are a legacy PoC kept only as a behavioral reference — **do not treat them as the target**.

## Authoritative docs — read before editing

- **This `CLAUDE.md`** is the source of truth for architecture + invariants. Read the relevant sections before any non-trivial change.
- **`ADDING_AN_INSTRUMENT.md`** — the exact step-by-step procedure for adding a synthesis voice. The instrument system is data-driven from `src/instrument_registry.rs`; follow this guide rather than improvising.
- **`TODO.md`** — active priorities and known issues. **`CHANGELOG.md`** — build history (one entry per installed build, newest first).
- **`docs/design/UI-REDESIGN-HANDOFF.md`** — ⚠️ **read before touching `src/ui/`**: the UI redesign's scope, design-fidelity rules, and pitfalls already hit (egui has no blur → no glow halos; a flex slider eats the inline ADSR graph's space). The rendered mockup in `design-pack/Flash_Drum_design_11062026/` is the pixel source of truth.
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

- **Studio One locks the VST3 DLL** — it must be fully closed before `-Install`, or the copy fails with *Access denied*.
- **Run `build.ps1 -Install` PLAINLY in the foreground.** Do not pipe/redirect it: in PowerShell 5.1, `2>&1` / `2>$null` make PS wrap cargo's stderr as a `NativeCommandError` and abort the run; a backgrounded `... 2>$null` once spawned two contending `cargo` processes deadlocked on the build lock. If a build looks stuck: check `Get-Process cargo,rustc`, kill them, re-run plainly.
- `build.ps1` runs `cargo build` in the **current** directory (no `--manifest-path`) — run it from `drum-pattern-vst/`. For raw `cargo`, pass an absolute `--manifest-path`.
- There is no lint config beyond `cargo`'s default warnings. Keep the build **warning-clean**.

## Deployment rule

After completing a task that changes plugin behavior, build and install the VST3 (`build.ps1 -Install`) and update `CHANGELOG.md` with the build ID. Don't mark a task done until it builds and installs cleanly.

### Manual test instructions — MANDATORY after every installed build

Every installed build **MUST** be announced to the user by **ending the report with a numbered "À tester dans Studio One (build <ID>)" checklist** covering what changed in THIS build. Never announce an installed build without it — the user validates every build by hand. Each item states:

1. the **exact manipulation** (which control, which lane/slot, which menu),
2. the **expected result** (what should be heard/seen),
3. when relevant, the **regression to watch for** (what used to go wrong).

## Architecture (layers)

- **`src/lib.rs`** — plugin entry. Declares `DrumFlashParams` (every persisted/automatable param), `AUDIO_IO_LAYOUTS` (Main Mix + `AUX_OUT_COUNT` stereo auxes), the sample loop, and host-transport sync (`Sequencer::sync_to_host` detects seeks). `process()` calls `Sequencer::process_sample` → `DrumSynthesizer::trigger` → mixes into main + aux buffers, emitting NoteOn/NoteOff on MIDI channel 10 (index 9). `apply_choke_groups()` silences same-group slots at trigger time.
- **`src/sequencer/`** — one master `beat_position`; per-track `TrackState` (length, push/pull, humanize). `SharedPattern` is the lock-free `Arc<…>` grid (step masks + fusions) mutated by UI, read by audio; the step bitmask is masked at write time via `INSTRUMENT_COUNT = 14`. Humanize affects **velocity only**, never timing (avoids double triggers). Groove/swing in `src/groove.rs`.
- **`src/synthesis/`** — one DSP file per voice (`kick`, `snare`, `hihat`, `open_hihat`, `tom`, `clap`, `ride`, `cymbal`, `snare606`, `kick_808`, `perc1`, `bd606`, `sd606`, `ch606`, `buzz`) + `dsp.rs` primitives, `saturation.rs`, `sample_bank.rs` (loads the `assets/*.wav` 606 samples). Voice config flows UI → atomics (`SoundSettingsState`, versioned) → audio thread polls per block → `set_settings()` on each voice. `instrument_registry.rs` is the **single source of truth** for the 17 voices (names, MIDI notes, params, defaults); `INSTRUMENTS: [InstrumentDef; DrumVoice::COUNT]`. Typed per-voice settings structs live in `src/synthesis/settings/`.
- **Amplitude envelope model (`DecayReleaseEnvelope`, `dsp.rs`)** — **A-H-D** (Attack-Hold-Decay, **no release stage**), time-based, with **independent bipolar curve shaping** on attack and decay (`shape_curve(e,c)`: `c≥0 → eᶜᵒⁿᵛᵉˣ`, `c<0 → concave`). All amp voices use it. Its param names are kept for persistence compatibility: `decay_curve` = bipolar **decay** curve, `release_curve` = bipolar **attack** curve, `set_release`/`release_time` = **no-op**. Buzz's amp also uses it (machine-gun retrigger via `trigger_hard`). Filter/gate/click envelopes still use `ExpDecayEnvelope`.
- **`src/generator/`** — four pattern generators (probabilistic, markov, euclidean, classic) driven by `Style` + density/variation; output writes back into `SharedPattern`. **Two style systems exist and are distinct**: fixed "preset" chips (`Pattern::xxx_pattern()` in `sequencer/pattern.rs`, wired in `ui/bottom_panel.rs`) vs the Generator's "Style" dropdowns (`enum Style`/`MusicalTemplate` in `generator/styles.rs`).
- **`src/ui.rs` + `src/ui/`** — `egui` editor; `ui.rs` orchestrates, thematic modules in `src/ui/` (`theme.rs` tokens + font helpers `f_sans_*`/`f_mono_*`, `skeuo.rs` centralises all skeuomorphic element rendering, `grid.rs`, `plock.rs`, `sound_editor.rs`, `pattern_bank.rs`, `bottom_panel.rs`, `envelope_viz.rs`, `midi.rs`, …). The Sound Panel and plock menu are **data-driven** from the registry — adding params there usually needs no UI edits. Avoid hardcoding behavior by instrument index; loop over `instrument_registry` instead.
- **`src/plock.rs`** — per-step sound overrides ("parameter locks"). Lock-free atomics keyed **per slot**: `FIELD_COUNT = 46` fields (13 standard + 1 algo + 32 special) × `INSTRUMENT_COUNT = 14` slots × `STEP_COUNT = 64` steps, plus per-step **field masks** so **Link** mode (only the touched fields override; the rest follow live globals) coexists with **Snapshot** mode (all fields copied). Persistence keeps the legacy 18-field layout loadable.
- **`src/midi_export.rs`** — pattern → MIDI (Export + Windows drag helper). Replicates the sequencer: fused-cell groups emit `step_count` pulses over their span; sequencer-plock **stutters** emit N notes over one step.

## Invariants that will silently break things

- **Vendored `nih-plug` (`vendor/nih-plug/`) is patched** for Studio One multi-out and state save/restore parity; `Cargo.toml` points at it on purpose. The fork routes buses via `get_unit_by_bus()`, accepts progressive output activation, remaps sparse S1 aux buffers (`active_output_buses`), allows `num_ins == 0`, links MIDI-in to main out, and saves/restores state on the `IEditController` side too. **Replacing it with the crates.io version silently breaks multi-out and state restore. Do not unvendor.**
- **Real-time audio thread** (`process()` and everything it calls): no allocation, no blocking locks (use atomics / `SharedPattern`), no panic / `unwrap()` on host data, preallocate and reuse buffers. Enforced by review, not tooling.
- **Anti-click**: `trigger()` must not reset oscillator phase, filter state, or reseed noise — continuity prevents retrigger clicks. On `DecayReleaseEnvelope`, use `trigger()` (ramps from the current value → anti-click on a ringing tail) or `trigger_hard()` (from zero, machine-gun/stutter); never set `value = 1.0`. Route freq/cutoff changes through `OnePoleSmoother`. **Never recreate envelopes in `set_settings()`** — use their setters (recreating resets internal state and clicks mid-slider-drag). `DcBlocker` on asymmetric-retrigger voices (kick).
- **Saturation chain**: `SaturationConfig::process_at(pre_stage, x)` is the ONLY way voices apply saturation — called twice (pre- and post-filter), the `pre_filter` flag routes which is active. `settings.volume` multiplies **after** saturation (analog level drift stays pre-sat). Never call `process()` directly from a voice.
- **Choke groups**: per-slot `routing.choke_group` (0 = none, 1..=4), packed into bits 4-6 of the routing byte (`track.rs`); legacy sessions migrate HiHat/OpenHiHat → group 1.
- **Persistence contracts** (all positional — the blob **length IS the version**, never reuse a length or renumber a field):
  - Grid → VST3 state field **`pattern-v5`** (14 slots). Legacy `pattern-v1..v4` and `st01…st16` params migrate on load. Don't rename it or reintroduce `stNN`.
  - Sound settings → **`sound-settings-v2`** blob (per-slot: standard + `special[32]` + `freq_mode`).
  - Plocks → **`plock-v1`** blob (`[values][step_masks][field_masks]`); presets without field masks load as full snapshots.
- **`VST3_CLASS_ID = *b"DrumFlashPlugin1"`** is frozen for V1 to preserve saved-project compatibility. Do not change it.
- When adding a synthesis voice, **`DrumVoice::COUNT` and the registry array must stay in sync**. `AUX_OUT_COUNT` follows the fixed slot pool (`MAX_TRACKS = 14`), not the voice count.
- **Per-slot instances (ST-7)**: special params and the Hz/Notes mode live **per slot** in `SoundSettingsState` (`special[32]` + `freq_mode`), seeded from registry defaults — a new special param only needs its `SpecialParamDef` in the registry plus `set_special_param()` in the voice. The legacy per-voice nih-plug params and `special_param()` are a **migration source only** (`needs_param_seed`) — never read them elsewhere. Everything keyed by lane (settings, plocks, seq-plocks, algo, lane-length locks, mute/solo/mix) is indexed by **slot**; only registry/schema lookups use the voice index derived from the slot's kind (`schema_voice_idx` in `ui.rs`, `kind.drum_voice_index()` elsewhere).

## Portability rule

All Rust code must stay **compile- and run-ready on Windows and macOS**. Keep core DSP/sequencer/UI platform-agnostic; gate Windows-only features (MIDI drag, Win32) behind `#[cfg(target_os = "windows")]` with a macOS fallback; avoid hardcoded paths (`USERPROFILE`) — use `dirs`/`HOME`. `build.ps1` is Windows-only (macOS packaging comes later) but the plugin must always compile on both.

## Workflow rule: "next" / "on continue"

When the user signals they want to move to the next task ("next", "on continue", "qu'est-ce qu'on fait maintenant", or similar): **do not start coding.** Read `TODO.md`, present a curated list of unchecked tasks organized by priority, and wait for the user to explicitly choose before implementing.
