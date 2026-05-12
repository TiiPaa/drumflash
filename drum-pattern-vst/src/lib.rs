use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use nih_plug::{
    params::persist::serialize_field,
    wrapper::state::{ParamValue, PluginState},
};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc,
};

mod generator;
mod midi_export;
mod sequencer;
mod sound_settings;
mod synthesis;
mod ui;

use generator::{GeneratorType, Style};
use sequencer::{pattern::PersistentPattern, Pattern, Sequencer, SharedPattern};
use sound_settings::{PersistentSoundSettings, SoundSettingsState};
use synthesis::{DrumSynthesizer, DrumVoice};

const VST3_CLASS_ID: [u8; 16] = *b"DrumFlashPlugin1";
pub(crate) const BUILD_ID: &str = match option_env!("DRUM_PATTERN_BUILD_ID") {
    Some(build_id) => build_id,
    None => "dev",
};
const OUTPUT_PORT_NAMES: [&str; DrumVoice::COUNT] = [
    "Kick", "Snare", "Hi-Hat", "Open HH", "Tom 1", "Tom 2", "Tom 3",
];
const MIDI_NOTE_MAP: [u8; DrumVoice::COUNT] = [36, 38, 42, 46, 50, 47, 43];
const STEP_COUNT: usize = 16;

const PATTERN_STATE_FIELD: &str = "pattern-v1";

pub struct DrumFlashVst {
    params: Arc<DrumFlashParams>,
    sequencer: Sequencer,
    synthesizer: DrumSynthesizer,
    sample_rate: f32,
    current_step: Arc<AtomicU32>,
    pattern: Arc<SharedPattern>,
    last_step_masks: [u8; STEP_COUNT],
    voice_test_triggers: Arc<[AtomicBool; DrumVoice::COUNT]>,
    sound_settings_state: Arc<SoundSettingsState>,
    last_sound_settings_version: u64,
}

#[derive(Params)]
pub struct DrumFlashParams {
    #[persist = "editor-state-v2"]
    pub editor_state: Arc<EguiState>,

    #[persist = "pattern-v1"]
    pub pattern_state: PersistentPattern,

    #[persist = "sound-settings-v1"]
    pub sound_settings: PersistentSoundSettings,

    #[id = "master_vol"]
    pub master_volume: FloatParam,

    #[id = "bpm"]
    pub bpm: FloatParam,

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
}

