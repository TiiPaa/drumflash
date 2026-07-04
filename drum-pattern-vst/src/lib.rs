use nih_plug::prelude::*;
use nih_plug::{
    params::persist::{deserialize_field, serialize_field, PersistentField},
    wrapper::state::{ParamValue, PluginState},
};
use nih_plug_egui::EguiState;
use std::sync::{
    atomic::{AtomicBool, AtomicI32, AtomicU16, AtomicU32, Ordering},
    Arc,
};

mod generator;
mod groove;
mod instrument_registry;
mod midi_export;
mod pattern_bank;
mod plock;
mod preset_dumps;
mod sequencer;
mod sound_settings;
mod synthesis;
mod track;
mod ui;

use generator::{GeneratorType, Style};
use plock::{PersistentPlockState, PersistentSequencerPlockState};
use sequencer::{pattern::PersistentPattern, Pattern, Sequencer, SharedPattern};
use sound_settings::{PersistentSoundSettings, SoundSettingsState};
use synthesis::{DrumSynthesizer, DrumVoice};

const VST3_CLASS_ID: [u8; 16] = *b"DrumFlashPlugin1";
pub(crate) const BUILD_ID: &str = match option_env!("DRUM_PATTERN_BUILD_ID") {
    Some(build_id) => build_id,
    None => "dev",
};
/// Number of dedicated stereo aux outputs. Changed to 14 for the modular grid.
const AUX_OUT_COUNT: usize = 14;
const OUTPUT_PORT_NAMES: [&str; AUX_OUT_COUNT] = [
    "Kick",
    "Snare",
    "Hi-Hat",
    "Open HH",
    "Tom 1",
    "Tom 2",
    "Tom 3",
    "Clap",
    "Ride",
    "Cymbal",
    "Snare 606",
    "808 Kick",
    "Perc1",
    "Out 14",
];
const STEP_COUNT: usize = 64;

const PATTERN_STATE_FIELD: &str = "pattern-v5";

#[derive(Clone)]
pub struct LaneLengthLocks {
    mask: Arc<AtomicU16>,
}

impl LaneLengthLocks {
    pub fn new() -> Self {
        Self {
            mask: Arc::new(AtomicU16::new(0)),
        }
    }

    pub fn is_locked(&self, instrument: usize) -> bool {
        instrument < crate::track::MAX_TRACKS
            && (self.mask.load(Ordering::Relaxed) & (1u16 << instrument)) != 0
    }

    pub fn set_locked(&self, instrument: usize, locked: bool) {
        if instrument >= crate::track::MAX_TRACKS {
            return;
        }
        let bit = 1u16 << instrument;
        if locked {
            self.mask.fetch_or(bit, Ordering::Relaxed);
        } else {
            self.mask.fetch_and(!bit, Ordering::Relaxed);
        }
    }
}

impl Default for LaneLengthLocks {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> PersistentField<'a, u16> for LaneLengthLocks {
    fn set(&self, new_value: u16) {
        let valid_bits = (1u16 << crate::track::MAX_TRACKS).wrapping_sub(1);
        self.mask.store(new_value & valid_bits, Ordering::Relaxed);
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&u16) -> R,
    {
        let mask = self.mask.load(Ordering::Relaxed);
        f(&mask)
    }
}

fn resolve_track_length(raw_length: usize, master_length: usize, locked: bool) -> usize {
    let master_length = master_length.clamp(1, 64);
    if locked {
        raw_length.clamp(1, 64)
    } else {
        master_length
    }
}

pub struct DrumFlashVst {
    params: Arc<DrumFlashParams>,
    sequencer: Sequencer,
    synthesizer: DrumSynthesizer,
    sample_rate: f32,
    current_step: Arc<AtomicU32>,
    current_steps: Arc<[AtomicU32; crate::track::MAX_TRACKS]>,
    pattern: Arc<SharedPattern>,
    last_step_masks: [u16; STEP_COUNT],
    voice_test_triggers: Arc<[AtomicBool; crate::track::MAX_TRACKS]>,
    sound_settings_state: Arc<SoundSettingsState>,
    last_sound_settings_version: u64,
    /// Last host beat position, used to detect seeks.
    last_host_pos: Option<f64>,
    /// Simple LCG RNG state for probability checks (audio-thread safe).
    rng_state: AtomicU32,
    /// Pending delayed triggers: (samples_until, slot_idx, velocity, step, hard, settings).
    /// Fixed-size queue for audio-thread safety (no allocation).
    /// Max: 14 slots x 7 extra triggers = 98, rounded up to 128.
    pending_triggers: [(u32, usize, f32, u32, bool, synthesis::VoiceSettings); 128],
    pending_trigger_count: usize,
    /// Pattern save request from UI: slot index + 1 (0 = none).
    save_pattern_request: Arc<AtomicU32>,
    /// Pattern load request from UI: slot index + 1 (0 = none).
    load_pattern_request: Arc<AtomicU32>,
    /// Clear all plocks request from UI (true = clear on next process()).
    clear_plocks_request: Arc<AtomicBool>,
    /// Song mode state (UI-driven, atomic for lock-free access).
    song_mode: Arc<AtomicBool>,
    /// Current position in the song sequence (0-63).
    song_position: Arc<AtomicU32>,
    /// Last observed loop count to detect pattern wrap.
    last_loop_count: usize,
    /// Last observed master length so we can resync the sequencer when it changes.
    last_master_length: usize,
    /// Last active track layout kinds, so the audio thread can reinitialize
    /// synthesizer voices when a slot is added, removed or reassigned.
    last_slot_kinds: [Option<crate::track::TrackInstrumentKind>; crate::track::MAX_TRACKS],
    /// Pending pattern length update after a slot load (1-64, 0 = none).
    /// The UI thread applies this to the IntParam on the next frame.
    pending_pattern_length: Arc<AtomicI32>,
    /// Pre-allocated scratch buffers so restore() never allocates in the audio thread.
    temp_plock_bytes: [u8; pattern_bank::MAX_PLOCK_BYTES],
    temp_seq_plock_bytes: [u8; pattern_bank::MAX_SEQ_PLOCK_BYTES],
    temp_fusion_bytes: [u8; pattern_bank::MAX_FUSION_BYTES],
    /// Which slot the audio thread last successfully loaded (0-7), or u32::MAX if none.
    /// The UI thread reads this to keep last_loaded_slot in sync with reality.
    audio_last_loaded_slot: Arc<AtomicU32>,
}

#[derive(Params)]
pub struct DrumFlashParams {
    #[persist = "editor-state-v2"]
    pub editor_state: Arc<EguiState>,

    #[persist = "pattern-v5"]
    pub pattern_state: PersistentPattern,

    #[persist = "sound-settings-v2"]
    pub sound_settings: PersistentSoundSettings,

    #[persist = "track-layout-v1"]
    pub track_layout: track::PersistentTrackLayout,

    #[persist = "plock-v1"]
    pub plock_state: PersistentPlockState,

    #[persist = "seq-plock-v1"]
    pub seq_plock_state: PersistentSequencerPlockState,

    #[persist = "pattern-bank-v1"]
    pub pattern_bank: pattern_bank::PersistentPatternBank,

    #[persist = "lane-locks-v1"]
    pub lane_length_locks: LaneLengthLocks,

    #[id = "master_vol"]
    pub master_volume: FloatParam,

    #[id = "bpm"]
    pub bpm: FloatParam,

    #[id = "swing"]
    pub swing: FloatParam,

    #[id = "groove_type"]
    pub groove_type: EnumParam<groove::GrooveType>,

    /// When true (default), the internal step sequencer generates triggers.
    /// When false, the plugin responds to incoming MIDI notes on the configured channel.
    #[id = "int_seq"]
    pub use_internal_sequencer: BoolParam,

    /// When true, the sequencer plays through the song sequence (P1-P8 chain).
    /// When false, the current pattern loops normally.
    #[id = "song_mode"]
    pub song_mode: BoolParam,

    // Per-track groove parameters (3 × 7 instruments)
    #[id = "hu_kick"]
    pub humanize_kick: FloatParam,
    #[id = "hu_snare"]
    pub humanize_snare: FloatParam,
    #[id = "hu_hihat"]
    pub humanize_hihat: FloatParam,
    #[id = "hu_ohh"]
    pub humanize_open_hh: FloatParam,
    #[id = "hu_t1"]
    pub humanize_tom1: FloatParam,
    #[id = "hu_t2"]
    pub humanize_tom2: FloatParam,
    #[id = "hu_t3"]
    pub humanize_tom3: FloatParam,
    #[id = "hu_clap"]
    pub humanize_clap: FloatParam,
    #[id = "hu_ride"]
    pub humanize_ride: FloatParam,
    #[id = "hu_cymbal"]
    pub humanize_cymbal: FloatParam,
    #[id = "hu_snare606"]
    pub humanize_snare606: FloatParam,
    #[id = "hu_b8"]
    pub humanize_bassdrum808: FloatParam,
    #[id = "hu_perc1"]
    pub humanize_perc1: FloatParam,
    #[id = "hum_s13"]
    pub hum_s13: FloatParam,

    #[id = "pp_kick"]
    pub push_kick: FloatParam,
    #[id = "pp_snare"]
    pub push_snare: FloatParam,
    #[id = "pp_hihat"]
    pub push_hihat: FloatParam,
    #[id = "pp_ohh"]
    pub push_open_hh: FloatParam,
    #[id = "pp_t1"]
    pub push_tom1: FloatParam,
    #[id = "pp_t2"]
    pub push_tom2: FloatParam,
    #[id = "pp_t3"]
    pub push_tom3: FloatParam,
    #[id = "pp_clap"]
    pub push_clap: FloatParam,
    #[id = "pp_ride"]
    pub push_ride: FloatParam,
    #[id = "pp_cymbal"]
    pub push_cymbal: FloatParam,
    #[id = "pp_snare606"]
    pub push_snare606: FloatParam,
    #[id = "pp_b8"]
    pub push_bassdrum808: FloatParam,
    #[id = "pp_perc1"]
    pub push_perc1: FloatParam,
    #[id = "push_s13"]
    pub push_s13: FloatParam,

    #[id = "pl_kick"]
    pub length_kick: IntParam,
    #[id = "pl_snare"]
    pub length_snare: IntParam,
    #[id = "pl_hihat"]
    pub length_hihat: IntParam,
    #[id = "pl_ohh"]
    pub length_open_hh: IntParam,
    #[id = "pl_t1"]
    pub length_tom1: IntParam,
    #[id = "pl_t2"]
    pub length_tom2: IntParam,
    #[id = "pl_t3"]
    pub length_tom3: IntParam,
    #[id = "pl_clap"]
    pub length_clap: IntParam,
    #[id = "pl_ride"]
    pub length_ride: IntParam,
    #[id = "pl_cymbal"]
    pub length_cymbal: IntParam,
    #[id = "pl_snare606"]
    pub length_snare606: IntParam,
    #[id = "pl_b8"]
    pub length_bassdrum808: IntParam,
    #[id = "pl_perc1"]
    pub length_perc1: IntParam,
    #[id = "len_s13"]
    pub len_s13: IntParam,

    // Global pattern length (master length)
    #[id = "pat_len"]
    pub pattern_length: IntParam,

    #[id = "kick_click"]
    pub kick_click: FloatParam,
    #[id = "kick_click_type"]
    pub kick_click_type: FloatParam,
    // Kick saturation parameters
    #[id = "kick_sat_type"]
    pub kick_saturation_type: FloatParam,
    #[id = "kick_sat_amt"]
    pub kick_saturation_amount: FloatParam,
    #[id = "kick_sat_mix"]
    pub kick_saturation_mix: FloatParam,
    #[id = "kick_sat_out"]
    pub kick_saturation_output_gain: FloatParam,
    #[id = "kick_sat_pre"]
    pub kick_saturation_pre_filter: FloatParam,

    #[id = "tom_stick"]
    pub tom_stick: FloatParam,
    // Tom saturation parameters (shared by all 3 toms)
    #[id = "tom_sat_type"]
    pub tom_saturation_type: FloatParam,
    #[id = "tom_sat_amt"]
    pub tom_saturation_amount: FloatParam,
    #[id = "tom_sat_mix"]
    pub tom_saturation_mix: FloatParam,
    #[id = "tom_sat_out"]
    pub tom_saturation_output_gain: FloatParam,
    #[id = "tom_sat_pre"]
    pub tom_saturation_pre_filter: FloatParam,

    // 808 Bass Drum special parameters
    #[id = "b8_accent"]
    pub bassdrum808_accent: FloatParam,

    #[id = "b8_snap"]
    pub bassdrum808_snap: FloatParam,

    #[id = "b8_drop"]
    pub bassdrum808_pitch_drop: FloatParam,

