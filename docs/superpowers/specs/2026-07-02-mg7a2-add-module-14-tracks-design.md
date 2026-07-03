# MG-7a.2 Design — Activate + Add Module (14 Independent Tracks)

## 1. Goal

Make the existing `+ Add Module` button functional. When clicked, the user selects one of the 11 `TrackInstrumentKind` types and the first inactive internal slot becomes an independent track with its own pattern, parameters, synthesis voice, and audio routing.

## 2. Scope

### In scope
- 14 internal track slots (`MAX_TRACKS = 14`).
- Each active slot is independent for:
  - pattern grid row,
  - mute / solo / mix gain,
  - algorithm selection,
  - humanize, push/pull, lane length,
  - sound settings,
  - audio routing (Main + Aux Out),
  - MIDI note.
- Instrument selection popup on `+ Add Module`.
- A new **Track** tab in the Sound Editor (next to Sound):
  - track name,
  - instrument kind selector,
  - Main Mix toggle + Aux Out selector,
  - MIDI note editor,
  - Humanize slider,
  - Push/Pull slider.
- `Lane Length` control stays in the grid lane.
- Migration from old 13-row patterns (`pattern-v4` → `pattern-v5`).

### Out of scope
- Track reordering / drag-and-drop.
- Track deletion / deactivate (this design only adds tracks).
- Sample tracks, MIDI-out tracks, effect tracks.
- New synthesis families beyond the existing 13 `DrumVoice` models.

## 3. Key Design Decisions

### 3.1. 14 slots, 13 synthesis voices
- `MAX_TRACKS` stays 14.
- `DrumVoice::COUNT` stays 13 (legacy synthesis voices, including three Tom variants).
- `TrackInstrumentKind` stays 11 functional kinds.
- A slot chooses a `TrackInstrumentKind`, which maps to one of the 13 `DrumVoice` indices via `drum_voice_index()`.
- No new synthesis algorithm is introduced; the 14th slot reuses an existing voice model.

### 3.2. Backward compatibility is intentionally broken
This build changes the VST3 I/O contract and the persisted pattern format. Existing Studio One projects will need their aux routing reapplied. This is accepted and will be clearly documented in `CHANGELOG.md`.

## 4. Data Model Changes

### 4.1. Pattern storage
- `sequencer/pattern.rs::INSTRUMENT_COUNT` changes from 13 to 14.
- `SharedPattern` already stores 14 slots internally.
- Persisted field changes from `pattern-v4` to `pattern-v5`.
- Migration: insert an empty 14th row when loading a 13-row `pattern-v4`.

### 4.2. nih-plug parameters
Add a 14th element to every per-track parameter array in `DrumFlashParams`:
- `mute_s13`, `solo_s13`, `mix_s13`, `algo_s13`
- `humanize_s13`, `push_s13`, `len_s13`
- special-parameter slots for slot 13

Existing parameter IDs (`*_s00` … `*_s12`) are preserved. New parameters are appended with new IDs so automation on the first 13 slots is not shifted.

### 4.3. Sound settings
- `SoundSettingsState` is already sized for 14 slots.
- `reset_slot_to_defaults(slot, kind)` initializes slot 13 with the chosen instrument defaults.

### 4.4. Track layout
- `track-layout-v1` already supports 14 slots.
- `TrackLayoutState::default_layout()` may later return the modular 4-track template; for this task it remains `from_legacy_13()` until the UI is proven stable.

## 5. Audio Engine Changes

### 5.1. Sequencer
- Iterate over `0..MAX_TRACKS` instead of `0..DrumVoice::COUNT`.
- Use the active slot layout (`AtomicTrackLayout`) to know which slots participate.
- For each active slot, derive `voice_idx` from `TrackInstrumentKind::drum_voice_index()` and trigger that voice.
- `TrackState` arrays (length, push, humanize) are sized to 14.

### 5.2. Synthesizer
- `DrumSynthesizer` already owns 14 instances.
- On `track_layout.version` change, `process()` calls `reinitialize_slot(slot, kind)` for newly active slots.
- `set_voice_settings` / `trigger` continue to be indexed by `voice_idx` (0..12).