impl Default for DrumFlashParams {
    fn default() -> Self {
        let default_pattern = Pattern::rock_pattern();
        let _default_masks = default_pattern.step_masks();
        let pattern_state = PersistentPattern::new(&default_pattern);

        Self {
            editor_state: EguiState::from_size(980, 560),
            pattern_state,
            sound_settings: PersistentSoundSettings::new(),

            master_volume: FloatParam::new(
                "Master Volume",
                0.8,
                FloatRange::Linear { min: 0.0, max: 1.5 },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
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

            mute_kick: BoolParam::new("Mute Kick", false),
            mute_snare: BoolParam::new("Mute Snare", false),
            mute_hihat: BoolParam::new("Mute Hi-Hat", false),
            mute_open_hh: BoolParam::new("Mute Open HH", false),
            mute_tom1: BoolParam::new("Mute Tom 1", false),
            mute_tom2: BoolParam::new("Mute Tom 2", false),
            mute_tom3: BoolParam::new("Mute Tom 3", false),
            solo_kick: BoolParam::new("Solo Kick", false),
            solo_snare: BoolParam::new("Solo Snare", false),
            solo_hihat: BoolParam::new("Solo Hi-Hat", false),
            solo_open_hh: BoolParam::new("Solo Open HH", false),
            solo_tom1: BoolParam::new("Solo Tom 1", false),
            solo_tom2: BoolParam::new("Solo Tom 2", false),
            solo_tom3: BoolParam::new("Solo Tom 3", false),

            generator_type: EnumParam::new("Generator", GeneratorType::Probabilistic),
            style_primary: EnumParam::new("Style A", Style::Rock),
            style_secondary: EnumParam::new("Style B", Style::Rock),
            style_mix: FloatParam::new(
                "Style Mix",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            gen_density: FloatParam::new(
                "Density",
                0.7,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            gen_variation: FloatParam::new(
                "Variation",
                0.3,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
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
            last_step_masks: default_masks,
            voice_test_triggers: voice_test_triggers.clone(),
            sound_settings_state: sound_settings_state.clone(),
            last_sound_settings_version: 0,
        };
        plugin.sequencer.play();
        plugin
    }
}

impl DrumFlashVst {
    fn remember_current_pattern(&mut self) {
        self.last_step_masks = self.pattern.step_masks();
    }
}

impl Plugin for DrumFlashVst {
    const NAME: &'static str = "Drum Flash";
    const VENDOR: &'static str = "DrumFlash";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        aux_input_ports: &[],
        aux_output_ports: &[new_nonzero_u32(2); DrumVoice::COUNT],
        names: PortNames {
            layout: Some("Stereo mix + 7 stereo drum outs"),
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
            self.pattern.clone(),
            self.voice_test_triggers.clone(),
            self.sound_settings_state.clone(),
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

        nih_log!(
            "Drum Flash initialized at {} Hz",
            buffer_config.sample_rate
        );
        true
    }

    fn reset(&mut self) {
        self.sequencer.stop();
        self.synthesizer.reset();
        self.current_step.store(0, Ordering::Relaxed);
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let transport = context.transport();
        let sample_rate = self.sample_rate;
        let bpm = transport
            .tempo
            .map(|tempo| tempo as f32)
            .unwrap_or_else(|| self.params.bpm.smoothed.next());
        let host_reports_timeline = transport.pos_beats().is_some() || transport.tempo.is_some();

        if host_reports_timeline {
            if transport.playing != self.sequencer.is_playing() {
                if transport.playing {
                    self.sequencer.play();
                } else {
                    self.sequencer.stop();
                }
            }

            if transport.playing {
                if let Some(position_beats) = transport.pos_beats() {
                    self.sequencer
                        .sync_to_host(position_beats, bpm, sample_rate);
                }
            }
        } else if !self.sequencer.is_playing() {
            self.sequencer.play();
        }

        let mute_states = [
            self.params.mute_kick.value(),
            self.params.mute_snare.value(),
            self.params.mute_hihat.value(),
            self.params.mute_open_hh.value(),
            self.params.mute_tom1.value(),
            self.params.mute_tom2.value(),
            self.params.mute_tom3.value(),
        ];
        let solo_states = [
            self.params.solo_kick.value(),
            self.params.solo_snare.value(),
            self.params.solo_hihat.value(),
            self.params.solo_open_hh.value(),
            self.params.solo_tom1.value(),
            self.params.solo_tom2.value(),
            self.params.solo_tom3.value(),
        ];
        let any_solo_active = solo_states.iter().copied().any(|solo| solo);
        let effective_mutes = std::array::from_fn(|index| {
            if any_solo_active {
                !solo_states[index]
            } else {
                mute_states[index]
            }
        });

        self.sequencer.set_mutes(effective_mutes);

        for aux_buffer in aux.outputs.iter_mut() {
            for channel in aux_buffer.as_slice().iter_mut() {
                channel.fill(0.0);
            }
        }

        for (sample_idx, channel_samples) in buffer.iter_samples().enumerate() {
            let triggers = self.sequencer.process_sample(bpm, sample_rate);

            for (voice_idx, should_trigger) in triggers.iter().enumerate() {
                if *should_trigger {
                    self.synthesizer.trigger(voice_idx);

                    let note = MIDI_NOTE_MAP[voice_idx];
                    context.send_event(NoteEvent::NoteOn {
                        timing: sample_idx as u32,
                        voice_id: None,
                        channel: 9,
                        note,
                        velocity: 0.8,
                    });
                    context.send_event(NoteEvent::NoteOff {
                        timing: (sample_idx + 1) as u32,
                        voice_id: None,
                        channel: 9,
                        note,
                        velocity: 0.0,
                    });
                }
            }

            for (voice_idx, trigger) in self.voice_test_triggers.iter().enumerate() {
                if trigger.swap(false, Ordering::Relaxed) {
                    self.synthesizer.trigger(voice_idx);
                }
            }

            let current_version = self.sound_settings_state.version.load(Ordering::Relaxed);
            if current_version != self.last_sound_settings_version {
                self.last_sound_settings_version = current_version;
                for (i, inst) in self.sound_settings_state.instruments.iter().enumerate() {
                    let (freq, decay, vol, filt) = inst.load();
                    self.synthesizer.set_voice_settings(
                        synthesis::DrumVoice::from_index(i).unwrap(),
                        synthesis::VoiceSettings {
                            frequency: freq,
                            decay,
                            volume: vol,
                            filter_freq: filt,
                        },
                    );
                }
            }

            let master_vol = self.params.master_volume.smoothed.next();
            let mut voice_outputs = [0.0f32; DrumVoice::COUNT];
            self.synthesizer.process_voice_samples(&mut voice_outputs);

            let mixed_sample = voice_outputs.iter().copied().sum::<f32>() * master_vol;

            for sample in channel_samples {
                *sample = mixed_sample;
            }

            for (voice_idx, aux_buffer) in aux.outputs.iter_mut().enumerate() {
                if voice_idx >= DrumVoice::COUNT {
                    break;
                }

                let voice_sample = voice_outputs[voice_idx] * master_vol;
                for channel in aux_buffer.as_slice().iter_mut() {
                    channel[sample_idx] = voice_sample;
                }
            }
        }

        self.current_step
            .store(self.sequencer.current_step() as u32, Ordering::Relaxed);

        ProcessStatus::Normal
    }

    fn filter_state(state: &mut PluginState) {
        if state.fields.contains_key(PATTERN_STATE_FIELD) {
            return;
        }

        let masks: [u8; STEP_COUNT] = std::array::from_fn(|step| {
            let key = format!("st{:02}", step + 1);
            match state.params.get(&key) {
                Some(ParamValue::I32(value)) => (*value).clamp(0, 127) as u8,
                _ => 0,
            }
        });

        if let Ok(serialized_pattern) = serialize_field(&masks) {
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
    fn persistent_pattern_serializes_shared_pattern_edits() {
        let pattern_state = PersistentPattern::new(&Pattern::rock_pattern());
        let shared_pattern = pattern_state.shared();

        shared_pattern.set_step_mask(0, 0);
        shared_pattern.set_step_mask(1, 0b0111_1111);

        pattern_state.map(|masks| {
            assert_eq!(masks[0], 0);
            assert_eq!(masks[1], 0b0111_1111);
        });

        let restored_masks = [3u8; STEP_COUNT];
        pattern_state.set(restored_masks);

        assert_eq!(shared_pattern.load_step_mask(0), 3);
        assert_eq!(shared_pattern.load_step_mask(15), 3);
    }

    #[test]
    fn legacy_step_params_migrate_to_persistent_pattern_field() {
        let mut params = BTreeMap::new();
        params.insert("st01".to_string(), ParamValue::I32(0));
        params.insert("st02".to_string(), ParamValue::I32(0b0111_1111));

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
        let masks: [u8; STEP_COUNT] =
            deserialize_field(serialized_pattern).expect("pattern field should deserialize");

        assert_eq!(masks[0], 0);
        assert_eq!(masks[1], 0b0111_1111);
        assert_eq!(masks[2], 0);
    }
}
