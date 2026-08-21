//! [184] Where a Lane Editor row's value comes from.
//!
//! One row implementation has to serve three stores:
//!
//! | Scope | Store | Fallback |
//! |---|---|---|
//! | lane global | `SoundSettingsState` atomics + the algo `IntParam` | — |
//! | per-step p-lock | `PlockState` (value + field mask) | the lane global |
//! | fusion morph endpoint | `FusedGroup.morph_targets` | the start cell's p-lock |
//!
//! This is a **trait with composition**, not an enum: `PlockSource` *owns* a
//! `GlobalSource`, and the morph source will own a `PlockSource`. That is what
//! makes [`ParamSource::inherited`] — the target of the revert affordance and of
//! double-click — correct by construction instead of re-implementing the
//! fallback chain in three `match` arms.
//!
//! The algo is reached through the [`AlgoSink`] seam rather than a `ParamSetter`
//! directly, because `ParamSetter` needs a live `GuiContext`: the seam is what
//! makes this whole layer testable headless.

use crate::param_id::ParamId;
use crate::plock::PlockState;
use crate::sound_settings::SoundSettingsState;

/// Whether a row may be edited in the current scope.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Support {
    Editable,
    /// Greyed with this reason on hover — **never hidden**. Hiding a row is what
    /// makes today's behaviour inexplicable (a p-lockable-looking parameter that
    /// simply is not there), and it breaks the "UI zones stables" rule.
    Disabled(&'static str),
}

// Consumed by the row driver in the p-lock scope ([184] phase 2).
#[allow(dead_code)]
impl Support {
    pub fn is_editable(self) -> bool {
        matches!(self, Support::Editable)
    }

    pub fn reason(self) -> Option<&'static str> {
        match self {
            Support::Editable => None,
            Support::Disabled(reason) => Some(reason),
        }
    }
}

/// The algo's store, abstracted so this layer never needs a `ParamSetter`.
///
/// The algo is the one parameter with two homes: a nih-plug `IntParam` when
/// lane-global (so the host sees automation), and p-lock field 13 per step.
pub trait AlgoSink {
    fn get(&self) -> u8;
    fn set(&self, value: u8);
}

/// Reads and writes one slot's parameters for the active scope.
// `is_overridden` / `clear` / `supports` / `salt` are exercised by the tests and
// consumed by the p-lock scope in the next phase; the lane-global panel only
// needs get/set/inherited/commit.
#[allow(dead_code)]
pub trait ParamSource {
    /// The value the row displays: the scope's override when it has one,
    /// otherwise whatever it inherits.
    fn get(&self, id: ParamId) -> f32;
    fn set(&mut self, id: ParamId, value: f32);
    /// True when the value comes from this scope rather than being inherited.
    /// Always false on the lane global — it inherits from nothing.
    fn is_overridden(&self, id: ParamId) -> bool;
    /// Drop this scope's override. No-op on the lane global.
    fn clear(&mut self, id: ParamId);
    /// What [`ParamSource::get`] would return with the override dropped: the
    /// revert target, and what the row shows greyed behind an override.
    fn inherited(&self, id: ParamId) -> f32;
    fn supports(&self, id: ParamId) -> Support;
    /// Discriminator for egui widget ids, so two live sources (a panel row and
    /// the popup, or two scopes) can never collide on `make_persistent_id`.
    fn salt(&self) -> (u8, usize, usize);
    /// Flush anything this source batches. Called once per frame by the panel.
    fn commit(&mut self) {}
}

// ── Lane global ─────────────────────────────────────────────────────────────

pub struct GlobalSource<'a> {
    pub settings: &'a SoundSettingsState,
    pub slot: usize,
    pub voice_idx: usize,
    pub algo: &'a dyn AlgoSink,
    /// Set by `set`, consumed by `commit`: one version bump per frame instead of
    /// one per edited row.
    dirty: bool,
}

