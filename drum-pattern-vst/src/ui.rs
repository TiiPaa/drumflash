use nih_plug::prelude::*;
use nih_plug_egui::{
    create_egui_editor,
    egui::{self, Color32, RichText, ScrollArea, Vec2},
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
    plock::PlockState,
    sequencer::{Pattern, SharedPattern},
    sound_settings::SoundSettingsState,
    synthesis::{self, DrumVoice, VoiceSettings},
    DrumFlashParams, BUILD_ID,
};

// Instrument labels and names are sourced from instrument_registry::INSTRUMENTS

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct EditorUIState {
    selected_instrument: usize,
    selected_pattern_slot: usize,
    last_midi_export_path: Option<String>,
    last_midi_export_error: Option<String>,
}

pub fn create_editor(
    params: Arc<DrumFlashParams>,
    current_step: Arc<AtomicU32>,
    current_steps: Arc<[AtomicU32; DrumVoice::COUNT]>,
    pattern: Arc<SharedPattern>,
    voice_test_triggers: Arc<[AtomicBool; DrumVoice::COUNT]>,
    sound_settings_state: Arc<SoundSettingsState>,
    plock_state: Arc<PlockState>,
) -> Option<Box<dyn Editor>> {
    let params_for_ui = params.clone();
    let editor_state = params.editor_state.clone();
    let pattern_for_ui = pattern.clone();
    let voice_test_triggers_for_ui = voice_test_triggers.clone();
    let sound_settings_for_ui = sound_settings_state.clone();
    let current_steps_for_ui = current_steps.clone();
    let plock_for_ui = plock_state.clone();

    create_egui_editor(
        params.editor_state.clone(),
        EditorUIState::default(),
        |_egui_ctx, _state| {},
        move |egui_ctx, setter, state| {
            ResizableWindow::new("drum-pattern-generator")
                .min_size(Vec2::new(1200.0, 720.0))
                .show(egui_ctx, editor_state.as_ref(), |ui| {
                    ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.heading("Drum Flash");
                            ui.label(format!(
                                "v{} — build {}",
                                env!("CARGO_PKG_VERSION"),
                                BUILD_ID
                            ));
                            ui.separator();

                            draw_top_bar(ui, setter, &params_for_ui);
                            draw_song_bar(ui, state);
                            draw_preset_bar(
                                ui,
                                &pattern_for_ui,
                                &params_for_ui,
                                setter,
                                state,
                            );
                            draw_generator_bar(ui, setter, &params_for_ui, &pattern_for_ui);

                            ui.separator();
                            draw_grid(
                                ui,
                                setter,
                                &params_for_ui,
                                &pattern_for_ui,
                                &voice_test_triggers_for_ui,
                                &current_step,
                                &current_steps_for_ui,
                                &sound_settings_for_ui,
                                &plock_for_ui,
                                state,
                            );

                            ui.separator();
                            draw_sound_panel(
                                ui,
                                &sound_settings_for_ui,
                                &params_for_ui,
                                setter,
                                state,
                            );

                            ui.separator();
                            ui.label("La grille édite le pattern joué en temps réel.");
                        });
                });
        },
    )
}

// ─────────────────────────────────────
// Top bar: Master Volume / Swing / Groove / Choke
// ─────────────────────────────────────
fn draw_top_bar(ui: &mut egui::Ui, setter: &ParamSetter, params: &DrumFlashParams) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Vol").strong());
        ui.add(widgets::ParamSlider::for_param(&params.master_volume, setter).with_width(80.0));

        ui.add_space(16.0);
        ui.label(RichText::new("Swing").strong());
        ui.add(widgets::ParamSlider::for_param(&params.swing, setter).with_width(80.0));

        ui.add_space(8.0);
        enum_combo(ui, setter, &params.groove_type, "Groove");

        ui.add_space(16.0);
        bool_checkbox(ui, setter, &params.hihat_chokes_oh, "Choke");

        ui.add_space(16.0);
        bool_checkbox(ui, setter, &params.auto_edit, "Auto Edit");
    });
}

