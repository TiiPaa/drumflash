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
    DrumFlashParams, BUILD_ID,
};

const INSTRUMENT_LABELS: [&str; 7] = ["BD", "SD", "HH", "OH", "T1", "T2", "T3"];

pub fn create_editor(
    params: Arc<DrumFlashParams>,
    current_step: Arc<AtomicU32>,
    pattern: Arc<SharedPattern>,
    voice_test_triggers: Arc<[AtomicBool; 7]>,
    sound_settings_state: Arc<SoundSettingsState>,
) -> Option<Box<dyn Editor>> {
    let params_for_ui = params.clone();
    let editor_state = params.editor_state.clone();
    let pattern_for_ui = pattern.clone();
    let voice_test_triggers_for_ui = voice_test_triggers.clone();
    let sound_settings_for_ui = sound_settings_state.clone();

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

                            ui.label("Sorties: Main Mix + Kick, Snare, Hi-Hat, Open HH, Tom 1, Tom 2, Tom 3");

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
                                ui.label("Type");
                                ui.add(widgets::ParamSlider::for_param(&params_for_ui.generator_type, setter).with_width(100.0));
                                ui.label("Style A");
                                ui.add(widgets::ParamSlider::for_param(&params_for_ui.style_primary, setter).with_width(80.0));
                                ui.label("Style B");
                                ui.add(widgets::ParamSlider::for_param(&params_for_ui.style_secondary, setter).with_width(80.0));
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
                                            let is_current = current_step.load(Ordering::Relaxed)
                                                as usize
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
                                draw_sound_panel(ui, &sound_settings_for_ui, selected_instrument);
                            }

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
    let bit = 1u8 << instrument;
    let next_mask = current_mask ^ bit;
    pattern_for_ui.set_step_mask(step, next_mask);
}

struct MixerRow<'a> {
    mute: &'a BoolParam,
    solo: &'a BoolParam,
}

fn mixer_rows(params: &DrumFlashParams) -> [MixerRow<'_>; 7] {
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
    ]
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
    let (mut freq, mut decay, mut vol, mut filt) = inst.load();
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
            .add(egui::Slider::new(&mut decay, 0.01..=1.0))
            .changed()
        {
            inst.decay.store(decay.to_bits(), Ordering::Relaxed);
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

    if changed {
        sound_settings.bump_version();
    }
}
