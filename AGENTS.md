# Flash Drum - Agent Guide

## Project Overview

Primary goal: build a VST3 drum sequencer plugin in Rust.

The web app in `index.html` and `index.js` is a legacy PoC and functional reference. It is not the target product anymore.

Authoritative product docs: `TODO.md` (active priorities), `drum-pattern-vst/README.md` (real plugin state), `CHANGELOG.md` (build history). Older scope/backlog notes live under `docs/historique/` and are historical references, not the active source of truth.

## Current Project Layout

- `drum-pattern-vst/` - primary implementation target
- `index.html` - browser PoC used as behavioral reference
- `index.js` - modular PoC variant, not the main product
- `TODO.md` - active priorities and known issues
- `CHANGELOG.md` - build history and validation notes
- `docs/historique/PROJECT_BRIEF.md` - historical V1 scope reference
- `docs/historique/BACKLOG_VST.md` - historical backlog reference
- `ADDING_AN_INSTRUMENT.md` - guide for adding new synthesis voices

## Development Priority

When changing product behavior, prioritize the Rust plugin unless the task is explicitly about the PoC.

## Deployment Rule

After every task completion, **systematically build and install** the VST3 plugin. Do not mark a task as done until the build is installed and ready to test in Studio One.

- Run `build.ps1 -Install`
- If the build fails, fix before marking done
- If Studio One is open and locks the DLL, close it before installing
- Update CHANGELOG.md with the build ID and changes

### Manual test instructions — MANDATORY after every installed build

Every time a build is installed, **end the report to the user with a section
"À tester dans Studio One"**: a short numbered checklist of the precise manual
tests that cover what changed in THIS build. Never announce an installed build
without it — the user validates every build by hand and must know exactly what
to exercise.

Each checklist item must state:
1. the exact manipulation (which control, which lane/slot, which menu),
2. the expected result (what should be heard/seen),
3. when relevant, the regression to watch for (what used to go wrong).

Example format:

> **À tester dans Studio One (build 20260704-XXXXXX)**
> 1. Ajoute une 2e BD via la pastille `+5`, change son Click Type dans SAT →
>    celui de la lane 1 ne doit PAS bouger (bug précédent : partagé).
> 2. Recharge une song sauvegardée avant cette build → les réglages
>    click/saturation existants doivent être conservés (migration).

Use the web files mainly to:
- confirm instrument mapping
- confirm preset content
- compare sequencing behavior
- compare export expectations

## Plugin Technical Focus

- Framework: Rust + `nih-plug`
- Primary entry point: `drum-pattern-vst/src/lib.rs`
- Sequencer logic: `drum-pattern-vst/src/sequencer/`
- Synthesis: `drum-pattern-vst/src/synthesis/`
- UI work: `drum-pattern-vst/src/ui.rs`

## UI Redesign (active) — read before editing `src/ui/`

A visual reskin of the existing **fixed 13-voice** plugin to the designer mockup is in progress.
**Read `docs/design/UI-REDESIGN-HANDOFF.md` first** — scope, design-fidelity rules, remaining worklist, and pitfalls already hit. Non-negotiables:

- **Source of truth = the rendered mockup** `design-pack/Flash_Drum_design_11062026/flash-drum-source/` (`index.html` + `assets/fd-base.css` + `assets/fd-core.js`). Where `DESIGN-SYSTEM.md`/`LAYOUT.md` disagree with the rendered CSS/JS, the mockup wins.
- **Scope = visual only** on the 13 fixed voices. The modular lanes / Sample / MIDI-Out / song-arranger architecture is a **later phase** — don't start it as part of "apply the design".
- **Transport (play/stop/rec) was removed from the header on purpose** (a VST follows host transport); `src/ui/schema.rs` + `src/ui/engine_registry.rs` were deleted as dead stubs. Don't reintroduce them.
- Use `theme.rs` tokens + the font-weight helpers (`f_sans_med/_sb/_bold`, `f_mono_med/_sb`) — never faux-bold via `.strong()`.

### Pitfalls already hit (do not repeat)
- **egui has no blur**: don't fake `box-shadow`/glow with an expanded translucent rect — on adjacent step cells the hard halos overlap and smear. Use flat fills + crisp 1px borders.
- **Flex widths eat inline neighbours**: a slider track sized from `ui.available_width()` consumed the space reserved for the inline ADSR graph (graph vanished, sliders stretched). Constrain a section's params-column width (`ui.set_max_width`) when it has an inline graph; constrain standalone rows (Volume) to the same width.
- **`add_sized(W, Label)` centers** the label — use a left-to-right layout (`editor_label`) for left-aligned form labels.
- **Skip empty sections** — no orphan section title when an instrument lacks that family's params.

## Architecture — read these before editing

The plugin is a single `nih-plug` VST3 with an internal step sequencer, modular drum synthesis, and an `egui` UI. Layers:

