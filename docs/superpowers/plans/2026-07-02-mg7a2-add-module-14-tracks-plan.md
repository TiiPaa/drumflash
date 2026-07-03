# MG-7a.2 Implementation Plan — Activate + Add Module (14 Independent Tracks)

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the `+ Add Module` button functional by extending the plugin from 13 fixed voices to 14 independent track slots, each with its own pattern, parameters, synthesis voice, and routing.

**Architecture:** Keep the 13 legacy `DrumVoice` synthesis models but iterate the sequencer, audio engine, and UI over `MAX_TRACKS = 14` slots. Add a 14th set of nih-plug parameters, extend pattern persistence to `pattern-v5`, expose a 14th aux output, and add a Track tab in the Sound Editor.

**Tech Stack:** Rust, `nih-plug`, `nih-plug-egui`, `egui`, PowerShell build scripts.

---

## File Structure

| File | Responsibility |
|------|----------------|
| `src/lib.rs` | Plugin entry, `DrumFlashParams` (14 param sets), `process()` loop, audio I/O layout, state migration (`filter_state`). |
| `src/track.rs` | `TrackLayoutState`, `AtomicTrackLayout`, `PersistentTrackLayout` (already 14 slots, minor helpers). |
| `src/sequencer/mod.rs` | Sequencer loop, `TrackState` arrays, trigger generation over active slots. |
| `src/sequencer/pattern.rs` | `INSTRUMENT_COUNT`, `SharedPattern`, `PatternState`, persistence format. |
| `src/sound_settings.rs` | `SoundSettingsState`, default settings per instrument, version bumping. |
| `src/instrument_registry.rs` | `DrumVoice` enum, `INSTRUMENTS` array, `TrackInstrumentKind` mapping. |
| `src/synthesis/mod.rs` | `DrumSynthesizer` (already 14 instances), `reinitialize_slot`, voice settings. |
| `src/ui.rs` | Grid rendering, `+ Add Module` popup, Track tab, parameter binding. |
| `CHANGELOG.md` | Build history and breaking-change notice. |

---

## Pre-Flight Checks

- [ ] **Step 1: Verify repository state**

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
git status --short
```

Expected: clean working tree or only known uncommitted files.

- [ ] **Step 2: Baseline build**

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
cargo check
cargo test
```

Expected: `cargo check` and `cargo test` pass.

---

## Checkpoint 1: Extend `DrumFlashParams` to 14 slots

**Files:**
- Modify: `src/lib.rs`

- [ ] **Step 3: Add 14th `BoolParam` to mutes**

In `src/lib.rs`, locate the `mute_params` field and add one entry:

```rust
#[id = "mute_s13"]
pub mute_s13: BoolParam,
```

Update `pub fn mutes(&self) -> [&BoolParam; DrumVoice::COUNT]` to return 14 elements and include `&self.mute_s13`.

- [ ] **Step 4: Add 14th `BoolParam` to solos**

```rust
#[id = "solo_s13"]
pub solo_s13: BoolParam,
```

Update `pub fn solos(&self)` to return 14 elements.

- [ ] **Step 5: Add 14th `BoolParam` to mixes**

```rust
#[id = "mix_s13"]
pub mix_s13: BoolParam,
```

Update `pub fn mixes(&self)` to return 14 elements.

- [ ] **Step 6: Add 14th `IntParam` to algos**

```rust
#[id = "algo_s13"]
pub algo_s13: IntParam,
```

Update `pub fn algos(&self)` to return 14 elements. Use `IntRange::Linear { min: 0, max: 0 }` as a safe default; the actual range is set per instrument in the UI/audio path.

- [ ] **Step 7: Add 14th `FloatParam` to humanizes**

```rust
#[id = "hum_s13"]
pub hum_s13: FloatParam,
```

Update `pub fn humanizes(&self)` to return 14 elements, range `0.0..=1.0`.

- [ ] **Step 8: Add 14th `FloatParam` to pushes**

```rust
#[id = "push_s13"]
pub push_s13: FloatParam,
```

Update `pub fn pushes(&self)` to return 14 elements, range `-50.0..=50.0`.

- [ ] **Step 9: Add 14th `IntParam` to lengths**

```rust
#[id = "len_s13"]
pub len_s13: IntParam,
```

Update `pub fn lengths(&self)` to return 14 elements, range `1..=64`.

