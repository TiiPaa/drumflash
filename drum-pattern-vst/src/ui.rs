use nih_plug::prelude::*;
use nih_plug_egui::{
    create_egui_editor,
    egui::{self, RichText, ScrollArea, Vec2},
    resizable_window::ResizableWindow,
    widgets,
};
use std::{
    fs::create_dir_all,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
};

use crate::{
    generator,
    midi_export,
    sequencer::{Pattern, SharedPattern},
    sound_settings::SoundSettingsState,
    synthesis::{self, DrumVoice},
    DrumFlashParams, BUILD_ID,
};

const INSTRUMENT_LABELS: [&str; DrumVoice::COUNT] = ["BD", "SD", "HH", "OH", "T1", "T2", "T3", "CL", "RD", "CY", "S6"];

pub fn create_editor(
    params: Arc<DrumFlashParams>,
    current_step: Arc<AtomicU32>,
    current_steps: Arc<[AtomicU32; DrumVoice::COUNT]>,
    pattern: Arc<SharedPattern>,
    voice_test_triggers: Arc<[AtomicBool; DrumVoice::COUNT]>,
    sound_settings_state: Arc<SoundSettingsState>,
) -> Option<Box<dyn Editor>> {
    let params_for_ui = params.clone();
    let editor_state = params.editor_state.clone();
    let pattern_for_ui = pattern.clone();
    let voice_test_triggers_for_ui = voice_test_triggers.clone();
    let sound_settings_for_ui = sound_settings_state.clone();
    let current_steps_for_ui = current_steps.clone();

    create_egui_editor(
        params.editor_state.clone(),
        (false, 0usize),
        |_egui_ctx, _state| {},
        move |egui_ctx, setter, (show_sound_panel, selected_instrument)| {
            ResizableWindow::new("drum-pattern-generator")
                .min_size(Vec2::new(900.0, 520.0))
                .show(egui_ctx, editor_state.as_ref(), |ui| {
                    ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.heading("Drum Flash");
                            ui.label(format!(
                                "Pattern editor - v{} - build {}",
                                env!("CARGO_PKG_VERSION"),
                                BUILD_ID
                            ));
                            ui.separator();

                            ui.label(format!(
                                "Current step: {}",
                                current_step.load(Ordering::Relaxed) + 1
                            ));

                            ui.label("Master Volume");
                            ui.add(widgets::ParamSlider::for_param(
                                &params_for_ui.master_volume,
                                setter,
                            ));

                            ui.label("Fallback BPM");
                            ui.add(widgets::ParamSlider::for_param(&params_for_ui.bpm, setter));
                            ui.horizontal(|ui| {
                                ui.label("Swing");
                                ui.add(widgets::ParamSlider::for_param(&params_for_ui.swing, setter).with_width(100.0));
                                enum_combo(ui, setter, &params_for_ui.groove_type, "Groove");
                                bool_checkbox(ui, setter, &params_for_ui.hihat_chokes_oh, "Choke");
                            });

                            ui.label("Sorties: Main Mix + Kick, Snare, Hi-Hat, Open HH, Tom 1, Tom 2, Tom 3, Clap, Ride, Cymbal");

                            ui.separator();
                            ui.horizontal(|ui| {
                                if ui.button("Rock").clicked() {
                                    load_pattern_for_ui(&pattern_for_ui, &Pattern::rock_pattern());
                                }
                                if ui.button("Funk").clicked() {
                                    load_pattern_for_ui(&pattern_for_ui, &Pattern::funk_pattern());
                                }
                                if ui.button("Disco").clicked() {
                                    load_pattern_for_ui(&pattern_for_ui, &Pattern::disco_pattern());
                                }
                                if ui.button("Clear").clicked() {
                                    load_pattern_for_ui(&pattern_for_ui, &Pattern::empty());
                                }
                                if ui.button("Random").clicked() {
                                    load_pattern_for_ui(&pattern_for_ui, &Pattern::random_pattern());
                                }
                                if ui.button(if *show_sound_panel { "Hide Sons" } else { "Sons" }).clicked() {
                                    *show_sound_panel = !*show_sound_panel;
                                }
                                if ui.button("Export MIDI").clicked() {
                                    let bpm = params_for_ui.bpm.value();
                                    match export_midi_to_documents(&pattern_for_ui, bpm) {
                                        Ok(path) => {
                                            nih_log!("MIDI exported to: {}", path.display());
                                        }
                                        Err(e) => {
                                            nih_log!("MIDI export failed: {}", e);
                                        }
                                    }
                                }
                            });

                            ui.separator();
                            ui.label(egui::RichText::new("Pattern Generator").strong());
                            ui.horizontal(|ui| {
                                enum_combo(ui, setter, &params_for_ui.generator_type, "Type");
                                enum_combo(ui, setter, &params_for_ui.style_primary, "Style A");
                                enum_combo(ui, setter, &params_for_ui.style_secondary, "Style B");
                            });
                            ui.horizontal(|ui| {
                                ui.label("Mix");
                                ui.add(widgets::ParamSlider::for_param(&params_for_ui.style_mix, setter).with_width(60.0));
                                ui.label("Density");
                                ui.add(widgets::ParamSlider::for_param(&params_for_ui.gen_density, setter).with_width(60.0));
                                ui.label("Variation");
                                ui.add(widgets::ParamSlider::for_param(&params_for_ui.gen_variation, setter).with_width(60.0));
                                let gen_btn = egui::Button::new(egui::RichText::new(" GENERATE ").strong().size(16.0))
                                    .fill(egui::Color32::from_rgb(56, 132, 255));
                                if ui.add(gen_btn).clicked() {
                                    let gen_params = generator::GeneratorParams {
                                        generator_type: params_for_ui.generator_type.value(),
                                        style_primary: params_for_ui.style_primary.value(),
                                        style_secondary: params_for_ui.style_secondary.value(),
                                        style_mix: params_for_ui.style_mix.value(),
                                        density: params_for_ui.gen_density.value(),
                                        variation: params_for_ui.gen_variation.value(),
                                    };
                                    let seed = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_nanos() as u64;
                                    let mut rng_state = seed;
                                    let mut rng = || {
                                        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                                        (rng_state as f32) / (u64::MAX as f32)
                                    };
                                    let pattern = generator::generate(&gen_params, &mut rng);
                                    load_pattern_for_ui(&pattern_for_ui, &pattern);
                                }
                            });

                            ui.separator();
                            let mixer_rows = mixer_rows(&params_for_ui);
                            egui::Grid::new("pattern-grid")
                                .spacing(Vec2::new(6.0, 6.0))
                                .show(ui, |ui| {
                                    ui.label("");
                                    ui.label(RichText::new("M").strong());
                                    ui.label(RichText::new("S").strong());
                                    ui.label(RichText::new("T").strong());
                                    for step in 0..16 {
                                        let label = if current_step.load(Ordering::Relaxed) as usize
                                            == step
                                        {
                                            RichText::new(format!("{}", step + 1)).strong()
                                        } else {
                                            RichText::new(format!("{}", step + 1))
                                        };
                                        ui.label(label);
                                    }
                                    ui.end_row();

                                    for (instrument, label) in INSTRUMENT_LABELS.iter().enumerate()
                                    {
                                        ui.monospace(*label);
                                        let row = &mixer_rows[instrument];
                                        draw_bool_toggle(ui, setter, row.mute, "M", "Mute");
                                        draw_bool_toggle(ui, setter, row.solo, "S", "Solo");
                                        if ui.button("T").on_hover_text("Test").clicked() {
                                            voice_test_triggers_for_ui[instrument].store(true, Ordering::Relaxed);
                                        }
                                        for step in 0..16 {
                                            let active = pattern_for_ui.is_active(step, instrument);
                                            let is_current = current_steps_for_ui[instrument]
                                                .load(Ordering::Relaxed) as usize
                                                == step;
                                            let button =
                                                egui::Button::new(if active { "X" } else { "." })
                                                    .min_size(Vec2::new(20.0, 20.0))
                                                    .fill(if active {
                                                        egui::Color32::from_rgb(56, 132, 255)
                                                    } else if is_current {
                                                        egui::Color32::from_rgb(48, 48, 48)
                                                    } else {
                                                        egui::Color32::from_rgb(28, 28, 28)
                                                    });

                                            if ui.add(button).clicked() {
                                                toggle_step_for_ui(
                                                    &pattern_for_ui,
                                                    step,
                                                    instrument,
                                                );
                                            }
                                        }
                                        ui.end_row();
                                    }
                                });

                            if *show_sound_panel {
                                ui.separator();
                                draw_sound_panel(ui, &sound_settings_for_ui, selected_instrument, &params_for_ui, setter);
                            }

                            ui.separator();
                            ui.label(egui::RichText::new("Track Groove").strong());
                            egui::Grid::new("track-groove-grid")
                                .spacing(Vec2::new(4.0, 4.0))
                                .show(ui, |ui| {
                                    ui.label("");
                                    ui.label("Hum");
                                    ui.label("Push");
                                    ui.label("Len");
                                    ui.end_row();

                                    let hums = [
                                        &params_for_ui.humanize_kick,
                                        &params_for_ui.humanize_snare,
                                        &params_for_ui.humanize_hihat,
                                        &params_for_ui.humanize_open_hh,
                                        &params_for_ui.humanize_tom1,
                                        &params_for_ui.humanize_tom2,
                                        &params_for_ui.humanize_tom3,
                                        &params_for_ui.humanize_clap,
                                        &params_for_ui.humanize_ride,
                                        &params_for_ui.humanize_cymbal,
                                        &params_for_ui.humanize_snare606,
                                    ];
                                    let pushes = [
                                        &params_for_ui.push_kick,
                                        &params_for_ui.push_snare,
                                        &params_for_ui.push_hihat,
                                        &params_for_ui.push_open_hh,
                                        &params_for_ui.push_tom1,
                                        &params_for_ui.push_tom2,
                                        &params_for_ui.push_tom3,
                                        &params_for_ui.push_clap,
                                        &params_for_ui.push_ride,
                                        &params_for_ui.push_cymbal,
                                        &params_for_ui.push_snare606,
                                    ];
                                    let lengths = [
                                        &params_for_ui.length_kick,
                                        &params_for_ui.length_snare,
                                        &params_for_ui.length_hihat,
                                        &params_for_ui.length_open_hh,
                                        &params_for_ui.length_tom1,
                                        &params_for_ui.length_tom2,
                                        &params_for_ui.length_tom3,
                                        &params_for_ui.length_clap,
                                        &params_for_ui.length_ride,
                                        &params_for_ui.length_cymbal,
                                        &params_for_ui.length_snare606,
                                    ];

                                    for (i, label) in INSTRUMENT_LABELS.iter().enumerate() {
                                        ui.monospace(*label);
                                        ui.add(widgets::ParamSlider::for_param(hums[i], setter).with_width(50.0));
                                        ui.add(widgets::ParamSlider::for_param(pushes[i], setter).with_width(60.0));
                                        ui.add(widgets::ParamSlider::for_param(lengths[i], setter).with_width(40.0));
                                        ui.end_row();
                                    }
                                });

                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.label("Kick Click");
                                ui.add(widgets::ParamSlider::for_param(&params_for_ui.kick_click, setter).with_width(80.0));
                                ui.label("Tom Stick");
                                ui.add(widgets::ParamSlider::for_param(&params_for_ui.tom_stick, setter).with_width(80.0));
                            });

                            ui.separator();
                            ui.label("La grille edite le pattern joue en temps reel.");
                        });
                });
        },
    )
}

