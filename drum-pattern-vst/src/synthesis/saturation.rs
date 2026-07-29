//! Saturation and distortion algorithms for drum synthesis.
//!
//! Five distinct characters:
//! 1. SoftClip  — smooth tanh, warm and musical, most "safe"
//! 2. Valve     — strong asymmetry, tube glow, even harmonics
//! 3. Transistor— germanium grit, crunchy, emphasizes highs
//! 4. HardClip  — brutal digital clipping, aggressive and square
//! 5. Tape      — soft compression "glue", smooth transient taming

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaturationType {
    None = 0,
    SoftClip = 1,
    Valve = 2,
    Transistor = 3,
    HardClip = 4,
    Tape = 5,
}

impl From<u8> for SaturationType {
    fn from(value: u8) -> Self {
        match value {
            1 => SaturationType::SoftClip,
            2 => SaturationType::Valve,
            3 => SaturationType::Transistor,
            4 => SaturationType::HardClip,
            5 => SaturationType::Tape,
            _ => SaturationType::None,
        }
    }
}

impl From<SaturationType> for u8 {
    fn from(value: SaturationType) -> Self {
        value as u8
    }
}

/// ## Paramètres de saturation
///
/// - **Amount** (0-1) : Contrôle le gain d'entrée envoyé dans l'algorithme de saturation.
///   0 = pas de saturation (drive 1×), 1 = saturation maximale (drive 20×).
///   C'est le « bouton de distorsion » principal — plus il est haut, plus le signal est écrasé.
///
/// - **Mix** (0-1) : Balance dry/wet. 0% = son original pur, 100% = son saturé pur.
///   À 50% tu mixes les deux — idéal pour ajouter de la chaleur sans dénaturer.
///
/// - **Output Gain** (0.5-2.0) : Gain de compensation manuel (makeup) après saturation.
///   La saturation applique déjà une compensation automatique pour garder le niveau
///   perçu stable ; ce paramètre permet d'ajuster finement au goût.
#[derive(Clone, Copy, Debug)]
pub struct SaturationConfig {
    pub saturation_type: SaturationType,
    pub amount: f32,      // 0.0 to 1.0 — input drive (maps to 1×..20×)
    pub mix: f32,         // 0.0 to 1.0 (dry/wet)
    pub output_gain: f32, // linear makeup gain
    pub pre_filter: bool, // true = pre-filter, false = post-filter
    /// Gain appliqué au signal saturé pour que le niveau global reste proche du sec.
    pub compensation_gain: f32,
}

impl SaturationConfig {
    #[inline]
    pub fn process(&self, x: f32) -> f32 {
        if self.saturation_type == SaturationType::None || self.amount <= 0.001 {
            return x;
        }

        // amount=0 → drive=1 (bypass), amount=1 → drive=20 (heavy)
        let drive = 1.0 + self.amount * self.amount * 19.0;

        let dry = x;
        let wet_raw = match self.saturation_type {
            SaturationType::SoftClip => soft_clip(x, drive),
            SaturationType::Valve => valve(x, drive),
            SaturationType::Transistor => transistor(x, drive),
            SaturationType::HardClip => hard_clip(x, drive),
            SaturationType::Tape => tape(x, drive),
            _ => x,
        };
        // La compensation normalise le signal saturé pour que le niveau global
        // reste proche du signal sec (référence = 0.5).
        let wet = wet_raw * self.compensation_gain;

        let mixed = dry * (1.0 - self.mix) + wet * self.mix;
        mixed * self.output_gain
    }

    /// Apply saturation only when this call site matches the configured stage.
    /// `pre_stage = true` marks a call placed BEFORE the voice's filter,
    /// `false` a call placed after it. With `pre_filter` off (default) the
    /// drive happens post-filter; enabling it moves the drive in front of the
    /// filter so the filter tames the generated harmonics.
    #[inline]
    pub fn process_at(&self, pre_stage: bool, x: f32) -> f32 {
        if pre_stage == self.pre_filter {
            self.process(x)
        } else {
            x
        }
    }

