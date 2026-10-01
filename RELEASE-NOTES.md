# Release notes

Downloads: [releases page](https://github.com/TiiPaa/flashdrum/releases).

## 0.9.2 — 2026-10-01 (beta)

- **Grid header**: the column headings sit over their columns again (they were shifted one column to the left).
- **Kick-like voices**: switching the Frequency row between Hz and Notes no longer shifts the rows below it.
- **Installer**: the uninstaller now lives in `C:\Program Files\Flash Drum`, outside the plugin folder; the GPL is shown for information (nothing to accept); the third-party notice is plain text; the full license texts of every third-party component ship with the plugin (`THIRD-PARTY-LICENSES.txt`).
- Internal clean-up of the audio engine and the Sound panel, with no change to the sound.

## 0.9.1 — 2026-10-01

- **Grid**: two arrows ‹ › above the lane names shift the whole grid by one step — steps, fused cells and parameter locks move together and wrap inside the pattern length. When a fused cell sits on the edge, a warning asks before breaking it.
- **Randomize Lane**: the default density is now 80 %.
- **Settings › MIDI**: the Auto-assign caption sits beside its button, like the Macros row.
- **Snare606 and 808 Kick**: they now declare the single algorithm they actually have (no audible change).
- Internal clean-up: instrument traits declared in one place, dead code removed, the voice engine driven by a single list.