fn load_pattern_for_ui(pattern_for_ui: &SharedPattern, pattern: &Pattern) {
    let masks = pattern.step_masks();
    pattern_for_ui.load_step_masks(&masks);
}

fn toggle_step_for_ui(pattern_for_ui: &SharedPattern, step: usize, instrument: usize) {
    let current_mask = pattern_for_ui.load_step_mask(step);
    let bit = 1u16 << instrument;
    let next_mask = current_mask ^ bit;
    pattern_for_ui.set_step_mask(step, next_mask);
}

struct MixerRow<'a> {
    mute: &'a BoolParam,
    solo: &'a BoolParam,
}

fn mixer_rows(params: &DrumFlashParams) -> [MixerRow<'_>; DrumVoice::COUNT] {
    [
        MixerRow {
            mute: &params.mute_kick,
            solo: &params.solo_kick,
        },
        MixerRow {
            mute: &params.mute_snare,
            solo: &params.solo_snare,
        },
        MixerRow {
            mute: &params.mute_hihat,
            solo: &params.solo_hihat,
        },
        MixerRow {
            mute: &params.mute_open_hh,
            solo: &params.solo_open_hh,
        },
        MixerRow {
            mute: &params.mute_tom1,
            solo: &params.solo_tom1,
        },
        MixerRow {
            mute: &params.mute_tom2,
            solo: &params.solo_tom2,
        },
        MixerRow {
            mute: &params.mute_tom3,
            solo: &params.solo_tom3,
        },
        MixerRow {
            mute: &params.mute_clap,
            solo: &params.solo_clap,
        },
        MixerRow {
            mute: &params.mute_ride,
            solo: &params.solo_ride,
        },
        MixerRow {
            mute: &params.mute_cymbal,
            solo: &params.solo_cymbal,
        },
        MixerRow {
            mute: &params.mute_snare606,
            solo: &params.solo_snare606,
        },
    ]
}