    /// Recalcule le gain de compensation à chaque changement de type ou d'amount.
    /// La compensation est définie pour que l'entrée de référence 0.5 donne
    /// approximativement 0.5 en sortie (wet), quelle que soit la drive.
    pub fn update_compensation(&mut self) {
        if self.saturation_type == SaturationType::None || self.amount <= 0.001 {
            self.compensation_gain = 1.0;
            return;
        }

        let drive = 1.0 + self.amount * self.amount * 19.0;
        let reference = 0.5;
        let wet = match self.saturation_type {
            SaturationType::SoftClip => soft_clip(reference, drive),
            SaturationType::Valve => valve(reference, drive),
            SaturationType::Transistor => transistor(reference, drive),
            SaturationType::HardClip => hard_clip(reference, drive),
            SaturationType::Tape => tape(reference, drive),
            _ => reference,
        };
        self.compensation_gain = reference / wet.max(0.0001);
    }
}

// ── 1. SoftClip ──────────────────────────────────────────────────────────
// Classic tanh — round, smooth, the "safest" saturation.
// Good for: gentle warmth, subtle harmonics.
#[inline]
fn soft_clip(x: f32, drive: f32) -> f32 {
    let s = x * drive;
    s.tanh() * (1.0 + (drive - 1.0) * 0.06)
}

// ── 2. Valve ─────────────────────────────────────────────────────────────
// Strong asymmetry: positive side compresses like a glowing tube,
// negative side "sags" differently. Rich even harmonics, warm.
// Good for: tube amp character, fattening.
#[inline]
fn valve(x: f32, drive: f32) -> f32 {
    let s = x * drive;
    let pos = s.max(0.0);
    let neg = s.min(0.0);
    // Positive: soft knee tube compression
    let sat_pos = (pos / (1.0 + pos * 0.35)).powf(0.88);
    // Negative: less compressed, more "sag"
    let sat_neg = neg / (1.0 + (-neg) * 0.12);
    (sat_pos + sat_neg) * (1.0 + (drive - 1.0) * 0.12)
}

// ── 3. Transistor ────────────────────────────────────────────────────────
// Germanium/BJT grit — crunchy, granular, emphasizes upper mids.
// Strong asymmetry with a "bump" in the positive region.
// Good for: 70s distortion, fuzzy character.
#[inline]
fn transistor(x: f32, drive: f32) -> f32 {
    let s = x * drive;
    if s > 0.0 {
        // Positive: early clip with grit, 30% louder
        let t = s.powf(0.82);
        (t / (1.0 + t * 0.6)) * 1.35
    } else {
        // Negative: softer, different curve
        let t = (-s).powf(0.92);
        -(t / (1.0 + t * 0.45)) * 1.05
    }
}

// ── 4. HardClip ──────────────────────────────────────────────────────────
// Brutal digital clipping — pure truncation, very aggressive.
// The "most distorted" sound, square-like on high drive.
// Good for: industrial, aggressive drums, bit-crushing feel.
#[inline]
fn hard_clip(x: f32, drive: f32) -> f32 {
    let s = x * drive;
    // Pure clip at ±1, no softening
    s.clamp(-1.0, 1.0) * (1.0 + (drive - 1.0) * 0.02)
}

