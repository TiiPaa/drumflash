//! [184] The canonical identity of an editable per-slot sound parameter.
//!
//! This module owns the **only** conversion between a parameter and its
//! positional `plock-v1` / morph-target field index. Nothing else may hardcode
//! `0..=45`.
//!
//! # Why this exists
//!
//! The same `StandardField` mapping used to be hand-written **six** times and
//! had to stay in sync by discipline alone:
//!
//! | Where | Direction |
//! |---|---|
//! | `instrument_registry::plock_field_index` | `StandardField` → p-lock field |
//! | `ui/plock.rs` `get_global_value` | `StandardField` → the `load()` tuple |
//! | `ui/plock.rs` morph reset values | the same, again |
//! | `ui/grid.rs` `current_field_value_for_fusion` | p-lock field → the `load()` tuple |
//! | `lib.rs` `read_morph_value` / `apply_morph_value` | p-lock field → `VoiceSettings` |
//! | `ui/preset_browser.rs` `ORDER` | `StandardField` order for presets |
//!
//! # The two numbering schemes
//!
//! - The **`StandardField` discriminant** (0..12) is the order of
//!   `InstrumentSettingsState::load()`, of `sound_settings_default`, and of the
//!   `sound-settings-v2` blob. **Attack is 4.**
//! - The **p-lock field index** is the order of the `plock-v1` blob, where
//!   Attack was appended late as field **18** and specials live at
//!   `SPECIAL_FIELD_START + special_index`. **Attack is 18.**
//!
//! Confusing the two silently shifts every value, which is exactly what this
//! module makes impossible.
//!
//! # The Attack / special-4 collision
//!
//! Because Attack landed on 18 == `SPECIAL_FIELD_START + 4`, the special of
//! index 4 shares Attack's storage slot and is therefore **not** independently
//! p-lockable (see [`ParamId::is_lockable`]). Both `PlockState::get_settings`
//! and `set_settings` skip it, so no existing blob holds a value for it.
//!
//! It **can** be repaired without touching the format — the highest
//! `special_index` declared by any voice is 17, so slots 18..31 are free and
//! `Special(4)` could be re-homed on field 45 — but a legacy full snapshot has
//! all 46 mask bits set, so the mask needs a version marker before that value
//! can be trusted. That is deliberately a separate task, not part of [184].

use crate::instrument_registry::StandardField;

// ── The `plock-v1` field layout ─────────────────────────────────────────────
// These constants live here, in the module that owns the layout, and are
// re-exported by `plock.rs` so every existing `crate::plock::X` path keeps
// working. The dependency runs identity -> storage, never the other way, which
// is what lets the headless `test_standalone` binary include this module
// without dragging in the persistence layer.

/// 13 standard + 1 algo + 32 special.
pub const FIELD_COUNT: usize = 46;
/// The per-slot synthesis algorithm.
pub const ALGO_FIELD: usize = 13;
/// Where the 32 special fields start.
pub const SPECIAL_FIELD_START: usize = 14;
pub const SPECIAL_FIELD_COUNT: usize = 32;
/// Attack was appended late, landing on `SPECIAL_FIELD_START + 4`.
pub const ATTACK_FIELD: usize = 18;
/// Dead storage: only the Clap still reads it, as a legacy fallback.
pub const LEGACY_CLAP_ECHO_FIELD: usize = 12;

const _: () = assert!(ATTACK_FIELD == SPECIAL_FIELD_START + 4);
const _: () = assert!(FIELD_COUNT == SPECIAL_FIELD_START + SPECIAL_FIELD_COUNT);

/// Identity of one editable per-slot parameter, independent of which store
/// holds its value.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ParamId {
    /// One of the 13 named f32 fields of a slot's sound settings.
    Std(StandardField),
    /// The per-slot synthesis algorithm. A nih-plug `IntParam` when lane-global,
    /// p-lock field 13 when per-step — the only parameter with two stores.
    Algo,
    /// A registry `special_index` (0..32).
    Special(usize),
    /// The per-slot Hz/Note display flag. Never p-locked and never morphed: it
    /// is a display preference, not a sound value, so every scope routes it to
    /// the lane's own state.
    FreqMode,
}

impl ParamId {
    /// Index in the 46-field layout shared by `plock-v1` and `MorphTarget`.
    /// `None` when the parameter has no per-step storage of its own.
    pub const fn plock_field(self) -> Option<usize> {
        match self {
            ParamId::Std(field) => Some(field.plock_field_index()),
            ParamId::Algo => Some(ALGO_FIELD),
            ParamId::Special(index) => {
                if index < SPECIAL_FIELD_COUNT {
                    Some(SPECIAL_FIELD_START + index)
                } else {
                    None
                }
            }
            ParamId::FreqMode => None,
        }
    }

    /// The inverse of [`ParamId::plock_field`].
    ///
    /// Field 18 resolves to `Std(Attack)`, not `Special(4)` — that is the
    /// documented winner of the collision, matching `PlockState::get_settings`.
    /// Field 12 (the dead legacy clap-echo slot) belongs to no parameter.
    pub fn from_plock_field(field: usize) -> Option<Self> {
        if field == StandardField::Attack.plock_field_index() {
            return Some(ParamId::Std(StandardField::Attack));
        }
        if field == ALGO_FIELD {
            return Some(ParamId::Algo);
        }
        if let Some(std_field) = StandardField::ALL
            .iter()
            .copied()
            .find(|f| f.plock_field_index() == field)
        {
            return Some(ParamId::Std(std_field));
        }
        if field >= SPECIAL_FIELD_START && field < FIELD_COUNT {
            return Some(ParamId::Special(field - SPECIAL_FIELD_START));
        }
        None
    }

