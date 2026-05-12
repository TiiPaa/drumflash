# Drum Flash - Agent Guide

## Project Overview

Primary goal: build a VST3 drum sequencer plugin in Rust.

The web app in `index.html` and `index.js` is a legacy PoC and functional reference. It is not the target product anymore.

## Current Project Layout

- `drum-pattern-vst/` - primary implementation target
- `index.html` - browser PoC used as behavioral reference
- `index.js` - modular PoC variant, not the main product
- `PROJECT_BRIEF.md` - product scope and V1 definition
- `BACKLOG_VST.md` - prioritized work list

## Development Priority

When changing product behavior, prioritize the Rust plugin unless the task is explicitly about the PoC.

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

## Real-time Constraints

- no allocation in `process()`
- no blocking locks in the audio thread
- no panic in audio processing
- prefer deterministic, preallocated state

## Documentation Rules

- Do not describe a feature as implemented unless it is visible in the current Rust code path.
- Keep docs aligned with the actual plugin state.
- Treat older markdown files as potentially stale unless updated alongside code.

## Build Notes

Typical build commands:

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
.\build.ps1 -Install
cargo test
cargo check
```

`build.ps1` applies the local `nih-plug` VST3 patch required for Studio One multi-out,
builds the release DLL, regenerates the VST3 bundle, and installs it when `-Install` is used.

If build/test results differ from documentation, trust the actual command result and update the docs.
