# Flash Drum patches on the vendored nih-plug

Upstream: [robbert-vdh/nih-plug](https://github.com/robbert-vdh/nih-plug).
**Upstream revision: unknown** — the copy was imported already patched
(commit `ea6e69d`, 2026-07) without recording the base revision. The upstream
project entered maintenance mode on 2026-05-10. Until a sync is attempted,
this file is the authoritative inventory of every local change, so a diff
can be reconstructed by content. See also `../STUDIO_ONE_MULTI_OUT.md`.

> **Never replace this copy with crates.io nih-plug** — every patch below is
> load-bearing (Studio One multi-out, state restore, keyboard).

## Studio One multi-out (documented in `../STUDIO_ONE_MULTI_OUT.md`)

1. `get_unit_by_bus()` — resolves valid audio/event buses to the root unit
   instead of returning `kResultFalse`.
2. `set_bus_arrangements()` — accepts progressive output activation (any
   valid count from main-only up to the full layout); only validates the aux
   buses the host actually requested.
3. `num_ins == 0` — accepts a null audio input layout pointer (valid for an
   instrument without audio inputs).
4. Buffer validation — ignores disabled auxiliary output buses so the main
   output (or a partial layout) can still process.
5. `getRoutingInfo()` — maps the event input bus to audio output bus 0;
   without it Studio One lists the outputs but keeps them grayed.

## Found later (were undocumented until 2026-09-25, audit finding)

6. **Sparse aux-buffer remap** (`src/wrapper/vst3/wrapper.rs`,
   `active_output_buses` / `mapped_aux_output_idx`) — Studio One hands the
   plugin buffers only for the auxes the user enabled, in compact order; the
   remap routes each provided buffer to the right aux slot. Complemented on
   the plugin side by `add_stereo_aux_sample` (`src/lib.rs`).
7. **State save/restore on the IEditController side** — Studio One calls
   `getState`/`setState` on the controller, not (only) the component; both
   paths serialize/restore the full state (10 persisted fields).
8. **Keyboard message window** — a hidden message-only window so Studio One /
   REAPER / Live stop swallowing key events destined to the editor
   (`FLASH_DRUM_KBD_LOG` env var enables a `%TEMP%\flash_drum_kbd.log`
   diagnostic).
9. **State diagnostics journal** (`src/wrapper/vst3/wrapper.rs`,
   `log_state_diag` / `log_current_params`) — logs a summary of every
   get/set_state. [249] Now **opt-in**: active only when
   `FLASH_DRUM_STATE_LOG` is set, written to
   `%TEMP%\flash-drum-state.log` (it used to write unconditionally to a
   hardcoded `E:\tmp` developer path).

## Git dependencies (not vendored, pinned)

| Crate | Pin | Note |
|---|---|---|
| `vst3-sys` | `rev = "b3ff4d775940f5b476b9d1cca02a90e07e1922a2"` | Fork `robbert-vdh/vst3-sys`. [267] Pinned by **rev** since 2026-09-25 (was `branch = "fix/drop-box-from-raw"` — a moving, deletable branch). A controlled fork is still the long-term fix if the upstream repo ever disappears. |
| `baseview` | `rev = "9a0b42c09d712777b2edb4c5e0cb6baf21e988f0"` | Pinned in `vendor/egui-baseview/Cargo.toml`. |
| `clap-sys` | `rev = "25d7f53fdb6363ad63fbd80049cb7a42a97ac156"` | CLAP support is unused; candidate for removal. |

The exemplary model for this file is `../egui-baseview/FLASH-DRUM-PATCHES.md`
(upstream revision recorded, patches explained, tests included).