    /// `false` when this parameter cannot own a per-step override.
    ///
    /// Two cases: [`ParamId::FreqMode`] (a display mode), and the special that
    /// shares its field with Attack. A UI must **grey such a row with its
    /// reason**, never hide it — hiding is what makes today's behaviour
    /// inexplicable to the user.
    pub const fn is_lockable(self) -> bool {
        match self {
            ParamId::FreqMode => false,
            ParamId::Special(index) => {
                index < SPECIAL_FIELD_COUNT
                    && SPECIAL_FIELD_START + index != StandardField::Attack.plock_field_index()
            }
            _ => true,
        }
    }

    /// Why this parameter has no per-step slot, for the greyed row's tooltip.
    pub const fn unlockable_reason(self) -> Option<&'static str> {
        match self {
            ParamId::FreqMode => Some("Display mode: applies to the whole lane"),
            ParamId::Special(_) if !self.is_lockable() => {
                Some("Shares its p-lock slot with Attack - lock Attack instead")
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The exact table the six hand-written mappings agreed on, frozen so their
    /// removal is provably behaviour-preserving.
    #[test]
    fn plock_field_mapping_is_unchanged() {
        let expect = [
            (StandardField::Freq, 0usize),
            (StandardField::Decay, 1),
            (StandardField::Volume, 2),
            (StandardField::FilterFreq, 3),
            (StandardField::Release, 4),
            (StandardField::DecayCurve, 5),
            (StandardField::ReleaseCurve, 6),
            (StandardField::Hold, 7),
            (StandardField::FilterEnvAmount, 8),
            (StandardField::FilterEnvDecay, 9),
            (StandardField::Analog, 10),
            (StandardField::Stereo, 11),
            (StandardField::Attack, 18),
        ];
        assert_eq!(expect.len(), StandardField::ALL.len());
        for (field, index) in expect {
            assert_eq!(ParamId::Std(field).plock_field(), Some(index), "{field:?}");
        }
        assert_eq!(ParamId::Algo.plock_field(), Some(13));
        assert_eq!(ParamId::FreqMode.plock_field(), None);
        for index in 0..SPECIAL_FIELD_COUNT {
            assert_eq!(
                ParamId::Special(index).plock_field(),
                Some(SPECIAL_FIELD_START + index),
                "special {index}"
            );
        }
        assert_eq!(ParamId::Special(SPECIAL_FIELD_COUNT).plock_field(), None);
    }

    #[test]
    fn from_plock_field_round_trips_every_id() {
        for field in StandardField::ALL {
            let id = ParamId::Std(field);
            let index = id.plock_field().expect("standard fields are addressable");
            assert_eq!(ParamId::from_plock_field(index), Some(id), "{field:?}");
        }
        assert_eq!(ParamId::from_plock_field(ALGO_FIELD), Some(ParamId::Algo));
        for index in 0..SPECIAL_FIELD_COUNT {
            let field = SPECIAL_FIELD_START + index;
            let expected = if field == StandardField::Attack.plock_field_index() {
                // The collision: field 18 belongs to Attack, not to special 4.
                ParamId::Std(StandardField::Attack)
            } else {
                ParamId::Special(index)
            };
            assert_eq!(ParamId::from_plock_field(field), Some(expected), "field {field}");
        }
        // Field 12 is the dead legacy clap-echo slot: no parameter claims it.
        assert_eq!(ParamId::from_plock_field(12), None);
        assert_eq!(ParamId::from_plock_field(FIELD_COUNT), None);
    }

    /// Every addressable field is claimed by exactly one id, and every id maps
    /// to a distinct field. A bijection with exactly one hole (field 12).
    #[test]
    fn the_field_layout_is_a_bijection_with_one_documented_hole() {
        let mut owner: Vec<Option<ParamId>> = vec![None; FIELD_COUNT];
        let ids = StandardField::ALL
            .iter()
            .map(|f| ParamId::Std(*f))
            .chain(std::iter::once(ParamId::Algo))
            .chain((0..SPECIAL_FIELD_COUNT).map(ParamId::Special));
        for id in ids {
            if !id.is_lockable() {
                continue;
            }
            let field = id.plock_field().expect("lockable ids are addressable");
            assert!(
                owner[field].is_none(),
                "field {field} claimed twice: {:?} and {id:?}",
                owner[field]
            );
            owner[field] = Some(id);
        }
        let unclaimed: Vec<usize> = (0..FIELD_COUNT).filter(|f| owner[*f].is_none()).collect();
        assert_eq!(
            unclaimed,
            vec![12],
            "only the legacy clap-echo field may be unclaimed"
        );
    }

    #[test]
    fn only_the_colliding_special_is_unlockable() {
        let unlockable: Vec<usize> = (0..SPECIAL_FIELD_COUNT)
            .filter(|i| !ParamId::Special(*i).is_lockable())
            .collect();
        assert_eq!(
            unlockable,
            vec![StandardField::Attack.plock_field_index() - SPECIAL_FIELD_START],
            "exactly one special may be shadowed by the Attack field"
        );
        assert!(ParamId::Std(StandardField::Attack).is_lockable());
        assert!(ParamId::Algo.is_lockable());
        assert!(!ParamId::FreqMode.is_lockable());
        // Every unlockable id must be able to explain itself to the user.
        for id in [ParamId::FreqMode, ParamId::Special(4)] {
            assert!(id.unlockable_reason().is_some(), "{id:?} needs a reason");
        }
        assert!(ParamId::Std(StandardField::Freq).unlockable_reason().is_none());
    }
}