    #[id = "b8_click_tone"]
    pub bassdrum808_click_tone: FloatParam,
    // 808 Bass Drum saturation parameters
    #[id = "b8_sat_type"]
    pub bassdrum808_saturation_type: FloatParam,
    #[id = "b8_sat_amt"]
    pub bassdrum808_saturation_amount: FloatParam,
    #[id = "b8_sat_mix"]
    pub bassdrum808_saturation_mix: FloatParam,
    #[id = "b8_sat_out"]
    pub bassdrum808_saturation_output_gain: FloatParam,

    #[id = "mute_kick"]
    pub mute_kick: BoolParam,

    #[id = "mute_snare"]
    pub mute_snare: BoolParam,

    #[id = "mute_hihat"]
    pub mute_hihat: BoolParam,

    #[id = "mute_open_hh"]
    pub mute_open_hh: BoolParam,

    #[id = "mute_tom1"]
    pub mute_tom1: BoolParam,

    #[id = "mute_tom2"]
    pub mute_tom2: BoolParam,

    #[id = "mute_tom3"]
    pub mute_tom3: BoolParam,
    #[id = "mute_clap"]
    pub mute_clap: BoolParam,
    #[id = "mute_ride"]
    pub mute_ride: BoolParam,
    #[id = "mute_cymbal"]
    pub mute_cymbal: BoolParam,
    #[id = "mute_snare606"]
    pub mute_snare606: BoolParam,
    #[id = "mute_b8"]
    pub mute_bassdrum808: BoolParam,
    #[id = "mute_perc1"]
    pub mute_perc1: BoolParam,
    #[id = "mute_s13"]
    pub mute_s13: BoolParam,

    // Per-instrument Main Mix inclusion (true = routed to Main Mix)
    #[id = "mix_kick"]
    pub mix_kick: BoolParam,
    #[id = "mix_snare"]
    pub mix_snare: BoolParam,
    #[id = "mix_hihat"]
    pub mix_hihat: BoolParam,
    #[id = "mix_open_hh"]
    pub mix_open_hh: BoolParam,
    #[id = "mix_tom1"]
    pub mix_tom1: BoolParam,
    #[id = "mix_tom2"]
    pub mix_tom2: BoolParam,
    #[id = "mix_tom3"]
    pub mix_tom3: BoolParam,
    #[id = "mix_clap"]
    pub mix_clap: BoolParam,
    #[id = "mix_ride"]
    pub mix_ride: BoolParam,
    #[id = "mix_cymbal"]
    pub mix_cymbal: BoolParam,
    #[id = "mix_snare606"]
    pub mix_snare606: BoolParam,
    #[id = "mix_b8"]
    pub mix_bassdrum808: BoolParam,
    #[id = "mix_perc1"]
    pub mix_perc1: BoolParam,
    #[id = "mix_s13"]
    pub mix_s13: BoolParam,

    #[id = "solo_kick"]
    pub solo_kick: BoolParam,

    #[id = "solo_snare"]
    pub solo_snare: BoolParam,

    #[id = "solo_hihat"]
    pub solo_hihat: BoolParam,

    #[id = "solo_open_hh"]
    pub solo_open_hh: BoolParam,

    #[id = "solo_tom1"]
    pub solo_tom1: BoolParam,

    #[id = "solo_tom2"]
    pub solo_tom2: BoolParam,

    #[id = "solo_tom3"]
    pub solo_tom3: BoolParam,
    #[id = "solo_clap"]
    pub solo_clap: BoolParam,
    #[id = "solo_ride"]
    pub solo_ride: BoolParam,
    #[id = "solo_cymbal"]
    pub solo_cymbal: BoolParam,
    #[id = "solo_snare606"]
    pub solo_snare606: BoolParam,
    #[id = "solo_b8"]
    pub solo_bassdrum808: BoolParam,
    #[id = "solo_perc1"]
    pub solo_perc1: BoolParam,
    #[id = "solo_s13"]
    pub solo_s13: BoolParam,

    #[id = "gen_type"]
    pub generator_type: EnumParam<GeneratorType>,

    #[id = "style_pri"]
    pub style_primary: EnumParam<Style>,

    #[id = "style_sec"]
    pub style_secondary: EnumParam<Style>,

    #[id = "style_mix"]
    pub style_mix: FloatParam,

    #[id = "gen_density"]
    pub gen_density: FloatParam,

    #[id = "gen_var"]
    pub gen_variation: FloatParam,

    // Synthesis algorithms per instrument (0 = default)
    #[id = "algo_kick"]
    pub algo_kick: IntParam,
    #[id = "algo_snare"]
    pub algo_snare: IntParam,
    #[id = "algo_hihat"]
    pub algo_hihat: IntParam,
    #[id = "algo_open_hh"]
    pub algo_open_hh: IntParam,
    #[id = "algo_tom1"]
    pub algo_tom1: IntParam,
    #[id = "algo_tom2"]
    pub algo_tom2: IntParam,
    #[id = "algo_tom3"]
    pub algo_tom3: IntParam,
    #[id = "algo_clap"]
    pub algo_clap: IntParam,
    #[id = "algo_ride"]
    pub algo_ride: IntParam,
    #[id = "algo_cymbal"]
    pub algo_cymbal: IntParam,
    #[id = "algo_snare606"]
    pub algo_snare606: IntParam,
    #[id = "algo_b8"]
    pub algo_bassdrum808: IntParam,
    #[id = "algo_perc1"]
    pub algo_perc1: IntParam,
    #[id = "algo_s13"]
    pub algo_s13: IntParam,

    // Frequency display mode per bass drum (false = Hz, true = Notes)
    #[id = "freq_mode_kick"]
    pub freq_mode_kick: BoolParam,
    #[id = "freq_mode_b8"]
    pub freq_mode_bassdrum808: BoolParam,

    // Special parameters per instrument
    #[id = "snare_snap"]
    pub snare_snap: FloatParam,
    // Snare saturation parameters
    #[id = "snare_sat_type"]
    pub snare_saturation_type: FloatParam,
    #[id = "snare_sat_amt"]
    pub snare_saturation_amount: FloatParam,
    #[id = "snare_sat_mix"]
    pub snare_saturation_mix: FloatParam,
    #[id = "snare_sat_out"]
    pub snare_saturation_output_gain: FloatParam,
    #[id = "snare_sat_pre"]
    pub snare_saturation_pre_filter: FloatParam,

    // Hi-Hat saturation parameters
    #[id = "hihat_sat_type"]
    pub hihat_saturation_type: FloatParam,
    #[id = "hihat_sat_amt"]
    pub hihat_saturation_amount: FloatParam,
    #[id = "hihat_sat_mix"]
    pub hihat_saturation_mix: FloatParam,
    #[id = "hihat_sat_out"]
    pub hihat_saturation_output_gain: FloatParam,
    #[id = "hihat_sat_pre"]
    pub hihat_saturation_pre_filter: FloatParam,

    // Open Hi-Hat saturation parameters
    #[id = "oh_sat_type"]
    pub open_hihat_saturation_type: FloatParam,
    #[id = "oh_sat_amt"]
    pub open_hihat_saturation_amount: FloatParam,
    #[id = "oh_sat_mix"]
    pub open_hihat_saturation_mix: FloatParam,
    #[id = "oh_sat_out"]
    pub open_hihat_saturation_output_gain: FloatParam,
    #[id = "oh_sat_pre"]
    pub open_hihat_saturation_pre_filter: FloatParam,

    // Hi-hat chokes open hi-hat when triggered
    #[id = "hihat_chokes_oh"]
    pub hihat_chokes_oh: BoolParam,

    // Auto-edit: clicking a step in the grid automatically selects its instrument
    #[id = "auto_edit"]
    pub auto_edit: BoolParam,

    // Clap echo: scales burst spacing and tone diffusion between the 4 bursts.
    // 0 = collapse to a single burst, 1 = default 12 ms spread, 2 = wider.
    #[id = "clap_echo"]
    pub clap_echo: FloatParam,

    // Clap saturation parameters
    #[id = "clap_sat_type"]
    pub clap_saturation_type: FloatParam,
    #[id = "clap_sat_amt"]
    pub clap_saturation_amount: FloatParam,
    #[id = "clap_sat_mix"]
    pub clap_saturation_mix: FloatParam,
    #[id = "clap_sat_out"]
    pub clap_saturation_output_gain: FloatParam,
    #[id = "clap_sat_pre"]
    pub clap_saturation_pre_filter: FloatParam,

    // Snare 606 specials: bridged-T resonator fine-tuning.
    #[id = "sn606_res"]
    pub snare606_resonance: FloatParam,
    #[id = "sn606_tone"]
    pub snare606_tone: FloatParam,
    #[id = "sn606_snap"]
    pub snare606_snap: FloatParam,
    // Snare 606 saturation parameters
    #[id = "sn606_sat_type"]
    pub snare606_saturation_type: FloatParam,
    #[id = "sn606_sat_amt"]
    pub snare606_saturation_amount: FloatParam,
    #[id = "sn606_sat_mix"]
    pub snare606_saturation_mix: FloatParam,
    #[id = "sn606_sat_out"]
    pub snare606_saturation_output_gain: FloatParam,
    #[id = "sn606_sat_pre"]
    pub snare606_saturation_pre_filter: FloatParam,

    // Ride saturation parameters
    #[id = "ride_sat_type"]
    pub ride_saturation_type: FloatParam,
    #[id = "ride_sat_amt"]
    pub ride_saturation_amount: FloatParam,
    #[id = "ride_sat_mix"]
    pub ride_saturation_mix: FloatParam,
    #[id = "ride_sat_out"]
    pub ride_saturation_output_gain: FloatParam,
    #[id = "ride_sat_pre"]
    pub ride_saturation_pre_filter: FloatParam,

    // Perc1 special parameters
    #[id = "perc1_sweep"]
    pub perc1_sweep: FloatParam,
    #[id = "perc1_speed"]
    pub perc1_speed: FloatParam,
    #[id = "perc1_bite"]
    pub perc1_bite: FloatParam,
    #[id = "perc1_width"]
    pub perc1_width: FloatParam,
    // Perc1 saturation parameters
    #[id = "perc1_sat_type"]
    pub perc1_saturation_type: FloatParam,
    #[id = "perc1_sat_amt"]
    pub perc1_saturation_amount: FloatParam,
    #[id = "perc1_sat_mix"]
    pub perc1_saturation_mix: FloatParam,
    #[id = "perc1_sat_out"]
    pub perc1_saturation_output_gain: FloatParam,
    #[id = "perc1_sat_pre"]
    pub perc1_saturation_pre_filter: FloatParam,

    // Cymbal special parameters
    #[id = "cy_shimmer"]
    pub cymbal_shimmer_freq: FloatParam,
    #[id = "cy_noise"]
    pub cymbal_noise_type: FloatParam,
    #[id = "cy_shimmer_amt"]
    pub cymbal_shimmer_amount: FloatParam,
    // Cymbal saturation parameters
    #[id = "cy_sat_type"]
    pub cymbal_saturation_type: FloatParam,
    #[id = "cy_sat_amt"]
    pub cymbal_saturation_amount: FloatParam,
    #[id = "cy_sat_mix"]
    pub cymbal_saturation_mix: FloatParam,
    #[id = "cy_sat_out"]
    pub cymbal_saturation_output_gain: FloatParam,
    #[id = "cy_sat_pre"]
    pub cymbal_saturation_pre_filter: FloatParam,
}

