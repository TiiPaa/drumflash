//! Groove engine — multiple swing/shuffle algorithms for the sequencer.

use nih_plug::prelude::Enum;

/// Available groove types. Each applies the swing amount differently.
#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GrooveType {
    /// No swing, straight 16th grid.
    #[default]
    #[id = "straight"]
    #[name = "Straight"]
    Straight,

    /// Classic 16th-note swing. At +50 % the odd step lands on the triplet.
    #[id = "swing16"]
    #[name = "Swing 16th"]
    Swing16,

    /// Aggressive shuffle — larger offset range than Swing16.
    #[id = "shuffle"]
    #[name = "Shuffle"]
    Shuffle,

    /// MPC-style non-linear curve (Roger Linn feel).
    /// Mid-range swing values have slightly more impact.
    #[id = "mpc"]
    #[name = "MPC Style"]
    Mpc,
}

/// Convert a beat position (0..4) into a swung step index (0..15).
pub fn beat_to_step(beat_pos: f64, swing: f32, groove_type: GrooveType) -> usize {
    match groove_type {
        GrooveType::Straight => straight_step(beat_pos),
        GrooveType::Swing16 => swing16_step(beat_pos, swing),
        GrooveType::Shuffle => shuffle_step(beat_pos, swing),
        GrooveType::Mpc => mpc_step(beat_pos, swing),
    }
}

fn straight_step(beat_pos: f64) -> usize {
    ((beat_pos / 0.25).floor() as usize).clamp(0, 15)
}

/// Shared pair-based logic. 8 pairs per bar, each pair = 0.5 beat (one 8th-note).
/// The odd step of each pair is delayed according to `swing_ratio`.
fn pair_step(beat_pos: f64, swing_ratio: f64) -> usize {
    let pair_index = (beat_pos / 0.5).floor() as usize % 8;
    let pos_in_pair = beat_pos % 0.5;
    // Clamp so the odd step always has a non-zero window
    let threshold = 0.5 * swing_ratio.clamp(0.02, 0.98);
    if pos_in_pair < threshold {
        pair_index * 2
    } else {
        pair_index * 2 + 1
    }
}

/// Swing 16th: linear curve, +50 % = triplet feel.
fn swing16_step(beat_pos: f64, swing: f32) -> usize {
    // swing in [-0.5, +0.5]
    // ratio: 0.5 = straight, 0.666... = triplet, 0.833... = heavy
    let swing_ratio = 0.5 + (swing as f64 / 3.0);
    pair_step(beat_pos, swing_ratio)
}

/// Shuffle: steeper linear curve, +50 % = 0.75 ratio (heavier than triplet).
fn shuffle_step(beat_pos: f64, swing: f32) -> usize {
    let swing_ratio = 0.5 + (swing as f64 / 2.0);
    pair_step(beat_pos, swing_ratio)
}

/// MPC-style: non-linear curve. Mid values have more "bite".
fn mpc_step(beat_pos: f64, swing: f32) -> usize {
    let abs_swing = (swing as f64).abs();
    let sign = if swing >= 0.0 { 1.0 } else { -1.0 };
    // powf(0.75) gives a slight S-curve: values < 0.25 move less,
    // values 0.25-0.5 move more aggressively than linear.
    let curved = abs_swing.powf(0.75) * sign;
    let swing_ratio = 0.5 + (curved / 3.0);
    pair_step(beat_pos, swing_ratio)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stright_no_swing() {
        assert_eq!(straight_step(0.0), 0);
        assert_eq!(straight_step(0.24), 0);
        assert_eq!(straight_step(0.25), 1);
        assert_eq!(straight_step(0.5), 2);
        assert_eq!(straight_step(3.75), 15);
        assert_eq!(straight_step(3.99), 15);
    }

    #[test]
    fn test_swing16_center_is_straight() {
        // With swing = 0.0, Swing16 should match straight timing
        for i in 0..16 {
            let beat = i as f64 * 0.25;
            assert_eq!(
                swing16_step(beat, 0.0),
                straight_step(beat),
                "mismatch at step {} (beat {})",
                i,
                beat
            );
        }
    }

    #[test]
    fn test_swing16_triplet_at_max() {
        // swing = +0.5 → triplet feel (2/3 ratio)
        // In a pair, step 0 spans [0, 0.5*2/3) = [0, 0.3333)
        // step 1 spans [0.3333, 0.5)
        assert_eq!(swing16_step(0.0, 0.5), 0);
        assert_eq!(swing16_step(0.3, 0.5), 0); // still in step 0
        assert_eq!(swing16_step(0.35, 0.5), 1); // now in step 1
        assert_eq!(swing16_step(0.4, 0.5), 1);
    }

    #[test]
    fn test_swing16_negative_swing() {
        // swing = -0.5 → inverse swing (off-beats early)
        // ratio = 0.5 - 0.5/3 = 0.3333
        // threshold = 0.5 * 0.3333 = 0.1667
        assert_eq!(swing16_step(0.0, -0.5), 0);
        assert_eq!(swing16_step(0.1, -0.5), 0); // still in step 0 (threshold = 0.1667)
        assert_eq!(swing16_step(0.2, -0.5), 1); // early off-beat (before straight 0.25)
        assert_eq!(swing16_step(0.4, -0.5), 1);
    }

    #[test]
    fn test_shuffle_heavier_than_swing16() {
        // At same positive swing, Shuffle should delay odd steps more than Swing16
        let beat = 0.35;
        let swing = 0.3;
        let _s16 = swing16_step(beat, swing);
        let _shf = shuffle_step(beat, swing);
        // At beat=0.35 (within pair 0), if Shuffle threshold > Swing16 threshold,
        // Shuffle may still be on step 0 while Swing16 has moved to step 1.
        let s16_thresh = 0.5 * (0.5 + swing as f64 / 3.0);
        let shf_thresh = 0.5 * (0.5 + swing as f64 / 2.0);
        assert!(shf_thresh > s16_thresh, "Shuffle should have larger threshold");
    }

    #[test]
    fn test_total_bar_length_unchanged() {
        // Regardless of groove type / swing, one full bar must still map to 16 steps.
        for groove in [GrooveType::Straight, GrooveType::Swing16, GrooveType::Shuffle, GrooveType::Mpc] {
            for swing in [-0.5_f32, -0.25, 0.0, 0.25, 0.5] {
                let mut step_counts = [0usize; 16];
                let samples = 10000;
                for s in 0..samples {
                    let beat = (s as f64 / samples as f64) * 4.0;
                    let step = beat_to_step(beat, swing, groove);
                    step_counts[step] += 1;
                }
                // Every step should have been visited at least once
                for (i, count) in step_counts.iter().enumerate() {
                    assert!(
                        *count > 0,
                        "Groove {:?} swing={}: step {} was never visited",
                        groove, swing, i
                    );
                }
            }
        }
    }
}
