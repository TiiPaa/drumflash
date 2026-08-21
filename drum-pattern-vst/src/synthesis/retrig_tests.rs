//! [179] Retrigger consistency contract for the synthesis kicks.
//!
//! A hit's attack must not depend on the spacing between two steps: whether the
//! previous tail is still ringing or not, the transient has to be the same. The
//! probes that motivated the fix live on here as regression guards.
//!
//! Baseline before the fix (identical settings, digital mode — so no drift at
//! all — only the step spacing varies):
//! - Kick: peak spread **3.71 dB**, first half-cycle polarity flipping,
//!   time-to-peak wandering from 1.5 ms to 8.3 ms.
//! - BD808: time-to-peak 12.7 ms isolated vs 1.5 ms at 62–125 ms spacing; in
//!   analog mode even isolated hits spread 2.6 dB and the per-hit level drift
//!   stepped the ringing tail by 0.244.
//!
//! After: peak spread 0.00 dB, constant time-to-peak, worst sample step 0.014.

use super::dsp;
use super::settings::kick::KickSettings;
use super::settings::kick_808::Kick808Settings;
use super::{Kick808Voice, KickVoice, Voice, VoiceSettings};

const SR: f32 = 48_000.0;
/// Step spacings to probe, in ms: from a half note at 120 BPM down to a stutter
/// repeat. 500 ms lets the tail die out (cold start), 15 ms lands deep inside it.
const GAPS_MS: [f32; 7] = [500.0, 250.0, 125.0, 83.0, 62.0, 31.0, 15.0];
/// Window rendered after the probed trigger.
const WINDOW_S: f32 = 0.15;

struct Attack {
    peak: f32,
    peak_sign: f32,
    time_to_peak_ms: f32,
    /// Worst sample-to-sample step over the first 2 ms, trigger edge included.
    max_step: f32,
    /// Level of the tail the hit landed on (0 for an isolated hit).
    tail: f32,
}

fn analyse(buf: &[f32], previous_out: f32) -> Attack {
    let (idx, peak) = buf.iter().enumerate().fold((0usize, 0.0f32), |acc, (i, s)| {
        if s.abs() > acc.1 {
            (i, s.abs())
        } else {
            acc
        }
    });
    let mut max_step = (buf[0] - previous_out).abs();
    let two_ms = (SR * 0.002) as usize;
    for i in 1..two_ms.min(buf.len()) {
        max_step = max_step.max((buf[i] - buf[i - 1]).abs());
    }
    Attack {
        peak,
        peak_sign: if buf[idx] >= 0.0 { 1.0 } else { -1.0 },
        time_to_peak_ms: idx as f32 / SR * 1000.0,
        max_step,
        tail: previous_out,
    }
}

/// Render a first hit, wait `gap_ms`, then hit again and measure the second
/// attack. `hard` uses the stutter path (`trigger_hard`) instead of `trigger`.
fn probe_gap(voice: &mut impl Voice, gap_ms: f32, hard: bool) -> Attack {
    voice.trigger();
    let mut last = 0.0f32;
    for _ in 0..(SR * gap_ms / 1000.0) as usize {
        last = voice.process_sample();
    }
    if hard {
        voice.trigger_hard();
    } else {
        voice.trigger();
    }
    let buf: Vec<f32> = (0..(SR * WINDOW_S) as usize)
        .map(|_| voice.process_sample())
        .collect();
    analyse(&buf, last)
}

/// Same measurement on a fresh voice, with no tail at all: the reference every
/// retrigger has to match, since a trigger is now always a cold start.
fn probe_isolated(voice: &mut impl Voice) -> Attack {
    voice.trigger();
    let buf: Vec<f32> = (0..(SR * WINDOW_S) as usize)
        .map(|_| voice.process_sample())
        .collect();
    analyse(&buf, 0.0)
}

fn kick(analog: f32) -> KickVoice {
    let mut s = KickSettings::default_at(SR);
    s.analog = analog;
    KickVoice::new(SR, s)
}

fn kick808(analog: f32) -> Kick808Voice {
    let mut s = Kick808Settings::from(VoiceSettings::kick808());
    s.analog = analog;
    Kick808Voice::new(SR, s)
}

/// Peak spread in dB across the probed spacings.
fn spread_db(attacks: &[Attack]) -> f32 {
    let min = attacks.iter().fold(f32::MAX, |m, a| m.min(a.peak));
    let max = attacks.iter().fold(0.0f32, |m, a| m.max(a.peak));
    20.0 * (max / min).log10()
}

fn assert_consistent(name: &str, attacks: &[Attack]) {
    let spread = spread_db(attacks);
    eprintln!(
        "{name}: peak spread {spread:.3} dB, worst step {:.4}",
        attacks.iter().fold(0.0f32, |m, a| m.max(a.max_step))
    );
    assert!(
        spread <= 0.3,
        "{name}: attack level depends on the step spacing — {spread:.2} dB of peak spread \
         (was 3.71 dB before [179]). Peaks: {:?}",
        attacks.iter().map(|a| a.peak).collect::<Vec<_>>()
    );
    let ttp_min = attacks
        .iter()
        .fold(f32::MAX, |m, a| m.min(a.time_to_peak_ms));
    let ttp_max = attacks.iter().fold(0.0f32, |m, a| m.max(a.time_to_peak_ms));
    assert!(
        ttp_max - ttp_min <= 0.5,
        "{name}: attack SHAPE depends on the step spacing — time-to-peak spans \
         {ttp_min:.2}..{ttp_max:.2} ms"
    );
    let sign = attacks[0].peak_sign;
    assert!(
        attacks.iter().all(|a| a.peak_sign == sign),
        "{name}: attack polarity flips with the step spacing"
    );
}

