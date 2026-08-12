# Handoff — Flash Drum VST3

**Date:** 2026-08-12 · **Branch:** `skeuo-vector` · **Latest installed build:** `20260807-170048`
**Read [`CLAUDE.md`](../CLAUDE.md) first** — it is the canonical architecture + invariants + workflow reference for all AI agents. This file is a session-state handoff, not a replacement for it.

---

## 1. Snapshot / working-tree state

- The whole session's work sits **uncommitted** on `skeuo-vector` (~29 tracked files modified, several new files). **Nothing has been committed** — the user commits explicitly, on request only. Do not commit or push unless asked.
- Build is **green**: `cargo test` = **421 tests pass** (259 lib + 1 + 161 integration), `cargo check` warning-clean (one pre-existing `kick_out` unused-var in an integration test), `build.ps1 -Install` OK.
- New untracked files this session:
  - Voices: `src/synthesis/{bd606,sd606,ch606,buzz,sample_bank}.rs` + their `src/synthesis/settings/{bd606,sd606,ch606,buzz}.rs`.
  - Assets: `assets/{bd606,sd606,ch606}.wav` (TR-606 samples loaded by `sample_bank.rs`).
  - `wav/` is a scratch folder, **gitignored on purpose** — leave it.
- **Deploy caveat:** Studio One locks the VST3 DLL — it must be closed before `build.ps1 -Install`. Run the build **plainly** (no `2>&1`/`2>$null` — PowerShell 5.1 turns cargo stderr into a fake `NativeCommandError`).

## 2. In-flight — awaiting user validation

### [159] Amp envelope → A-H-D bipolar (Release removed) — DONE, needs Studio One sign-off
All voices' **amplitude** envelope went from decay+release to **Attack-Hold-Decay, no release**, with **independent bipolar concave/convex curves** on attack and decay (generalises the Buzz filter-env the user liked).

- Core: **`DecayReleaseEnvelope` was rewritten internally** in `dsp.rs` (time-based A-H-D) keeping its public signatures, so the amp voices barely changed. `decay_curve` = bipolar **decay** curve; `release_curve` **repurposed** as bipolar **attack** curve; `set_release`/`release_time` = **no-op**. `shape_curve(e,c)` shared. `trigger()` ramps from current value (anti-click), `trigger_hard()` from zero (machine-gun/stutter).
- Registry: **Release slider removed** from every `*_STD` table; **"Release Curve" → "Attack Curve"** (−1..1); **"Decay Curve"** → −1..1.
- Graph `draw_amp_envelope` (envelope_viz.rs): A-H-D bipolar, Hold plateau, no Release segment.
- Buzz amp migrated `ExpDecayEnvelope` → `DecayReleaseEnvelope` (its Decay Curve is bipolar now too).
- Bonus bug fixes: `open_hihat` recreated its envelope in `set_settings` (state reset → click on slider drag) → replaced by setters; `open_hihat`/`cymbal` discarded `with_attack_ms` results (`Copy` no-op) → `set_attack_ms`; cymbal's duplicated attack-drift cleaned.

**⚠️ Known limitation (accepted in the plan):** per-voice **defaults were not retuned**. Old stiffness values (0.1–20) are read on the new −1..1 range and **clamp to +1**, so a *fresh* instance shows Attack/Decay Curve sliders **at max (convex)**. Sound stays punchy; if the user wants centered (0 = linear) defaults, retune `sound_settings_default` (pos 6 = decay_curve, pos 7 = release_curve/attack) per voice in `instrument_registry.rs` + `VoiceSettings::*()` in `synthesis/mod.rs`.

**À tester dans Studio One (build 20260807-170048):** no Release slider anywhere; Attack Curve & Decay Curve move the graph (concave↔convex) *and* are audible; Hold still shows (regression [154]); no click on fast retrigger or slider-drag; old sessions reload close-sounding, no crash; Buzz still fine.

## 3. Pending tasks (TODO.md)