- `src/lib.rs` — plugin entry: declares `DrumFlashParams` (every persisted/automatable param), `AUDIO_IO_LAYOUTS` (Main Mix + `AUX_OUT_COUNT = 14` stereo aux outs for `MAX_TRACKS = 14` slots. `DrumVoice::COUNT = 13` legacy synthesis voices: Kick, Snare, HiHat, OpenHiHat, Tom1, Tom2, Tom3, Clap, Ride, Cymbal, Snare606, BassDrum808, Zap). Wires `transport` → `Sequencer::sync_to_host` (detects seeks via 4-beat-circle diff), and runs the sample loop that calls `Sequencer::process_sample` → `DrumSynthesizer::trigger` → mixes into main + aux buffers and emits `NoteOn/NoteOff` on MIDI channel 10 (index 9).
- `src/sequencer/` — `Sequencer` holds one master `beat_position` (0..4 = 1 bar = 16 steps). Per-track state (`TrackState`) supplies `track_length`, `push_pull_ms`, `humanize_amount`. Humanize affects **velocity only**, not timing (avoids double triggers). Groove/swing is applied to the master grid (`src/groove.rs`). `SharedPattern` is the lock-free `Arc<…>` grid mutated by the UI and read by the audio thread. Step bitmask uses 13 bits (one per voice), masked at write time via `INSTRUMENT_COUNT = 13`.
- `src/synthesis/` — one DSP file per voice (`kick.rs`, `snare.rs`, `hihat.rs`, `open_hihat.rs`, `tom.rs`, `clap.rs`, `ride.rs`, `cymbal.rs`, `snare606.rs`, `kick_808.rs`, `zap.rs`) plus `dsp.rs` primitives. `algos_for` describes per-voice algorithm variants exposed to the UI. Voice settings come from `SoundSettingsState` (versioned, polled per sample-block in `process()`). The amplitude envelope (`DecayReleaseEnvelope`) is bi-stage with `max(decay, release)` crossover; snare/HH/OH/Snare 606 also use a Hold phase between attack and decay.
- `src/generator/` — four pattern generators (`probabilistic`, `markov`, `euclidean`, `classic`) driven by `Style` + density/variation params. Output writes back into `SharedPattern`.
- `src/ui.rs` + `src/ui/` — `egui` editor.
- `src/midi_export.rs`, `src/sound_settings.rs`, `src/groove.rs` — auxiliary modules referenced from `lib.rs`.
- `src/bin/test_standalone.rs` — pulls modules via `#[path]` to exercise the engine without `nih-plug`.

### Vendored nih-plug — do not unvendor

`Cargo.toml` resolves `nih_plug` and `nih_plug_egui` to `vendor/nih-plug/`. That copy carries patches required for Studio One multi-out and state save/restore parity. **Replacing it with the crates.io version will silently break multi-out and state restore.** Specifically the local fork:
- routes audio/event bus → root unit via `get_unit_by_bus()`
- accepts progressive output activation in `set_bus_arrangements()`
- remaps sparse Studio One aux buffers via `active_output_buses` / `mapped_aux_output_idx()` instead of assuming active outputs are a prefix
- allows `num_ins == 0` with a null input audio pointer
- ignores non-activated outputs during buffer validation
- links event/MIDI input to main audio out via `getRoutingInfo()`
- saves/restores plugin state on the `IEditController` side in addition to `IComponent`

### Anti-click conventions (audio thread invariants)

- `trigger()` of any voice **must not** reset oscillator phase, filter state or
  reseed the noise generator — analog-style continuity preserves smoothness on
  retriggers during a ringing tail. See `src/synthesis/kick.rs` and friends.
- `DecayReleaseEnvelope::trigger_at_peak(peak)` ramps from the current value
  to `peak` (or skips ramp if already above peak). Direct `value = 1.0` jumps
  on retrigger cause audible clicks.
- Frequency / cutoff changes go through `OnePoleSmoother` to absorb sub-sample
  discontinuities — see kick's `freq_smoother` and the velocity smoothers in
  `DrumSynthesizer::process_voice_samples`.
- `DcBlocker` on voices with asymmetric retriggers (kick) catches the DC drift
  that builds up over dense patterns.

### Parameter Locks (plocks)

`src/plock.rs` stores per-step sound overrides via three lock-free structures:
- `PlockMasks` (`AtomicU16`) — one bit per step indicating whether a plock exists at all.
- `PlockValues` (`AtomicU32` bitcast f32) — 18 fields per instrument × step.
- `PlockFieldMasks` (`AtomicU32`) — **18-bit mask per instrument × step** tracking which individual fields have been explicitly overridden.

Creation modes:
- **Snapshot** (`set_settings`) : copies all global values and sets all 18 bits — the old behavior.
- **Link** (`masks.set_active` only) : leaves the field mask at 0; unmodified fields follow live global settings at trigger time.

Audio-thread merge (`get_settings(instrument, step, &global)`):
- If step mask inactive → `None` (use global).
- If field mask is 0 → `Some(global)` (link mode, nothing overridden yet).
- Otherwise → copy `global`, overwrite only the fields whose bit is set.

