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
use crate::synthesis::DrumVoice;
pub use pattern::{FusedGroup, Pattern, SharedPattern, MAX_FUSIONS};

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
        }
    }
}

impl TrackState {
    fn next_rand(&mut self) -> f32 {
        self.rng_state = self.rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        ((self.rng_state >> 16) & 0x7FFF) as f32 / 32767.0
    }
}

pub struct Sequencer {
    is_playing: bool,
    /// Master beat position (0.0 .. master_length * 0.25). 1 step = 0.25 beat.
    beat_position: f64,
    tracks: [TrackState; DrumVoice::COUNT],
    /// Cached values for sync_to_host.
    swing: f32,
    groove_type: GrooveType,
    pattern: Arc<SharedPattern>,
    mutes: [bool; DrumVoice::COUNT],
    /// Global pattern length (1-64 steps). Controls master loop point.
    master_length: usize,
    /// How many times the master pattern has looped (for step conditions).
    loop_count: usize,
    /// Per-instrument fused cell groups (Step Fusion), copied from SharedPattern once per buffer.
    fusions: [FusionTrack; DrumVoice::COUNT],
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
    /// Morphing target field (255 = none). Only meaningful for fused triggers.
    pub morph_field: u8,
    /// Morphing target value at the last pulse.
    pub morph_end_value: f32,
}

impl Default for TriggerResult {
    fn default() -> Self {
        Self {
            should_trigger: false,
            velocity: 0.0,
            step: 0,
            fusion_pulse_count: 1,
            fusion_span_cells: 1,
            morph_field: 255,
            morph_end_value: 0.0,
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
            tracks: [TrackState::default(); DrumVoice::COUNT],
            swing: 0.0,
            groove_type: GrooveType::Swing16,
            pattern,
            mutes: [false; DrumVoice::COUNT],
            master_length: 16,
            loop_count: 0,
            fusions: [FusionTrack::default(); DrumVoice::COUNT],
        }
    }

    /// Copy fusion groups from SharedPattern into fixed local arrays.
    /// Call once per audio buffer, never per sample.
    pub fn sync_fusions_from_pattern(&mut self) {
        for instrument in 0..DrumVoice::COUNT {
            self.fusions[instrument].set_from_pattern(&self.pattern, instrument);
        }
    }

    #[cfg(test)]
    fn set_fusions_for_test(&mut self, instrument: usize, fusions: &[FusedGroup]) {
        if instrument < DrumVoice::COUNT {
            self.fusions[instrument].set_from_slice(fusions);
        }
    }

    pub fn process_sample(
        &mut self,
        bpm: f32,
        sample_rate: f32,
        swing: f32,
        groove_type: GrooveType,
    ) -> [TriggerResult; DrumVoice::COUNT] {
        let mut triggers = [TriggerResult::default(); DrumVoice::COUNT];

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

        for instrument in 0..DrumVoice::COUNT {
            let track = &mut self.tracks[instrument];

            // Push/pull: convert ms to beats and subtract so positive = late.
            let push_pull_beats = track.push_pull_ms as f64 * bpm as f64 / (60.0 * 1000.0);
            let shifted_beat =
                (self.beat_position - push_pull_beats).rem_euclid(master_beat_length);

            // Re-compute master step for this track's shifted timeline.
            let shifted_master = groove::beat_to_step(shifted_beat, swing, groove_type);

            // Trigger on master step transition.
            // Using shifted_master (not current_step) fixes the track_length=1 bug
            // where current_step never changes (always 0).
            if shifted_master != track.previous_shifted_master {
                // True polyrhythm: each track advances its own independent counter.
                track.step_counter = track.step_counter.wrapping_add(1);
                let current_step = track.step_counter % track.track_length.max(1);

                let fusion = self.fusions[instrument].containing(current_step);
                let (
                    source_step,
                    fusion_pulse_count,
                    fusion_span_cells,
                    morph_field,
                    morph_end_value,
                ) = match fusion {
                    Some(group) if group.is_start(current_step) => (
                        group.start_cell as usize,
                        group.step_count.clamp(1, 64),
                        group.cell_span().min(64) as u8,
                        group.morph_field,
                        group.morph_end_value,
                    ),
                    Some(_) => {
                        // Covered cells do not trigger independently. The start cell
                        // schedules all pulses for the fused region.
                        track.previous_shifted_master = shifted_master;
                        track.previous_step = current_step;
                        continue;
                    }
                    None => (current_step, 1, 1, 255, 0.0),
                };

                let step_mask = self.pattern.load_step_mask(source_step);
                let active = (step_mask & (1 << instrument)) != 0 && !self.mutes[instrument];

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

                triggers[instrument] = TriggerResult {
                    should_trigger: active,
                    velocity,
                    step: source_step,
                    fusion_pulse_count,
                    fusion_span_cells,
                    morph_field,
                    morph_end_value,
                };
                track.previous_shifted_master = shifted_master;
                track.previous_step = current_step;
            }
        }

        triggers
    }

    pub fn sync_to_host(&mut self, position_beats: f64, bpm: f32, _sample_rate: f32) {
        let master_beat_length = self.master_length as f64 * 0.25;
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
        }
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
        lengths: [usize; DrumVoice::COUNT],
        push_pulls: [f32; DrumVoice::COUNT],
        humanizes: [f32; DrumVoice::COUNT],
        master_length: usize,
    ) {
        self.master_length = master_length.clamp(1, 64);
        for i in 0..DrumVoice::COUNT {
            self.tracks[i].track_length = lengths[i].clamp(1, self.master_length);
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
    pub fn current_steps(&self) -> [usize; DrumVoice::COUNT] {
        let mut steps = [0usize; DrumVoice::COUNT];
        for (i, track) in self.tracks.iter().enumerate() {
            steps[i] = track.step_counter % track.track_length.max(1);
        }
        steps
    }

    pub fn loop_count(&self) -> usize {
        self.loop_count
    }

    pub fn set_mutes(&mut self, mutes: [bool; DrumVoice::COUNT]) {
        self.mutes = mutes;
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
}

// Stress tests module
#[cfg(test)]
mod stress_tests;
