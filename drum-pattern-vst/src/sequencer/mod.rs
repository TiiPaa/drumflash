//! Sequencer module for 16-step drum patterns
//!
//! Architecture:
//! - One master beat position advances uniformly (0..4 beats = 1 bar = 16 steps).
//! - Groove (swing type + amount) is applied to the master grid.
//! - Each track reads  master_step % track_length  → loops cleanly on its own length.
//! - Per-track push/pull shifts the effective beat before step lookup.
//! - Per-track humanize only affects velocity (no timing jitter → no double triggers).

pub(crate) mod pattern;

use std::sync::Arc;

use crate::groove::{self, GrooveType};
pub use pattern::{FusedGroup, MorphTarget, Pattern, SharedPattern, MAX_FUSIONS};

const MAX_TRACKS: usize = crate::track::MAX_TRACKS;
/// Mirrors `plock::STEP_COUNT` (kept literal: `test_standalone` compiles this
/// module without the `plock` module).
const SEQ_STEP_COUNT: usize = 64;

/// Per-instrument state.
#[derive(Clone, Copy, Debug)]
pub struct TrackState {
    pub previous_step: usize,
    /// Previous master step (0-15), used for trigger detection.
    /// Detecting on shifted_master instead of current_step fixes the
    /// track_length=1 bug where current_step never changes.
    pub previous_shifted_master: usize,
    pub track_length: usize,
    pub push_pull_ms: f32,
    pub humanize_amount: f32,
    /// Independent step counter for true polyrhythm.
    /// Increments by 1 on every master-step transition.
    /// current_step = step_counter % track_length.
    pub step_counter: usize,
    /// Simple LCG RNG state for deterministic per-track randomness.
    rng_state: u32,
    /// Microtiming late-fire: the current cell's trigger, deferred by a
    /// positive nudge; emitted once the shifted beat passes `late_fire_beat`.
    pub late_trigger: Option<TriggerResult>,
    pub late_fire_beat: f64,
    /// Set when the next boundary's cell was already fired early (negative
    /// microtiming): the normal transition still advances the state but must
    /// stay silent for that cell.
    pub suppress_next: bool,
    /// Set once the next boundary's cell has been fired early, so the early
    /// check does not re-fire on every remaining sample before the boundary.
    pub early_fired: bool,
}

#[derive(Clone, Copy, Debug)]
struct FusionTrack {
    groups: [FusedGroup; MAX_FUSIONS],
    count: usize,
}

impl Default for FusionTrack {
    fn default() -> Self {
        Self {
            groups: [FusedGroup::default(); MAX_FUSIONS],
            count: 0,
        }
    }
}

impl FusionTrack {
    #[cfg(test)]
    fn set_from_slice(&mut self, fusions: &[FusedGroup]) {
        self.count = 0;
        for group in fusions
            .iter()
            .copied()
            .filter(FusedGroup::is_valid)
            .take(MAX_FUSIONS)
        {
            self.groups[self.count] = group;
            self.count += 1;
        }
    }

    fn set_from_pattern(&mut self, pattern: &SharedPattern, instrument: usize) {
        self.count = pattern.load_fusions_into(instrument, &mut self.groups);
    }

    fn containing(&self, step: usize) -> Option<FusedGroup> {
        for i in 0..self.count {
            let group = self.groups[i];
            if group.contains(step) {
                return Some(group);
            }
        }
        None
    }
}

impl Default for TrackState {
    fn default() -> Self {
        Self {
            previous_step: 15,
            previous_shifted_master: 15,
            track_length: 16,
            push_pull_ms: 0.0,
            humanize_amount: 0.0,
            step_counter: 0,
            rng_state: 0xACE1_0000,
            late_trigger: None,
            late_fire_beat: 0.0,
            suppress_next: false,
            early_fired: false,
        }
    }
}

impl TrackState {
    fn next_rand(&mut self) -> f32 {
        self.rng_state = self.rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        ((self.rng_state >> 16) & 0x7FFF) as f32 / 32767.0
    }

    /// Drop any pending microtiming state (seek/stop/reset): a late trigger
    /// scheduled before the jump must not fire after it.
    fn clear_microtiming_state(&mut self) {
        self.late_trigger = None;
        self.suppress_next = false;
        self.early_fired = false;
    }
}

/// What a cell does when the playhead reaches it.
enum CellFire {
    /// Covered by a fused cell starting earlier: no independent trigger.
    Covered,
    /// Emits a trigger (a plain cell or the start of a fused cell).
    Fire {
        source_step: usize,
        fusion_pulse_count: u8,
        fusion_span_cells: u8,
        morph_count: u8,
        morph_targets: [MorphTarget; 4],
    },
}

fn classify_cell(fusions: &FusionTrack, step: usize) -> CellFire {
    match fusions.containing(step) {
        Some(group) if group.is_start(step) => CellFire::Fire {
            source_step: group.start_cell as usize,
            fusion_pulse_count: group.step_count.clamp(1, 64),
            fusion_span_cells: group.cell_span().min(64) as u8,
            morph_count: group.morph_count,
            morph_targets: group.morph_targets,
        },
        Some(_) => CellFire::Covered,
        None => CellFire::Fire {
            source_step: step,
            fusion_pulse_count: 1,
            fusion_span_cells: 1,
            morph_count: 0,
            morph_targets: [MorphTarget::default(); 4],
        },
    }
}