Persistence format is `[values][step_masks][field_masks]`. Old presets without `field_masks` are loaded as full snapshots (retro-compatibility).

### Per-slot instances (ST-7, 2026-07-05)

Two slots of the same kind (e.g. two Kicks) are fully independent:

- Standard settings, **special params** (`special[32]`) and the Hz/Notes
  display mode all live per SLOT in `SoundSettingsState`, persisted in
  `sound-settings-v2` (v3 layout, 46 floats/slot — the blob length IS the
  format version, never reuse a length).
- The legacy per-voice nih-plug params (`kick_click`, `freq_mode_kick`, …)
  and `special_param()` exist **only** to seed old sessions once
  (`needs_param_seed` flag, seeded in `process()`); never read them elsewhere.
- Algo params are positional per slot ("Slot N Algo") and share the widest
  range (`instrument_registry::max_algo_index()`); UI and engine clamp to the
  current kind's `algo_count`.
- Rule of thumb: anything keyed by lane (settings, plocks, seq-plocks, algo,
  lane-length locks, mute/solo/mix) is indexed by **slot**; only
  registry/schema lookups use the voice index derived from the slot's kind
  (`schema_voice_idx` in ui.rs, `kind.drum_voice_index()` elsewhere).

### Pattern persistence + legacy migration

The grid is persisted in the VST3 state field **`pattern-v5`** (see `PATTERN_STATE_FIELD` in `lib.rs`), serialized directly from `SharedPattern` with 14 slot rows. Older builds used `pattern-v1`..`pattern-v4` or 16 hidden `IntParam`s named `st01`…`st16`; `DrumFlashVst::filter_state` migrates those to `pattern-v5` on load. Don’t reintroduce `stNN` params or rename `pattern-v5` — it’s the contract that keeps existing Studio One sessions loading.

### Frozen VST3 identity

`VST3_CLASS_ID = *b"DrumFlashPlugin1"` is **deliberately stable** to preserve compatibility with saved DAW projects. Do not change it for the V1 line.

## Real-time Constraints

Enforced by code review, not by tooling. In `process()` and everything it calls:

- no allocation
- no blocking locks in the audio thread (use atomics / lock-free; e.g. `AtomicU32`, `AtomicBool`, `SharedPattern`)
- no panic — no `unwrap()` on host-supplied data, prefer `Option` handling like the `transport.tempo` fallback in `lib.rs`
- preallocate and reuse buffers
- prefer deterministic, preallocated state

## Documentation Rules

- Do not describe a feature as implemented unless it is visible in the current Rust code path.
- Keep docs aligned with the actual plugin state.
- Treat older markdown files as potentially stale unless updated alongside code.
- If build/test results differ from documentation, trust the actual command result and update the docs.

## Build Notes

Typical build commands:

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
.\build.ps1 -Install
cargo test
cargo check
cargo run --bin test_standalone    # headless audio engine harness
```

`build.ps1` injects a `DRUM_PATTERN_BUILD_ID` env var (timestamp) that ends up displayed in the plugin UI. It builds the `cdylib` to `target/release/drum_pattern_vst.dll`, copies it into the `build/drum-pattern-vst.vst3/Contents/x86_64-win/` bundle, and (with `-Install`) deploys to the system VST3 folder.

There is no lint config beyond `cargo`’s default warnings.

**Studio One file lock:** the DAW must be fully closed during install because it locks the VST3 DLL (otherwise the copy to `Program Files` fails with *Access denied*).

**Run `build.ps1 -Install` PLAINLY in the foreground.** Do not pipe or redirect it: in PowerShell 5.1, `2>&1` / `2>$null` make PS wrap cargo's stderr as a `NativeCommandError` and abort the run; a *backgrounded* `... 2>$null` once spawned two contending `cargo` processes deadlocked on the build-directory lock (0% CPU for ~30 min). If a build looks stuck, check `Get-Process cargo,rustc` (CPU/StartTime), kill them, and re-run plainly. **`build.ps1` runs `cargo build` in the *current* directory** (no `--manifest-path`), so run it from `drum-pattern-vst` (`Set-Location "E:\…\drum-pattern-vst"` first) — the shell cwd can drift back to the repo root and break the build with `could not find Cargo.toml`. For raw `cargo`, pass an absolute `--manifest-path`.

## Agent Workflow Rule

When the user says **"next"**, **"on continue"**, **"qu'est-ce qu'on fait maintenant"**, or any similar phrase indicating they want to proceed to the next task:

1. **DO NOT** immediately start coding.
2. Read `TODO.md` to find all unchecked tasks (`- [ ]`).
3. Present a **curated list** of the available tasks to the user, organized by priority/impact.
4. Wait for the user to **explicitly choose** which task to tackle.
5. Only then proceed with implementation.

This prevents the agent from making decisions on behalf of the user about what to work on next.