impl<'a> GlobalSource<'a> {
    pub fn new(
        settings: &'a SoundSettingsState,
        slot: usize,
        voice_idx: usize,
        algo: &'a dyn AlgoSink,
    ) -> Self {
        Self {
            settings,
            slot,
            voice_idx,
            algo,
            dirty: false,
        }
    }

    /// The registry's factory default for this parameter — the one definition,
    /// replacing the three the Sound panel used to carry (per-voice
    /// `sound_settings_default`, `VoiceSettings::default()`, and `def.default`
    /// for specials).
    pub fn factory_default(&self, id: ParamId) -> f32 {
        crate::instrument_registry::param_default(self.voice_idx, id)
    }
}

impl ParamSource for GlobalSource<'_> {
    fn get(&self, id: ParamId) -> f32 {
        match id {
            ParamId::Algo => self.algo.get() as f32,
            _ => self.settings.instruments[self.slot].get(id),
        }
    }

    fn set(&mut self, id: ParamId, value: f32) {
        match id {
            ParamId::Algo => self.algo.set(value.round().clamp(0.0, 255.0) as u8),
            _ => {
                self.settings.instruments[self.slot].set(id, value);
                self.dirty = true;
            }
        }
    }

    fn is_overridden(&self, _id: ParamId) -> bool {
        false
    }

    fn clear(&mut self, _id: ParamId) {}

    fn inherited(&self, id: ParamId) -> f32 {
        self.factory_default(id)
    }

    fn supports(&self, _id: ParamId) -> Support {
        Support::Editable
    }

    fn salt(&self) -> (u8, usize, usize) {
        (0, self.slot, 0)
    }

    fn commit(&mut self) {
        if self.dirty {
            // The audio thread re-polls on a version change, once per buffer.
            self.settings.bump_version();
            self.dirty = false;
        }
    }
}

// ── Per-step p-lock ─────────────────────────────────────────────────────────

pub struct PlockSource<'a> {
    pub plock: &'a PlockState,
    pub step: usize,
    pub base: GlobalSource<'a>,
}

// The p-lock scope is wired to the panel in [184] phase 2; its behaviour is
// already pinned by the tests below.
#[allow(dead_code)]
impl<'a> PlockSource<'a> {
    pub fn new(plock: &'a PlockState, step: usize, base: GlobalSource<'a>) -> Self {
        Self { plock, step, base }
    }

    fn field(&self, id: ParamId) -> Option<usize> {
        if id.is_lockable() {
            id.plock_field()
        } else {
            None
        }
    }
}