- [ ] **Step 10: Initialize new params in `DrumFlashParams::default()`**

Ensure each new parameter is initialized with sensible defaults matching the existing first 13 entries.

- [ ] **Step 11: Extend `LaneLengthLocks` to 14 bits**

`LaneLengthLocks` currently masks to 13 bits. Update masks and helpers to cover slot 13.

- [ ] **Step 12: Validate checkpoint 1**

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
cargo check
cargo test
```

Expected: no errors. Fix any compilation issues before proceeding.

---

## Checkpoint 2: Extend pattern storage to 14 rows

**Files:**
- Modify: `src/sequencer/pattern.rs`
- Modify: `src/lib.rs` (state migration)

- [ ] **Step 13: Change `INSTRUMENT_COUNT` to 14**

In `src/sequencer/pattern.rs`:

```rust
pub const INSTRUMENT_COUNT: usize = 14;
```

- [ ] **Step 14: Update `SharedPattern` constants**

Ensure `MAX_FUSIONS`, `FUSION_SLOT_COUNT`, and any bit-mask constants account for 14 instruments.

- [ ] **Step 15: Add `PatternStateV4` and migration to `pattern-v5`**

In `src/lib.rs`, add a legacy `PatternStateV4` struct holding 13 rows. Implement `expand()` that returns a `PatternStateV5` with an empty 14th row.

- [ ] **Step 16: Update `PATTERN_STATE_FIELD` and `filter_state` migration**

Change `PATTERN_STATE_FIELD` from `"pattern-v4"` to `"pattern-v5"`. In `filter_state`, migrate `pattern-v4` → `pattern-v5`, and keep older paths (`pattern-v3`, `pattern-v2`, `pattern-v1`, `st01..st16`) pointing to `pattern-v5`.

- [ ] **Step 17: Validate checkpoint 2**

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
cargo check
cargo test
```

Expected: pattern round-trip tests still pass; migration tests added for 13→14 rows.

---

## Checkpoint 3: Make the sequencer iterate over active slots

**Files:**
- Modify: `src/sequencer/mod.rs`

- [ ] **Step 18: Resize sequencer state arrays to `MAX_TRACKS`**

Locate `current_steps`, `track_states`, and any `[T; DrumVoice::COUNT]` arrays. Resize to `[T; crate::track::MAX_TRACKS]`.

- [ ] **Step 19: Iterate over active slots in `process_sample`**

Replace loops over `0..DrumVoice::COUNT` with iteration over `0..crate::track::MAX_TRACKS`. For each active slot, derive `voice_idx` from the active `TrackInstrumentKind` (passed into the sequencer or stored there) and trigger the corresponding `DrumVoice`.

- [ ] **Step 20: Update `set_mutes` / `set_track_params` signatures**

Change signatures to accept arrays of length `MAX_TRACKS`.

- [ ] **Step 21: Validate checkpoint 3**

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
cargo check
cargo test
```

Expected: sequencer tests pass.

---

## Checkpoint 4: Extend audio engine to 14 slots

**Files:**
- Modify: `src/lib.rs`
- Modify: `src/synthesis/mod.rs` (if needed)

- [ ] **Step 22: Change `AUX_OUT_COUNT` to 14**

```rust
const AUX_OUT_COUNT: usize = 14;
```

Update `OUTPUT_PORT_NAMES` to include `"Out 14"`.

- [ ] **Step 23: Resize audio-thread arrays to `MAX_TRACKS`**

In `process()`:
- `mute_states` → length 14.
- `solo_states` → length 14.
- `effective_mutes` → length 14.
- `raw_lengths` → length 14.
- `effective_lengths` → length 14.
- Propagate algos for `0..MAX_TRACKS`.

- [ ] **Step 24: Observe `track_layout` version change**

At the start of `process()`, snapshot `params.track_layout.state.version`. If changed, call `synthesizer.reinitialize_slot(slot, kind)` for newly active slots and reset their sound settings defaults.

- [ ] **Step 25: Write all 14 slots to aux outputs**

Remove the `if voice_idx >= DrumVoice::COUNT { break; }` guard in the aux output loop.

- [ ] **Step 26: Validate checkpoint 4**

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
cargo check
cargo test
```

Expected: all audio tests pass.

---

## Checkpoint 5: Activate `+ Add Module` and render slot 14

**Files:**
- Modify: `src/ui.rs`