// ── 5. Tape ──────────────────────────────────────────────────────────────
// Soft compression "glue" — tames transients smoothly.
// Double atan for more compression character, very musical.
// Good for: glue, smoothing harsh transients, vintage feel.
#[inline]
fn tape(x: f32, drive: f32) -> f32 {
    let s = x * drive;
    // Primary atan + secondary softer atan for "glue"
    let a1 = s.atan();
    let a2 = (s * 0.4).atan() * 0.25;
    (a1 + a2) * 1.15 * (1.0 + (drive - 1.0) * 0.06)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passthrough_when_disabled() {
        let cfg = SaturationConfig {
            saturation_type: SaturationType::SoftClip,
            amount: 0.0,
            mix: 1.0,
            output_gain: 1.0,
            pre_filter: false,
            compensation_gain: 1.0,
        };
        assert_eq!(cfg.process(0.5), 0.5);
    }

    #[test]
    fn algorithms_produce_different_outputs() {
        let mut configs = vec![];
        for t in [
            SaturationType::SoftClip,
            SaturationType::Valve,
            SaturationType::Transistor,
            SaturationType::HardClip,
            SaturationType::Tape,
        ] {
            configs.push(SaturationConfig {
                saturation_type: t,
                amount: 0.7,
                mix: 1.0,
                output_gain: 1.0,
                pre_filter: false,
                compensation_gain: 1.0,
            });
        }
        let input = 0.6;
        let out0 = configs[0].process(input);
        let out1 = configs[1].process(input);
        let out2 = configs[2].process(input);
        let out3 = configs[3].process(input);
        let out4 = configs[4].process(input);

        // All should be different from each other (at least one differs by 0.01)
        assert!(
            (out0 - out1).abs() > 0.01,
            "SoftClip vs Valve should differ"
        );
        assert!(
            (out1 - out2).abs() > 0.01,
            "Valve vs Transistor should differ"
        );
        assert!(
            (out2 - out3).abs() > 0.01,
            "Transistor vs HardClip should differ"
        );
        assert!((out3 - out4).abs() > 0.01, "HardClip vs Tape should differ");
    }

    #[test]
    fn hard_clip_is_most_aggressive() {
        let input = 0.8;
        let drive = 10.0; // high drive

        let soft = soft_clip(input, drive);
        let valve = valve(input, drive);
        let trans = transistor(input, drive);
        let hard = hard_clip(input, drive);
        let tape = tape(input, drive);

        // Hard clip should have lowest absolute output (most limited)
        assert!(
            hard.abs() <= soft.abs(),
            "hard clip should be more limited than soft"
        );
        assert!(
            hard.abs() <= valve.abs(),
            "hard clip should be more limited than valve"
        );
        assert!(
            hard.abs() <= trans.abs(),
            "hard clip should be more limited than transistor"
        );
        assert!(
            hard.abs() <= tape.abs(),
            "hard clip should be more limited than tape"
        );
    }

    #[test]
    fn process_at_routes_by_stage() {
        let mk = |pre_filter: bool| {
            let mut cfg = SaturationConfig {
                saturation_type: SaturationType::HardClip,
                amount: 1.0,
                mix: 1.0,
                output_gain: 1.0,
                pre_filter,
                compensation_gain: 1.0,
            };
            cfg.update_compensation();
            cfg
        };
        // Post-filter (default): only the post-stage call saturates.
        let post = mk(false);
        assert_eq!(post.process_at(true, 0.9), 0.9, "pre stage must pass through");
        assert!(post.process_at(false, 0.9) != 0.9, "post stage must saturate");
        // Pre-filter: only the pre-stage call saturates.
        let pre = mk(true);
        assert!(pre.process_at(true, 0.9) != 0.9, "pre stage must saturate");
        assert_eq!(pre.process_at(false, 0.9), 0.9, "post stage must pass through");
    }

    #[test]
    fn compensation_keeps_reference_level_stable() {
        // With compensation, a 0.5 input at amount=0.5 should stay near 0.5
        // at full wet / unity output gain.
        for t in [
            SaturationType::SoftClip,
            SaturationType::Valve,
            SaturationType::Transistor,
            SaturationType::HardClip,
            SaturationType::Tape,
        ] {
            let mut cfg = SaturationConfig {
                saturation_type: t,
                amount: 0.5,
                mix: 1.0,
                output_gain: 1.0,
                pre_filter: false,
                compensation_gain: 1.0,
            };
            cfg.update_compensation();
            let out = cfg.process(0.5);
            assert!(
                (out - 0.5).abs() < 0.05,
                "compensation should keep reference level near 0.5 for {:?}, got {}",
                t,
                out
            );
        }
    }
}