fn bool_checkbox(ui: &mut egui::Ui, setter: &ParamSetter, param: &BoolParam, label: &str) {
    let mut value = param.value();
    if ui.checkbox(&mut value, label).changed() {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, value);
        setter.end_set_parameter(param);
    }
}

fn enum_combo<E: nih_plug::prelude::Enum + PartialEq + 'static>(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    param: &EnumParam<E>,
    label: &str,
) {
    let current = param.value();
    let current_idx = current.to_index();
    let variants = E::variants();
    egui::ComboBox::from_label(label)
        .selected_text(variants[current_idx])
        .show_ui(ui, |ui| {
            for (i, name) in variants.iter().enumerate() {
                let selected = i == current_idx;
                if ui.selectable_label(selected, *name).clicked() && !selected {
                    setter.begin_set_parameter(param);
                    setter.set_parameter(param, E::from_index(i));
                    setter.end_set_parameter(param);
                }
            }
        });
}

fn algo_combo(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    param: &IntParam,
    algo_names: &[&str],
) {
    let current = param.value() as usize;
    let current_clamped = current.min(algo_names.len().saturating_sub(1));
    egui::ComboBox::from_label("")
        .selected_text(*algo_names.get(current_clamped).unwrap_or(&"?"))
        .show_ui(ui, |ui| {
            for (i, name) in algo_names.iter().enumerate() {
                let selected = i == current_clamped;
                if ui.selectable_label(selected, *name).clicked() && !selected {
                    setter.begin_set_parameter(param);
                    setter.set_parameter(param, i as i32);
                    setter.end_set_parameter(param);
                }
            }
        });
}