pub struct Sequencer {
    is_playing: bool,
    /// Master beat position (0.0 .. master_length * 0.25). 1 step = 0.25 beat.
    beat_position: f64,
    tracks: [TrackState; MAX_TRACKS],
    /// Cached values for sync_to_host.
    swing: f32,
    groove_type: GrooveType,
    pattern: Arc<SharedPattern>,
    mutes: [bool; MAX_TRACKS],
    /// Global pattern length (1-64 steps). Controls master loop point.
    master_length: usize,
    /// How many times the master pattern has looped (for step conditions).
    loop_count: usize,
    /// Per-slot mapping to the legacy `DrumVoice` index used for synthesis.
    /// `None` means the slot is inactive.
    slot_voices: [Option<usize>; MAX_TRACKS],
    /// Per-instrument fused cell groups (Step Fusion), copied from SharedPattern once per buffer.
    fusions: [FusionTrack; MAX_TRACKS],
    /// Per-slot grid source: a lane linked to the one above plays that lane's
    /// steps + fusions (layering). Identity by default; refreshed each buffer
    /// from the track layout via `set_grid_slots`.
    grid_slots: [usize; MAX_TRACKS],
    /// Per-cell microtiming (ms, -50..+50), indexed by (slot, source step) like
    /// the sequencer plocks. Copied from the atomics once per buffer.
    microtimings: [[f32; SEQ_STEP_COUNT]; MAX_TRACKS],
}

/// Per-instrument trigger result.
#[derive(Clone, Copy, Debug)]
pub struct TriggerResult {
    pub should_trigger: bool,
    pub velocity: f32,
    /// Cell used for sound plocks, sequencer plocks and MIDI export of this hit.
    pub step: usize,
    /// Number of evenly spaced pulses to emit over `fusion_span_cells` cells.
    /// `1` means a normal, non-fused trigger.
    pub fusion_pulse_count: u8,
    pub fusion_span_cells: u8,
    /// Number of active morphing targets (0-4). Only meaningful for fused triggers.
    pub morph_count: u8,
    /// Morphing targets. Only the first `morph_count` entries are valid.
    pub morph_targets: [MorphTarget; 4],
    /// This trigger fired EARLY (negative microtiming) across the pattern wrap:
    /// it belongs to the NEXT loop, so step conditions must be evaluated with
    /// `loop_count + 1`.
    pub early_next_loop: bool,
}

impl Default for TriggerResult {
    fn default() -> Self {
        Self {
            should_trigger: false,
            velocity: 0.0,
            step: 0,
            fusion_pulse_count: 1,
            fusion_span_cells: 1,
            morph_count: 0,
            morph_targets: [MorphTarget::default(); 4],
            early_next_loop: false,
        }
    }
}

impl TriggerResult {
    pub fn is_fusion(&self) -> bool {
        self.fusion_span_cells > 1
    }
}

impl Sequencer {
    pub fn new(pattern: Arc<SharedPattern>) -> Self {
        Self {
            is_playing: false,
            beat_position: 0.0,
            tracks: [TrackState::default(); MAX_TRACKS],
            swing: 0.0,
            groove_type: GrooveType::Swing16,
            pattern,
            mutes: [false; MAX_TRACKS],
            master_length: 16,
            loop_count: 0,
            slot_voices: [None; MAX_TRACKS],
            fusions: [FusionTrack::default(); MAX_TRACKS],
            grid_slots: std::array::from_fn(|i| i),
            microtimings: [[0.0; SEQ_STEP_COUNT]; MAX_TRACKS],
        }
    }

    /// Set the per-slot grid source (for lane linking / layering). Call once per
    /// buffer from the audio thread. Identity = no linking.
    pub fn set_grid_slots(&mut self, grid_slots: [usize; MAX_TRACKS]) {
        self.grid_slots = grid_slots;
    }

    /// Copy the per-cell microtiming (ms) from the seq-plock atomics. Call once
    /// per audio buffer, never per sample.
    pub fn set_microtimings(
        &mut self,
        microtimings: [[f32; SEQ_STEP_COUNT]; MAX_TRACKS],
    ) {
        self.microtimings = microtimings;
    }

    /// Copy fusion groups from SharedPattern into fixed local arrays.
    /// Call once per audio buffer, never per sample.
    pub fn sync_fusions_from_pattern(&mut self) {
        for slot in 0..MAX_TRACKS {
            self.fusions[slot].set_from_pattern(&self.pattern, slot);
        }
    }

    #[cfg(test)]
    fn set_fusions_for_test(&mut self, instrument: usize, fusions: &[FusedGroup]) {
        if instrument < MAX_TRACKS {
            self.fusions[instrument].set_from_slice(fusions);
        }
    }

