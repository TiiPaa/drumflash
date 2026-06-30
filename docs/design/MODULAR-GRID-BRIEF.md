# Modular Grid Redesign — Implementation Brief

## 1. Goal

Refactor Drum Flash from a fixed 13-voice grid into a modular 14-slot track grid.

The user sees only the active/used tracks. The default preset contains 4 tracks: `BD`, `SD`, `HH`, `Tom`. Additional tracks can be added up to a hard internal maximum of 14 slots.

## 2. Scope

### In scope (V1)

- 14 fixed internal slots (`MAX_TRACKS = 14`).
- Each track is an independent instance with its own:
  - instrument type,
  - sound settings (full per-track plock support),
  - pattern data,
  - routing (Main on/off + audio output selection),
  - MIDI note.
- UI shows only active tracks.
- `+ Add Track` button.
- Per-track menu: Rename / Change instrument / Delete.
- Two tabs per track: `Sound` and `Track`.
- Mute / Audio / MIDI toggles per track.
- Multiple instances of the same instrument allowed.
- Pattern bank stores only musical data; the global track layout is never changed by loading a pattern.
- Migration of old 13-voice sessions to slots 1–13 (slot 14 empty).

### Out of scope (post-V1)

- Track reordering / drag-and-drop.
- Effect tracks.
- Sample playback tracks.
- MIDI-out tracks.
- Song arranger.

## 3. Track Model

### `TrackInstrumentKind` (11 types)

```rust
pub enum TrackInstrumentKind {
    Kick,
    Snare,
    HiHat,
    OpenHiHat,
    Tom,
    Clap,
    Ride,
    Cymbal,
    Snare606,
    BassDrum808,
    Zap,
}
```

Note: the three legacy tom voices (`Tom1`, `Tom2`, `Tom3`) collapse into a single `Tom` type.

### `TrackRouting`

```rust
pub struct TrackRouting {
    pub main_on: bool,                // summed into main stereo mix
    pub out_select: TrackAudioOut,    // Main | Out1 .. Out14
}

pub enum TrackAudioOut {
    Main,
    Out(u8), // 1..=14
}
```

### `TrackSlot`

```rust
pub struct TrackSlot {
    pub id: TrackSlotId,              // 0..13
    pub name: String,                 // user-editable, default = instrument short name + occurrence
    pub kind: TrackInstrumentKind,
    pub active: bool,                 // visible/used
    pub routing: TrackRouting,
    pub midi_note: u8,                // default = global_base_note + id
    pub sound: SoundSettingsBlock,    // full per-instrument settings
    pub pattern: TrackPatternData,    // 16-step base + plocks + fusion + morphing
}
```

### `TrackLayoutState`

```rust
pub struct TrackLayoutState {
    pub slots: [TrackSlot; MAX_TRACKS],
    pub global_midi_channel: u8,      // 1..16, default 10
    pub global_base_note: u8,         // default 36
}
```

The track layout is persisted in a new state field `track-layout-v1`.

## 4. Pattern Storage

A pattern slot (`PatternBankSlot`) stores only musical data for the current active tracks:

- for each active track: step bitmask, plock values, fusion groups, morph targets, lane params (length, push/pull, humanize).

It does **not** store:

- instrument type,
- track order,
- routing,
- MIDI note,
- sound settings default values (plock overrides only).

Loading a pattern must never alter the global track layout.

## 5. Migration from 13-voice sessions

On `filter_state` / state load:

1. If `track-layout-v1` exists, deserialize it.
2. Else, build a default layout from the legacy 13 voices:
   - slot 0  → Kick
   - slot 1  → Snare
   - slot 2  → HiHat
   - slot 3  → OpenHiHat
   - slot 4  → Tom
   - slot 5  → Tom
   - slot 6  → Tom
   - slot 7  → Clap
   - slot 8  → Ride
   - slot 9  → Cymbal
   - slot 10 → Snare606
   - slot 11 → BassDrum808
   - slot 12 → Zap
   - slot 13 → empty (inactive)
3. Migrate the legacy `pattern-v4` data into slot 0..12.
4. Keep the old fields for compatibility; do not rename `pattern-v1` legacy migration path.

## 6. Audio Engine Changes

- Replace the single `DrumSynthesizer` with 14 independent voice instances.
- `DrumVoice` no longer has a fixed voice index semantic; it is selected per track via `kind`.
- Routing is applied per track after synthesis:
  - always sum into Main if `main_on` is true,
  - additionally write to the selected `Out N` aux bus if `Out N` is selected.
- Mute logic:
  - `mute` → no audio, no MIDI.
  - `audio` off + no Main/aux routing → no internal synth output (MIDI still fires).
  - `midi` off → no MIDI note events.

## 7. Sequencer Changes

- Iterate over active tracks instead of `DrumVoice::COUNT`.
- Each active track triggers its own synth instance.
- Humanize remains velocity-only.
- Swing/groove remains global.

## 8. UI Changes

- Header: global params + `+ Add Track`.
- Grid lanes: one per active track, ordered by slot index.
- Lane header: track name, `Sound`/`Track` tabs, mute/audio/midi toggles, menu button.
- `Sound` tab: current per-instrument panel for this track.
- `Track` tab: name, instrument kind selector, routing, MIDI note.
- Context menus reuse the existing plock/fusion/morphing logic but keyed by slot index instead of `DrumVoice`.

## 9. MIDI Behavior

- Global MIDI channel: default 10.
- Global base note: default 36.
- Per-track default note: `base_note + slot_index`.
- Per-track note editable in `Track` tab.
- Notes fire on the configured channel regardless of Main/aux routing.

## 10. Generator Changes

- Generator receives the list of active track `kind`s.
- Duplicate kinds get slight variations based on appearance order (e.g. second `Tom` uses `Tom2` density/tuning hints if available; otherwise generic variation).
- Style + density + variation still drive the algorithm.

## 11. Build & Validation

- Build and install with `build.ps1 -Install` from `drum-pattern-vst/`.
- Run `cargo check` and `cargo test`.
- Update `CHANGELOG.md` with build ID and changes.
- Commit and push only when explicitly requested.

## 12. Files Likely to Change

- `src/lib.rs`
- `src/sequencer/mod.rs`
- `src/sequencer/pattern.rs`
- `src/sound_settings.rs`
- `src/instrument_registry.rs`
- `src/synthesis/mod.rs`
- `src/ui.rs`
- `src/pattern_bank.rs`
- `src/plock.rs`
- `src/generator/*.rs`
- `CHANGELOG.md`