fn draw_bool_toggle(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    param: &BoolParam,
    short_label: &str,
    hover_label: &str,
) {
    let enabled = param.value();
    let button = egui::Button::new(short_label)
        .min_size(Vec2::new(34.0, 24.0))
        .fill(if enabled {
            egui::Color32::from_rgb(56, 132, 255)
        } else {
            egui::Color32::from_rgb(36, 36, 36)
        });

    let response = ui.add(button).on_hover_text(hover_label);
    if response.clicked() {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, !enabled);
        setter.end_set_parameter(param);
    }
}

fn export_midi_to_documents(
    pattern: &SharedPattern,
    bpm: f32,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let docs = std::env::var("USERPROFILE")
        .ok()
        .map(PathBuf::from)
        .map(|p| p.join("Documents"))
        .ok_or("Cannot find Documents folder")?;
    let export_dir = docs.join("Drum Flash").join("exports");
    create_dir_all(&export_dir)?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let filename = format!("drum_pattern_{:.0}bpm_{}.mid", bpm, timestamp);
    let path = export_dir.join(filename);

    midi_export::export_pattern_to_midi(pattern, bpm, &path)?;
    Ok(path)
}

fn draw_sound_panel(
    ui: &mut egui::Ui,
    sound_settings: &SoundSettingsState,
    selected_instrument: &mut usize,
    params: &DrumFlashParams,
    setter: &ParamSetter,
) {
    ui.label(egui::RichText::new("Configuration des Sons").strong());
    ui.horizontal(|ui| {
        for (i, label) in INSTRUMENT_LABELS.iter().enumerate() {
            if ui.selectable_label(*selected_instrument == i, *label).clicked() {
                *selected_instrument = i;
            }
        }
    });

    let inst = &sound_settings.instruments[*selected_instrument];
    let (mut freq, mut decay, mut vol, mut filt, mut release, mut decay_curve, mut release_curve) =
        inst.load();
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label("Frequency");
        if ui
            .add(egui::Slider::new(&mut freq, 20.0..=12000.0).logarithmic(true))
            .changed()
        {
            inst.frequency.store(freq.to_bits(), Ordering::Relaxed);
            changed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Decay");
        if ui
            .add(egui::Slider::new(&mut decay, 0.01..=0.5))
            .changed()
        {
            inst.decay.store(decay.to_bits(), Ordering::Relaxed);
            changed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Decay Curve");
        if ui
            .add(egui::Slider::new(&mut decay_curve, 2.0..=10.0))
            .changed()
        {
            inst.decay_curve.store(decay_curve.to_bits(), Ordering::Relaxed);
            changed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Release");
        if ui
            .add(egui::Slider::new(&mut release, 0.0..=5.0))
            .changed()
        {
            inst.release.store(release.to_bits(), Ordering::Relaxed);
            changed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Release Curve");
        if ui
            .add(egui::Slider::new(&mut release_curve, 2.0..=10.0))
            .changed()
        {
            inst.release_curve.store(release_curve.to_bits(), Ordering::Relaxed);
            changed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Volume");
        if ui
            .add(egui::Slider::new(&mut vol, 0.0..=1.5))
            .changed()
        {
            inst.volume.store(vol.to_bits(), Ordering::Relaxed);
            changed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Filter");
        if ui
            .add(egui::Slider::new(&mut filt, 50.0..=15000.0).logarithmic(true))
            .changed()
        {
            inst.filter_freq.store(filt.to_bits(), Ordering::Relaxed);
            changed = true;
        }
    });

    // Algorithm selector
    let voice = DrumVoice::from_index(*selected_instrument).unwrap();
    let algos = synthesis::algos_for(voice);
    if algos.len() > 1 {
        let algo_param = match *selected_instrument {
            0 => &params.algo_kick,
            1 => &params.algo_snare,
            2 => &params.algo_hihat,
            3 => &params.algo_open_hh,
            4 => &params.algo_tom1,
            5 => &params.algo_tom2,
            6 => &params.algo_tom3,
            7 => &params.algo_clap,
            8 => &params.algo_ride,
            9 => &params.algo_cymbal,
            10 => &params.algo_snare606,
            _ => &params.algo_kick,
        };
        ui.horizontal(|ui| {
            ui.label("Algorithm");
            let algo_names: Vec<&str> = algos.iter().map(|a| a.name).collect();
            algo_combo(ui, setter, algo_param, &algo_names);
        });
    }

    // Per-instrument special parameters
    if *selected_instrument == 0 {
        ui.horizontal(|ui| {
            ui.label("Click Level");
            ui.add(widgets::ParamSlider::for_param(&params.kick_click, setter).with_width(120.0));
        });
    }
    if *selected_instrument == 1 {
        ui.horizontal(|ui| {
            ui.label("Snap");
            ui.add(widgets::ParamSlider::for_param(&params.snare_snap, setter).with_width(120.0));
        });
    }
    if *selected_instrument == 4 || *selected_instrument == 5 || *selected_instrument == 6 {
        ui.horizontal(|ui| {
            ui.label("Stick Attack");
            ui.add(widgets::ParamSlider::for_param(&params.tom_stick, setter).with_width(120.0));
        });
    }
    if *selected_instrument == 7 {
        ui.horizontal(|ui| {
            ui.label("Echo");
            ui.add(widgets::ParamSlider::for_param(&params.clap_echo, setter).with_width(120.0));
        });
    }
    if *selected_instrument == 10 {
        ui.horizontal(|ui| {
            ui.label("Resonance");
            ui.add(widgets::ParamSlider::for_param(&params.snare606_resonance, setter).with_width(120.0));
        });
        ui.horizontal(|ui| {
            ui.label("Tone");
            ui.add(widgets::ParamSlider::for_param(&params.snare606_tone, setter).with_width(120.0));
        });
        ui.horizontal(|ui| {
            ui.label("Snap");
            ui.add(widgets::ParamSlider::for_param(&params.snare606_snap, setter).with_width(120.0));
        });
    }

    if changed {
        sound_settings.bump_version();
    }
}