### 5.3. Audio output
- `AUX_OUT_COUNT` increases from 13 to 14.
- Slot 13 can be routed to `Out 14`.
- Main mix sums all 14 slots.

### 5.4. MIDI output
- Per-slot MIDI notes are already stored in `AtomicTrackLayout::slot_midi_notes` (14 entries).
- Outgoing MIDI events use the slot's configured note and the global channel.

## 6. UI Changes

### 6.1. Grid lanes
- Replace `visible_legacy_lane_count()` with iteration over active slots from `track_layout`.
- Render one lane per active slot, in slot order.
- Remove Hum / Push mini sliders from the lane; keep Lane Length in the lane.

### 6.2. + Add Module
- Make `draw_add_module_row_v2` clickable.
- On click, show a popup / styled combo with the 11 `TrackInstrumentKind` options.
- On selection:
  1. Read current layout via `params.track_layout.map()`.
  2. Find first inactive slot (`first_inactive_slot()`).
  3. Create `TrackSlot::active_with_kind(kind)`.
  4. Call `sound_settings_state.reset_slot_to_defaults(slot, kind)`.
  5. Call `params.track_layout.set(new_layout)` (bumps atomic version).
  6. Set `state.selected_track_slot = slot`.

### 6.3. Sound Editor tabs
- Add a **Track** tab next to **Sound**.
- **Sound tab** (existing): per-slot synthesis parameters.
- **Track tab** (new):
  - track name text field,
  - instrument kind selector,
  - Main Mix toggle,
  - Aux Out selector (Main / Out 1..14),
  - MIDI note editor,
  - Humanize slider,
  - Push/Pull slider.

## 7. Migration Strategy

| Old | New | Migration |
|-----|-----|-----------|
| `pattern-v4` (13 rows) | `pattern-v5` (14 rows) | `filter_state` inserts an empty 14th row. |
| `track-layout-v1` 13 slots | `track-layout-v1` 14 slots | Already handled by flexible deserialization. |
| 13 aux outputs | 14 aux outputs | No automatic migration; Studio One projects must reassign aux routing. |
| 13 parameter sets | 14 parameter sets | Existing IDs unchanged; new `*_s13` params appear at the end. |

## 8. Implementation Checkpoints

Each checkpoint ends with `cargo check`, `cargo test`, and `build.ps1 -Install`.

1. **Checkpoint 1**: Extend `DrumFlashParams` per-track arrays to 14 entries.
2. **Checkpoint 2**: Extend `INSTRUMENT_COUNT` to 14; add `pattern-v5` migration.
3. **Checkpoint 3**: Make the sequencer iterate over `MAX_TRACKS` using the active layout.
4. **Checkpoint 4**: Extend `process()` to 14 slots and `AUX_OUT_COUNT` to 14.
5. **Checkpoint 5**: Activate `+ Add Module` and render the 14th lane when active.
6. **Checkpoint 6**: Add the Track tab in the Sound Editor.
7. **Final validation**: Studio One fresh project, save/reopen, aux routing test.

## 9. Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Studio One crashes on project reload due to changed aux topology | Validate every checkpoint in Studio One with a fresh project; document the breaking change. |
| Automation on existing 13 slots breaks | Preserve existing parameter IDs; append new slot 13 params with new IDs. |
| Pattern migration loses data | Keep `pattern-v4` migration path; insert empty 14th row, do not discard old rows. |
| Audio thread performance with 14 voices | Synthesizer already allocates 14 instances; only active slots are processed. |

## 10. Success Criteria

- `cargo test` passes after every checkpoint.
- `build.ps1 -Install` succeeds.
- Clicking `+ Add Module` opens an instrument selector.
- Selecting an instrument activates slot 13 as an independent lane.
- Slot 13 can be sequenced, muted, soloed, routed, and edited in the Sound Editor.
- A fresh Studio One project can use all 14 outputs and survive save/reopen.