    pub fn process_sample(
        &mut self,
        bpm: f32,
        sample_rate: f32,
        swing: f32,
        groove_type: GrooveType,
    ) -> [TriggerResult; MAX_TRACKS] {
        let mut triggers = [TriggerResult::default(); MAX_TRACKS];

        if !self.is_playing {
            return triggers;
        }

        self.swing = swing;
        self.groove_type = groove_type;

        // Advance master beat position uniformly. Wrap at master_length steps.
        let beat_increment = (bpm as f64 / 60.0) / sample_rate as f64;
        let master_beat_length = self.master_length as f64 * 0.25;
        let prev_beat = self.beat_position;
        self.beat_position += beat_increment;
        if self.beat_position >= master_beat_length {
            self.beat_position -= master_beat_length;
            // Detect loop wrap (only when actually wrapping, not on seek)
            if prev_beat + beat_increment >= master_beat_length {
                self.loop_count = self.loop_count.wrapping_add(1);
            }
        }

        // Master beat advances uniformly; each track derives its own step.

        let master_beat_length = self.master_length as f64 * 0.25;

        for slot in 0..MAX_TRACKS {
            let track = &mut self.tracks[slot];

            // Push/pull: convert ms to beats and subtract so positive = late.
            let push_pull_beats = track.push_pull_ms as f64 * bpm as f64 / (60.0 * 1000.0);
            let shifted_beat =
                (self.beat_position - push_pull_beats).rem_euclid(master_beat_length);

            // Re-compute master step for this track's shifted timeline.
            let shifted_master = groove::beat_to_step(shifted_beat, swing, groove_type);

            // Grid source: a linked lane plays the steps + fusions of the
            // lane above it (layering); identity otherwise.
            let grid = self.grid_slots[slot];

            // 1) Microtiming late-fire: a positively nudged cell whose delayed
            // fire time has arrived.
            if let Some(late) = track.late_trigger {
                if shifted_beat >= track.late_fire_beat {
                    track.late_trigger = None;
                    triggers[slot] = late;
                }
            }

            // 2) Trigger on master step transition.
            // Using shifted_master (not current_step) fixes the track_length=1 bug
            // where current_step never changes (always 0).
            if shifted_master != track.previous_shifted_master {
                // True polyrhythm: each track advances its own independent counter.
                track.step_counter = track.step_counter.wrapping_add(1);
                let current_step = track.step_counter % track.track_length.max(1);
                track.previous_shifted_master = shifted_master;
                track.previous_step = current_step;
                track.early_fired = false;

                if track.suppress_next {
                    // This cell already fired early (negative microtiming).
                    track.suppress_next = false;
                } else if let CellFire::Fire {
                    source_step,
                    fusion_pulse_count,
                    fusion_span_cells,
                    morph_count,
                    morph_targets,
                } = classify_cell(&self.fusions[grid], current_step)
                {
                    let trig = Self::eval_trigger(
                        &self.pattern,
                        self.mutes[slot],
                        track,
                        source_step,
                        fusion_pulse_count,
                        fusion_span_cells,
                        morph_count,
                        morph_targets,
                        grid,
                    );
                    if trig.should_trigger {
                        let micro = self.microtimings[slot][source_step];
                        if micro > 0.0 {
                            // Positive nudge: defer the whole trigger (its
                            // stutter/fusion pulses expand from the delayed
                            // fire time in the audio engine).
                            let micro_beats = micro as f64 * bpm as f64 / 60000.0;
                            track.late_trigger = Some(trig);
                            track.late_fire_beat = shifted_beat + micro_beats;
                        } else if triggers[slot].should_trigger {
                            // Rare collision with a late fire this very sample:
                            // defer this trigger by one sample instead of
                            // dropping a hit.
                            track.late_trigger = Some(trig);
                            track.late_fire_beat = shifted_beat;
                        } else {
                            triggers[slot] = trig;
                        }
                    }
                }
            }

            // 3) Microtiming early-fire: the NEXT boundary's cell has a
            // negative nudge — fire it up to 50 ms before its step boundary.
            if !track.early_fired && !track.suppress_next {
                let next_step = track.step_counter.wrapping_add(1) % track.track_length.max(1);
                if let CellFire::Fire { source_step, .. } =
                    classify_cell(&self.fusions[grid], next_step)
                {
                    let micro = self.microtimings[slot][source_step];
                    if micro < 0.0 {
                        let next_master = shifted_master + 1;
                        let (delta_beats, crosses_wrap) = if next_master < self.master_length {
                            (
                                groove::step_start_beat(next_master, swing, groove_type)
                                    - shifted_beat,
                                false,
                            )
                        } else {
                            (master_beat_length - shifted_beat, true)
                        };
                        let delta_ms = delta_beats / bpm as f64 * 60000.0;
                        if delta_ms <= -(micro as f64) {
                            if let CellFire::Fire {
                                source_step,
                                fusion_pulse_count,
                                fusion_span_cells,
                                morph_count,
                                morph_targets,
                            } = classify_cell(&self.fusions[grid], next_step)
                            {
                                let mut trig = Self::eval_trigger(
                                    &self.pattern,
                                    self.mutes[slot],
                                    track,
                                    source_step,
                                    fusion_pulse_count,
                                    fusion_span_cells,
                                    morph_count,
                                    morph_targets,
                                    grid,
                                );
                                if trig.should_trigger {
                                    trig.early_next_loop = crosses_wrap;
                                    if triggers[slot].should_trigger {
                                        // Same-sample collision: one sample
                                        // late beats a dropped hit.
                                        track.late_trigger = Some(trig);
                                        track.late_fire_beat = shifted_beat;
                                    } else {
                                        triggers[slot] = trig;
                                    }
                                    track.early_fired = true;
                                    track.suppress_next = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        triggers
    }

    /// Evaluate the trigger for a firing cell: mask + mute gate, humanized
    /// velocity. Shared by the on-boundary path and the microtiming
    /// early/late paths so all three produce identical triggers.
    #[allow(clippy::too_many_arguments)]
    fn eval_trigger(
        pattern: &SharedPattern,
        muted: bool,
        track: &mut TrackState,
        source_step: usize,
        fusion_pulse_count: u8,
        fusion_span_cells: u8,
        morph_count: u8,
        morph_targets: [MorphTarget; 4],
        grid: usize,
    ) -> TriggerResult {
        let step_mask = pattern.load_step_mask(source_step);
        let active = (step_mask & (1 << grid)) != 0 && !muted;

        let velocity = if active {
            if track.humanize_amount > 0.0 {
                let r = track.next_rand();
                (0.8 + (r - 0.5) * track.humanize_amount).clamp(0.1, 1.0)
            } else {
                0.8
            }
        } else {
            0.0
        };

        TriggerResult {
            should_trigger: active,
            velocity,
            step: source_step,
            fusion_pulse_count,
            fusion_span_cells,
            morph_count,
            morph_targets,
            early_next_loop: false,
        }
    }

    pub fn sync_to_host(&mut self, position_beats: f64, bpm: f32, _sample_rate: f32) {        let master_beat_length = self.master_length as f64 * 0.25;
        self.beat_position = position_beats.rem_euclid(master_beat_length);
        // Keep loop_count in sync with the host's absolute timeline so
        // step conditions (1st loop, 2/2, etc.) work when driven by DAW transport.
        self.loop_count = (position_beats / master_beat_length).floor() as usize;
        for track in self.tracks.iter_mut() {
            let push_pull_beats = track.push_pull_ms as f64 * bpm as f64 / (60.0 * 1000.0);
            let shifted_beat =
                (self.beat_position - push_pull_beats).rem_euclid(master_beat_length);
            let shifted_master = groove::beat_to_step(shifted_beat, self.swing, self.groove_type);
            track.previous_shifted_master = shifted_master;

            // Reconstruct the number of shifted step boundaries crossed so far.
            // Using the shifted timeline (master position minus push/pull offset)
            // keeps each track's phase correct after a seek, instead of snapping
            // every track to the master step count.
            let shifted_steps = ((position_beats - push_pull_beats) / 0.25).floor() as i64;
            track.step_counter = shifted_steps as usize;
            track.previous_step = track.step_counter % track.track_length.max(1);
            track.clear_microtiming_state();
        }
    }

    pub fn reset(&mut self) {
        self.beat_position = 0.0;
        self.loop_count = 0;
        let max_step = self.master_length.saturating_sub(1);
        for track in self.tracks.iter_mut() {
            track.previous_step = max_step;
            track.previous_shifted_master = max_step;
            track.step_counter = 0;
            track.clear_microtiming_state();
        }
        self.is_playing = false;
    }

    pub fn play(&mut self) {
        self.is_playing = true;
        self.loop_count = 0;
        let max_step = self.master_length.saturating_sub(1);
        for track in self.tracks.iter_mut() {
            track.previous_step = max_step; // Force trigger on next step 0
            track.previous_shifted_master = max_step;
            track.step_counter = track.track_length.wrapping_sub(1);
            track.clear_microtiming_state();
        }
    }

    /// Force a trigger on step 0 for all tracks.
    /// Call after sync_to_host when starting near beat 0 to avoid missing the first step.
    pub fn force_step0_trigger(&mut self) {
        let max_step = self.master_length.saturating_sub(1);
        for track in self.tracks.iter_mut() {
            track.previous_step = max_step;
            track.previous_shifted_master = max_step;
            track.step_counter = track.track_length.wrapping_sub(1);
            track.clear_microtiming_state();
        }
    }

    /// Start the current pattern from step 0 on the next processed sample.
    pub fn restart_pattern_from_step0(&mut self) {
        self.beat_position = 0.0;
        self.loop_count = 0;
        self.force_step0_trigger();
    }

    pub fn stop(&mut self) {
        self.is_playing = false;
        self.reset();
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    /// Set per-track parameters. Called once per buffer (or when params change).
    pub fn set_track_params(
        &mut self,
        lengths: [usize; MAX_TRACKS],
        push_pulls: [f32; MAX_TRACKS],
        humanizes: [f32; MAX_TRACKS],
        master_length: usize,
    ) {
        self.master_length = master_length.clamp(1, 64);
        for i in 0..MAX_TRACKS {
            self.tracks[i].track_length = lengths[i].clamp(1, 64);
            self.tracks[i].push_pull_ms = push_pulls[i];
            self.tracks[i].humanize_amount = humanizes[i].clamp(0.0, 1.0);
        }
    }

    /// Set position from DAW transport (in steps 0-63).
    #[allow(dead_code)]
    pub fn set_position(&mut self, step: usize) {
        self.beat_position = (step as f64) * 0.25;
        let master_step = groove::beat_to_step(self.beat_position, self.swing, self.groove_type);
        for track in self.tracks.iter_mut() {
            track.previous_shifted_master = master_step;
            track.step_counter = master_step;
            track.previous_step = track.step_counter % track.track_length.max(1);
            track.clear_microtiming_state();
        }
    }

    /// Returns the master step for UI highlighting.
    pub fn current_step(&self) -> usize {
        groove::beat_to_step(self.beat_position, self.swing, self.groove_type)
    }

    /// Returns the internal beat position (0..4).
    pub fn beat_position(&self) -> f64 {
        self.beat_position
    }

    /// Returns per-track current step for UI highlighting.
    pub fn current_steps(&self) -> [usize; MAX_TRACKS] {
        let mut steps = [0usize; MAX_TRACKS];
        for (i, track) in self.tracks.iter().enumerate() {
            steps[i] = track.step_counter % track.track_length.max(1);
        }
        steps
    }

    pub fn loop_count(&self) -> usize {
        self.loop_count
    }

    pub fn set_mutes(&mut self, mutes: [bool; MAX_TRACKS]) {
        self.mutes = mutes;
    }

    pub fn set_slot_voices(&mut self, slot_voices: [Option<usize>; MAX_TRACKS]) {
        self.slot_voices = slot_voices;
    }

    #[allow(dead_code)]
    pub fn slot_voices(&self) -> &[Option<usize>; MAX_TRACKS] {
        &self.slot_voices
    }

    #[allow(dead_code)]
    pub fn pattern(&self) -> &Arc<SharedPattern> {
        &self.pattern
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequencer_timing() {
        let shared_pattern = SharedPattern::new(&Pattern::rock_pattern());
        let mut seq = Sequencer::new(shared_pattern);
        let sample_rate = 44100.0;
        let bpm = 120.0;

        seq.play();

        let mut triggers_count = 0;
        for _ in 0..6000 {
            let triggers = seq.process_sample(bpm, sample_rate, 0.0, GrooveType::Swing16);
            if triggers.iter().any(|trigger| trigger.should_trigger) {
                triggers_count += 1;
            }
        }

        assert!(triggers_count > 0);
    }

    #[test]
    fn test_sequencer_drift_over_bars() {
        let shared_pattern = SharedPattern::new(&Pattern::empty());
        for step in 0..16 {
            shared_pattern.set_step_mask(step, 0x3ff);
        }
        let mut seq = Sequencer::new(shared_pattern);
        let sample_rate = 44100.0;
        let bpm = 120.0;
        seq.play();

        let samples_per_bar_exact = (60.0 / bpm * 4.0 * sample_rate) as usize; // 88200
        let total_bars = 10;
        let total_samples = samples_per_bar_exact * total_bars + 10;

        let mut step_triggers = Vec::new();
        for sample_idx in 0..total_samples {
            let triggers = seq.process_sample(bpm, sample_rate, 0.0, GrooveType::Swing16);
            if triggers.iter().any(|trigger| trigger.should_trigger) {
                step_triggers.push(sample_idx);
            }
        }

        let start_of_bar_2 = step_triggers[16];
        let bar_2_drift = (start_of_bar_2 as isize) - (samples_per_bar_exact as isize);
        assert!(
            bar_2_drift.abs() <= 1,
            "Timing drift: bar 2 started at sample {} instead of {} (drift = {})",
            start_of_bar_2,
            samples_per_bar_exact,
            bar_2_drift
        );

        let start_of_bar_11 = step_triggers[160];
        let expected_bar_11_start = 10 * samples_per_bar_exact;
        let drift = expected_bar_11_start as isize - start_of_bar_11 as isize;
        assert!(
            drift.abs() <= 1,
            "Timing drift: {} samples over 10 bars",
            drift
        );
    }

    #[test]
    fn test_swing_delays_odd_steps() {
        let shared_pattern = SharedPattern::new(&Pattern::empty());
        for step in 0..16 {
            shared_pattern.set_step_mask(step, 0x3ff);
        }

        let sample_rate = 44100.0;
        let bpm = 120.0;

        let mut seq = Sequencer::new(shared_pattern.clone());
        seq.play();
        let mut straight_positions = Vec::new();
        for sample_idx in 0..(44100 * 2) {
            let triggers = seq.process_sample(bpm, sample_rate, 0.0, GrooveType::Swing16);
            if triggers.iter().any(|trigger| trigger.should_trigger) {
                straight_positions.push(sample_idx);
            }
        }

        let mut seq = Sequencer::new(shared_pattern);
        seq.play();
        let mut swing_positions = Vec::new();
        for sample_idx in 0..(44100 * 2) {
            let triggers = seq.process_sample(bpm, sample_rate, 0.5, GrooveType::Swing16);
            if triggers.iter().any(|trigger| trigger.should_trigger) {
                swing_positions.push(sample_idx);
            }
        }

        let step1_straight = straight_positions[1];
        let step1_swing = swing_positions[1];
        assert!(
            step1_swing > step1_straight,
            "Step 1 with swing should be delayed (straight={}, swing={})",
            step1_straight,
            step1_swing
        );

        let step0_delta = (swing_positions[0] as isize - straight_positions[0] as isize).abs();
        assert!(
            step0_delta <= 1,
            "Step 0 should not drift with swing (delta={})",
            step0_delta
        );
    }

    #[test]
    fn test_swing_total_bar_duration_unchanged() {
        let shared_pattern = SharedPattern::new(&Pattern::empty());
        for step in 0..16 {
            shared_pattern.set_step_mask(step, 0x3ff);
        }

        let sample_rate = 44100.0;
        let bpm = 120.0;
        let samples_per_bar = (60.0 / bpm * 4.0 * sample_rate) as usize;

        for swing in [0.0_f32, 0.25, 0.5] {
            let mut seq = Sequencer::new(shared_pattern.clone());
            seq.play();

            let mut bar_start = 0_usize;
            let mut triggers_seen = 0_usize;

            for sample_idx in 0..(samples_per_bar * 3 + 100) {
                let triggers = seq.process_sample(bpm, sample_rate, swing, GrooveType::Swing16);
                if triggers.iter().any(|trigger| trigger.should_trigger) {
                    triggers_seen += 1;
                    if triggers_seen % 16 == 1 && triggers_seen > 1 {
                        let measured_bar_duration = sample_idx - bar_start;
                        let drift = (measured_bar_duration as isize) - (samples_per_bar as isize);
                        assert!(
                            drift.abs() <= 1,
                            "Swing={}: bar duration drift = {} samples",
                            swing,
                            drift
                        );
                        bar_start = sample_idx;
                    }
                }
            }
        }
    }

    #[test]
    fn test_polyrhythm_different_lengths() {
        // Kick on every step (16-step cycle), snare on every step (12-step cycle).
        let shared_pattern = SharedPattern::new(&Pattern::empty());
        for step in 0..16 {
            shared_pattern.set_step_mask(step, 0b0000_0000_0011); // kick + snare
        }

        let mut seq = Sequencer::new(shared_pattern);
        seq.tracks[0].track_length = 16; // Kick
        seq.tracks[1].track_length = 12; // Snare
        seq.play();

        let sample_rate = 44100.0;
        let bpm = 120.0;

        let mut kick_steps = Vec::new();
        let mut snare_steps = Vec::new();

        for _ in 0..(88200 * 4) {
            let triggers = seq.process_sample(bpm, sample_rate, 0.0, GrooveType::Straight);
            if triggers[0].should_trigger {
                kick_steps.push(seq.tracks[0].previous_step);
            }
            if triggers[1].should_trigger {
                snare_steps.push(seq.tracks[1].previous_step);
            }
        }

        // Kick should cycle 0..15 repeatedly
        assert!(
            kick_steps
                .windows(16)
                .any(|w| w == [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
            "Kick should cycle through all 16 steps"
        );

        // Snare should cycle 0..11 repeatedly
        assert!(
            snare_steps
                .windows(12)
                .any(|w| w == [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]),
            "Snare should cycle through all 12 steps"
        );
    }

    #[test]
    fn test_push_pull_delays_track() {
        // Kick triggers on every step so we can measure interval differences.
        let shared_pattern = SharedPattern::new(&Pattern::empty());
        for step in 0..16 {
            shared_pattern.set_step_mask(step, 0b0000_0000_0001); // kick on every step
        }

        let mut seq = Sequencer::new(shared_pattern.clone());
        seq.tracks[0].push_pull_ms = 20.0; // 20 ms late
        seq.play();

        let sample_rate = 44100.0;
        let bpm = 120.0;

        let mut seq_straight = Sequencer::new(shared_pattern);
        seq_straight.play();

        let mut delayed_steps = Vec::new();
        let mut straight_steps = Vec::new();

        for sample_idx in 0..20000 {
            let t1 = seq.process_sample(bpm, sample_rate, 0.0, GrooveType::Straight);
            let t2 = seq_straight.process_sample(bpm, sample_rate, 0.0, GrooveType::Straight);
            if t1[0].should_trigger {
                delayed_steps.push(sample_idx);
            }
            if t2[0].should_trigger {
                straight_steps.push(sample_idx);
            }
        }

        // With +20 ms push/pull (positive = late), step 1 should arrive later.
        let step1_delayed: usize = delayed_steps[1];
        let step1_straight: usize = straight_steps[1];
        let expected_delay: usize = (0.020 * sample_rate as f64) as usize;
        let actual_delay = step1_delayed.saturating_sub(step1_straight);
        assert!(
            actual_delay >= expected_delay - 50 && actual_delay <= expected_delay + 50,
            "Push/pull delay mismatch: expected ~{}, got {} (delayed={}, straight={})",
            expected_delay,
            actual_delay,
            step1_delayed,
            step1_straight
        );
    }

    #[test]
    fn restart_pattern_from_step0_forces_step0_after_length_change() {
        let shared_pattern = SharedPattern::new(&Pattern::empty());
        shared_pattern.set_step_mask(0, 0b0000_0000_0001);

        let mut seq = Sequencer::new(shared_pattern);
        seq.play();
        seq.set_track_params([8; MAX_TRACKS], [0.0; MAX_TRACKS], [0.0; MAX_TRACKS], 8);
        seq.set_track_params([16; MAX_TRACKS], [0.0; MAX_TRACKS], [0.0; MAX_TRACKS], 16);
        seq.restart_pattern_from_step0();

        let triggers = seq.process_sample(120.0, 44_100.0, 0.0, GrooveType::Swing16);

        assert!(triggers[0].should_trigger);
        assert_eq!(triggers[0].step, 0);
        assert_eq!(seq.current_step(), 0);
    }

    #[test]
    fn test_push_pull_sync_to_host_preserves_phase() {
        // Kick on every step, with +30 ms push/pull.
        // Verify that after a sync_to_host the push/pull track keeps a stable
        // ~30 ms offset relative to a straight track, and that triggers stay
        // regular (no skipped or doubled steps).
        let shared_pattern = SharedPattern::new(&Pattern::empty());
        for step in 0..16 {
            shared_pattern.set_step_mask(step, 0b0000_0000_0001);
        }

        let mut seq = Sequencer::new(shared_pattern.clone());
        seq.tracks[0].push_pull_ms = 30.0;
        seq.play();

        let mut straight = Sequencer::new(shared_pattern);
        straight.play();

        let sample_rate = 44100.0;
        let bpm = 120.0;
        let samples_per_step = (60.0 / bpm / 4.0 * sample_rate) as usize;

        // Run 2 bars, then sync to beat 2.0 in the third bar, then run 2 more bars.
        for _ in 0..(samples_per_step * 32) {
            seq.process_sample(bpm, sample_rate, 0.0, GrooveType::Straight);
            straight.process_sample(bpm, sample_rate, 0.0, GrooveType::Straight);
        }
        seq.sync_to_host(2.0, bpm, sample_rate);
        straight.sync_to_host(2.0, bpm, sample_rate);

        let mut delayed_samples = Vec::new();
        let mut straight_samples = Vec::new();
        for sample_idx in 0..(samples_per_step * 32) {
            let t1 = seq.process_sample(bpm, sample_rate, 0.0, GrooveType::Straight);
            let t2 = straight.process_sample(bpm, sample_rate, 0.0, GrooveType::Straight);
            if t1[0].should_trigger {
                delayed_samples.push(sample_idx);
            }
            if t2[0].should_trigger {
                straight_samples.push(sample_idx);
            }
        }

        // Roughly the same number of triggers after the sync (±1 because the
        // sync can land on either side of a step boundary).
        assert!(
            delayed_samples.len().abs_diff(straight_samples.len()) <= 1,
            "Push/pull and straight should produce roughly the same number of triggers after sync ({} vs {})",
            delayed_samples.len(),
            straight_samples.len()
        );
        assert!(
            straight_samples.len() >= 30 && straight_samples.len() <= 34,
            "Expected ~32 triggers after sync, got {}",
            straight_samples.len()
        );

        // Intervals between consecutive triggers must stay regular (one step
        // apart). Allow a small tolerance for the first interval after sync.
        for i in 1..delayed_samples.len() {
            let interval = delayed_samples[i] - delayed_samples[i - 1];
            assert!(
                interval >= samples_per_step - 10 && interval <= samples_per_step + 10,
                "Irregular push/pull interval at trigger {}: {} samples (expected ~{})",
                i,
                interval,
                samples_per_step
            );
        }

        // Average offset to the nearest straight trigger should be ~30 ms.
        let expected_delay = (0.030 * sample_rate as f64) as usize;
        let mut total_delay = 0_usize;
        let mut count = 0_usize;
        for &d in &delayed_samples {
            // Find the nearest straight trigger.
            let nearest = straight_samples
                .iter()
                .map(|&s| s.abs_diff(d))
                .min()
                .unwrap_or(0);
            total_delay += nearest;
            count += 1;
        }
        if count > 0 {
            let avg_delay = total_delay / count;
            assert!(
                avg_delay >= expected_delay - 100 && avg_delay <= expected_delay + 100,
                "Average delay mismatch: expected ~{}, got {} (expected_ms=30)",
                expected_delay,
                avg_delay
            );
        }
    }

    #[test]
    fn test_humanize_no_double_triggers() {
        // High humanize should never create two triggers for the same instrument
        // within the same step interval.
        let shared_pattern = SharedPattern::new(&Pattern::empty());
        for step in 0..16 {
            shared_pattern.set_step_mask(step, 0b0000_0000_0001); // kick every step
        }

        let mut seq = Sequencer::new(shared_pattern);
        seq.tracks[0].humanize_amount = 1.0; // max humanize
        seq.play();

        let sample_rate = 44100.0;
        let bpm = 120.0;

        let mut trigger_count = 0_usize;
        let mut last_step = 99_usize;

        for _ in 0..88200 {
            let triggers = seq.process_sample(bpm, sample_rate, 0.0, GrooveType::Straight);
            if triggers[0].should_trigger {
                let current = seq.tracks[0].previous_step;
                assert!(
                    current != last_step,
                    "Double trigger detected on step {} (humanize should not duplicate)",
                    current
                );
                last_step = current;
                trigger_count += 1;
            }
        }

        // Over one bar we expect exactly 16 triggers (or very close).
        assert!(
            trigger_count >= 15 && trigger_count <= 17,
            "Expected ~16 triggers, got {} (no duplicates)",
            trigger_count
        );
    }

    #[test]
    fn test_fusion_filters_invalid_groups() {
        let shared_pattern = SharedPattern::new(&Pattern::empty());
        let mut seq = Sequencer::new(shared_pattern);
        seq.set_fusions_for_test(
            0,
            &[
                FusedGroup {
                    start_cell: 0,
                    end_cell: 3,
                    step_count: 3,
                    ..Default::default()
                },
                FusedGroup {
                    start_cell: 5,
                    end_cell: 5,
                    step_count: 1,
                    ..Default::default()
                }, // single-cell
                FusedGroup {
                    start_cell: 14,
                    end_cell: 17,
                    step_count: 3,
                    ..Default::default()
                }, // crosses page
            ],
        );
        let kick_fusions = &seq.fusions[0];
        assert_eq!(
            kick_fusions.count, 1,
            "Invalid fusions should be filtered out"
        );
        assert_eq!(kick_fusions.groups[0].start_cell, 0);
        assert_eq!(kick_fusions.groups[0].end_cell, 3);
        assert_eq!(kick_fusions.groups[0].step_count, 3);
    }

    #[test]
    fn test_fusion_triggers_only_start_cell_with_pulse_metadata() {
        let shared_pattern = SharedPattern::new(&Pattern::empty());
        // Start and internal cells are active to prove internals do not trigger independently.
        for step in 0..=3 {
            shared_pattern.set_step_mask(step, 0b0000_0000_0001);
        }

        let mut seq = Sequencer::new(shared_pattern);
        seq.set_fusions_for_test(
            0,
            &[FusedGroup {
                start_cell: 0,
                end_cell: 3,
                step_count: 3,
                ..Default::default()
            }],
        );
        seq.play();

        let sample_rate = 44100.0;
        let bpm = 120.0;
        let samples_per_step = (60.0 / bpm / 4.0 * sample_rate) as usize;
        let mut hits = Vec::new();
        for _ in 0..(samples_per_step * 5) {
            let triggers = seq.process_sample(bpm, sample_rate, 0.0, GrooveType::Straight);
            if triggers[0].should_trigger {
                hits.push(triggers[0]);
            }
        }

        assert_eq!(
            hits.len(),
            1,
            "Fusion internals should be suppressed by the sequencer"
        );
        assert_eq!(hits[0].step, 0);
        assert_eq!(hits[0].fusion_pulse_count, 3);
        assert_eq!(hits[0].fusion_span_cells, 4);
        assert!(hits[0].is_fusion());
    }

    // -- Microtiming (per-cell nudge) -----------------------------------------

    /// Collect (sample_idx, trigger) for slot 0 over `total_samples`.
    fn collect_slot0_hits(
        seq: &mut Sequencer,
        bpm: f32,
        sample_rate: f32,
        total_samples: usize,
    ) -> Vec<(usize, TriggerResult)> {
        let mut hits = Vec::new();
        for sample_idx in 0..total_samples {
            let triggers = seq.process_sample(bpm, sample_rate, 0.0, GrooveType::Straight);
            if triggers[0].should_trigger {
                hits.push((sample_idx, triggers[0]));
            }
        }
        hits
    }

    fn one_cell_pattern(step: usize) -> Arc<SharedPattern> {
        let shared_pattern = SharedPattern::new(&Pattern::empty());
        shared_pattern.set_step_mask(step, 0b1);
        shared_pattern
    }

    fn microtiming_grid(step: usize, ms: f32) -> [[f32; SEQ_STEP_COUNT]; MAX_TRACKS] {
        let mut grid = [[0.0; SEQ_STEP_COUNT]; MAX_TRACKS];
        grid[0][step] = ms;
        grid
    }

    #[test]
    fn test_microtiming_positive_delays_trigger() {
        let sample_rate = 44100.0;
        let bpm = 120.0;
        let samples_per_step = 60.0 / bpm / 4.0 * sample_rate; // 5512.5

        let baseline = {
            let mut seq = Sequencer::new(one_cell_pattern(1));
            seq.play();
            collect_slot0_hits(&mut seq, bpm, sample_rate, (samples_per_step * 3.0) as usize)
        };
        let nudged = {
            let mut seq = Sequencer::new(one_cell_pattern(1));
            seq.set_microtimings(microtiming_grid(1, 25.0));
            seq.play();
            collect_slot0_hits(&mut seq, bpm, sample_rate, (samples_per_step * 3.0) as usize)
        };

        assert_eq!(baseline.len(), 1);
        assert_eq!(nudged.len(), 1, "positive nudge must not double-fire");
        let expected_shift = 0.025 * sample_rate as f64; // 25 ms
        let shift = nudged[0].0 as f64 - baseline[0].0 as f64;
        assert!(
            (shift - expected_shift).abs() <= 2.0,
            "trigger should be ~25 ms late (shift = {shift} samples, expected {expected_shift})"
        );
        assert_eq!(nudged[0].1.step, 1);
        assert!(!nudged[0].1.early_next_loop);
    }

    #[test]
    fn test_microtiming_negative_fires_early() {
        let sample_rate = 44100.0;
        let bpm = 120.0;
        let samples_per_step = 60.0 / bpm / 4.0 * sample_rate;

        let baseline = {
            let mut seq = Sequencer::new(one_cell_pattern(5));
            seq.play();
            collect_slot0_hits(&mut seq, bpm, sample_rate, (samples_per_step * 7.0) as usize)
        };
        let nudged = {
            let mut seq = Sequencer::new(one_cell_pattern(5));
            seq.set_microtimings(microtiming_grid(5, -25.0));
            seq.play();
            collect_slot0_hits(&mut seq, bpm, sample_rate, (samples_per_step * 7.0) as usize)
        };

        assert_eq!(baseline.len(), 1);
        assert_eq!(nudged.len(), 1, "negative nudge must not double-fire");
        let expected_shift = 0.025 * sample_rate as f64;
        let shift = baseline[0].0 as f64 - nudged[0].0 as f64;
        assert!(
            (shift - expected_shift).abs() <= 2.0,
            "trigger should be ~25 ms early (shift = {shift} samples, expected {expected_shift})"
        );
        assert_eq!(nudged[0].1.step, 5);
    }

    #[test]
    fn test_microtiming_negative_across_wrap_flags_next_loop() {
        let sample_rate = 44100.0;
        let bpm = 120.0;
        let samples_per_bar = (60.0 / bpm * 4.0 * sample_rate) as usize; // 88200

        // Step 0 nudged early: the SECOND loop's step 0 must fire ~25 ms before
        // the wrap and report early_next_loop so conditions use loop_count + 1.
        let mut seq = Sequencer::new(one_cell_pattern(0));
        seq.set_microtimings(microtiming_grid(0, -25.0));
        seq.play();
        let hits = collect_slot0_hits(&mut seq, bpm, sample_rate, samples_per_bar + 100);

        assert_eq!(hits.len(), 2, "one hit per loop (start + early wrap)");
        let early = hits[1];
        let expected = samples_per_bar as f64 - 0.025 * sample_rate as f64;
        assert!(
            (early.0 as f64 - expected).abs() <= 3.0,
            "loop-1 step 0 should fire ~25 ms before the wrap (at {}, expected ~{expected})",
            early.0
        );
        assert!(
            early.1.early_next_loop,
            "early fire across the wrap must flag early_next_loop"
        );
    }

    #[test]
    fn test_microtiming_zero_keeps_grid() {
        let sample_rate = 44100.0;
        let bpm = 120.0;
        let samples_per_step = 60.0 / bpm / 4.0 * sample_rate;

        let baseline = {
            let mut seq = Sequencer::new(one_cell_pattern(3));
            seq.play();
            collect_slot0_hits(&mut seq, bpm, sample_rate, (samples_per_step * 5.0) as usize)
        };
        let zeroed = {
            let mut seq = Sequencer::new(one_cell_pattern(3));
            seq.set_microtimings(microtiming_grid(3, 0.0));
            seq.play();
            collect_slot0_hits(&mut seq, bpm, sample_rate, (samples_per_step * 5.0) as usize)
        };

        assert_eq!(baseline.len(), 1);
        assert_eq!(zeroed.len(), 1);
        assert_eq!(baseline[0].0, zeroed[0].0, "zero nudge must not move the hit");
    }
}

// Stress tests module
#[cfg(test)]
mod stress_tests;