impl ParamSource for PlockSource<'_> {
    fn get(&self, id: ParamId) -> f32 {
        match self.field(id) {
            Some(field) if self.plock.field_masks.is_set(self.base.slot, self.step, field) => {
                self.plock.values.get(self.base.slot, self.step, field)
            }
            _ => self.base.get(id),
        }
    }

    fn set(&mut self, id: ParamId, value: f32) {
        // A display mode belongs to the lane, in every scope.
        if id == ParamId::FreqMode {
            return self.base.set(id, value);
        }
        if let Some(field) = self.field(id) {
            // No `if overridden` gate: that gate is why the popup's checkbox rows
            // could never CREATE an override. `set_field` also raises the step's
            // active bit, so touching any row makes the p-lock exist.
            self.plock
                .set_field(self.base.slot, self.step, field, value);
        }
    }

    fn is_overridden(&self, id: ParamId) -> bool {
        self.field(id)
            .is_some_and(|field| self.plock.field_masks.is_set(self.base.slot, self.step, field))
    }

    fn clear(&mut self, id: ParamId) {
        if let Some(field) = self.field(id) {
            self.plock.clear_field(self.base.slot, self.step, field);
        }
    }

    /// The **live** lane value, not the factory default: dropping a per-step
    /// override means "follow the lane again".
    fn inherited(&self, id: ParamId) -> f32 {
        self.base.get(id)
    }

    fn supports(&self, id: ParamId) -> Support {
        match id.unlockable_reason() {
            Some(reason) => Support::Disabled(reason),
            None => Support::Editable,
        }
    }

    fn salt(&self) -> (u8, usize, usize) {
        (1, self.base.slot, self.step)
    }

    fn commit(&mut self) {
        // P-lock writes are immediate (the audio thread reads them at trigger
        // time, not through the version counter), but the lane fallback may have
        // been touched by a FreqMode change.
        self.base.commit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instrument_registry::StandardField;
    use crate::track::TrackLayoutState;
    use std::cell::Cell;

    /// Headless stand-in for the nih-plug `IntParam` + `ParamSetter` pair.
    struct CellAlgo(Cell<u8>);
    impl AlgoSink for CellAlgo {
        fn get(&self) -> u8 {
            self.0.get()
        }
        fn set(&self, value: u8) {
            self.0.set(value);
        }
    }

    const SLOT: usize = 2;
    const VOICE: usize = 0; // Kick
    const STEP: usize = 5;

    fn global<'a>(settings: &'a SoundSettingsState, algo: &'a CellAlgo) -> GlobalSource<'a> {
        GlobalSource::new(settings, SLOT, VOICE, algo)
    }

    #[test]
    fn global_source_round_trips_every_kind_of_parameter() {
        let layout = TrackLayoutState::default_layout();
        let settings = SoundSettingsState::new(&layout);
        let algo = CellAlgo(Cell::new(0));
        let mut src = global(&settings, &algo);

        src.set(ParamId::Std(StandardField::Decay), 0.42);
        assert_eq!(src.get(ParamId::Std(StandardField::Decay)), 0.42);

        src.set(ParamId::Special(3), 0.75);
        assert_eq!(src.get(ParamId::Special(3)), 0.75);

        // The algo goes to its own store, not the atomics.
        src.set(ParamId::Algo, 2.0);
        assert_eq!(algo.get(), 2);
        assert_eq!(src.get(ParamId::Algo), 2.0);

        src.set(ParamId::FreqMode, 1.0);
        assert_eq!(src.get(ParamId::FreqMode), 1.0);

        // Nothing is ever "overridden" on the lane itself.
        assert!(!src.is_overridden(ParamId::Std(StandardField::Decay)));
        assert!(src.supports(ParamId::Std(StandardField::Decay)).is_editable());
    }

    #[test]
    fn global_source_bumps_the_version_once_per_commit() {
        let layout = TrackLayoutState::default_layout();
        let settings = SoundSettingsState::new(&layout);
        let algo = CellAlgo(Cell::new(0));
        let before = settings.version.load(std::sync::atomic::Ordering::Acquire);
        let mut src = global(&settings, &algo);

        src.set(ParamId::Std(StandardField::Decay), 0.1);
        src.set(ParamId::Std(StandardField::Volume), 0.2);
        src.set(ParamId::Special(1), 0.3);
        assert_eq!(
            settings.version.load(std::sync::atomic::Ordering::Acquire),
            before,
            "writes must not bump the version per row"
        );

        src.commit();
        assert_eq!(
            settings.version.load(std::sync::atomic::Ordering::Acquire),
            before + 1,
            "one bump per frame"
        );
        src.commit();
        assert_eq!(
            settings.version.load(std::sync::atomic::Ordering::Acquire),
            before + 1,
            "a clean commit is a no-op"
        );
    }

    #[test]
    fn plock_source_falls_back_to_the_lane_then_overrides_then_reverts() {
        let layout = TrackLayoutState::default_layout();
        let settings = SoundSettingsState::new(&layout);
        let algo = CellAlgo(Cell::new(0));
        let plock = PlockState::new();
        {
            let mut lane = global(&settings, &algo);
            lane.set(ParamId::Std(StandardField::Decay), 0.30);
            lane.commit();
        }

        let mut src = PlockSource::new(&plock, STEP, global(&settings, &algo));
        let decay = ParamId::Std(StandardField::Decay);

        // No override yet: the row shows the lane value and is not accented.
        assert_eq!(src.get(decay), 0.30);
        assert!(!src.is_overridden(decay));
        assert_eq!(src.inherited(decay), 0.30);

        // Touching the row creates the p-lock AND the field override.
        src.set(decay, 0.80);
        assert!(src.is_overridden(decay));
        assert_eq!(src.get(decay), 0.80);
        assert!(plock.masks.is_active(SLOT, STEP), "the step's plock now exists");

        // Only that field is overridden.
        assert!(!src.is_overridden(ParamId::Std(StandardField::Volume)));

        // The lane keeps moving underneath; the override does not follow it.
        {
            let mut lane = global(&settings, &algo);
            lane.set(decay, 0.35);
            lane.commit();
        }
        assert_eq!(src.get(decay), 0.80);
        assert_eq!(src.inherited(decay), 0.35, "revert target follows the lane");

        // Reverting drops the override but keeps the p-lock alive.
        src.clear(decay);
        assert!(!src.is_overridden(decay));
        assert_eq!(src.get(decay), 0.35);
        assert!(plock.masks.is_active(SLOT, STEP));
    }

    /// The bug this design removes by construction: the popup gated its write on
    /// `overridden`, so a switch row could never create its own override.
    #[test]
    fn plock_source_can_create_an_override_for_a_switch_parameter() {
        let layout = TrackLayoutState::default_layout();
        let settings = SoundSettingsState::new(&layout);
        let algo = CellAlgo(Cell::new(0));
        let plock = PlockState::new();
        let mut src = PlockSource::new(&plock, STEP, global(&settings, &algo));
        let stereo = ParamId::Std(StandardField::Stereo);

        assert!(!src.is_overridden(stereo));
        src.set(stereo, 1.0);
        assert!(src.is_overridden(stereo), "a switch must be able to create its override");
        assert_eq!(src.get(stereo), 1.0);
    }

    #[test]
    fn freq_mode_and_the_aliased_special_are_lane_wide_in_plock_scope() {
        let layout = TrackLayoutState::default_layout();
        let settings = SoundSettingsState::new(&layout);
        let algo = CellAlgo(Cell::new(0));
        let plock = PlockState::new();
        let mut src = PlockSource::new(&plock, STEP, global(&settings, &algo));

        // FreqMode: writing it in p-lock scope must move the LANE, not the step.
        src.set(ParamId::FreqMode, 1.0);
        assert_eq!(settings.instruments[SLOT].get(ParamId::FreqMode), 1.0);
        assert!(!src.is_overridden(ParamId::FreqMode));
        assert_eq!(
            src.supports(ParamId::FreqMode).reason(),
            ParamId::FreqMode.unlockable_reason()
        );

        // The special that shares Attack's slot: greyed, with a reason, and a
        // write must NOT land on Attack.
        let aliased = ParamId::Special(4);
        assert!(!aliased.is_lockable());
        assert!(src.supports(aliased).reason().is_some());
        src.set(aliased, 0.9);
        assert!(!src.is_overridden(ParamId::Std(StandardField::Attack)));
        assert!(!plock.masks.is_active(SLOT, STEP), "no plock should have been created");
    }

    #[test]
    fn salts_differ_between_scopes_and_steps() {
        let layout = TrackLayoutState::default_layout();
        let settings = SoundSettingsState::new(&layout);
        let algo = CellAlgo(Cell::new(0));
        let plock = PlockState::new();
        let lane = global(&settings, &algo);
        let lane_salt = lane.salt();
        let step_a = PlockSource::new(&plock, 1, global(&settings, &algo)).salt();
        let step_b = PlockSource::new(&plock, 2, global(&settings, &algo)).salt();
        assert_ne!(lane_salt, step_a);
        assert_ne!(step_a, step_b);
    }
}