/// A retrigger is a click only if it is steeper than the SAME hit played in
/// isolation. Comparing against that reference instead of an absolute threshold
/// keeps the guard meaningful: the amplitude envelope's own convex attack ramp
/// legitimately steps ~0.04/sample on the BD808, and that is punch, not a click.
///
/// Two allowances on top of the reference:
/// - the declicker's own slope, bounded by `(pi/2)·|tail| / fade_samples` — a
///   3 ms half-cosine, i.e. energy below ~300 Hz, never a click;
/// - 30 % for analog mode, where the per-hit level drift (±25 %) legitimately
///   makes one hit steeper than another.
///
/// A state reset WITHOUT the declicker measures ~0.35 here — 25x the reference,
/// so the guard still catches the real failure by a wide margin.
fn assert_click_free(name: &str, attacks: &[Attack], reference: &Attack) {
    let fade_samples = SR * dsp::RetrigDeclick::FADE_MS / 1000.0;
    for (gap, a) in GAPS_MS.iter().zip(attacks) {
        let declick_slope = std::f32::consts::FRAC_PI_2 * a.tail.abs() / fade_samples;
        assert!(
            a.max_step <= reference.max_step * 1.3 + declick_slope + 1e-4,
            "{name}: discontinuity at the retrigger (gap {gap} ms): step {:.4} vs {:.4} \
             for the same hit played in isolation. A state reset without \
             `RetrigDeclick` measures ~0.35 here.",
            a.max_step,
            reference.max_step
        );
    }
}

#[test]
fn kick_attack_is_independent_of_step_spacing() {
    let attacks: Vec<Attack> = GAPS_MS
        .iter()
        .map(|&g| probe_gap(&mut kick(0.0), g, false))
        .collect();
    assert_consistent("kick", &attacks);
    assert_click_free("kick", &attacks, &probe_isolated(&mut kick(0.0)));
}

#[test]
fn kick808_attack_is_independent_of_step_spacing() {
    let attacks: Vec<Attack> = GAPS_MS
        .iter()
        .map(|&g| probe_gap(&mut kick808(0.0), g, false))
        .collect();
    assert_consistent("bd808", &attacks);
    assert_click_free("bd808", &attacks, &probe_isolated(&mut kick808(0.0)));
}

/// Analog mode keeps its per-hit drift, so the LEVEL is expected to vary — but
/// never at the cost of a discontinuity on the ringing tail (the BD808 measured
/// 0.244 there before the fix).
#[test]
fn kicks_retrigger_click_free_in_analog_mode() {
    let attacks: Vec<Attack> = GAPS_MS
        .iter()
        .map(|&g| probe_gap(&mut kick(1.0), g, false))
        .collect();
    assert_click_free("kick analog", &attacks, &probe_isolated(&mut kick(1.0)));
    let attacks: Vec<Attack> = GAPS_MS
        .iter()
        .map(|&g| probe_gap(&mut kick808(1.0), g, false))
        .collect();
    assert_click_free("bd808 analog", &attacks, &probe_isolated(&mut kick808(1.0)));
}

/// Stutter repeats go through `trigger_hard`; same contract applies.
#[test]
fn stutter_repeats_keep_the_same_attack() {
    let attacks: Vec<Attack> = GAPS_MS
        .iter()
        .map(|&g| probe_gap(&mut kick(0.0), g, true))
        .collect();
    assert_consistent("kick stutter", &attacks);
    assert_click_free("kick stutter", &attacks, &probe_isolated(&mut kick(0.0)));

    let attacks: Vec<Attack> = GAPS_MS
        .iter()
        .map(|&g| probe_gap(&mut kick808(0.0), g, true))
        .collect();
    assert_consistent("bd808 stutter", &attacks);
    assert_click_free("bd808 stutter", &attacks, &probe_isolated(&mut kick808(0.0)));
}

/// The declicker must not leak: once the fade is over it contributes nothing, so
/// repeated isolated hits (voice fully silent in between) stay bit-identical.
#[test]
fn isolated_kick_hits_stay_bit_identical_in_digital_mode() {
    let mut voice = kick(0.0);
    let mut reference: Option<Vec<f32>> = None;
    for hit in 0..4 {
        voice.trigger();
        let buf: Vec<f32> = (0..(SR * WINDOW_S) as usize)
            .map(|_| voice.process_sample())
            .collect();
        for _ in 0..(SR * 1.5) as usize {
            voice.process_sample();
        }
        match &reference {
            None => reference = Some(buf),
            Some(r) => {
                let max_diff = r
                    .iter()
                    .zip(&buf)
                    .fold(0.0f32, |m, (a, b)| m.max((a - b).abs()));
                assert!(
                    max_diff < 1e-6,
                    "kick: hit {hit} differs from hit 0 by {max_diff} in digital mode"
                );
            }
        }
    }
}

#[test]
fn isolated_kick808_hits_stay_bit_identical_in_digital_mode() {
    let mut voice = kick808(0.0);
    let mut reference: Option<Vec<f32>> = None;
    for hit in 0..4 {
        voice.trigger();
        let buf: Vec<f32> = (0..(SR * WINDOW_S) as usize)
            .map(|_| voice.process_sample())
            .collect();
        for _ in 0..(SR * 1.5) as usize {
            voice.process_sample();
        }
        match &reference {
            None => reference = Some(buf),
            Some(r) => {
                let max_diff = r
                    .iter()
                    .zip(&buf)
                    .fold(0.0f32, |m, (a, b)| m.max((a - b).abs()));
                assert!(
                    max_diff < 1e-6,
                    "bd808: hit {hit} differs from hit 0 by {max_diff} in digital mode"
                );
            }
        }
    }
}
