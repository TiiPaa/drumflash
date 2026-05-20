# Changelog

## 2026-05-20 — Plock Snapshot vs Link mode

**Build:** `20260520-211700`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- **Plock per-field masks** (`PlockFieldMasks`) : each plock step now tracks which fields are explicitly overridden via an 18-bit `u32` mask.
- **Snapshot mode** (default) : "📸 Snapshot current settings" copies all global values and locks them — previous behavior.
- **Link mode** (new) : "🔗 Link to global" activates the plock without copying values; only fields you subsequently modify override the live global settings.
- **`get_settings` merge** : audio thread builds global `VoiceSettings`, then merges with plock — overridden fields come from plock storage, unmodified fields fall back to globals.
- **Plock editor UI** :
  - Mode indicator : `🔗 Linked`, `📸 Full snapshot`, or `🔀 Mixed`.
  - Bold labels for overridden fields, weak labels for linked fields.
  - `↺` reset button per field to revert to global (clears the bit).
  - Per-field `set_field` writes only the changed field instead of rewriting the entire `VoiceSettings`.
- **Persistence retro-compatibility** : old presets without field masks load as full snapshots (all bits set).
- New unit tests : `link_mode_returns_global`, `merge_takes_modified_fields`, `set_field_only_sets_one_bit`, `clear_field_unlinks_without_clearing_plock`, `clear_removes_field_mask`.

---

## 2026-05-20 — Sound Panel redesign (families + interactive envelope viz)

**Build:** `20260520-123040`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- Sound Panel fully data-driven from `instrument_registry.rs`:
  - New `ParamFamily` enum (Osc / Env / Filter / Output) with `StandardParamDef` metadata (range, log scale, suffix, checkbox).
  - Parameters grouped per family with titled frames.
  - Removed legacy `InstrumentCapabilities` — parameter visibility is now encoded in `standard_params` slices.
- Interactive envelope visualizations:
  - `draw_amp_envelope` : AHDSR-style curve with colour-coded phases (Hold=cyan, Decay=blue, Release=purple). Attack phase is hidden when no Attack parameter exists.
  - `draw_filter_envelope` : dedicated filter-env curve (orange) inside the FILTER family group.
  - Layout horizontal : params on the left, graph on the right.
  - Real-time update when moving Decay / Release / Curve sliders.
- Fixed decay slider ranges that were clamping long-decay voices (Ride 1.2s, Cymbal 2.0s).

---

## 2026-05-19 — Perc1 refactor (Zap → Perc1)

**Build:** `20260519-191344`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- Rename Zap → Perc1 (`perc1.rs`, `DrumVoice::Perc1`, label `"P1"`, all params `perc1_*`).
- Migrate Perc1 `amp_env` from `ExpDecayEnvelope` to `DecayReleaseEnvelope` — Release slider is now wired.
- Fix `set_settings` anti-click invariant: use `set_decay()` / `set_release()` / `set_curve()` instead of recreating envelopes.
- Add `filter` + `filter_env` to Perc1 with additive cutoff formula.
- Fix latent bug in `voice_settings_for`: index 12 now correctly reads `algo_perc1`.
- Update plock tests, MIDI export tests, generator comments, and algo registry for Perc1.

### Known issues
- Perc1 Release and other parameters reported as non-responsive in Studio One — under investigation ([50]).

---

## 2026-05-19 — Revert stable + documentation

**Build:** `20260519-163250`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- Revert code to stable commit `5ae1286` (Zap voice) after critical bugs identified in Perc1 commit `8d56e72` (envelope recreation in `set_settings`, broken release/filter env, hardcoded plock menu).
- Rebuild and reinstall VST3 bundle.
- Create `ADDING_AN_INSTRUMENT.md` — complete guide for adding new synthesis voices (architecture, step-by-step checklist, anti-patterns).
- Merge `CLAUDE.md` into `AGENTS.md` for unified agent documentation.
- Synchronize `BACKLOG_VST.md` and `TODO.md`.

