# Studio One Multi-Out Notes

## Current Diagnosis

Studio One only activates the first output pair of a multi-output instrument by default.
Additional outputs must be enabled from the instrument output menu.

The multi-out issue is fixed in the current local build. The decisive missing piece was
`getRoutingInfo()`: Studio One kept the output checkboxes grayed until the VST3 wrapper
reported that the instrument event/MIDI input routes to the main audio output.

The current working build reports the drum outputs as `kMain` and `kDefaultActive`. This is
the configuration validated in Studio One. Earlier attempts with the drum outputs reported
as `kAux` still showed grayed output checkboxes.

The vendored `nih-plug` copy in `vendor/nih-plug` patches `get_unit_by_bus()` so valid
audio/event buses resolve to the root unit instead of returning `kResultFalse`.

Studio One can also ask VST3 instruments to activate outputs progressively instead of
submitting the complete output bus layout in one call. The upstream `nih-plug` wrapper
rejected those partial `set_bus_arrangements()` calls for this plugin because it expected
all eight stereo outputs at once. The local patch accepts any valid output count from the
main output alone up to the full main-plus-seven-aux layout, and only validates the aux
buses that Studio One requested.

The wrapper also needs to accept a null audio input layout pointer when `num_ins == 0`, which
is valid for an instrument with no audio inputs. During processing, disabled auxiliary output
buses are ignored during buffer validation so the main output, or a partially enabled output
set, can still process normally.

Finally, `getRoutingInfo()` maps the event input bus to audio output bus 0. Without this,
Studio One can list the plugin outputs but keep them disabled/grayed.

During local testing, an older duplicate copy was also found at
`C:\Program Files\Common Files\VST3\tipa\drum-pattern-vst.vst3`. Studio One had scanned both
copies, which can make it show cached data for the wrong build. That duplicate was removed;
only `C:\Program Files\Common Files\VST3\drum-pattern-vst.vst3` should remain installed.

Session end state on 2026-05-11:

- `cargo test` passed with 16 tests.
- `build.ps1 -Install` completed successfully with the vendored `nih-plug` dependency.
- Installed bundle: `C:\Program Files\Common Files\VST3\drum-pattern-vst.vst3`.
- Installed build id shown in the plugin UI: `20260511-091259`.
- Installed VST3 class id: `DrumFlashPlugin1`.
- Installed binary timestamp: `2026-05-11 09:13:16`.
- Installed binary SHA-256: `62AA5FCC445FEFDBC1E30196E614BCAED53A61C9F9EB2AB9BD5A4E1C5C510CEF`.
- Previous system VST3 scan found only the expected installed bundle and its internal binary.
- Studio One multi-out activation was previously validated and should be rechecked after state changes if routing behavior regresses.
- Diagnostic logging showed that standard parameters are saved/restored by Studio One.
- Pattern grid persistence now uses the `pattern-v1` persistent field instead of relying on hidden host parameters.

## Test Steps

1. Close Studio One.
2. Run `.\build.ps1 -Install` from `drum-pattern-vst`.
3. Reopen Studio One and rescan VST3 plugins.
4. Add `Drum Flash` as an instrument.
5. Open the Console, then the Instruments panel.
6. Expand the plugin output list and enable the extra outputs.
7. Verify the output channels:
   - Main Mix
   - Kick
   - Snare
   - Hi-Hat
   - Open HH
   - Tom 1
   - Tom 2
   - Tom 3

Expected result: extra output checkboxes are clickable, and each enabled channel receives
the corresponding drum voice while the main mix still receives the full stereo mix.

If the plugin row is grayed in Studio One's Instruments section, first verify that the
instrument track is routed to `Drum Flash`. The plugin exposes a basic MIDI/event
input bus for this association even though the current sequencer does not need incoming notes
to play.

## Dependency State

The current build uses a committed vendored `nih-plug` dependency in `vendor/nih-plug`.
`Cargo.toml` points to this local path instead of patching Cargo's Git checkout at build time.
