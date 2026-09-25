# Studio One Multi-Out Notes

## Diagnosis

Studio One only activates the first output pair of a multi-output instrument by default.
Additional outputs must be enabled from the instrument output menu.

The decisive missing piece was `getRoutingInfo()`: Studio One kept the output checkboxes grayed until the VST3 wrapper reported that the instrument event/MIDI input routes to the main audio output.

The drum outputs are reported as `kMain` and `kDefaultActive`. Earlier attempts with `kAux` still showed grayed output checkboxes.

## Vendored patches

The `nih-plug` copy in `vendor/nih-plug` carries the following patches
(**the full inventory, including later additions, lives in
`vendor/nih-plug/FLASH-DRUM-PATCHES.md`**):

- `get_unit_by_bus()` — resolves valid audio/event buses to the root unit instead of returning `kResultFalse`.
- `set_bus_arrangements()` — accepts progressive output activation (any valid count from main-only up to full layout), only validating the aux buses that Studio One requested.
- `num_ins == 0` — accepts a null audio input layout pointer (valid for an instrument with no audio inputs).
- Buffer validation — ignores disabled auxiliary output buses so the main output or a partial layout can still process.
- `getRoutingInfo()` — maps the event input bus to audio output bus 0. Without this, Studio One lists the outputs but keeps them grayed.
- Sparse aux-buffer remap (`active_output_buses` / `mapped_aux_output_idx`) — Studio One only provides buffers for the enabled auxes, in compact order.
- State save/restore on the `IEditController` side too, not only `IComponent`.
- Hidden keyboard message window (Studio One / REAPER / Live key capture; diagnostic via `FLASH_DRUM_KBD_LOG`).
- Opt-in state journal via `FLASH_DRUM_STATE_LOG` → `%TEMP%\flash-drum-state.log` ([249]).

## Test steps

1. Close Studio One.
2. Run `.\build.ps1 -Install` from `drum-pattern-vst`.
3. Reopen Studio One and rescan VST3 plugins.
4. Add `Flash Drum` as an instrument.
5. Open the Console, then the Instruments panel.
6. Expand the plugin output list and enable the extra outputs.
7. Verify the output channels: Main Mix, then the generic stereo pairs `Out 1`…`Out 14` (one per lane, routed from the Track tab).

Expected result: extra output checkboxes are clickable, and each enabled channel receives the corresponding drum voice while the main mix still receives the full stereo mix.

If the plugin row is grayed in Studio One's Instruments section, first verify that the instrument track is routed to `Flash Drum`. The plugin exposes a basic MIDI/event input bus for this association even though the current sequencer does not need incoming notes to play.

## Duplicate bundle warning

During local testing, an older duplicate copy was found at `C:\Program Files\Common Files\VST3\tipa\drum-pattern-vst.vst3`. Studio One had scanned both copies, which can make it show cached data for the wrong build. Only `C:\Program Files\Common Files\VST3\drum-pattern-vst.vst3` should remain installed.