### Known issues to fix
- Perc1 needs clean re-implementation: do not recreate envelopes in `set_settings`, migrate to `DecayReleaseEnvelope`, make plock menu data-driven.

---

## 2026-05-16 — Mix Bus + plock fix + B8 + conditional params

**Build:** `20260516-205054`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- Per-instrument Mix Bus checkbox (route to Main Mix on/off, independent of Mute).
- Parameter Locks format expanded: `FIELD_COUNT` 12 → 14 (fields 12 = clap_echo, 13 = algo).
- Fix root cause of lost plock echo: `set_special_param()` removed from `process()`, special params now propagated only at trigger time.
- Sound Panel hides inactive parameters per instrument via `InstrumentCapabilities`.
- New instrument B8 (TR-808 Bass Drum) with accent, snap, pitch drop, analog, release, click tone.

---

## 2026-05-15 — B8 click tone + plock B8 fix + anti-click

**Build:** `20260515-124610`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- Dedicated LP filter for B8 click tone (100–8000 Hz), plockable (field 17).
- Plock B8 fix: special params (accent/snap/pitch_drop/click_tone) stored in fields 14–17.
- Attack ramp 1.5 ms on B8 envelope + cold-start-only phase reset + DcBlocker + freq_smoother.
- Cross-DAW validation: plugin loads in Reaper, audio stable.
- Warnings reduced: 17 → 0 (`cargo check --all-targets` clean).

---

## 2026-05-14 — DecayReleaseEnvelope + Snare 606 + Clap rework

**Build:** `20260514-220658`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- Bi-stage `DecayReleaseEnvelope` (decay + release) with persistent retrigger (`trigger_at_peak`).
- Hold phase between attack and decay for Snare/HiHat/OpenHH/Snare606.
- Analog-style continuity: no phase/filter/noise reset on retrigger.
- Kick: additive pitch sweep + freq smoother + DcBlocker.
- Clap rework: bandpass, snap transient, 4 bursts with irregular timing, Echo slider (0–3).
- New instrument: Snare 606 (TR-606 grey-box) with resonance, tone, snap.
- Fix crash on 11th voice: `IntRange` div-by-zero + index bounds + step mask hardcode.

---

## 2026-05-13 — Modular synthesis + groove + generators + UI polish

**Build:** `20260513-202946`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- Modular `Voice` architecture with `set_algo()` and `set_special_param()`.
- Kick: 3 algos (Sine/Square/FM) + click transient.
- Snare: 3 algos (Synth/Noise/Layered) + snap param.
- New voices: Clap, Ride, Cymbal.
- Groove engine: Straight, Swing 16th, Shuffle, MPC Style.
- Push/pull per instrument, humanize per instrument.
- Pattern generators: Euclidean, Markov, Probabilistic.
- MIDI export to `Documents/Drum Flash/exports/`.
- UI: BoolParam → checkbox, EnumParam → combobox, algo → named combobox.
- Sound panel per instrument with frequency, decay, volume, filter, algo, special params.

---

## 2026-05-11 — Grid persistence + Studio One save/restore fix

**Build:** `20260511-091259`  
**VST3 Class ID:** `DrumFlashPlugin1`  
**SHA-256:** `62AA5FCC445FEFDBC1E30196E614BCAED53A61C9F9EB2AB9BD5A4E1C5C510CEF`

### Changes
- Grid persisted via `pattern-v1` field (serialized from `SharedPattern`).
- Migration from legacy hidden params `st01`–`st16` to `pattern-v1`.
- Vendored `nih-plug` wrapper saves/restores state on both `IComponent` and `IEditController`.
- Studio One multi-out validated: `getRoutingInfo()` maps event input to main audio output.
- DAW sync validated: play, stop, tempo, repositionnement.
- Presets Rock, Funk, Disco.
- Mutes and solos per instrument.