- **[155]** SMP voices (BD6/SD6/CH6smp) attack barely audible **and** mismatches `draw_sample_amp_graph` (attack drawn as a sample fraction). Align audible attack ↔ graph. *(Note: amp attack was already converted to absolute ms — `MAX_AMP_ATTACK_SECS = 0.08` — but the graph/range still don't line up.)*
- **[160]** Draw a **Gate Shape** graph for Buzz (Smooth cosine tremolo vs Razor exp spike, driven by Rate/Depth/Shape) next to its gate controls.
- **[161]** Restore **per-cell microtiming** (nudge) in the sequencer p-lock.
- Backlog P2/P3: [144] [146] [150] [152] [69] [27] [56] [41] [84] [83cont] [94] [95] and **[BUG-LANE-DESYNC]**.

Reminder: when the user says "next" / "on continue", **do not start coding** — present the curated unchecked-TODO list and let them pick.

## 4. What shipped this session (all built & installed, uncommitted)

| Task | Summary | Build |
|---|---|---|
| CH6smp wiring + samplers | TR-606 closed-hat sampler finished; `sample_bank.rs` + wav assets | — |
| Sampler attack | amp attack converted sample-fraction → **absolute ms** (`MAX_AMP_ATTACK_SECS = 0.08`) | — |
| Step-drag phantom | fixed yellow-marker cell following the mouse on right-click/release | bb45c76 |
| **Buzz voice** | new voice: tonal osc (sine/square/saw) + noise + pitch sweep + Smooth/Razor amplitude gate + bipolar filter env (LP/HP/BP) + machine-gun amp retrigger | — |
| [148] Generator styles | +6 styles (Bossa/House/DnB/Afrobeat/Dub/Breakbeat); anchors kept sacred (four-on-floor fix) | — |
| Style preset chips | fixed chips that also install a **lane KIT** per style (`apply_style_preset`, bottom_panel.rs) | — |
| [153] | phantom "unsaved" star fixes incl. reopened-project-matching-a-slot nuance (pattern_bank.rs) | — |
| [154] | Hold shown in amp env graph | 20260807-111016 |
| [156] | Save keycap moved left of the pattern slots | 20260807-123130 |
| [157] | Buzz max gate rate raised | — |
| [158] | MIDI export includes fused-cell pulses + stutter notes | 20260807-140101 |
| **[159]** | amp env → A-H-D bipolar, Release removed | 20260807-170048 |

## 5. Gotchas / lessons learned (also in agent memory)

- **PowerShell 5.1**: never `2>&1`/`2>$null` a native exe (cargo/build.ps1) — stderr gets wrapped as `NativeCommandError` and aborts / can deadlock two cargo processes on the build lock. Run plainly; capture with `| Out-String` if you must.
- **Studio One DLL lock**: `-Install` copy fails if S1 is open. The rule is: run `build.ps1 -Install` directly, only surface S1 if the copy fails.
- **"No difference after a build"** is often a **cached DLL in Studio One** — check the build ID in the plugin header and remove/re-add the instance before suspecting the code.
- **Anti-click**: never recreate an envelope in `set_settings()` (resets state → click mid-drag); drive it via setters. `DecayReleaseEnvelope` is `Copy` — `x.with_attack_ms(..)` as a statement is a silent no-op; use `set_attack_ms`.
- **Two style systems** are distinct: fixed "preset" chips (`Pattern::xxx_pattern()`, bottom_panel) vs Generator "Style" dropdowns (`enum Style`/`MusicalTemplate`, generator/styles.rs). "preset de style" in French = the fixed chips.
- **Persistence blobs are positional; the blob length IS the version** — never renumber a field or reuse a length. `sound-settings-v2`, `plock-v1`, `pattern-v5`.
- **Pattern dirty-state** (the "unsaved ★"): a reopened project whose grid matches an occupied slot must NOT warn even when `last_loaded = None` — see `pattern_is_dirty` in pattern_bank.rs.
- **UI stable zones**: no conditional line that shifts UI zones — reserve the space (14 fixed rows), grey out rather than hide.
- **Skeuo rendering** all lives in `src/ui/skeuo.rs`, one function per element — never scatter it.
- **Don't add unrequested scope**: implement exactly the request/mockup; propose TODO ideas, don't silently build them.

## 6. Key file map (this session's features)

- Envelope engine: `src/synthesis/dsp.rs` (`DecayReleaseEnvelope`, `ExpDecayEnvelope`, `shape_curve`).
- Registry (single source of truth for the 17 voices, params, defaults): `src/instrument_registry.rs`.
- Buzz: `src/synthesis/buzz.rs` + `settings/buzz.rs`; graph `draw_buzz_filter_envelope` in `ui/envelope_viz.rs`.
- Samplers: `src/synthesis/{bd606,sd606,ch606,sample_bank}.rs` + `settings/*`; graphs `draw_sample_amp_graph`/`draw_sample_filter_graph`.
- Amp graph: `draw_amp_envelope` in `ui/envelope_viz.rs`; caller in `ui/sound_editor.rs`.
- Generator styles + kits: `src/generator/styles.rs`, `src/sequencer/pattern.rs`, `src/ui/bottom_panel.rs`.
- MIDI export (fusions/stutters): `src/midi_export.rs`, `src/ui/midi.rs`.
- Pattern bank / dirty-state / Save button: `src/ui/pattern_bank.rs`.

## 7. If you're picking this up

1. Read `CLAUDE.md`. 2. Get [159] validated in Studio One (§2) — or retune default curves if the user dislikes max-convex defaults. 3. For the next feature, wait for the user's explicit pick from TODO. 4. Build+install and end every build report with the "À tester dans Studio One" checklist. 5. Consider proposing a commit of this large uncommitted session before starting new work.