// ─────────────────────────────────────
// Song mode placeholder: P1..P8 slots
// ─────────────────────────────────────
fn draw_song_bar(ui: &mut egui::Ui, state: &mut EditorUIState) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Song").strong());
        ui.add_space(4.0);
        if ui.button("◀").clicked() && state.selected_pattern_slot > 0 {
            state.selected_pattern_slot -= 1;
        }
        for i in 0..8 {
            let selected = state.selected_pattern_slot == i;
            let text = format!("P{}", i + 1);
            let btn = egui::Button::new(RichText::new(text).size(11.0))
                .min_size(Vec2::new(28.0, 22.0))
                .fill(if selected {
                    Color32::from_rgb(56, 132, 255)
                } else {
                    Color32::from_rgb(36, 36, 36)
                });
            if ui.add(btn).clicked() {
                state.selected_pattern_slot = i;
            }
        }
        if ui.button("▶").clicked() && state.selected_pattern_slot < 7 {
            state.selected_pattern_slot += 1;
        }
    });
}

// ─────────────────────────────────────
// Presets / Random / Export MIDI
// ─────────────────────────────────────
fn draw_preset_bar(
    ui: &mut egui::Ui,
    pattern: &SharedPattern,
    params: &DrumFlashParams,
    _setter: &ParamSetter,
    state: &mut EditorUIState,
) {
    ui.horizontal(|ui| {
        // Presets (left)
        ui.label(RichText::new("Presets").strong().size(11.0));
        if ui.button("Rock").clicked() {
            load_pattern_for_ui(pattern, &Pattern::rock_pattern());
        }
        if ui.button("Funk").clicked() {
            load_pattern_for_ui(pattern, &Pattern::funk_pattern());
        }
        if ui.button("Disco").clicked() {
            load_pattern_for_ui(pattern, &Pattern::disco_pattern());
        }
        if ui.button("Clear").clicked() {
            load_pattern_for_ui(pattern, &Pattern::empty());
        }

        ui.add_space(16.0);

        // Random (middle)
        if ui.button("🎲 Random").clicked() {
            load_pattern_for_ui(pattern, &Pattern::random_pattern());
        }

        ui.add_space(16.0);

        // Export MIDI (right)
        ui.label(RichText::new("Export").strong().size(11.0));
        let export_btn = egui::Button::new("MIDI");
        let response = ui.add(export_btn);
        if response.clicked() {
            let bpm = params.bpm.value();
            match export_midi_to_documents(pattern, bpm) {
                Ok(path) => {
                    nih_log!("MIDI exported to: {}", path.display());
                    state.last_midi_export_path = Some(path.display().to_string());
                    state.last_midi_export_error = None;
                }
                Err(e) => {
                    nih_log!("MIDI export failed: {}", e);
                    state.last_midi_export_path = None;
                    state.last_midi_export_error = Some(e.to_string());
                }
            }
        }
        response.on_hover_text("Export MIDI file to Documents/Drum Flash/exports");

        if let Some(path) = &state.last_midi_export_path {
            if ui.button("Copy Path").clicked() {
                ui.ctx().copy_text(path.clone());
            }
            ui.label(RichText::new("Exported").size(10.0));
        } else if state.last_midi_export_error.is_some() {
            ui.label(RichText::new("Export failed").size(10.0).color(Color32::RED));
        }
    });
}

// ─────────────────────────────────────
// Generator parameters
// ─────────────────────────────────────
fn draw_generator_bar(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    pattern: &SharedPattern,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Generator").strong().size(11.0));
        enum_combo(ui, setter, &params.generator_type, "Type");
        enum_combo(ui, setter, &params.style_primary, "A");
        enum_combo(ui, setter, &params.style_secondary, "B");
        ui.label("Mix");
        ui.add(widgets::ParamSlider::for_param(&params.style_mix, setter).with_width(50.0));
        ui.label("Dens");
        ui.add(widgets::ParamSlider::for_param(&params.gen_density, setter).with_width(50.0));
        ui.label("Var");
        ui.add(widgets::ParamSlider::for_param(&params.gen_variation, setter).with_width(50.0));

        let gen_btn = egui::Button::new(RichText::new(" GENERATE ").strong().size(13.0))
            .fill(Color32::from_rgb(56, 132, 255));
        if ui.add(gen_btn).clicked() {
            let gen_params = generator::GeneratorParams {
                generator_type: params.generator_type.value(),
                style_primary: params.style_primary.value(),
                style_secondary: params.style_secondary.value(),
                style_mix: params.style_mix.value(),
                density: params.gen_density.value(),
                variation: params.gen_variation.value(),
            };
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos() as u64)
                .unwrap_or(0);
            let mut rng_state = seed;
            let mut rng = || {
                rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                (rng_state as f32) / (u64::MAX as f32)
            };
            let generated = generator::generate(&gen_params, &mut rng);
            load_pattern_for_ui(pattern, &generated);
        }
    });
}

