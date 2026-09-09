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

    /// Recalcule le gain de compensation a chaque changement de type ou d'amount.
    ///
    /// [207] La compensation egalise l'**energie** (RMS) du signal sature sur
    /// celle du signal sec, mesuree sur un petit signal de test. L'ancienne
    /// version normalisait la courbe en **un seul point** (entree 0.5), ce qui
    /// se tenait mal des que le signal reel sortait de ce point - et un coup de
    /// batterie culmine plutot vers 0,8-1,0 :
    ///
    /// - SoftClip **perdait** ~4 dB quand on montait l'amount (pic 0,80 -> 0,50),
    ///   parce que la courbe compresse plus a 0,8 qu'a 0,5 : sous-compense.
    /// - Valve **gagnait** au contraire (pic 0,80 -> 1,36, donc au-dela de 1,0),
    ///   sa courbe asymetrique ayant un gain > 1 autour du point de reference :
    ///   sur-compense.
    /// - Et la compensation sautait de 1,0 a ~1,08 des qu'on quittait zero,
    ///   puisque les courbes colorent deja a drive = 1 (`tanh(0.5)` = 0,462).
    ///
    /// Normaliser en RMS supprime les trois : le niveau ne bouge plus quand on
    /// tourne l'Amount, il ne depend plus du type choisi, et il est continu au
    /// depart de zero. Le caractere - les harmoniques - reste entier.
    ///
    /// Cout : `COMP_TEST_POINTS` evaluations de la courbe par changement de
    /// parametre. Jamais par echantillon - les appelants sont les
    /// constructeurs et `set_settings`, soit au plus une fois par bloc.
    pub fn update_compensation(&mut self) {
        if self.saturation_type == SaturationType::None || self.amount <= 0.001 {
            self.compensation_gain = 1.0;
            return;
        }

        let drive = 1.0 + self.amount * self.amount * 19.0;
        let mut dry_energy = 0.0f32;
        let mut wet_energy = 0.0f32;
        for i in 0..COMP_TEST_POINTS {
            let phase =
                std::f32::consts::TAU * i as f32 / COMP_TEST_POINTS as f32;
            let dry = COMP_TEST_AMPLITUDE * phase.sin();
            let wet = match self.saturation_type {
                SaturationType::SoftClip => soft_clip(dry, drive),
                SaturationType::Valve => valve(dry, drive),
                SaturationType::Transistor => transistor(dry, drive),
                SaturationType::HardClip => hard_clip(dry, drive),
                SaturationType::Tape => tape(dry, drive),
                _ => dry,
            };
            dry_energy += dry * dry;
            wet_energy += wet * wet;
        }
        // Borne large : elle n'existe que pour qu'une courbe pathologique ne
        // produise pas un gain absurde, pas pour faconner le son.
        self.compensation_gain =
            (dry_energy / wet_energy.max(1e-9)).sqrt().clamp(0.05, 20.0);
    }
}

/// [207] Points du signal de test de la compensation : un cycle de sinus.
/// 32 points suffisent a estimer un RMS et la boucle ne tourne qu'au
/// changement de parametre.
const COMP_TEST_POINTS: usize = 32;

