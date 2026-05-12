//! Sequencer module for 16-step drum patterns

pub(crate) mod pattern;

use std::sync::Arc;

pub use pattern::{Pattern, SharedPattern};

pub struct Sequencer {
    current_step: usize,
    is_playing: bool,
    step_phase: f64,
    pattern: Arc<SharedPattern>,
    mutes: [bool; 7],
}

impl Sequencer {
    pub fn new(pattern: Arc<SharedPattern>) -> Self {
        Self {
            current_step: 0,
            is_playing: false, // Start stopped, wait for DAW
            step_phase: 0.0,
            pattern,
            mutes: [false; 7],
        }
    }

    pub fn process_sample(&mut self, bpm: f32, sample_rate: f32) -> [bool; 7] {
        let mut triggers = [false; 7];

        if !self.is_playing {
            return triggers;
        }

        // 16 steps = 1 bar = 4 beats, 1 step = 1/4 beat = 15/BPM seconds
        let step_duration_seconds = 15.0 / bpm as f64;
        let phase_increment = 1.0 / (step_duration_seconds * sample_rate as f64);

        if self.step_phase < phase_increment {
            let step_mask = self.pattern.load_step_mask(self.current_step);
            for (instrument, trigger) in triggers.iter_mut().enumerate() {
                let instrument_active = (step_mask & (1 << instrument)) != 0;
                *trigger = instrument_active && !self.mutes[instrument];
            }
        }

        self.step_phase += phase_increment;
        if self.step_phase >= 1.0 {
            self.step_phase -= 1.0;
            self.current_step = (self.current_step + 1) % 16;
        }

        triggers
    }

    pub fn sync_to_host(&mut self, position_beats: f64, _bpm: f32, _sample_rate: f32) {
        let step_position = (position_beats * 4.0).rem_euclid(16.0);
        self.current_step = step_position.floor() as usize;
        self.step_phase = step_position.fract() as f64;
    }

    pub fn reset(&mut self) {
        self.current_step = 0;
        self.step_phase = 0.0;
    }

    pub fn play(&mut self) {
        self.is_playing = true;
    }

    pub fn stop(&mut self) {
        self.is_playing = false;
        self.reset();
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    /// Set position from DAW transport (in steps 0-15)
    #[allow(dead_code)]
    pub fn set_position(&mut self, step: usize) {
        self.current_step = step % 16;
        self.step_phase = 0.0;
    }

    pub fn current_step(&self) -> usize {
        self.current_step
    }

    pub fn set_mutes(&mut self, mutes: [bool; 7]) {
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

        seq.play(); // Need to start it manually for test

        let mut triggers_count = 0;
        for _ in 0..6000 {
            let triggers = seq.process_sample(bpm, sample_rate);
            if triggers.iter().any(|&t| t) {
                triggers_count += 1;
            }
        }

        assert!(triggers_count > 0);
    }

    #[test]
    fn test_sequencer_drift_over_bars() {
        // At 120 BPM, 44100 Hz: 1 bar = 88200 samples exactly.
        // 16 steps should fit in exactly 88200 samples if timing is precise.
        let shared_pattern = SharedPattern::new(&Pattern::empty());
        for step in 0..16 {
            shared_pattern.set_step_mask(step, 0b0111_1111);
        }
        let mut seq = Sequencer::new(shared_pattern);
        let sample_rate = 44100.0;
        let bpm = 120.0;
        seq.play();

        let samples_per_bar_exact = (60.0 / bpm * 4.0 * sample_rate) as usize; // 88200
        let total_bars = 10;
        let total_samples = samples_per_bar_exact * total_bars + 10; // +margin to capture the wrap trigger

        let mut step_triggers = Vec::new();
        for sample_idx in 0..total_samples {
            let triggers = seq.process_sample(bpm, sample_rate);
            if triggers.iter().any(|&t| t) {
                step_triggers.push(sample_idx);
            }
        }

        // With integer truncation, each step is 5512 samples instead of 5512.5
        // => 16 steps = 88192 samples instead of 88200 (8 sample drift per bar)
        // After 10 bars we expect 161 triggers (160 + wrap), but drift causes timing misalignment.
        // The critical check: start of bar 2 should be at sample 88200 exactly.
        let start_of_bar_2 = step_triggers[16]; // step 0 of bar 2 = 16th trigger
        let bar_2_drift = (start_of_bar_2 as isize) - (samples_per_bar_exact as isize);
        assert!(
            bar_2_drift.abs() <= 1,
            "Timing drift: bar 2 started at sample {} instead of {} (drift = {} samples, tolerance = +/-1)",
            start_of_bar_2, samples_per_bar_exact, bar_2_drift
        );

        // Check accumulated drift over 10 bars: bar 11 should start at sample 882000
        let start_of_bar_11 = step_triggers[160]; // step 0 of bar 11 = 160th trigger
        let expected_bar_11_start = 10 * samples_per_bar_exact;
        let drift = expected_bar_11_start as isize - start_of_bar_11 as isize;
        assert!(
            drift.abs() <= 1,
            "Timing drift detected: {} samples over 10 bars (tolerance = +/-1 sample)",
            drift
        );
    }
}