- [ ] **Step 27: Read active slots from `track_layout`**

In `draw_grid_v2`, replace `visible_legacy_lane_count()` with the active slot count from `params.track_layout.map(|state| state.active_count())`. Iterate over `active_slot_indices()`.

- [ ] **Step 28: Make `draw_add_module_row_v2` clickable**

Change the `Sense::hover()` to `Sense::click()`. On click, store an `add_module_popup` state in `EditorUIState` with the screen position.

- [ ] **Step 29: Implement instrument selector popup**

Draw a styled popup listing the 11 `TrackInstrumentKind` variants. On selection:

```rust
params.track_layout.map(|layout| {
    if let Some(slot) = layout.first_inactive_slot() {
        let mut new_layout = layout.clone();
        new_layout.slots[slot] = TrackSlot::active_with_kind(kind);
        params.track_layout.set(new_layout);
        sound_settings.reset_slot_to_defaults(slot, kind);
        state.selected_track_slot = slot;
    }
});
```

- [ ] **Step 30: Bind lane controls to slot index**

Ensure `draw_track_length_control`, `draw_tag_param_v2`, and the Test button use `slot_idx` and the 14-element parameter arrays.

- [ ] **Step 31: Validate checkpoint 5**

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
cargo check
cargo test
```

Expected: UI compiles; the 14th lane renders when active.

---

## Checkpoint 6: Add Track tab in Sound Editor

**Files:**
- Modify: `src/ui.rs`

- [ ] **Step 32: Add `SoundEditorTab` enum**

```rust
enum SoundEditorTab {
    Sound,
    Track,
}
```

Add `sound_editor_tab: SoundEditorTab` to `EditorUIState`.

- [ ] **Step 33: Render Sound/Track tabs above the editor content**

Use styled buttons or a segmented control. Toggle `state.sound_editor_tab` on click.

- [ ] **Step 34: Implement Track tab content**

For the selected slot:
- Name text edit bound to `track_layout.slots[slot].name`.
- Instrument selector bound to `track_layout.slots[slot].kind`.
- Main Mix toggle bound to `track_layout.slots[slot].routing.main_on`.
- Aux Out selector bound to `track_layout.slots[slot].routing.out_select`.
- MIDI note editor bound to `track_layout.slots[slot].midi_note`.
- Humanize slider bound to `params.humanizes()[slot]`.
- Push/Pull slider bound to `params.pushes()[slot]`.

Persist changes back to `params.track_layout.set(new_layout)` and bump version.

- [ ] **Step 35: Remove Hum/Push mini sliders from grid lanes**

Delete the `draw_param_mini_slider_with_value` calls for `hum` and `push` in `draw_legacy_slot_lane_v2`. Keep Lane Length.

- [ ] **Step 36: Validate checkpoint 6**

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
cargo check
cargo test
```

Expected: UI compiles and tests pass.

---

## Checkpoint 7: Build, install, and validate

- [ ] **Step 37: Full test run**

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
cargo test
```

Expected: all tests pass.

- [ ] **Step 38: Build and install**

Close Studio One if it is running, then:

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
.\build.ps1 -Install
```

Expected: build succeeds and VST3 bundle is deployed to `C:\Program Files\Common Files\VST3\drum-pattern-vst.vst3`.

- [ ] **Step 39: Studio One smoke test**

1. Open Studio One with a fresh song.
2. Insert Flash Drum.
3. Verify 14 outputs are available.
4. Click `+ Add Module`, choose an instrument.
5. Program a few steps on the new lane.
6. Verify audio on Main Mix and on the chosen Aux Out.
7. Save and reopen the song.
8. Verify the session reloads without crash.

- [ ] **Step 40: Update CHANGELOG.md**

Add an entry with the build ID, summary of changes, and a **breaking change** warning about Studio One aux routing.

---

## Self-Review Checklist

- [ ] Spec coverage: every section of the design spec maps to one or more tasks above.
- [ ] Placeholder scan: no `TBD`, `TODO`, or vague instructions remain.
- [ ] Type consistency: `MAX_TRACKS` is used for slot arrays; `DrumVoice::COUNT` remains for synthesis voices.
- [ ] Test coverage: each checkpoint includes `cargo check` / `cargo test`.
- [ ] Build discipline: final install uses `build.ps1 -Install` from `drum-pattern-vst/` without piping.
