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
    plock::{PlockState, STEP_COUNT as PLOCK_STEP_COUNT},
    sequencer::{Pattern, SharedPattern},
    sound_settings::SoundSettingsState,
    synthesis::{self, DrumVoice, VoiceSettings},
    DrumFlashParams, BUILD_ID,
};

const INSTRUMENT_LABELS: [&str; DrumVoice::COUNT] =
    ["BD", "SD", "HH", "OH", "T1", "T2", "T3", "CL", "RD", "CY", "S6", "B8"];

const INSTRUMENT_FULL_NAMES: [&str; DrumVoice::COUNT] = [
    "Kick", "Snare", "Hi-Hat", "Open HH", "Tom 1", "Tom 2", "Tom 3", "Clap", "Ride", "Cymbal",
    "Snare 606", "808 Bass Drum",
];

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct EditorUIState {
    selected_instrument: usize,
    selected_pattern_slot: usize,
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
    setter: &ParamSetter,
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

        // Export MIDI (right) with drag-and-drop support
        ui.label(RichText::new("Export").strong().size(11.0));
        let export_btn = egui::Button::new("📄 MIDI");
        let response = ui.add(export_btn);
        if response.clicked() {
            let bpm = params.bpm.value();
            match export_midi_to_documents(pattern, bpm) {
                Ok(path) => {
                    nih_log!("MIDI exported to: {}", path.display());
                }
                Err(e) => {
                    nih_log!("MIDI export failed: {}", e);
                }
            }
        }
        // Drag and drop payload
        if response.drag_started() {
            let bpm = params.bpm.value();
            if let Ok(bytes) = midi_export::export_pattern_to_midi_bytes(pattern, bpm) {
                response.dnd_set_drag_payload(bytes);
            }
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
                .unwrap()
                .as_nanos() as u64;
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
    let hums = [
        &params.humanize_kick, &params.humanize_snare, &params.humanize_hihat,
        &params.humanize_open_hh, &params.humanize_tom1, &params.humanize_tom2,
        &params.humanize_tom3, &params.humanize_clap, &params.humanize_ride,
        &params.humanize_cymbal, &params.humanize_snare606, &params.humanize_bassdrum808,
    ];
    let pushes = [
        &params.push_kick, &params.push_snare, &params.push_hihat,
        &params.push_open_hh, &params.push_tom1, &params.push_tom2,
        &params.push_tom3, &params.push_clap, &params.push_ride,
        &params.push_cymbal, &params.push_snare606, &params.push_bassdrum808,
    ];
    let lengths = [
        &params.length_kick, &params.length_snare, &params.length_hihat,
        &params.length_open_hh, &params.length_tom1, &params.length_tom2,
        &params.length_tom3, &params.length_clap, &params.length_ride,
        &params.length_cymbal, &params.length_snare606, &params.length_bassdrum808,
    ];

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
                    RichText::new(INSTRUMENT_LABELS[inst]).monospace().size(11.0),
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

    // Instrument tabs
    ui.horizontal(|ui| {
        for (i, label) in INSTRUMENT_LABELS.iter().enumerate() {
            let selected = state.selected_instrument == i;
            let btn = egui::Button::new(
                RichText::new(*label).monospace().size(11.0),
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

    // Two-column layout for sound parameters
    ui.columns(2, |cols| {
        cols[0].vertical(|ui| {
            let freq_capable = !matches!(state.selected_instrument, 7 | 9);
            if freq_capable {
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
            let hold_capable = matches!(state.selected_instrument, 1 | 2 | 3 | 10);
            if hold_capable {
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
            let filter_type_label = match state.selected_instrument {
                0 => "LP",
                1 => "HP",
                2 => "HP",
                3 => "HP",
                4 => "LP",
                5 => "LP",
                6 => "LP",
                7 => "HP",
                8 => "HP",
                9 => "HP",
                10 => "LP",
                _ => "",
            };
            ui.horizontal(|ui| {
                ui.label(format!("Filter ({filter_type_label})"));
                if ui.add(egui::Slider::new(&mut filt, 20.0..=15000.0).logarithmic(true)).changed() {
                    inst.filter_freq.store(filt.to_bits(), Ordering::Relaxed);
                    changed = true;
                }
            });
            let filter_env_capable = matches!(state.selected_instrument, 0 | 1 | 2 | 4 | 5 | 6 | 10);
            if filter_env_capable {
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
            let analog_capable = matches!(state.selected_instrument, 0 | 1 | 4 | 5 | 6 | 10 | 11);
            if analog_capable {
                ui.horizontal(|ui| {
                    ui.label("Analog");
                    if ui.add(egui::Slider::new(&mut analog, 0.0..=1.0)).changed() {
                        inst.analog.store(analog.to_bits(), Ordering::Relaxed);
                        changed = true;
                    }
                });
            }
            let stereo_capable = matches!(state.selected_instrument, 1 | 2 | 3 | 7 | 8 | 9 | 10);
            if stereo_capable {
                ui.horizontal(|ui| {
                    ui.label("Stereo");
                    if ui.add(egui::Checkbox::new(&mut (stereo >= 0.5), "")).changed() {
                        stereo = if stereo >= 0.5 { 0.0 } else { 1.0 };
                        inst.stereo.store(stereo.to_bits(), Ordering::Relaxed);
                        changed = true;
                    }
                });
            }

            let mix_param = match state.selected_instrument {
                0 => &params.mix_kick,
                1 => &params.mix_snare,
                2 => &params.mix_hihat,
                3 => &params.mix_open_hh,
                4 => &params.mix_tom1,
                5 => &params.mix_tom2,
                6 => &params.mix_tom3,
                7 => &params.mix_clap,
                8 => &params.mix_ride,
                9 => &params.mix_cymbal,
                10 => &params.mix_snare606,
                11 => &params.mix_bassdrum808,
                _ => &params.mix_kick,
            };
            ui.horizontal(|ui| {
                ui.label("Mix");
                let mut mix = mix_param.value();
                if ui.add(egui::Checkbox::new(&mut mix, "")).changed() {
                    setter.set_parameter(mix_param.into(), mix);
                }
            });

            // Algorithm selector
            let voice = DrumVoice::from_index(state.selected_instrument).unwrap();
            let algos = synthesis::algos_for(voice);
            if algos.len() > 1 && state.selected_instrument != 3 {
                let algo_param = match state.selected_instrument {
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
                    11 => &params.algo_bassdrum808,
                    _ => &params.algo_kick,
                };
                ui.horizontal(|ui| {
                    ui.label("Algorithm");
                    let algo_names: Vec<&str> = algos.iter().map(|a| a.name).collect();
                    algo_combo(ui, setter, algo_param, &algo_names);
                });
            }
        });
    });

    // Per-instrument special parameters (moved from bottom bar)
    ui.horizontal(|ui| {
        if state.selected_instrument == 0 {
            ui.label("Click Level");
            ui.add(widgets::ParamSlider::for_param(&params.kick_click, setter).with_width(120.0));
        }
        if state.selected_instrument == 1 {
            ui.label("Snap");
            ui.add(widgets::ParamSlider::for_param(&params.snare_snap, setter).with_width(120.0));
        }
        if matches!(state.selected_instrument, 4 | 5 | 6) {
            ui.label("Stick Attack");
            ui.add(widgets::ParamSlider::for_param(&params.tom_stick, setter).with_width(120.0));
        }
        if state.selected_instrument == 7 {
            ui.label("Echo");
            ui.add(widgets::ParamSlider::for_param(&params.clap_echo, setter).with_width(120.0));
        }
        if state.selected_instrument == 10 {
            ui.label("Resonance");
            ui.add(widgets::ParamSlider::for_param(&params.snare606_resonance, setter).with_width(120.0));
            ui.label("Tone");
            ui.add(widgets::ParamSlider::for_param(&params.snare606_tone, setter).with_width(120.0));
            ui.label("Snap");
            ui.add(widgets::ParamSlider::for_param(&params.snare606_snap, setter).with_width(120.0));
        }
        if state.selected_instrument == 11 {
            ui.label("Accent");
            ui.add(widgets::ParamSlider::for_param(&params.bassdrum808_accent, setter).with_width(120.0));
            ui.label("Snap");
            ui.add(widgets::ParamSlider::for_param(&params.bassdrum808_snap, setter).with_width(120.0));
            ui.label("Pitch Drop");
            ui.add(widgets::ParamSlider::for_param(&params.bassdrum808_pitch_drop, setter).with_width(120.0));
            ui.label("Click Tone");
            ui.add(widgets::ParamSlider::for_param(&params.bassdrum808_click_tone, setter).with_width(120.0));
        }
    });

    if changed {
        sound_settings.bump_version();
    }
}

// ─────────────────────────────────────
// Helpers
// ─────────────────────────────────────
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
        MixerRow { mute: &params.mute_kick, solo: &params.solo_kick },
        MixerRow { mute: &params.mute_snare, solo: &params.solo_snare },
        MixerRow { mute: &params.mute_hihat, solo: &params.solo_hihat },
        MixerRow { mute: &params.mute_open_hh, solo: &params.solo_open_hh },
        MixerRow { mute: &params.mute_tom1, solo: &params.solo_tom1 },
        MixerRow { mute: &params.mute_tom2, solo: &params.solo_tom2 },
        MixerRow { mute: &params.mute_tom3, solo: &params.solo_tom3 },
        MixerRow { mute: &params.mute_clap, solo: &params.solo_clap },
        MixerRow { mute: &params.mute_ride, solo: &params.solo_ride },
        MixerRow { mute: &params.mute_cymbal, solo: &params.solo_cymbal },
        MixerRow { mute: &params.mute_snare606, solo: &params.solo_snare606 },
        MixerRow { mute: &params.mute_bassdrum808, solo: &params.solo_bassdrum808 },
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
            INSTRUMENT_LABELS[instrument],
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
            if instrument == 7 {
                special[0] = params.clap_echo.value();
            }
            if instrument == 10 {
                special[0] = params.snare606_resonance.value();
                special[1] = params.snare606_tone.value();
                special[2] = params.snare606_snap.value();
            }
            if instrument == 11 {
                special[0] = params.bassdrum808_accent.value();
                special[1] = params.bassdrum808_snap.value();
                special[2] = params.bassdrum808_pitch_drop.value();
                special[3] = params.bassdrum808_click_tone.value();
            }
            let algo = match instrument {
                0 => params.algo_kick.value() as u8,
                1 => params.algo_snare.value() as u8,
                2 => params.algo_hihat.value() as u8,
                3 => params.algo_open_hh.value() as u8,
                4 => params.algo_tom1.value() as u8,
                5 => params.algo_tom2.value() as u8,
                6 => params.algo_tom3.value() as u8,
                7 => params.algo_clap.value() as u8,
                8 => params.algo_ride.value() as u8,
                9 => params.algo_cymbal.value() as u8,
                10 => params.algo_snare606.value() as u8,
                11 => params.algo_bassdrum808.value() as u8,
                _ => 0,
            };
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
        let mut clap_echo = if instrument == 7 {
            plock.values.get(instrument, step, 12)
        } else {
            0.0
        };
        let mut b8_accent = if instrument == 11 {
            plock.values.get(instrument, step, 14)
        } else {
            0.0
        };
        let mut b8_snap = if instrument == 11 {
            plock.values.get(instrument, step, 15)
        } else {
            0.0
        };
        let mut b8_pitch_drop = if instrument == 11 {
            plock.values.get(instrument, step, 16)
        } else {
            0.0
        };
        let mut b8_click_tone = if instrument == 11 {
            plock.values.get(instrument, step, 17)
        } else {
            4000.0
        };
        let algo = plock.values.get(instrument, step, 13) as u8;

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

        if instrument == 7 {
            ui.horizontal(|ui| {
                ui.label("Echo");
                if ui.add(egui::Slider::new(&mut clap_echo, 0.0..=3.0)).changed() {
                    changed = true;
                }
            });
        }

        if instrument == 11 {
            ui.horizontal(|ui| {
                ui.label("Accent");
                if ui.add(egui::Slider::new(&mut b8_accent, 0.0..=2.0)).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Snap");
                if ui.add(egui::Slider::new(&mut b8_snap, 0.0..=2.0)).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Pitch Drop");
                if ui.add(egui::Slider::new(&mut b8_pitch_drop, 0.0..=2.0)).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Click Tone");
                if ui.add(egui::Slider::new(&mut b8_click_tone, 100.0..=8000.0).logarithmic(true)).changed() {
                    changed = true;
                }
            });
        }

        if changed {
            let mut special = [0.0f32; 8];
            if instrument == 7 {
                special[0] = clap_echo;
            }
            if instrument == 11 {
                special[0] = b8_accent;
                special[1] = b8_snap;
                special[2] = b8_pitch_drop;
                special[3] = b8_click_tone;
            }
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