// ─────────────────────────────────────
// Pattern grid with per-row Hum/Push/Len
// ─────────────────────────────────────
fn draw_grid(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    pattern: &SharedPattern,
    voice_test_triggers: &[AtomicBool; DrumVoice::COUNT],
    current_step: &AtomicU32,
    current_steps: &[AtomicU32; DrumVoice::COUNT],
    sound_settings: &SoundSettingsState,
    plock: &PlockState,
    state: &mut EditorUIState,
) {
    let mixer_rows = mixer_rows(params);
    let hums: [&FloatParam; DrumVoice::COUNT] = std::array::from_fn(|i| params.humanizes()[i]);
    let pushes: [&FloatParam; DrumVoice::COUNT] = std::array::from_fn(|i| params.pushes()[i]);
    let lengths: [&IntParam; DrumVoice::COUNT] = std::array::from_fn(|i| params.lengths()[i]);

    egui::Grid::new("pattern-grid")
        .spacing(Vec2::new(4.0, 4.0))
        .show(ui, |ui| {
            // Header row
            ui.label("");
            ui.label(RichText::new("M").strong().size(10.0));
            ui.label(RichText::new("S").strong().size(10.0));
            ui.label(RichText::new("T").strong().size(10.0));
            for step in 0..16 {
                let is_current = (current_step.load(Ordering::Relaxed) as usize) == step;
                let text = if step % 4 == 0 {
                    RichText::new(format!("{}", step + 1)).strong().size(10.0)
                } else {
                    RichText::new(format!("{}", step + 1)).size(10.0)
                };
                let label = if is_current {
                    RichText::new(text.text()).strong().color(Color32::YELLOW).size(10.0)
                } else {
                    text
                };
                ui.label(label);
            }
            ui.label(RichText::new("Hum").strong().size(10.0));
            ui.label(RichText::new("Push").strong().size(10.0));
            ui.label(RichText::new("Len").strong().size(10.0));
            ui.end_row();

            // Instrument rows
            for inst in 0..DrumVoice::COUNT {
                let row = &mixer_rows[inst];
                let track_len = lengths[inst].value() as usize;

                // Instrument label (clickable)
                let label_btn = egui::Button::new(
                    RichText::new(crate::instrument_registry::INSTRUMENTS[inst].label).monospace().size(11.0),
                )
                .min_size(Vec2::new(28.0, 22.0))
                .fill(if state.selected_instrument == inst {
                    Color32::from_rgb(56, 132, 255)
                } else {
                    Color32::from_rgb(28, 28, 28)
                });
                if ui.add(label_btn).clicked() {
                    state.selected_instrument = inst;
                }

                // Mute / Solo / Test
                draw_bool_toggle(ui, setter, row.mute, "M", "Mute");
                draw_bool_toggle(ui, setter, row.solo, "S", "Solo");
                if ui.button("T").on_hover_text("Test").clicked() {
                    voice_test_triggers[inst].store(true, Ordering::Relaxed);
                }

                // 16 steps
                for step in 0..16 {
                    let active = pattern.is_active(step, inst);
                    let is_current = current_steps[inst].load(Ordering::Relaxed) as usize == step;
                    let beyond_len = step >= track_len;
                    let has_plock = plock.masks.is_active(inst, step);

                    let bg = if active && has_plock {
                        Color32::from_rgb(255, 140, 0) // orange for plock+active
                    } else if active {
                        Color32::from_rgb(56, 132, 255)
                    } else if has_plock {
                        Color32::from_rgb(180, 100, 0) // darker orange for plock only
                    } else if is_current {
                        Color32::from_rgb(48, 48, 48)
                    } else {
                        Color32::from_rgb(28, 28, 28)
                    };

                    let block_color = if step < 4 {
                        Color32::from_rgb(32, 32, 32)
                    } else if step < 8 {
                        Color32::from_rgb(40, 40, 40)
                    } else if step < 12 {
                        Color32::from_rgb(32, 32, 32)
                    } else {
                        Color32::from_rgb(40, 40, 40)
                    };

                    let btn = egui::Button::new(if active { "X" } else { "." })
                        .min_size(Vec2::new(20.0, 20.0))
                        .fill(if active || is_current || has_plock { bg } else { block_color })
                        .stroke(if beyond_len && !active && !has_plock {
                            egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 60))
                        } else {
                            egui::Stroke::NONE
                        });

                    let response = ui.add(btn);
                    if response.clicked() {
                        toggle_step_for_ui(pattern, step, inst);
                        if params.auto_edit.value() {
                            state.selected_instrument = inst;
                        }
                    }
                    response.context_menu(|ui| {
                        draw_plock_menu(ui, plock, sound_settings, params, inst, step, state);
                    });
                }

                // Hum / Push / Len (compact sliders)
                ui.add(widgets::ParamSlider::for_param(hums[inst], setter).with_width(40.0));
                ui.add(widgets::ParamSlider::for_param(pushes[inst], setter).with_width(45.0));
                ui.add(widgets::ParamSlider::for_param(lengths[inst], setter).with_width(35.0));

                ui.end_row();
            }
        });
}