/// Amplitude de ce sinus. Un coup de batterie culmine pres de 1,0 ; un pic a
/// 0,7 donne un RMS de 0,5, le niveau que visait l'ancienne reference ponctuelle.
const COMP_TEST_AMPLITUDE: f32 = 0.7;

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

    /// [207] The contract the compensation actually owes the user: turning the
    /// Amount knob changes the CHARACTER, not the level.
    ///
    /// This replaces a test that pinned the old single-point rule ("a 0.5 input
    /// gives ~0.5 out"), which was the defect itself: normalising the curve at
    /// one input point drifted as soon as the real signal sat elsewhere, and a
    /// drum hit peaks near 0.8-1.0, not 0.5. Measured then: SoftClip lost ~4 dB
    /// across the sweep while Valve gained ~3.5 dB and pushed past 1.0.
    #[test]
    fn compensation_keeps_the_level_steady_across_the_amount_sweep() {
        const TYPES: [SaturationType; 5] = [
            SaturationType::SoftClip,
            SaturationType::Valve,
            SaturationType::Transistor,
            SaturationType::HardClip,
            SaturationType::Tape,
        ];
        // A hit-sized test signal, deliberately louder than the compensation's
        // own reference so the test would catch a single-point calibration.
        let signal: Vec<f32> = (0..512)
            .map(|i| 0.85 * (std::f32::consts::TAU * i as f32 / 64.0).sin())
            .collect();
        let rms = |cfg: &SaturationConfig| {
            let sum: f32 = signal.iter().map(|&x| cfg.process(x).powi(2)).sum();
            (sum / signal.len() as f32).sqrt()
        };
        let cfg_at = |t: SaturationType, amount: f32| {
            let mut cfg = SaturationConfig {
                saturation_type: t,
                amount,
                mix: 1.0,
                output_gain: 1.0,
                pre_filter: false,
                compensation_gain: 1.0,
            };
            cfg.update_compensation();
            cfg
        };

        let dry = rms(&cfg_at(SaturationType::None, 0.0));
        for t in TYPES {
            let mut worst = 0.0f32;
            for step in 0..=20 {
                let amount = step as f32 / 20.0;
                let db = 20.0 * (rms(&cfg_at(t, amount)) / dry).log10();
                worst = worst.max(db.abs());
            }
            assert!(
                worst <= 2.0,
                "{t:?} drifts {worst:.2} dB across the Amount sweep - the level \
                 must not follow the knob (SoftClip measured ~4 dB before [207])"
            );
        }
    }

    /// The level must not jump the instant the knob leaves zero. It used to:
    /// `process` bypasses entirely at amount <= 0.001 while the curves already
    /// colour at drive = 1, so the compensation stepped from 1.0 to ~1.08.
    #[test]
    fn leaving_zero_amount_does_not_step_the_level() {
        for t in [
            SaturationType::SoftClip,
            SaturationType::Valve,
            SaturationType::Transistor,
            SaturationType::HardClip,
            SaturationType::Tape,
        ] {
            let mk = |amount: f32| {
                let mut cfg = SaturationConfig {
                    saturation_type: t,
                    amount,
                    mix: 1.0,
                    output_gain: 1.0,
                    pre_filter: false,
                    compensation_gain: 1.0,
                };
                cfg.update_compensation();
                cfg
            };
            let signal: Vec<f32> = (0..256)
                .map(|i| 0.85 * (std::f32::consts::TAU * i as f32 / 64.0).sin())
                .collect();
            let rms = |cfg: &SaturationConfig| {
                let sum: f32 = signal.iter().map(|&x| cfg.process(x).powi(2)).sum();
                (sum / signal.len() as f32).sqrt()
            };
            let off = rms(&mk(0.0));
            let barely_on = rms(&mk(0.02));
            let step_db = 20.0 * (barely_on / off).log10();
            assert!(
                step_db.abs() <= 1.0,
                "{t:?} steps {step_db:.2} dB on leaving zero"
            );
        }
    }

    /// Two types at the same Amount must sit at the same level, so switching
    /// type auditions a character instead of a volume change.
    #[test]
    fn every_saturation_type_lands_at_the_same_level() {
        let signal: Vec<f32> = (0..512)
            .map(|i| 0.85 * (std::f32::consts::TAU * i as f32 / 64.0).sin())
            .collect();
        let level = |t: SaturationType| {
            let mut cfg = SaturationConfig {
                saturation_type: t,
                amount: 0.6,
                mix: 1.0,
                output_gain: 1.0,
                pre_filter: false,
                compensation_gain: 1.0,
            };
            cfg.update_compensation();
            let sum: f32 = signal.iter().map(|&x| cfg.process(x).powi(2)).sum();
            20.0 * ((sum / signal.len() as f32).sqrt()).log10()
        };
        let levels: Vec<f32> = [
            SaturationType::SoftClip,
            SaturationType::Valve,
            SaturationType::Transistor,
            SaturationType::HardClip,
            SaturationType::Tape,
        ]
        .into_iter()
        .map(level)
        .collect();
        let min = levels.iter().cloned().fold(f32::MAX, f32::min);
        let max = levels.iter().cloned().fold(f32::MIN, f32::max);
        assert!(
            max - min <= 2.0,
            "the types spread {:.2} dB at the same Amount: {levels:?}",
            max - min
        );
    }
}