impl Default for DrumFlashParams {
    fn default() -> Self {
        let default_layout = crate::track::TrackLayoutState::default_layout();
        let default_pattern = Pattern::rock_pattern();
        let _default_masks = default_pattern.step_masks();
        let pattern_state = PersistentPattern::new(&default_pattern);

        Self {
            editor_state: EguiState::from_size(1480, 900),
            pattern_state,
            sound_settings: PersistentSoundSettings::new(&default_layout),
            track_layout: track::PersistentTrackLayout::new(),
            plock_state: PersistentPlockState::new(),
            seq_plock_state: PersistentSequencerPlockState::new(),
            pattern_bank: pattern_bank::PersistentPatternBank::new(),
            lane_length_locks: LaneLengthLocks::new(),

            master_volume: FloatParam::new(
                "Master Volume",
                0.8,
                FloatRange::Linear { min: 0.0, max: 2.0 },
            )
            // The slider can reach 0.0 (-inf dB), so logarithmic smoothing is unsafe here.
            .with_smoother(SmoothingStyle::Exponential(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2)),

            bpm: FloatParam::new(
                "BPM",
                120.0,
                FloatRange::Linear {
                    min: 60.0,
                    max: 180.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),

            swing: FloatParam::new(
                "Swing",
                0.0,
                FloatRange::Linear {
                    min: -0.5,
                    max: 0.5,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(0)),

            groove_type: EnumParam::new("Groove", groove::GrooveType::Swing16),

            use_internal_sequencer: BoolParam::new("Internal Sequencer", true),
            song_mode: BoolParam::new("Song Mode", false),

            // Humanize per track (0 = none, 1 = max)
            humanize_kick: FloatParam::new(
                "Humanize Kick",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            humanize_snare: FloatParam::new(
                "Humanize Snare",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            humanize_hihat: FloatParam::new(
                "Humanize Hi-Hat",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            humanize_open_hh: FloatParam::new(
                "Humanize Open HH",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            humanize_tom1: FloatParam::new(
                "Humanize Tom 1",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            humanize_tom2: FloatParam::new(
                "Humanize Tom 2",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            humanize_tom3: FloatParam::new(
                "Humanize Tom 3",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            humanize_clap: FloatParam::new(
                "Humanize Clap",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            humanize_ride: FloatParam::new(
                "Humanize Ride",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            humanize_cymbal: FloatParam::new(
                "Humanize Cymbal",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            humanize_snare606: FloatParam::new(
                "Humanize Snare 606",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            humanize_bassdrum808: FloatParam::new(
                "Humanize 808 Kick",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            humanize_perc1: FloatParam::new(
                "Humanize Perc1",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            hum_s13: FloatParam::new(
                "Humanize Slot 13",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),

            // Push/pull per track (-50 ms = early, +50 ms = late)
            push_kick: FloatParam::new(
                "Push/Pull Kick",
                0.0,
                FloatRange::Linear {
                    min: -50.0,
                    max: 50.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" ms"),
            push_snare: FloatParam::new(
                "Push/Pull Snare",
                0.0,
                FloatRange::Linear {
                    min: -50.0,
                    max: 50.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" ms"),
            push_hihat: FloatParam::new(
                "Push/Pull Hi-Hat",
                0.0,
                FloatRange::Linear {
                    min: -50.0,
                    max: 50.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" ms"),
            push_open_hh: FloatParam::new(
                "Push/Pull Open HH",
                0.0,
                FloatRange::Linear {
                    min: -50.0,
                    max: 50.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" ms"),
            push_tom1: FloatParam::new(
                "Push/Pull Tom 1",
                0.0,
                FloatRange::Linear {
                    min: -50.0,
                    max: 50.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" ms"),
            push_tom2: FloatParam::new(
                "Push/Pull Tom 2",
                0.0,
                FloatRange::Linear {
                    min: -50.0,
                    max: 50.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" ms"),
            push_tom3: FloatParam::new(
                "Push/Pull Tom 3",
                0.0,
                FloatRange::Linear {
                    min: -50.0,
                    max: 50.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" ms"),
            push_clap: FloatParam::new(
                "Push/Pull Clap",
                0.0,
                FloatRange::Linear {
                    min: -50.0,
                    max: 50.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" ms"),
            push_ride: FloatParam::new(
                "Push/Pull Ride",
                0.0,
                FloatRange::Linear {
                    min: -50.0,
                    max: 50.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" ms"),
            push_cymbal: FloatParam::new(
                "Push/Pull Cymbal",
                0.0,
                FloatRange::Linear {
                    min: -50.0,
                    max: 50.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" ms"),
            push_snare606: FloatParam::new(
                "Push/Pull Snare 606",
                0.0,
                FloatRange::Linear {
                    min: -50.0,
                    max: 50.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" ms"),
            push_bassdrum808: FloatParam::new(
                "Push/Pull 808 Kick",
                0.0,
                FloatRange::Linear {
                    min: -50.0,
                    max: 50.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" ms"),
            push_perc1: FloatParam::new(
                "Push/Pull Perc1",
                0.0,
                FloatRange::Linear {
                    min: -50.0,
                    max: 50.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" ms"),
            push_s13: FloatParam::new(
                "Push/Pull Slot 13",
                0.0,
                FloatRange::Linear {
                    min: -50.0,
                    max: 50.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" ms"),

            // Pattern length per track (1-64 steps)
            length_kick: IntParam::new("Length Kick", 16, IntRange::Linear { min: 1, max: 64 }),
            length_snare: IntParam::new("Length Snare", 16, IntRange::Linear { min: 1, max: 64 }),
            length_hihat: IntParam::new("Length Hi-Hat", 16, IntRange::Linear { min: 1, max: 64 }),
            length_open_hh: IntParam::new(
                "Length Open HH",
                16,
                IntRange::Linear { min: 1, max: 64 },
            ),
            length_tom1: IntParam::new("Length Tom 1", 16, IntRange::Linear { min: 1, max: 64 }),
            length_tom2: IntParam::new("Length Tom 2", 16, IntRange::Linear { min: 1, max: 64 }),
            length_tom3: IntParam::new("Length Tom 3", 16, IntRange::Linear { min: 1, max: 64 }),
            length_clap: IntParam::new("Length Clap", 16, IntRange::Linear { min: 1, max: 64 }),
            length_ride: IntParam::new("Length Ride", 16, IntRange::Linear { min: 1, max: 64 }),
            length_cymbal: IntParam::new("Length Cymbal", 16, IntRange::Linear { min: 1, max: 64 }),
            length_snare606: IntParam::new(
                "Length Snare 606",
                16,
                IntRange::Linear { min: 1, max: 64 },
            ),
            length_bassdrum808: IntParam::new(
                "Length 808 Kick",
                16,
                IntRange::Linear { min: 1, max: 64 },
            ),
            length_perc1: IntParam::new("Length Perc1", 16, IntRange::Linear { min: 1, max: 64 }),
            len_s13: IntParam::new("Length Slot 13", 16, IntRange::Linear { min: 1, max: 64 }),
            pattern_length: IntParam::new(
                "Pattern Length",
                16,
                IntRange::Linear { min: 1, max: 64 },
            ),

            kick_click: FloatParam::new(
                "Kick Click",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            kick_click_type: FloatParam::new(
                "Kick Click Type",
                1.0,
                FloatRange::Linear { min: 0.0, max: 2.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_step_size(1.0),
            kick_saturation_type: FloatParam::new(
                "Kick Saturation Type",
                0.0,
                FloatRange::Linear { min: 0.0, max: 5.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_step_size(1.0),
            kick_saturation_amount: FloatParam::new(
                "Kick Saturation Amount",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            kick_saturation_mix: FloatParam::new(
                "Kick Saturation Mix",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            kick_saturation_output_gain: FloatParam::new(
                "Kick Saturation Output Gain",
                1.0,
                FloatRange::Linear { min: 0.5, max: 2.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            kick_saturation_pre_filter: FloatParam::new(
                "Kick Saturation Pre-Filter",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),

            tom_stick: FloatParam::new(
                "Tom Stick Attack",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            tom_saturation_type: FloatParam::new(
                "Tom Saturation Type",
                0.0,
                FloatRange::Linear { min: 0.0, max: 5.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_step_size(1.0),
            tom_saturation_amount: FloatParam::new(
                "Tom Saturation Amount",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            tom_saturation_mix: FloatParam::new(
                "Tom Saturation Mix",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            tom_saturation_output_gain: FloatParam::new(
                "Tom Saturation Output Gain",
                1.0,
                FloatRange::Linear { min: 0.5, max: 2.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            tom_saturation_pre_filter: FloatParam::new(
                "Tom Saturation Pre-Filter",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),

            bassdrum808_accent: FloatParam::new(
                "808 Accent",
                0.0,
                FloatRange::Linear { min: 0.0, max: 2.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),

            bassdrum808_snap: FloatParam::new(
                "808 Snap",
                0.0,
                FloatRange::Linear { min: 0.0, max: 2.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),

            bassdrum808_pitch_drop: FloatParam::new(
                "808 Pitch Drop",
                0.0,
                FloatRange::Linear { min: 0.0, max: 2.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),

            bassdrum808_click_tone: FloatParam::new(
                "808 Click Tone",
                4000.0,
                FloatRange::Linear {
                    min: 100.0,
                    max: 8000.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            bassdrum808_saturation_type: FloatParam::new(
                "808 Saturation Type",
                0.0,
                FloatRange::Linear { min: 0.0, max: 5.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_step_size(1.0),
            bassdrum808_saturation_amount: FloatParam::new(
                "808 Saturation Amount",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            bassdrum808_saturation_mix: FloatParam::new(
                "808 Saturation Mix",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            bassdrum808_saturation_output_gain: FloatParam::new(
                "808 Saturation Output Gain",
                1.0,
                FloatRange::Linear { min: 0.5, max: 2.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),

            mute_kick: BoolParam::new("Mute Kick", false),
            mute_snare: BoolParam::new("Mute Snare", false),
            mute_hihat: BoolParam::new("Mute Hi-Hat", false),
            mute_open_hh: BoolParam::new("Mute Open HH", false),
            mute_tom1: BoolParam::new("Mute Tom 1", false),
            mute_tom2: BoolParam::new("Mute Tom 2", false),
            mute_tom3: BoolParam::new("Mute Tom 3", false),
            mute_clap: BoolParam::new("Mute Clap", false),
            mute_ride: BoolParam::new("Mute Ride", false),
            mute_cymbal: BoolParam::new("Mute Cymbal", false),
            mute_snare606: BoolParam::new("Mute Snare 606", false),
            mute_bassdrum808: BoolParam::new("Mute 808 Kick", false),
            mute_perc1: BoolParam::new("Mute Perc1", false),
            mute_s13: BoolParam::new("Mute Slot 13", false),
            mix_kick: BoolParam::new("Mix Kick", true),
            mix_snare: BoolParam::new("Mix Snare", true),
            mix_hihat: BoolParam::new("Mix Hi-Hat", true),
            mix_open_hh: BoolParam::new("Mix Open HH", true),
            mix_tom1: BoolParam::new("Mix Tom 1", true),
            mix_tom2: BoolParam::new("Mix Tom 2", true),
            mix_tom3: BoolParam::new("Mix Tom 3", true),
            mix_clap: BoolParam::new("Mix Clap", true),
            mix_ride: BoolParam::new("Mix Ride", true),
            mix_cymbal: BoolParam::new("Mix Cymbal", true),
            mix_snare606: BoolParam::new("Mix Snare 606", true),
            mix_bassdrum808: BoolParam::new("Mix 808 Kick", true),
            mix_perc1: BoolParam::new("Mix Perc1", true),
            mix_s13: BoolParam::new("Mix Slot 13", true),
            solo_kick: BoolParam::new("Solo Kick", false),
            solo_snare: BoolParam::new("Solo Snare", false),
            solo_hihat: BoolParam::new("Solo Hi-Hat", false),
            solo_open_hh: BoolParam::new("Solo Open HH", false),
            solo_tom1: BoolParam::new("Solo Tom 1", false),
            solo_tom2: BoolParam::new("Solo Tom 2", false),
            solo_tom3: BoolParam::new("Solo Tom 3", false),
            solo_clap: BoolParam::new("Solo Clap", false),
            solo_ride: BoolParam::new("Solo Ride", false),
            solo_cymbal: BoolParam::new("Solo Cymbal", false),
            solo_snare606: BoolParam::new("Solo Snare 606", false),
            solo_bassdrum808: BoolParam::new("Solo 808 Kick", false),
            solo_perc1: BoolParam::new("Solo Perc1", false),
            solo_s13: BoolParam::new("Solo Slot 13", false),

            generator_type: EnumParam::new("Generator", GeneratorType::Probabilistic),
            style_primary: EnumParam::new("Style A", Style::Rock),
            style_secondary: EnumParam::new("Style B", Style::Rock),
            style_mix: FloatParam::new("Style Mix", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            gen_density: FloatParam::new("Density", 0.7, FloatRange::Linear { min: 0.0, max: 1.0 }),
            gen_variation: FloatParam::new(
                "Variation",
                0.3,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

            algo_kick: IntParam::new("Kick Algo", 0, IntRange::Linear { min: 0, max: 2 }),
            algo_snare: IntParam::new("Snare Algo", 0, IntRange::Linear { min: 0, max: 2 }),
            algo_hihat: IntParam::new("HiHat Algo", 0, IntRange::Linear { min: 0, max: 1 }),
            algo_open_hh: IntParam::new("Open HH Algo", 0, IntRange::Linear { min: 0, max: 1 }),
            algo_tom1: IntParam::new("Tom1 Algo", 0, IntRange::Linear { min: 0, max: 1 }),
            algo_tom2: IntParam::new("Tom2 Algo", 0, IntRange::Linear { min: 0, max: 1 }),
            algo_tom3: IntParam::new("Tom3 Algo", 0, IntRange::Linear { min: 0, max: 1 }),
            algo_clap: IntParam::new("Clap Algo", 0, IntRange::Linear { min: 0, max: 1 }),
            algo_ride: IntParam::new("Ride Algo", 0, IntRange::Linear { min: 0, max: 1 }),
            algo_cymbal: IntParam::new("Cymbal Algo", 0, IntRange::Linear { min: 0, max: 0 }),
            // max=1 (not 0) even though there is only one algo today — nih-plug normalizes
            // params as (value - min) / (max - min), which divides by zero when min==max
            // and crashes the host at instantiation.
            algo_snare606: IntParam::new("Snare 606 Algo", 0, IntRange::Linear { min: 0, max: 1 }),
            algo_bassdrum808: IntParam::new(
                "808 Kick Algo",
                0,
                IntRange::Linear { min: 0, max: 1 },
            ),
            algo_perc1: IntParam::new("Perc1 Algo", 0, IntRange::Linear { min: 0, max: 1 }),
            algo_s13: IntParam::new("Slot 13 Algo", 0, IntRange::Linear { min: 0, max: 0 }),

            freq_mode_kick: BoolParam::new("Kick Freq in Notes", false),
            freq_mode_bassdrum808: BoolParam::new("808 Kick Freq in Notes", false),

            snare_snap: FloatParam::new(
                "Snare Snap",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            snare_saturation_type: FloatParam::new(
                "Snare Saturation Type",
                0.0,
                FloatRange::Linear { min: 0.0, max: 5.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_step_size(1.0),
            snare_saturation_amount: FloatParam::new(
                "Snare Saturation Amount",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            snare_saturation_mix: FloatParam::new(
                "Snare Saturation Mix",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            snare_saturation_output_gain: FloatParam::new(
                "Snare Saturation Output Gain",
                1.0,
                FloatRange::Linear { min: 0.5, max: 2.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            snare_saturation_pre_filter: FloatParam::new(
                "Snare Saturation Pre-Filter",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),

            hihat_saturation_type: FloatParam::new(
                "HiHat Saturation Type",
                0.0,
                FloatRange::Linear { min: 0.0, max: 5.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_step_size(1.0),
            hihat_saturation_amount: FloatParam::new(
                "HiHat Saturation Amount",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            hihat_saturation_mix: FloatParam::new(
                "HiHat Saturation Mix",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            hihat_saturation_output_gain: FloatParam::new(
                "HiHat Saturation Output Gain",
                1.0,
                FloatRange::Linear { min: 0.5, max: 2.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            hihat_saturation_pre_filter: FloatParam::new(
                "HiHat Saturation Pre-Filter",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),

            open_hihat_saturation_type: FloatParam::new(
                "OpenHiHat Saturation Type",
                0.0,
                FloatRange::Linear { min: 0.0, max: 5.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_step_size(1.0),
            open_hihat_saturation_amount: FloatParam::new(
                "OpenHiHat Saturation Amount",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            open_hihat_saturation_mix: FloatParam::new(
                "OpenHiHat Saturation Mix",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            open_hihat_saturation_output_gain: FloatParam::new(
                "OpenHiHat Saturation Output Gain",
                1.0,
                FloatRange::Linear { min: 0.5, max: 2.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            open_hihat_saturation_pre_filter: FloatParam::new(
                "OpenHiHat Saturation Pre-Filter",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),

            hihat_chokes_oh: BoolParam::new("HiHat Chokes OpenHH", true),
            auto_edit: BoolParam::new("Auto Edit", true),

            clap_echo: FloatParam::new("Clap Echo", 1.0, FloatRange::Linear { min: 0.0, max: 3.0 })
                .with_smoother(SmoothingStyle::Linear(10.0)),

            clap_saturation_type: FloatParam::new(
                "Clap Saturation Type",
                0.0,
                FloatRange::Linear { min: 0.0, max: 5.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_step_size(1.0),
            clap_saturation_amount: FloatParam::new(
                "Clap Saturation Amount",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            clap_saturation_mix: FloatParam::new(
                "Clap Saturation Mix",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            clap_saturation_output_gain: FloatParam::new(
                "Clap Saturation Output Gain",
                1.0,
                FloatRange::Linear { min: 0.5, max: 2.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            clap_saturation_pre_filter: FloatParam::new(
                "Clap Saturation Pre-Filter",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),

            snare606_resonance: FloatParam::new(
                "Snare 606 Resonance",
                4.5,
                FloatRange::Linear {
                    min: 0.5,
                    max: 12.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            snare606_tone: FloatParam::new(
                "Snare 606 Tone",
                0.55,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            snare606_snap: FloatParam::new(
                "Snare 606 Snap",
                0.3,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            snare606_saturation_type: FloatParam::new(
                "Snare 606 Saturation Type",
                0.0,
                FloatRange::Linear { min: 0.0, max: 5.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_step_size(1.0),
            snare606_saturation_amount: FloatParam::new(
                "Snare 606 Saturation Amount",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            snare606_saturation_mix: FloatParam::new(
                "Snare 606 Saturation Mix",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            snare606_saturation_output_gain: FloatParam::new(
                "Snare 606 Saturation Output Gain",
                1.0,
                FloatRange::Linear { min: 0.5, max: 2.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            snare606_saturation_pre_filter: FloatParam::new(
                "Snare 606 Saturation Pre-Filter",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),

            ride_saturation_type: FloatParam::new(
                "Ride Saturation Type",
                0.0,
                FloatRange::Linear { min: 0.0, max: 5.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_step_size(1.0),
            ride_saturation_amount: FloatParam::new(
                "Ride Saturation Amount",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            ride_saturation_mix: FloatParam::new(
                "Ride Saturation Mix",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            ride_saturation_output_gain: FloatParam::new(
                "Ride Saturation Output Gain",
                1.0,
                FloatRange::Linear { min: 0.5, max: 2.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            ride_saturation_pre_filter: FloatParam::new(
                "Ride Saturation Pre-Filter",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),

            perc1_sweep: FloatParam::new(
                "Perc1 Sweep",
                0.5,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            perc1_speed: FloatParam::new(
                "Perc1 Speed",
                80.0,
                FloatRange::Linear {
                    min: 5.0,
                    max: 300.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" ms"),
            perc1_bite: FloatParam::new(
                "Perc1 Bite",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            perc1_width: FloatParam::new(
                "Perc1 Width",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            perc1_saturation_type: FloatParam::new(
                "Perc1 Saturation Type",
                0.0,
                FloatRange::Linear { min: 0.0, max: 5.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_step_size(1.0),
            perc1_saturation_amount: FloatParam::new(
                "Perc1 Saturation Amount",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            perc1_saturation_mix: FloatParam::new(
                "Perc1 Saturation Mix",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            perc1_saturation_output_gain: FloatParam::new(
                "Perc1 Saturation Output Gain",
                1.0,
                FloatRange::Linear { min: 0.5, max: 2.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            perc1_saturation_pre_filter: FloatParam::new(
                "Perc1 Saturation Pre-Filter",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),

            cymbal_shimmer_freq: FloatParam::new(
                "Cymbal Shimmer Freq",
                15.0,
                FloatRange::Linear {
                    min: 1.0,
                    max: 50.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" Hz"),

            cymbal_noise_type: FloatParam::new(
                "Cymbal Noise Type",
                0.0,
                FloatRange::Linear { min: 0.0, max: 3.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_step_size(1.0),

            cymbal_shimmer_amount: FloatParam::new(
                "Cymbal Shimmer Amount",
                0.15,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(2)),

            cymbal_saturation_type: FloatParam::new(
                "Cymbal Saturation Type",
                0.0,
                FloatRange::Linear { min: 0.0, max: 5.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_step_size(1.0),
            cymbal_saturation_amount: FloatParam::new(
                "Cymbal Saturation Amount",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            cymbal_saturation_mix: FloatParam::new(
                "Cymbal Saturation Mix",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            cymbal_saturation_output_gain: FloatParam::new(
                "Cymbal Saturation Output Gain",
                1.0,
                FloatRange::Linear { min: 0.5, max: 2.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            cymbal_saturation_pre_filter: FloatParam::new(
                "Cymbal Saturation Pre-Filter",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
        }
    }
}

impl DrumFlashParams {
    /// Indexed access to mute parameters.
    pub fn mutes(&self) -> [&BoolParam; crate::track::MAX_TRACKS] {
        [
            &self.mute_kick,
            &self.mute_snare,
            &self.mute_hihat,
            &self.mute_open_hh,
            &self.mute_tom1,
            &self.mute_tom2,
            &self.mute_tom3,
            &self.mute_clap,
            &self.mute_ride,
            &self.mute_cymbal,
            &self.mute_snare606,
            &self.mute_bassdrum808,
            &self.mute_perc1,
            &self.mute_s13,
        ]
    }

    /// Indexed access to solo parameters.
    pub fn solos(&self) -> [&BoolParam; crate::track::MAX_TRACKS] {
        [
            &self.solo_kick,
            &self.solo_snare,
            &self.solo_hihat,
            &self.solo_open_hh,
            &self.solo_tom1,
            &self.solo_tom2,
            &self.solo_tom3,
            &self.solo_clap,
            &self.solo_ride,
            &self.solo_cymbal,
            &self.solo_snare606,
            &self.solo_bassdrum808,
            &self.solo_perc1,
            &self.solo_s13,
        ]
    }

    /// Indexed access to mix parameters.
    pub fn mixes(&self) -> [&BoolParam; crate::track::MAX_TRACKS] {
        [
            &self.mix_kick,
            &self.mix_snare,
            &self.mix_hihat,
            &self.mix_open_hh,
            &self.mix_tom1,
            &self.mix_tom2,
            &self.mix_tom3,
            &self.mix_clap,
            &self.mix_ride,
            &self.mix_cymbal,
            &self.mix_snare606,
            &self.mix_bassdrum808,
            &self.mix_perc1,
            &self.mix_s13,
        ]
    }

    /// Indexed access to algo parameters.
    pub fn algos(&self) -> [&IntParam; crate::track::MAX_TRACKS] {
        [
            &self.algo_kick,
            &self.algo_snare,
            &self.algo_hihat,
            &self.algo_open_hh,
            &self.algo_tom1,
            &self.algo_tom2,
            &self.algo_tom3,
            &self.algo_clap,
            &self.algo_ride,
            &self.algo_cymbal,
            &self.algo_snare606,
            &self.algo_bassdrum808,
            &self.algo_perc1,
            &self.algo_s13,
        ]
    }

    pub fn humanizes(&self) -> [&FloatParam; crate::track::MAX_TRACKS] {
        [
            &self.humanize_kick,
            &self.humanize_snare,
            &self.humanize_hihat,
            &self.humanize_open_hh,
            &self.humanize_tom1,
            &self.humanize_tom2,
            &self.humanize_tom3,
            &self.humanize_clap,
            &self.humanize_ride,
            &self.humanize_cymbal,
            &self.humanize_snare606,
            &self.humanize_bassdrum808,
            &self.humanize_perc1,
            &self.hum_s13,
        ]
    }

    pub fn pushes(&self) -> [&FloatParam; crate::track::MAX_TRACKS] {
        [
            &self.push_kick,
            &self.push_snare,
            &self.push_hihat,
            &self.push_open_hh,
            &self.push_tom1,
            &self.push_tom2,
            &self.push_tom3,
            &self.push_clap,
            &self.push_ride,
            &self.push_cymbal,
            &self.push_snare606,
            &self.push_bassdrum808,
            &self.push_perc1,
            &self.push_s13,
        ]
    }

    pub fn lengths(&self) -> [&IntParam; crate::track::MAX_TRACKS] {
        [
            &self.length_kick,
            &self.length_snare,
            &self.length_hihat,
            &self.length_open_hh,
            &self.length_tom1,
            &self.length_tom2,
            &self.length_tom3,
            &self.length_clap,
            &self.length_ride,
            &self.length_cymbal,
            &self.length_snare606,
            &self.length_bassdrum808,
            &self.length_perc1,
            &self.len_s13,
        ]
    }

    /// Lookup a special parameter by instrument index and special slot.
    pub fn special_param(&self, instrument: usize, special_idx: usize) -> Option<&FloatParam> {
        match (instrument, special_idx) {
            (0, 0) => Some(&self.kick_click),
            (0, 6) => Some(&self.kick_click_type),
            (0, 1) => Some(&self.kick_saturation_type),
            (0, 2) => Some(&self.kick_saturation_amount),
            (0, 3) => Some(&self.kick_saturation_mix),
            (0, 4) => Some(&self.kick_saturation_output_gain),
            (0, 5) => Some(&self.kick_saturation_pre_filter),
            (1, 0) => Some(&self.snare_snap),
            (1, 1) => Some(&self.snare_saturation_type),
            (1, 2) => Some(&self.snare_saturation_amount),
            (1, 3) => Some(&self.snare_saturation_mix),
            (1, 4) => Some(&self.snare_saturation_output_gain),
            (1, 5) => Some(&self.snare_saturation_pre_filter),
            (2, 0) => Some(&self.hihat_saturation_type),
            (2, 1) => Some(&self.hihat_saturation_amount),
            (2, 2) => Some(&self.hihat_saturation_mix),
            (2, 3) => Some(&self.hihat_saturation_output_gain),
            (2, 4) => Some(&self.hihat_saturation_pre_filter),
            (3, 0) => Some(&self.open_hihat_saturation_type),
            (3, 1) => Some(&self.open_hihat_saturation_amount),
            (3, 2) => Some(&self.open_hihat_saturation_mix),
            (3, 3) => Some(&self.open_hihat_saturation_output_gain),
            (3, 4) => Some(&self.open_hihat_saturation_pre_filter),
            (4 | 5 | 6, 0) => Some(&self.tom_stick),
            (4 | 5 | 6, 1) => Some(&self.tom_saturation_type),
            (4 | 5 | 6, 2) => Some(&self.tom_saturation_amount),
            (4 | 5 | 6, 3) => Some(&self.tom_saturation_mix),
            (4 | 5 | 6, 4) => Some(&self.tom_saturation_output_gain),
            (4 | 5 | 6, 5) => Some(&self.tom_saturation_pre_filter),
            (7, 0) => Some(&self.clap_echo),
            (7, 1) => Some(&self.clap_saturation_type),
            (7, 2) => Some(&self.clap_saturation_amount),
            (7, 3) => Some(&self.clap_saturation_mix),
            (7, 4) => Some(&self.clap_saturation_output_gain),
            (7, 5) => Some(&self.clap_saturation_pre_filter),
            (8, 0) => Some(&self.ride_saturation_type),
            (8, 1) => Some(&self.ride_saturation_amount),
            (8, 2) => Some(&self.ride_saturation_mix),
            (8, 3) => Some(&self.ride_saturation_output_gain),
            (8, 4) => Some(&self.ride_saturation_pre_filter),
            (9, 0) => Some(&self.cymbal_shimmer_freq),
            (9, 1) => Some(&self.cymbal_noise_type),
            (9, 2) => Some(&self.cymbal_shimmer_amount),
            (9, 3) => Some(&self.cymbal_saturation_type),
            (9, 4) => Some(&self.cymbal_saturation_amount),
            (9, 5) => Some(&self.cymbal_saturation_mix),
            (9, 6) => Some(&self.cymbal_saturation_output_gain),
            (9, 7) => Some(&self.cymbal_saturation_pre_filter),
            (10, 0) => Some(&self.snare606_resonance),
            (10, 1) => Some(&self.snare606_tone),
            (10, 2) => Some(&self.snare606_snap),
            (10, 3) => Some(&self.snare606_saturation_type),
            (10, 4) => Some(&self.snare606_saturation_amount),
            (10, 5) => Some(&self.snare606_saturation_mix),
            (10, 6) => Some(&self.snare606_saturation_output_gain),
            (10, 7) => Some(&self.snare606_saturation_pre_filter),
            (11, 0) => Some(&self.bassdrum808_accent),
            (11, 1) => Some(&self.bassdrum808_snap),
            (11, 2) => Some(&self.bassdrum808_pitch_drop),
            (11, 3) => Some(&self.bassdrum808_click_tone),
            (11, 4) => Some(&self.bassdrum808_saturation_type),
            (11, 5) => Some(&self.bassdrum808_saturation_amount),
            (11, 6) => Some(&self.bassdrum808_saturation_mix),
            (11, 7) => Some(&self.bassdrum808_saturation_output_gain),
            (12, 0) => Some(&self.perc1_sweep),
            (12, 1) => Some(&self.perc1_speed),
            (12, 2) => Some(&self.perc1_bite),
            (12, 3) => Some(&self.perc1_width),
            (12, 4) => Some(&self.perc1_saturation_type),
            (12, 5) => Some(&self.perc1_saturation_amount),
            (12, 6) => Some(&self.perc1_saturation_mix),
            (12, 7) => Some(&self.perc1_saturation_output_gain),
            (12, 8) => Some(&self.perc1_saturation_pre_filter),
            _ => None,
        }
    }
}

impl Default for DrumFlashVst {
    fn default() -> Self {
        let params = Arc::new(DrumFlashParams::default());
        let pattern = params.pattern_state.shared();
        let default_masks = pattern.step_masks();
        let voice_test_triggers = Arc::new(std::array::from_fn(|_| AtomicBool::new(false)));
        let sound_settings_state = params.sound_settings.state.clone();
        let mut plugin = Self {
            params,
            pattern: pattern.clone(),
            sequencer: Sequencer::new(pattern.clone()),
            synthesizer: DrumSynthesizer::new(),
            sample_rate: 44100.0,
            current_step: Arc::new(AtomicU32::new(0)),
            current_steps: Arc::new(std::array::from_fn(|_| AtomicU32::new(0))),
            last_step_masks: default_masks,
            voice_test_triggers: voice_test_triggers.clone(),
            sound_settings_state: sound_settings_state.clone(),
            last_sound_settings_version: 0,
            last_host_pos: None,
            rng_state: AtomicU32::new(0xACE1_0000),
            pending_triggers: [(0, 0, 0.0, 0, false, synthesis::VoiceSettings::default()); 128],
            pending_trigger_count: 0,
            save_pattern_request: Arc::new(AtomicU32::new(0)),
            load_pattern_request: Arc::new(AtomicU32::new(0)),
            clear_plocks_request: Arc::new(AtomicBool::new(false)),
            song_mode: Arc::new(AtomicBool::new(false)),
            song_position: Arc::new(AtomicU32::new(0)),
            last_loop_count: 0,
            last_master_length: 16,
            last_slot_kinds: [None; crate::track::MAX_TRACKS],
            pending_pattern_length: Arc::new(AtomicI32::new(0)),
            temp_plock_bytes: [0; pattern_bank::MAX_PLOCK_BYTES],
            temp_seq_plock_bytes: [0; pattern_bank::MAX_SEQ_PLOCK_BYTES],
            temp_fusion_bytes: [0; pattern_bank::MAX_FUSION_BYTES],
            audio_last_loaded_slot: Arc::new(AtomicU32::new(u32::MAX)),
        };
        plugin.sequencer.play();
        plugin
    }
}

impl DrumFlashVst {
    fn remember_current_pattern(&mut self) {
        self.last_step_masks = self.pattern.step_masks();
    }

    /// Simple LCG random number generator (audio-thread safe, no allocation).
    fn next_rand(&self) -> f32 {
        let prev = self.rng_state.load(Ordering::Relaxed);
        let next = prev.wrapping_mul(1103515245).wrapping_add(12345);
        self.rng_state.store(next, Ordering::Relaxed);
        ((next >> 16) & 0x7FFF) as f32 / 32767.0
    }

    /// Build VoiceSettings including the correct special params for each instrument.
    fn voice_settings_for(
        &self,
        voice_idx: usize,
        freq: f32,
        decay: f32,
        vol: f32,
        filt: f32,
        attack: f32,
        release: f32,
        decay_curve: f32,
        release_curve: f32,
        hold: f32,
        filter_env_amount: f32,
        filter_env_decay: f32,
        analog: f32,
        stereo: f32,
    ) -> synthesis::VoiceSettings {
        let mut special = [0.0f32; 32];
        for sp_def in crate::instrument_registry::INSTRUMENTS[voice_idx].special_params {
            if let Some(param) = self.params.special_param(voice_idx, sp_def.special_index) {
                special[sp_def.special_index] = param.value();
            }
        }
        synthesis::VoiceSettings {
            frequency: freq,
            decay,
            volume: vol,
            filter_freq: filt,
            attack,
            release,
            decay_curve,
            release_curve,
            hold,
            filter_env_amount,
            filter_env_decay,
            analog,
            stereo,
            algo: match voice_idx {
                0 => self.params.algo_kick.value() as u8,
                1 => self.params.algo_snare.value() as u8,
                2 => self.params.algo_hihat.value() as u8,
                3 => self.params.algo_open_hh.value() as u8,
                4 => self.params.algo_tom1.value() as u8,
                5 => self.params.algo_tom2.value() as u8,
                6 => self.params.algo_tom3.value() as u8,
                7 => self.params.algo_clap.value() as u8,
                8 => self.params.algo_ride.value() as u8,
                9 => self.params.algo_cymbal.value() as u8,
                10 => self.params.algo_snare606.value() as u8,
                11 => self.params.algo_bassdrum808.value() as u8,
                12 => self.params.algo_perc1.value() as u8,
                _ => 0,
            },
            special,
        }
    }

    /// Build the final VoiceSettings for a slot at the current sequencer step,
    /// merging global settings with any per-step plock override.
    ///
    /// Standard settings and plocks are stored PER SLOT; only the special
    /// params inside `voice_settings_for` are still per legacy voice (shared
    /// between slots of the same kind — see TODO ST-7).
    fn voice_settings_at_step(
        &self,
        slot_idx: usize,
        voice_idx: usize,
        step: usize,
    ) -> synthesis::VoiceSettings {
        let inst = &self.sound_settings_state.instruments[slot_idx];
        let (freq, decay, vol, filt, attack, release, dc, rc, hold, fea, fed, analog, stereo) =
            inst.load();
        let global = self.voice_settings_for(
            voice_idx, freq, decay, vol, filt, attack, release, dc, rc, hold, fea, fed, analog,
            stereo,
        );
        self.params
            .plock_state
            .state
            .get_settings(slot_idx, step, &global)
            .unwrap_or(global)
    }

    /// Read the current value of a plock field from VoiceSettings.
    fn read_morph_value(&self,
        settings: &synthesis::VoiceSettings,
        field_index: usize,
        _voice_idx: usize,
    ) -> f32 {
        match field_index {
            0 => settings.frequency,
            1 => settings.decay,
            2 => settings.volume,
            3 => settings.filter_freq,
            4 => settings.release,
            5 => settings.decay_curve,
            6 => settings.release_curve,
            7 => settings.hold,
            8 => settings.filter_env_amount,
            9 => settings.filter_env_decay,
            10 => settings.analog,
            11 => settings.stereo,
            18 => settings.attack,
            _ => {
                if field_index >= crate::plock::SPECIAL_FIELD_START
                    && field_index < crate::plock::FIELD_COUNT
                {
                    let special_index = field_index - crate::plock::SPECIAL_FIELD_START;
                    settings.special.get(special_index).copied().unwrap_or(0.0)
                } else {
                    0.0
                }
            }
        }
    }

    /// Apply a morphed value to the given VoiceSettings field.
    fn apply_morph_value(
        &self,
        settings: &mut synthesis::VoiceSettings,
        field_index: usize,
        _voice_idx: usize,
        value: f32,
    ) {
        match field_index {
            0 => settings.frequency = value,
            1 => settings.decay = value,
            2 => settings.volume = value,
            3 => settings.filter_freq = value,
            4 => settings.release = value,
            5 => settings.decay_curve = value,
            6 => settings.release_curve = value,
            7 => settings.hold = value,
            8 => settings.filter_env_amount = value,
            9 => settings.filter_env_decay = value,
            10 => settings.analog = value,
            11 => settings.stereo = value,
            18 => settings.attack = value,
            _ => {
                if field_index >= crate::plock::SPECIAL_FIELD_START
                    && field_index < crate::plock::FIELD_COUNT
                {
                    let special_index = field_index - crate::plock::SPECIAL_FIELD_START;
                    if special_index < settings.special.len() {
                        settings.special[special_index] = value;
                    }
                }
            }
        }
    }

    /// Save the current pattern into the bank slot.
    pub fn save_pattern_to_slot(&mut self, slot: usize) {
        if slot >= pattern_bank::SLOT_COUNT {
            return;
        }
        let plock = &self.params.plock_state.state;
        let seq_plock = &self.params.seq_plock_state.state;
        if let Ok(mut bank) = self.params.pattern_bank.bank.lock() {
            bank.slots[slot].capture(
                &self.pattern,
                plock,
                seq_plock,
                self.params.pattern_length.value() as u8,
            );
            nih_log!("Pattern saved to slot P{}", slot + 1);
        }
        // Mark this slot as the current one so the UI highlights it in green.
        self.audio_last_loaded_slot
            .store(slot as u32, Ordering::Relaxed);
    }

    /// Load a pattern from the bank slot.
    pub fn load_pattern_from_slot(&mut self, slot: usize) {
        if slot >= pattern_bank::SLOT_COUNT {
            return;
        }

        // 1. Copy slot data under the lock (fast), then release before restore.
        let mut step_masks = [0u16; crate::sequencer::pattern::STEP_COUNT];
        let plock_len: usize;
        let seq_plock_len: usize;
        let fusion_len: usize;
        let pattern_length: u8;
        {
            let bank = match self.params.pattern_bank.bank.lock() {
                Ok(guard) => guard,
                Err(_) => {
                    nih_log!("Pattern bank mutex poisoned, skipping load");
                    return;
                }
            };
            let slot_ref = &bank.slots[slot];
            if !slot_ref.occupied {
                nih_log!("Slot P{} is empty", slot + 1);
                return;
            }
            let maybe_len = slot_ref.copy_data_for_restore(
                &mut step_masks,
                &mut self.temp_plock_bytes,
                &mut self.temp_seq_plock_bytes,
                &mut self.temp_fusion_bytes,
            );
            pattern_length = match maybe_len {
                Some(len) => len,
                None => return,
            };
            plock_len = slot_ref.plock_bytes.len();
            seq_plock_len = slot_ref.seq_plock_bytes.len();
            fusion_len = slot_ref.fusion_bytes.len();
        } // lock released here

        // 2. Wipe old plocks so nothing leaks from the previous pattern, then restore.
        let plock = &self.params.plock_state.state;
        let seq_plock = &self.params.seq_plock_state.state;
        nih_log!(
            "[LOAD] Slot P{}: plock_len={}, seq_plock_len={}, fusion_len={}, pattern_length={}",
            slot + 1,
            plock_len,
            seq_plock_len,
            fusion_len,
            pattern_length
        );
        plock.clear_all();
        seq_plock.clear_all();
        // Clear fusions (Step Fusion feature)
        for inst in 0..crate::sequencer::pattern::INSTRUMENT_COUNT {
            self.pattern.store_fusions(inst, &[]);
        }
        nih_log!(
            "[LOAD] After clear_all: plock mask[0]={:016x}, seq_plock mask[0]={:016x}",
            plock.masks.masks[0].load(Ordering::Relaxed),
            seq_plock.masks[0].load(Ordering::Relaxed)
        );
        pattern_bank::restore_from_buffers(
            &step_masks,
            &self.temp_plock_bytes[..plock_len],
            &self.temp_seq_plock_bytes[..seq_plock_len],
            &self.temp_fusion_bytes[..fusion_len],
            &self.pattern,
            plock,
            seq_plock,
        );
        nih_log!(
            "[LOAD] After restore: plock mask[0]={:016x}, seq_plock mask[0]={:016x}",
            plock.masks.masks[0].load(Ordering::Relaxed),
            seq_plock.masks[0].load(Ordering::Relaxed)
        );

        // 3. Notify UI and record which slot the audio thread actually loaded.
        self.pending_pattern_length
            .store(pattern_length as i32, Ordering::Relaxed);
        self.audio_last_loaded_slot
            .store(slot as u32, Ordering::Relaxed);
        nih_log!("Pattern loaded from slot P{}", slot + 1);
    }

    /// Fire a single voice trigger (audio + MIDI) at the given sample offset.
    /// `slot_idx` is the active track slot; `voice_idx` is the legacy DrumVoice
    /// algorithm used for MIDI note / instrument metadata.
    /// `hard` uses `trigger_hard()` for machine-gun stutter repeats instead of
    /// the smooth anti-click `trigger()`.
    fn fire_voice_trigger(
        &mut self,
        slot_idx: usize,
        voice_idx: usize,
        velocity: f32,
        _step: u32,
        sample_idx: usize,
        context: &mut impl ProcessContext<Self>,
        hihat_chokes_oh: bool,
        hard: bool,
        settings: synthesis::VoiceSettings,
    ) {
        let Some(_voice) = synthesis::DrumVoice::from_index(voice_idx) else {
            return;
        };
        self.synthesizer.set_voice_settings(slot_idx, settings);
        if hard {
            self.synthesizer.trigger_hard(slot_idx, velocity);
        } else {
            self.synthesizer.trigger(slot_idx, velocity);
        }
        if hihat_chokes_oh && voice_idx == 2 {
            // Open hi-hat lives at the slot currently mapped to DrumVoice::OpenHiHat.
            // Reset that synthesizer slot rather than a hard-coded index.
            if let Some(oh_slot) = (0..crate::track::MAX_TRACKS)
                .find(|&s| self.sequencer.slot_voices()[s] == Some(DrumVoice::OpenHiHat as usize))
            {
                self.synthesizer.reset_voice(oh_slot);
            }
        }
        let note = crate::instrument_registry::INSTRUMENTS[voice_idx].midi_note;
        context.send_event(NoteEvent::NoteOn {
            timing: sample_idx as u32,
            voice_id: None,
            channel: 9,
            note,
            velocity,
        });
        context.send_event(NoteEvent::NoteOff {
            timing: sample_idx as u32,
            voice_id: None,
            channel: 9,
            note,
            velocity: 0.0,
        });
    }
}

impl Plugin for DrumFlashVst {
    const NAME: &'static str = "Flash Drum";
    const VENDOR: &'static str = "DrumFlash";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        aux_input_ports: &[],
        aux_output_ports: &[new_nonzero_u32(2); AUX_OUT_COUNT],
        names: PortNames {
            layout: Some("Stereo mix + 14 stereo drum outs"),
            main_input: None,
            main_output: Some("Main Mix"),
            aux_inputs: &[],
            aux_outputs: &OUTPUT_PORT_NAMES,
        },
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        self.remember_current_pattern();
        ui::create_editor(
            self.params.clone(),
            self.current_step.clone(),
            self.current_steps.clone(),
            self.pattern.clone(),
            self.voice_test_triggers.clone(),
            self.sound_settings_state.clone(),
            self.params.plock_state.state.clone(),
            self.save_pattern_request.clone(),
            self.load_pattern_request.clone(),
            self.song_mode.clone(),
            self.song_position.clone(),
            self.pending_pattern_length.clone(),
            self.audio_last_loaded_slot.clone(),
            self.clear_plocks_request.clone(),
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        self.synthesizer.initialize(buffer_config.sample_rate);
        self.last_sound_settings_version = u64::MAX; // force re-sync on next process()
        self.remember_current_pattern();
        self.sequencer.play();
        self.current_step.store(0, Ordering::Relaxed);
        for s in self.current_steps.iter() {
            s.store(0, Ordering::Relaxed);
        }

        nih_log!("Flash Drum initialized at {} Hz", buffer_config.sample_rate);
        true
    }

    fn reset(&mut self) {
        self.sequencer.stop();
        self.synthesizer.reset();
        self.current_step.store(0, Ordering::Relaxed);
        for s in self.current_steps.iter() {
            s.store(0, Ordering::Relaxed);
        }
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Process pattern save/load requests from UI
        if let Some(slot) = self
            .save_pattern_request
            .swap(0, Ordering::Relaxed)
            .checked_sub(1)
        {
            if (slot as usize) < pattern_bank::SLOT_COUNT {
                self.save_pattern_to_slot(slot as usize);
            }
        }
        if let Some(slot) = self
            .load_pattern_request
            .swap(0, Ordering::Relaxed)
            .checked_sub(1)
        {
            if (slot as usize) < pattern_bank::SLOT_COUNT {
                self.load_pattern_from_slot(slot as usize);
            }
        }
        if self.clear_plocks_request.swap(false, Ordering::Relaxed) {
            self.params.plock_state.state.clear_all();
            self.params.seq_plock_state.state.clear_all();
            nih_log!("All plocks cleared");
        }

        let transport = context.transport();
        let sample_rate = self.sample_rate;
        let bpm = transport
            .tempo
            .map(|tempo| tempo as f32)
            .unwrap_or_else(|| self.params.bpm.smoothed.next());
        let position_beats_opt = transport.pos_beats();
        let host_reports_timeline = position_beats_opt.is_some() || transport.tempo.is_some();

        if host_reports_timeline {
            if transport.playing != self.sequencer.is_playing() {
                if transport.playing {
                    self.sequencer.play();
                    // Sync on play start
                    if let Some(position_beats) = position_beats_opt {
                        self.sequencer
                            .sync_to_host(position_beats, bpm, sample_rate);
                        // If starting near beat 0, force step 0 trigger.
                        // sync_to_host overwrites previous_step, which would swallow the first step.
                        if position_beats.rem_euclid(4.0) < 0.1 {
                            self.sequencer.force_step0_trigger();
                        }
                        self.last_host_pos = Some(position_beats);
                    }
                } else {
                    self.sequencer.stop();
                    self.last_host_pos = None;
                }
            } else if transport.playing {
                // Detect significant host seeks and resync. Threshold raised
                // from 0.2 to 1.0 beats: at 0.2, Reaper and Bitwig's
                // sub-buffer position drift accumulated until a spurious
                // resync fired, skipping steps and producing audible drops in
                // the running mix. Studio One sends sample-accurate
                // pos_beats so the bug never surfaced there.
                if let Some(position_beats) = position_beats_opt {
                    let host_pos_mod = position_beats.rem_euclid(4.0);
                    let seq_pos_mod = self.sequencer.beat_position().rem_euclid(4.0);
                    let diff = (host_pos_mod - seq_pos_mod).abs();
                    // Use shortest distance on the 4-beat circle
                    let diff = diff.min(4.0 - diff);
                    if diff > 1.0 {
                        self.sequencer
                            .sync_to_host(position_beats, bpm, sample_rate);
                    }
                    self.last_host_pos = Some(position_beats);
                }
            }
        } else if !self.sequencer.is_playing() {
            self.sequencer.play();
        }

        let mute_states: [bool; crate::track::MAX_TRACKS] =
            std::array::from_fn(|i| self.params.mutes()[i].value());
        let solo_states: [bool; crate::track::MAX_TRACKS] =
            std::array::from_fn(|i| self.params.solos()[i].value());
        let any_solo_active = solo_states.iter().copied().any(|solo| solo);
        let effective_mutes = std::array::from_fn(|index| {
            if index >= crate::track::MAX_TRACKS {
                return false;
            }
            if any_solo_active {
                !solo_states[index]
            } else {
                mute_states[index]
            }
        });

        self.sequencer.set_mutes(effective_mutes);

        let mix_gains: [f32; crate::track::MAX_TRACKS] = std::array::from_fn(|i| {
            if i < crate::track::MAX_TRACKS && self.params.mixes()[i].value() {
                1.0f32
            } else {
                0.0f32
            }
        });

        for aux_buffer in aux.outputs.iter_mut() {
            for channel in aux_buffer.as_slice().iter_mut() {
                channel.fill(0.0);
            }
        }

        // Update per-track groove parameters once per buffer.
        // Lane lengths are clamped to the global pattern length.
        let master_length = self.params.pattern_length.value() as usize;
        let raw_lengths: [usize; crate::track::MAX_TRACKS] =
            std::array::from_fn(|i| self.params.lengths()[i].value() as usize);
        let effective_lengths = std::array::from_fn(|i| {
            resolve_track_length(
                raw_lengths[i],
                master_length,
                self.params.lane_length_locks.is_locked(i),
            )
        });
        self.sequencer.set_track_params(
            effective_lengths,
            std::array::from_fn(|i| self.params.pushes()[i].value()),
            std::array::from_fn(|i| self.params.humanizes()[i].value()),
            master_length,
        );

        // If the pattern length changed, resync the sequencer to the host timeline
        // so loop_count and beat_position stay consistent with the new master length.
        if master_length != self.last_master_length {
            self.last_master_length = master_length;
            if transport.playing {
                if let Some(position_beats) = position_beats_opt {
                    self.sequencer.sync_to_host(position_beats, bpm, sample_rate);
                }
            }
        }

        // Sync fusions from pattern to sequencer without allocating in the audio thread.
        self.sequencer.sync_fusions_from_pattern();

        // Hi-hat chokes open hi-hat
        let hihat_chokes_oh = self.params.hihat_chokes_oh.value();

        // Snapshot the active track layout and propagate it to the sequencer.
        let mut slot_voices = [None; crate::track::MAX_TRACKS];
        for slot in 0..crate::track::MAX_TRACKS {
            if let Some(kind) = self.params.track_layout.state.kind_for_slot(slot) {
                slot_voices[slot] = Some(kind.drum_voice_index());
            }
        }
        self.sequencer.set_slot_voices(slot_voices);

        // Reinitialize synthesizer slots whose kind changed (added/removed/reassigned).
        for slot in 0..crate::track::MAX_TRACKS {
            let current_kind = self.params.track_layout.state.kind_for_slot(slot);
            if current_kind != self.last_slot_kinds[slot] {
                if let Some(kind) = current_kind {
                    self.synthesizer.reinitialize_slot(slot, kind);
                }
                self.last_slot_kinds[slot] = current_kind;
            }
        }

        // Propagate synthesis algorithms (synthesizer is indexed by slot).
        for slot_idx in 0..crate::track::MAX_TRACKS {
            if slot_voices[slot_idx].is_some() {
                self.synthesizer
                    .set_algo(slot_idx, self.params.algos()[slot_idx].value() as u8);
            }
        }

        // Update global sound settings once per buffer, BEFORE triggers.
        // Previously this was inside iter_samples, which caused a click:
        // a trigger with plock settings would be overwritten by global settings
        // in the same buffer, creating a one-sample discontinuity.
        let current_version = self.sound_settings_state.version.load(Ordering::Acquire);
        if current_version != self.last_sound_settings_version {
            self.last_sound_settings_version = current_version;
            for (slot_idx, inst) in self.sound_settings_state.instruments.iter().enumerate() {
                let Some(voice_idx) = slot_voices[slot_idx] else {
                    continue;
                };
                let Some(_voice) = synthesis::DrumVoice::from_index(voice_idx) else {
                    continue;
                };
                let (
                    freq,
                    decay,
                    vol,
                    filt,
                    attack,
                    release,
                    decay_curve,
                    release_curve,
                    hold,
                    filter_env_amount,
                    filter_env_decay,
                    analog,
                    stereo,
                ) = inst.load();
                self.synthesizer.set_voice_settings(
                    slot_idx,
                    self.voice_settings_for(
                        voice_idx,
                        freq,
                        decay,
                        vol,
                        filt,
                        attack,
                        release,
                        decay_curve,
                        release_curve,
                        hold,
                        filter_env_amount,
                        filter_env_decay,
                        analog,
                        stereo,
                    ),
                );
            }
        }

        // Pre-calculate step duration in samples for stutter spacing.
        let step_duration_samples = (sample_rate * 60.0 / (bpm * 4.0)).max(1.0);

        let use_internal_sequencer = self.params.use_internal_sequencer.value();
        let mut next_event = if use_internal_sequencer {
            None
        } else {
            context.next_event()
        };

        for (sample_idx, mut channel_samples) in buffer.iter_samples().enumerate() {
            // Process pending delayed triggers (stutter or fusion pulses).
            let mut i = 0;
            while i < self.pending_trigger_count {
                if self.pending_triggers[i].0 == 0 {
                    let (_, slot_idx, velocity, step, hard, settings) = self.pending_triggers[i];
                    let Some(voice_idx) = slot_voices[slot_idx] else {
                        self.pending_triggers[i] =
                            self.pending_triggers[self.pending_trigger_count - 1];
                        self.pending_trigger_count -= 1;
                        continue;
                    };
                    self.fire_voice_trigger(
                        slot_idx,
                        voice_idx,
                        velocity,
                        step,
                        sample_idx,
                        context,
                        hihat_chokes_oh,
                        hard,
                        settings,
                    );
                    self.pending_triggers[i] =
                        self.pending_triggers[self.pending_trigger_count - 1];
                    self.pending_trigger_count -= 1;
                } else {
                    self.pending_triggers[i].0 -= 1;
                    i += 1;
                }
            }

            if use_internal_sequencer {
                let swing = self.params.swing.value();
                let groove_type = self.params.groove_type.value();
                let triggers = self
                    .sequencer
                    .process_sample(bpm, sample_rate, swing, groove_type);

                for (slot_idx, trigger) in triggers.iter().enumerate() {
                    let Some(voice_idx) = slot_voices[slot_idx] else {
                        continue;
                    };
                    if trigger.should_trigger {
                        let step = trigger.step;
                        let seq_params = self.params.seq_plock_state.state.get(slot_idx, step);

                        // Apply sequencer probability
                        let prob = seq_params.map(|sp| sp.probability).unwrap_or(1.0);
                        if prob < 1.0 && self.next_rand() > prob {
                            continue; // Skip this trigger based on probability
                        }

                        // Apply step conditions
                        let loop_count = self.sequencer.loop_count();
                        let condition_passes = if let Some(sp) = seq_params {
                            use crate::plock::StepCondition::*;
                            match sp.condition {
                                Always => true,
                                First => loop_count == 0,
                                NotFirst => loop_count > 0,
                                Half1 => loop_count % 2 == 0,
                                Half2 => loop_count % 2 == 1,
                                Third1 => loop_count % 3 == 0,
                                Third2 => loop_count % 3 == 1,
                                Third3 => loop_count % 3 == 2,
                                Fourth1 => loop_count % 4 == 0,
                                Fourth2 => loop_count % 4 == 1,
                                Fourth3 => loop_count % 4 == 2,
                                Fourth4 => loop_count % 4 == 3,
                            }
                        } else {
                            true // No condition = always pass
                        };
                        if !condition_passes {
                            continue;
                        }

                        let stutter = if trigger.is_fusion() {
                            1
                        } else {
                            seq_params
                                .as_ref()
                                .map(|sp| sp.stutter_count.max(1))
                                .unwrap_or(1)
                        };
                        let step_for_trigger = step as u32;

                        let base_settings = self.voice_settings_at_step(slot_idx, voice_idx, step);

                        // Fusion pulses are not stutter plocks: they are evenly
                        // distributed over the whole fused-cell duration and use
                        // the start cell's sound plock for every pulse.
                        if trigger.is_fusion() {
                            let pulse_count = trigger.fusion_pulse_count.max(1);
                            let spacing = if pulse_count > 1 {
                                (step_duration_samples
                                    * trigger.fusion_span_cells.max(1) as f32
                                    / pulse_count as f32)
                                    .max(1.0) as u32
                            } else {
                                0
                            };
                            let morph_active = trigger.morph_count > 0;
                            for k in 0..pulse_count {
                                let mut settings = base_settings;
                                if morph_active && pulse_count > 1 {
                                    let t = k as f32 / (pulse_count - 1) as f32;
                                    for m in 0..trigger.morph_count as usize {
                                        let target = trigger.morph_targets[m];
                                        let start_value = self.read_morph_value(
                                            &settings,
                                            target.field as usize,
                                            voice_idx,
                                        );
                                        let morphed =
                                            start_value + (target.end_value - start_value) * t;
                                        self.apply_morph_value(
                                            &mut settings,
                                            target.field as usize,
                                            voice_idx,
                                            morphed,
                                        );
                                    }
                                }
                                if k == 0 {
                                    self.fire_voice_trigger(
                                        slot_idx,
                                        voice_idx,
                                        trigger.velocity,
                                        step_for_trigger,
                                        sample_idx,
                                        context,
                                        hihat_chokes_oh,
                                        false,
                                        settings,
                                    );
                                } else if self.pending_trigger_count < self.pending_triggers.len() {
                                    self.pending_triggers[self.pending_trigger_count] = (
                                        spacing * k as u32,
                                        slot_idx,
                                        trigger.velocity,
                                        step_for_trigger,
                                        false,
                                        settings,
                                    );
                                    self.pending_trigger_count += 1;
                                }
                            }
                            continue;
                        }

                        // Non-fusion: fire the first trigger immediately.
                        self.fire_voice_trigger(
                            slot_idx,
                            voice_idx,
                            trigger.velocity,
                            step_for_trigger,
                            sample_idx,
                            context,
                            hihat_chokes_oh,
                            false,
                            base_settings,
                        );

                        // Schedule additional stutter triggers with temporal spacing.
                        // Spacing is derived from the current step duration (which itself
                        // depends on the DAW BPM) so the roll always fits musically inside
                        // the step.  stutter=N => N evenly-spaced hits across one step.
                        if stutter > 1 {
                            let spacing = (step_duration_samples / stutter as f32).max(1.0) as u32;
                            for k in 1..stutter {
                                if self.pending_trigger_count < self.pending_triggers.len() {
                                    self.pending_triggers[self.pending_trigger_count] = (
                                        spacing * k as u32,
                                        slot_idx,
                                        trigger.velocity,
                                        step_for_trigger,
                                        true,
                                        base_settings,
                                    );
                                    self.pending_trigger_count += 1;
                                }
                            }
                        }
                    }
                }
            } else {
                // MIDI mode: process incoming NoteOn events
                while let Some(event) = next_event {
                    if event.timing() != sample_idx as u32 {
                        break;
                    }
                    if let NoteEvent::NoteOn { note, velocity, .. } = event {
                        if let Some(voice_idx) =
                            crate::instrument_registry::voice_idx_from_midi_note(note)
                        {
                            let Some(slot_idx) = (0..crate::track::MAX_TRACKS)
                                .find(|&s| slot_voices[s] == Some(voice_idx))
                            else {
                                continue;
                            };
                            let settings = self.voice_settings_at_step(slot_idx, voice_idx, 0);
                            let Some(_voice) = synthesis::DrumVoice::from_index(voice_idx) else {
                                continue;
                            };
                            self.synthesizer.set_voice_settings(slot_idx, settings);
                            self.synthesizer.trigger(slot_idx, velocity);
                            if hihat_chokes_oh && voice_idx == 2 {
                                if let Some(oh_slot) = (0..crate::track::MAX_TRACKS)
                                    .find(|&s| slot_voices[s] == Some(DrumVoice::OpenHiHat as usize))
                                {
                                    self.synthesizer.reset_voice(oh_slot);
                                }
                            }
                            // Forward the MIDI event to the output
                            context.send_event(NoteEvent::NoteOn {
                                timing: sample_idx as u32,
                                voice_id: None,
                                channel: 9,
                                note,
                                velocity,
                            });
                            context.send_event(NoteEvent::NoteOff {
                                timing: sample_idx as u32,
                                voice_id: None,
                                channel: 9,
                                note,
                                velocity: 0.0,
                            });
                        }
                    }
                    next_event = context.next_event();
                }
            }

            for (slot_idx, trigger) in self.voice_test_triggers.iter().enumerate() {
                if trigger.swap(false, Ordering::Acquire) {
                    let Some(voice_idx) = slot_voices[slot_idx] else {
                        continue;
                    };
                    let Some(_voice) = synthesis::DrumVoice::from_index(voice_idx) else {
                        continue;
                    };
                    let settings = self.voice_settings_at_step(slot_idx, voice_idx, 0);
                    self.synthesizer.set_voice_settings(slot_idx, settings);
                    self.synthesizer.trigger(slot_idx, 0.8);
                }
            }

            let master_vol = self.params.master_volume.smoothed.next();
            let mut voice_outputs = [[0.0f32; 2]; crate::track::MAX_TRACKS];
            self.synthesizer
                .process_voice_samples_stereo(&mut voice_outputs);

            let mixed_left = voice_outputs
                .iter()
                .enumerate()
                .map(|(i, o)| o[0] * mix_gains[i])
                .sum::<f32>()
                * master_vol;
            let mixed_right = voice_outputs
                .iter()
                .enumerate()
                .map(|(i, o)| o[1] * mix_gains[i])
                .sum::<f32>()
                * master_vol;

            for (ch, sample) in channel_samples.iter_mut().enumerate() {
                *sample = if ch == 0 { mixed_left } else { mixed_right };
            }

            for (slot_idx, aux_buffer) in aux.outputs.iter_mut().enumerate() {
                if slot_idx >= crate::track::MAX_TRACKS {
                    break;
                }
                let channels = aux_buffer.as_slice();
                channels[0][sample_idx] = voice_outputs[slot_idx][0] * master_vol;
                channels[1][sample_idx] = voice_outputs[slot_idx][1] * master_vol;
            }
        }

        // Song mode: advance to next pattern when current pattern wraps.
        if self.params.song_mode.value() && self.sequencer.is_playing() {
            let current_loop_count = self.sequencer.loop_count();
            if current_loop_count != self.last_loop_count {
                self.last_loop_count = current_loop_count;
                let (next_slot, next_pos, _should_loop) = {
                    if let Ok(bank) = self.params.pattern_bank.bank.lock() {
                        let song = &bank.song;
                        let current_pos = self.song_position.load(Ordering::Relaxed) as usize;
                        let next_pos = current_pos + 1;

                        if next_pos < song.length as usize {
                            let slot = song.slot_at(next_pos);
                            (slot, Some(next_pos), false)
                        } else if song.loop_enabled && song.length > 0 {
                            let slot = song.slot_at(0);
                            (slot, Some(0), true)
                        } else {
                            (None, None, false)
                        }
                    } else {
                        (None, None, false)
                    }
                };

                if let Some(pos) = next_pos {
                    if let Some(slot) = next_slot {
                        self.load_pattern_from_slot(slot);
                    }
                    self.song_position.store(pos as u32, Ordering::Relaxed);
                }
            }
        } else {
            // Reset song position when not in song mode
            self.last_loop_count = self.sequencer.loop_count();
        }

        self.current_step
            .store(self.sequencer.current_step() as u32, Ordering::Relaxed);
        for (i, step) in self.sequencer.current_steps().iter().enumerate() {
            self.current_steps[i].store(*step as u32, Ordering::Relaxed);
        }

        ProcessStatus::Normal
    }

    fn filter_state(state: &mut PluginState) {
        if state.fields.contains_key(PATTERN_STATE_FIELD) {
            return;
        }

        // Migration pattern-v4 (13 instruments) → pattern-v5 (14 instruments)
        if let Some(data) = state.fields.get("pattern-v4") {
            if let Ok(old_state) = deserialize_field::<sequencer::pattern::PatternStateV4>(data) {
                let new_state = old_state.expand();
                if let Ok(serialized) = serialize_field(&new_state) {
                    state
                        .fields
                        .insert(PATTERN_STATE_FIELD.to_string(), serialized);
                    return;
                }
            }
        }

        // Migration pattern-v3 (old broken fusion layout, 13 instruments) → pattern-v5 (fixed layout)
        if let Some(data) = state.fields.get("pattern-v3") {
            if let Ok(old_state) = deserialize_field::<sequencer::pattern::PatternStateV3>(data) {
                let mut new_fusions = [0u64; sequencer::pattern::INSTRUMENT_COUNT
                    * sequencer::pattern::MAX_FUSIONS
                    * sequencer::pattern::FUSION_SLOT_COUNT];
                for inst in 0..13 {
                    let base = inst
                        * sequencer::pattern::MAX_FUSIONS
                        * sequencer::pattern::FUSION_SLOT_COUNT;
                    let count_packed = old_state.fusions[base];
                    let count = (count_packed & 0xFF) as usize;
                    let mut new_count = 0usize;
                    for i in 0..count.min(sequencer::pattern::MAX_FUSIONS - 1) {
                        let slot_base = base + (1 + i) * sequencer::pattern::FUSION_SLOT_COUNT;
                        let old_slots = [
                            old_state.fusions[slot_base],
                            old_state.fusions[slot_base + 1],
                            old_state.fusions[slot_base + 2],
                        ];
                        if let Some(group) =
                            sequencer::pattern::unpack_fusion_v3_old(old_slots)
                        {
                            let packed = sequencer::pattern::pack_fusion(&group);
                            let new_slot_base =
                                base + (1 + new_count) * sequencer::pattern::FUSION_SLOT_COUNT;
                            new_fusions[new_slot_base] = packed[0];
                            new_fusions[new_slot_base + 1] = packed[1];
                            new_fusions[new_slot_base + 2] = packed[2];
                            new_count += 1;
                        }
                    }
                    for i in new_count..(sequencer::pattern::MAX_FUSIONS - 1) {
                        let new_slot_base =
                            base + (1 + i) * sequencer::pattern::FUSION_SLOT_COUNT;
                        new_fusions[new_slot_base] = 0;
                        new_fusions[new_slot_base + 1] = 0;
                        new_fusions[new_slot_base + 2] = 0;
                    }
                    new_fusions[base] = (new_count as u64) | (1u64 << 24);
                }
                // The 14th instrument row starts empty.
                let empty_base = 13
                    * sequencer::pattern::MAX_FUSIONS
                    * sequencer::pattern::FUSION_SLOT_COUNT;
                new_fusions[empty_base] = 0 | (1u64 << 24);
                let new_state = sequencer::pattern::PatternState {
                    masks: old_state.masks,
                    fusions: new_fusions,
                };
                if let Ok(serialized) = serialize_field(&new_state) {
                    state
                        .fields
                        .insert(PATTERN_STATE_FIELD.to_string(), serialized);
                    return;
                }
            }
        }

        // Migration pattern-v2 (masks only) → pattern-v4 (masks + fusions)
        if let Some(data) = state.fields.get("pattern-v2") {
            if let Ok(old_masks) = deserialize_field::<sequencer::pattern::PatternMasks>(data) {
                let new_state = sequencer::pattern::PatternState {
                    masks: old_masks.0,
                    fusions: [0u64; sequencer::pattern::INSTRUMENT_COUNT
                        * sequencer::pattern::MAX_FUSIONS
                        * sequencer::pattern::FUSION_SLOT_COUNT],
                };
                if let Ok(serialized) = serialize_field(&new_state) {
                    state
                        .fields
                        .insert(PATTERN_STATE_FIELD.to_string(), serialized);
                    return;
                }
            }
        }

        // Migration pattern-v1 (16 steps) → pattern-v4 (64 steps)
        if let Some(data) = state.fields.get("pattern-v1") {
            if let Ok(old_masks) = deserialize_field::<[u16; 16]>(data) {
                let mut new_masks = [0u16; STEP_COUNT];
                new_masks[..16].copy_from_slice(&old_masks);
                let new_state = sequencer::pattern::PatternState {
                    masks: new_masks,
                    fusions: [0u64; sequencer::pattern::INSTRUMENT_COUNT
                        * sequencer::pattern::MAX_FUSIONS
                        * sequencer::pattern::FUSION_SLOT_COUNT],
                };
                if let Ok(serialized) = serialize_field(&new_state) {
                    state
                        .fields
                        .insert(PATTERN_STATE_FIELD.to_string(), serialized);
                    return;
                }
            }
        }

        // Migration legacy st01..st16 (8-bit masks) → pattern-v4 (64 steps)
        let masks: [u16; STEP_COUNT] = std::array::from_fn(|step| {
            if step < 16 {
                let key = format!("st{:02}", step + 1);
                match state.params.get(&key) {
                    Some(ParamValue::I32(value)) => (*value).clamp(0, 127) as u16,
                    _ => 0,
                }
            } else {
                0
            }
        });
        let new_state = sequencer::pattern::PatternState {
            masks,
            fusions: [0u64; sequencer::pattern::INSTRUMENT_COUNT
                * sequencer::pattern::MAX_FUSIONS
                * sequencer::pattern::FUSION_SLOT_COUNT],
        };
        if let Ok(serialized_pattern) = serialize_field(&new_state) {
            state
                .fields
                .insert(PATTERN_STATE_FIELD.to_string(), serialized_pattern);
        }
    }
}

impl Vst3Plugin for DrumFlashVst {
    const VST3_CLASS_ID: [u8; 16] = VST3_CLASS_ID;
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Drum, Vst3SubCategory::Instrument];
}

nih_export_vst3!(DrumFlashVst);

#[cfg(test)]
mod tests {
    use super::*;
    use nih_plug::params::persist::{deserialize_field, PersistentField};
    use std::collections::BTreeMap;

    #[test]
    fn morphable_fields_are_non_empty_for_all_voices() {
        for inst in 0..crate::instrument_registry::INSTRUMENTS.len() {
            let fields = crate::instrument_registry::morphable_fields(inst);
            assert!(
                !fields.is_empty(),
                "instrument {} should have morphable fields",
                inst
            );
        }
    }

    #[test]
    fn persistent_pattern_serializes_shared_pattern_edits() {
        let pattern_state = PersistentPattern::new(&Pattern::rock_pattern());
        let shared_pattern = pattern_state.shared();

        shared_pattern.set_step_mask(0, 0);
        shared_pattern.set_step_mask(1, 0x3ff);

        pattern_state.map(|state| {
            assert_eq!(state.masks[0], 0);
            assert_eq!(state.masks[1], 0x3ff);
        });

        let restored_state = sequencer::pattern::PatternState {
            masks: [3u16; STEP_COUNT],
            fusions: [0u64; sequencer::pattern::INSTRUMENT_COUNT
                * sequencer::pattern::MAX_FUSIONS
                * sequencer::pattern::FUSION_SLOT_COUNT],
        };
        pattern_state.set(restored_state);

        assert_eq!(shared_pattern.load_step_mask(0), 3);
        assert_eq!(shared_pattern.load_step_mask(15), 3);
    }

    #[test]
    fn pattern_v3_migrates_to_v5_preserving_fusion_geometry() {
        // Build an old-pattern-v3 PatternState with one fusion in old layout (13 instruments).
        let mut old_fusions = [0u64; 13 * sequencer::pattern::MAX_FUSIONS
            * sequencer::pattern::FUSION_SLOT_COUNT];
        let inst = 4usize;
        let base = inst * sequencer::pattern::MAX_FUSIONS * sequencer::pattern::FUSION_SLOT_COUNT;
        let old_meta: u64 = (2u64 << 0) | (6u64 << 8) | (4u64 << 16) | (1u64 << 24);
        let slot_base = base + sequencer::pattern::FUSION_SLOT_COUNT;
        old_fusions[slot_base] = old_meta;
        old_fusions[slot_base + 1] = 0;
        old_fusions[slot_base + 2] = 0;
        old_fusions[base] = (1u64) | (1u64 << 24);

        let mut masks = [0u16; STEP_COUNT];
        masks[0] = 0x10;

        let old_state = sequencer::pattern::PatternStateV3 { masks, fusions: old_fusions };
        let mut fields = BTreeMap::new();
        fields.insert(
            "pattern-v3".to_string(),
            serialize_field(&old_state).unwrap(),
        );
        let mut state = PluginState {
            version: "0.1.0".to_string(),
            params: BTreeMap::new(),
            fields,
        };

        <DrumFlashVst as Plugin>::filter_state(&mut state);

        let serialized_pattern = state
            .fields
            .get(PATTERN_STATE_FIELD)
            .expect("pattern-v5 field should be created");
        let pattern_state: sequencer::pattern::PatternState =
            deserialize_field(serialized_pattern).expect("pattern field should deserialize");

        assert_eq!(pattern_state.masks[0], 0x10);

        // Verify the fusion geometry was preserved via PersistentPattern::set.
        let persistent = PersistentPattern::new(&Pattern::empty());
        persistent.set(pattern_state);
        let loaded = persistent.shared().load_fusions(inst);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].start_cell, 2);
        assert_eq!(loaded[0].end_cell, 6);
        assert_eq!(loaded[0].step_count, 4);
        assert_eq!(loaded[0].morph_count, 0);
    }

    #[test]
    fn pattern_v4_migrates_to_v5_adding_empty_14th_row() {
        // Build a pattern-v4 state (13 instruments) with a Kick on step 0.
        let mut masks = [0u16; STEP_COUNT];
        masks[0] = 0x0001; // Kick bit
        let fusions = [0u64; 13
            * sequencer::pattern::MAX_FUSIONS
            * sequencer::pattern::FUSION_SLOT_COUNT];
        let old_state = sequencer::pattern::PatternStateV4 { masks, fusions };

        let mut fields = BTreeMap::new();
        fields.insert(
            "pattern-v4".to_string(),
            serialize_field(&old_state).unwrap(),
        );
        let mut state = PluginState {
            version: "0.1.0".to_string(),
            params: BTreeMap::new(),
            fields,
        };

        <DrumFlashVst as Plugin>::filter_state(&mut state);

        let serialized_pattern = state
            .fields
            .get(PATTERN_STATE_FIELD)
            .expect("pattern-v5 field should be created");
        let pattern_state: sequencer::pattern::PatternState =
            deserialize_field(serialized_pattern).expect("pattern field should deserialize");

        // Kick bit preserved.
        assert_eq!(pattern_state.masks[0], 0x0001);
        // 14th instrument row is empty.
        assert_eq!(pattern_state.masks[13], 0);
    }

    #[test]
    fn legacy_step_params_migrate_to_persistent_pattern_field() {
        let mut params = BTreeMap::new();
        params.insert("st01".to_string(), ParamValue::I32(0));
        params.insert("st02".to_string(), ParamValue::I32(0x7f));

        let mut state = PluginState {
            version: "0.1.0".to_string(),
            params,
            fields: BTreeMap::new(),
        };

        <DrumFlashVst as Plugin>::filter_state(&mut state);

        let serialized_pattern = state
            .fields
            .get(PATTERN_STATE_FIELD)
            .expect("legacy pattern field should be created");
        let pattern_state: sequencer::pattern::PatternState =
            deserialize_field(serialized_pattern).expect("pattern field should deserialize");

        assert_eq!(pattern_state.masks[0], 0);
        assert_eq!(pattern_state.masks[1], 0x7f);
        assert_eq!(pattern_state.masks[2], 0);
    }

    #[test]
    fn master_volume_smoothing_stays_finite_from_silence() {
        let params = DrumFlashParams::default();

        params.master_volume.smoothed.reset(0.0);
        params.master_volume.smoothed.set_target(44_100.0, 0.8);

        for _ in 0..2_205 {
            assert!(params.master_volume.smoothed.next().is_finite());
        }
    }

    #[test]
    fn track_length_follows_master_when_unlocked() {
        // Unlocked : toujours master
        assert_eq!(resolve_track_length(16, 64, false), 64);
        assert_eq!(resolve_track_length(32, 16, false), 16);
        assert_eq!(resolve_track_length(12, 32, false), 32);
    }

    #[test]
    fn track_length_locked_uses_raw_length() {
        // Locked : vraie longueur individuelle 1..64, indépendamment du master.
        assert_eq!(resolve_track_length(12, 64, true), 12);
        assert_eq!(resolve_track_length(16, 32, true), 16);
        assert_eq!(resolve_track_length(32, 16, true), 32);
        assert_eq!(resolve_track_length(16, 16, true), 16);
        assert_eq!(resolve_track_length(64, 8, true), 64);
    }

    #[test]
    fn lane_length_locks_persistence_roundtrips() {
        let locks = LaneLengthLocks::new();
        locks.set_locked(0, true);
        locks.set_locked(5, true);
        locks.set_locked(12, true);

        locks.map(|mask| {
            assert_eq!(*mask, (1u16 << 0) | (1u16 << 5) | (1u16 << 12));
        });

        let restored = LaneLengthLocks::new();
        restored.set((1u16 << 2) | (1u16 << 12));
        assert!(!restored.is_locked(0));
        assert!(restored.is_locked(2));
        assert!(restored.is_locked(12));
        assert!(!restored.is_locked(5));
    }

    #[test]
    fn cymbal_shimmer_freq_propagates_through_voice_settings_for() {
        let vst = DrumFlashVst::default();
        let cy_idx = DrumVoice::Cymbal as usize;

        let settings = vst.voice_settings_for(
            cy_idx, 6000.0, // freq
            2.0,    // decay
            0.4,    // volume
            8000.0, // filter_freq
            0.002,  // attack
            2.5,    // release
            2.8,    // decay_curve
            3.0,    // release_curve
            0.0,    // hold
            0.0,    // filter_env_amount
            0.05,   // filter_env_decay
            1.0,    // analog
            1.0,    // stereo
        );

        assert!(
            (settings.special[0] - 15.0).abs() < 0.001,
            "cymbal_shimmer_freq should propagate to special[0], expected ~15.0 got {}",
            settings.special[0]
        );
    }
}
