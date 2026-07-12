//! Typed per-instrument settings — replaces opaque `special: [f32; 32]`.
//!
//! Each instrument gets its own struct with named fields for standard
//! and special parameters.  `VoiceSettings` remains the serialization/
//! persistence format; conversion happens inside each voice's
//! `set_settings()` wrapper.

pub mod clap;
pub mod cymbal;
pub mod hihat;
pub mod kick;
pub mod kick_808;
pub mod open_hihat;
pub mod perc1;
pub mod ride;
pub mod snare;
pub mod snare606;
pub mod tom;

#[macro_export]
macro_rules! settings_roundtrip_test {
    ($name:ident, $voice:ident, $settings:ty) => {
        $crate::settings_roundtrip_test!($name, $voice, $settings, assert_frequency);
    };
    ($name:ident, $voice:ident, $settings:ty, $freq_check:ident) => {
        #[test]
        fn $name() {
            let v = $crate::synthesis::VoiceSettings::$voice();
            let s = <$settings>::from(v);
            let v2 = $crate::synthesis::VoiceSettings::from(s);

            $crate::settings_roundtrip_test!(@freq $freq_check, v, v2);

            assert_eq!(v.attack, v2.attack);
            assert_eq!(v.decay, v2.decay);
            assert_eq!(v.decay_curve, v2.decay_curve);
            assert_eq!(v.release, v2.release);
            assert_eq!(v.release_curve, v2.release_curve);
            assert_eq!(v.volume, v2.volume);
            assert_eq!(v.filter_freq, v2.filter_freq);
            assert_eq!(v.filter_env_amount, v2.filter_env_amount);
            assert_eq!(v.filter_env_decay, v2.filter_env_decay);
            assert_eq!(v.hold, v2.hold);
            assert_eq!(v.analog, v2.analog);
            assert_eq!(v.stereo, v2.stereo);
            assert_eq!(v.algo, v2.algo);
            assert_eq!(v.special, v2.special);
        }
    };
    (@freq assert_frequency, $v:expr, $v2:expr) => { assert_eq!($v.frequency, $v2.frequency); };
    (@freq skip_frequency, $v:expr, $v2:expr) => {};
}