// ─────────────────────────────────────
// Sound Panel (always visible, tabbed by instrument)
// ─────────────────────────────────────
fn draw_sound_panel(
    ui: &mut egui::Ui,
    sound_settings: &SoundSettingsState,
    params: &DrumFlashParams,
    setter: &ParamSetter,
    state: &mut EditorUIState,
) {
    ui.label(RichText::new("Sound Editor").strong());
    state.selected_instrument = state.selected_instrument.min(DrumVoice::COUNT - 1);

    // Instrument tabs
    ui.horizontal(|ui| {
        for (i, label) in crate::instrument_registry::INSTRUMENTS.iter().map(|d| d.label).enumerate() {
            let selected = state.selected_instrument == i;
            let btn = egui::Button::new(
                RichText::new(label).monospace().size(11.0),
            )
            .min_size(Vec2::new(32.0, 22.0))
            .fill(if selected {
                Color32::from_rgb(56, 132, 255)
            } else {
                Color32::from_rgb(36, 36, 36)
            });
            if ui.add(btn).clicked() {
                state.selected_instrument = i;
            }
        }
    });

    let inst = &sound_settings.instruments[state.selected_instrument];
    let (
        mut freq,
        mut decay,
        mut vol,
        mut filt,
        mut release,
        mut decay_curve,
        mut release_curve,
        mut hold,
        mut filter_env_amount,
        mut filter_env_decay,
        mut analog,
        mut stereo,
    ) = inst.load();
    let mut changed = false;
    let caps = crate::instrument_registry::capabilities(state.selected_instrument);

    // Two-column layout for sound parameters
    ui.columns(2, |cols| {
        cols[0].vertical(|ui| {
            if caps.freq {
                ui.horizontal(|ui| {
                    ui.label("Frequency");
                    if ui.add(egui::Slider::new(&mut freq, 20.0..=12000.0).logarithmic(true)).changed() {
                        inst.frequency.store(freq.to_bits(), Ordering::Relaxed);
                        changed = true;
                    }
                });
            }
            ui.horizontal(|ui| {
                ui.label("Decay");
                if ui.add(egui::Slider::new(&mut decay, 0.01..=0.5)).changed() {
                    inst.decay.store(decay.to_bits(), Ordering::Relaxed);
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Decay Curve");
                if ui.add(egui::Slider::new(&mut decay_curve, 2.0..=10.0)).changed() {
                    inst.decay_curve.store(decay_curve.to_bits(), Ordering::Relaxed);
                    changed = true;
                }
            });
            if caps.hold {
                ui.horizontal(|ui| {
                    ui.label("Hold");
                    if ui.add(egui::Slider::new(&mut hold, 0.0..=0.5).suffix(" s")).changed() {
                        inst.hold.store(hold.to_bits(), Ordering::Relaxed);
                        changed = true;
                    }
                });
            }
            ui.horizontal(|ui| {
                ui.label("Release");
                if ui.add(egui::Slider::new(&mut release, 0.0..=5.0)).changed() {
                    inst.release.store(release.to_bits(), Ordering::Relaxed);
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Release Curve");
                if ui.add(egui::Slider::new(&mut release_curve, 2.0..=10.0)).changed() {
                    inst.release_curve.store(release_curve.to_bits(), Ordering::Relaxed);
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Volume");
                if ui.add(egui::Slider::new(&mut vol, 0.0..=1.5)).changed() {
                    inst.volume.store(vol.to_bits(), Ordering::Relaxed);
                    changed = true;
                }
            });
        });

        cols[1].vertical(|ui| {
            let filter_type_label = crate::instrument_registry::filter_type_label(state.selected_instrument);
            ui.horizontal(|ui| {
                ui.label(format!("Filter ({filter_type_label})"));
                if ui.add(egui::Slider::new(&mut filt, 20.0..=15000.0).logarithmic(true)).changed() {
                    inst.filter_freq.store(filt.to_bits(), Ordering::Relaxed);
                    changed = true;
                }
            });
            if caps.filter_env {
                ui.horizontal(|ui| {
                    ui.label("Filter Env");
                    if ui.add(egui::Slider::new(&mut filter_env_amount, 0.0..=1.0)).changed() {
                        inst.filter_env_amount.store(filter_env_amount.to_bits(), Ordering::Relaxed);
                        changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Filter Decay");
                    if ui.add(egui::Slider::new(&mut filter_env_decay, 0.001..=0.2).suffix(" s")).changed() {
                        inst.filter_env_decay.store(filter_env_decay.to_bits(), Ordering::Relaxed);
                        changed = true;
                    }
                });
            }
            if caps.analog {
                ui.horizontal(|ui| {
                    ui.label("Analog");
                    if ui.add(egui::Slider::new(&mut analog, 0.0..=1.0)).changed() {
                        inst.analog.store(analog.to_bits(), Ordering::Relaxed);
                        changed = true;
                    }
                });
            }
            if caps.stereo {
                ui.horizontal(|ui| {
                    ui.label("Stereo");
                    if ui.add(egui::Checkbox::new(&mut (stereo >= 0.5), "")).changed() {
                        stereo = if stereo >= 0.5 { 0.0 } else { 1.0 };
                        inst.stereo.store(stereo.to_bits(), Ordering::Relaxed);
                        changed = true;
                    }
                });
            }

            let mix_param = params.mixes()[state.selected_instrument];
            ui.horizontal(|ui| {
                ui.label("Mix");
                let mut mix = mix_param.value();
                if ui.add(egui::Checkbox::new(&mut mix, "")).changed() {
                    setter.set_parameter(mix_param.into(), mix);
                }
            });

            // Algorithm selector
            if let Some(voice) = DrumVoice::from_index(state.selected_instrument) {
                let algos = synthesis::algos_for(voice);
                if algos.len() > 1 && state.selected_instrument != 3 {
                    let algo_param = params.algos()[state.selected_instrument];
                    ui.horizontal(|ui| {
                        ui.label("Algorithm");
                        let algo_names: Vec<&str> = algos.iter().map(|a| a.name).collect();
                        algo_combo(ui, setter, algo_param, &algo_names);
                    });
                }
            }
        });
    });

    // Per-instrument special parameters (data-driven from registry)
    ui.horizontal(|ui| {
        let special_defs = crate::instrument_registry::special_params(state.selected_instrument);
        for def in special_defs {
            if let Some(param) = params.special_param(state.selected_instrument, def.special_index) {
                ui.label(def.label);
                ui.add(widgets::ParamSlider::for_param(param, setter).with_width(120.0));
            }
        }
    });

    if changed {
        sound_settings.bump_version();
    }
}

// ─────────────────────────────────────
// Helpers
// ─────────────────────────────────────
fn plock_special_field(special_index: usize) -> Option<usize> {
    let field = crate::plock::SPECIAL_FIELD_START.checked_add(special_index)?;
    (field < crate::plock::FIELD_COUNT).then_some(field)
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
    std::array::from_fn(|i| MixerRow {
        mute: params.mutes()[i],
        solo: params.solos()[i],
    })
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
        .min_size(Vec2::new(24.0, 20.0))
        .fill(if enabled {
            Color32::from_rgb(56, 132, 255)
        } else {
            Color32::from_rgb(36, 36, 36)
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


// ─────────────────────────────────────
// Plock context menu
// ─────────────────────────────────────
fn draw_plock_menu(
    ui: &mut egui::Ui,
    plock: &PlockState,
    sound_settings: &SoundSettingsState,
    params: &DrumFlashParams,
    instrument: usize,
    step: usize,
    _state: &mut EditorUIState,
) {
    ui.label(
        RichText::new(format!(
            "Plock {} — Step {}",
            crate::instrument_registry::INSTRUMENTS[instrument].label,
            step + 1
        ))
        .strong(),
    );
    ui.separator();

    let inst = &sound_settings.instruments[instrument];
    let global = inst.load();

    let has_plock = plock.masks.is_active(instrument, step);

    // If no plock exists yet, seed from global settings
    if !has_plock {
        if ui.button("➕ Create plock from current settings").clicked() {
            let mut special = [0.0f32; 8];
            for def in crate::instrument_registry::special_params(instrument) {
                if def.special_index < special.len() {
                    special[def.special_index] = params
                        .special_param(instrument, def.special_index)
                        .map(|param| param.value())
                        .unwrap_or(def.default);
                }
            }
            let algo = params.algos()[instrument].value() as u8;
            let settings = VoiceSettings {
                frequency: global.0,
                decay: global.1,
                volume: global.2,
                filter_freq: global.3,
                release: global.4,
                decay_curve: global.5,
                release_curve: global.6,
                hold: global.7,
                filter_env_amount: global.8,
                filter_env_decay: global.9,
                analog: global.10,
                stereo: global.11,
                algo,
                special,
            };
            plock.set_settings(instrument, step, &settings);
        }
    } else {
        let mut changed = false;

        let mut freq = plock.values.get(instrument, step, 0);
        let mut decay = plock.values.get(instrument, step, 1);
        let mut vol = plock.values.get(instrument, step, 2);
        let mut filt = plock.values.get(instrument, step, 3);
        let mut release = plock.values.get(instrument, step, 4);
        let mut decay_curve = plock.values.get(instrument, step, 5);
        let mut release_curve = plock.values.get(instrument, step, 6);
        let mut hold = plock.values.get(instrument, step, 7);
        let mut filter_env_amount = plock.values.get(instrument, step, 8);
        let mut filter_env_decay = plock.values.get(instrument, step, 9);
        let mut analog = plock.values.get(instrument, step, 10);
        let mut stereo = plock.values.get(instrument, step, 11);
        let algo = plock.values.get(instrument, step, 13) as u8;
        let special_defs = crate::instrument_registry::special_params(instrument);
        let mut special = [0.0f32; 8];
        for def in special_defs {
            if def.special_index < special.len() {
                special[def.special_index] = plock_special_field(def.special_index)
                    .map(|field| plock.values.get(instrument, step, field))
                    .unwrap_or(def.default);
            }
        }

        ui.horizontal(|ui| {
            ui.label("Freq");
            if ui.add(egui::Slider::new(&mut freq, 20.0..=12000.0).logarithmic(true)).changed() {
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Decay");
            if ui.add(egui::Slider::new(&mut decay, 0.01..=0.5)).changed() {
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Vol");
            if ui.add(egui::Slider::new(&mut vol, 0.0..=1.5)).changed() {
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Filter");
            if ui.add(egui::Slider::new(&mut filt, 20.0..=15000.0).logarithmic(true)).changed() {
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Release");
            if ui.add(egui::Slider::new(&mut release, 0.0..=5.0)).changed() {
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("DecCurve");
            if ui.add(egui::Slider::new(&mut decay_curve, 2.0..=10.0)).changed() {
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("RelCurve");
            if ui.add(egui::Slider::new(&mut release_curve, 2.0..=10.0)).changed() {
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Hold");
            if ui.add(egui::Slider::new(&mut hold, 0.0..=0.5).suffix(" s")).changed() {
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("FiltEnv");
            if ui.add(egui::Slider::new(&mut filter_env_amount, 0.0..=1.0)).changed() {
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("FiltDec");
            if ui.add(egui::Slider::new(&mut filter_env_decay, 0.001..=0.2).suffix(" s")).changed() {
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Analog");
            if ui.add(egui::Slider::new(&mut analog, 0.0..=1.0)).changed() {
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Stereo");
            if ui.add(egui::Checkbox::new(&mut (stereo >= 0.5), "")).changed() {
                stereo = if stereo >= 0.5 { 0.0 } else { 1.0 };
                changed = true;
            }
        });

        for def in special_defs {
            if def.special_index >= special.len() {
                continue;
            }
            let mut value = special[def.special_index];
            let changed_special = ui.horizontal(|ui| {
                let mut slider = egui::Slider::new(&mut value, def.min..=def.max);
                if def.min > 0.0 && def.max / def.min >= 20.0 {
                    slider = slider.logarithmic(true);
                }
                ui.label(def.label);
                ui.add(slider).changed()
            }).inner;
            if changed_special {
                special[def.special_index] = value;
                changed = true;
            }
        }

        if changed {
            let settings = VoiceSettings {
                frequency: freq,
                decay,
                volume: vol,
                filter_freq: filt,
                release,
                decay_curve,
                release_curve,
                hold,
                filter_env_amount,
                filter_env_decay,
                analog,
                stereo,
                algo,
                special,
            };
            plock.set_settings(instrument, step, &settings);
        }

        ui.separator();
        if ui.button("🗑 Clear plock").clicked() {
            plock.clear(instrument, step);
        }
    }
}
