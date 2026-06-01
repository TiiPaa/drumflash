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
    process::Command,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
};

use crate::{
    preset_dumps,
    generator, midi_export,
    plock::PlockState,
    sequencer::{Pattern, SharedPattern},
    sound_settings::SoundSettingsState,
    synthesis::{self, DrumVoice, VoiceSettings},
    DrumFlashParams, BUILD_ID,
};

mod envelope_viz;
mod local_param_slider;
use envelope_viz::{draw_amp_envelope, draw_filter_envelope};
use local_param_slider::LocalParamSlider;

// ---------------------------------------------------------------------------------------------------------------
// Frequency / Note conversion utilities
// ---------------------------------------------------------------------------------------------------------------
fn freq_to_note(freq: f32) -> f32 {
    69.0 + 12.0 * (freq / 440.0).log2()
}

fn note_to_freq(note: f32) -> f32 {
    440.0 * 2.0f32.powf((note - 69.0) / 12.0)
}

fn note_name(note: f32) -> String {
    let note = note.round() as i32;
    let octave = (note / 12) - 1;
    let note_idx = note % 12;
    let names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    format!("{}{}", names[note_idx as usize], octave)
}

// Instrument labels and names are sourced from instrument_registry::INSTRUMENTS

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct EditorUIState {
    selected_instrument: usize,
    selected_pattern_slot: usize,
    last_midi_export_path: Option<String>,
    last_midi_export_error: Option<String>,
    dump_name_input: String,
    current_page: usize,     // 0-3 (displaying steps current_page*16 .. current_page*16+15)
    follow_mode: bool,       // if true, page follows the playhead
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
            #[cfg(target_os = "windows")]
            nih_plug_egui::set_keyboard_focus(egui_ctx.wants_keyboard_input());
            ResizableWindow::new("drum-pattern-generator")
                .min_size(Vec2::new(1200.0, 720.0))
                .show(egui_ctx, editor_state.as_ref(), |ui| {
                    ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.heading("Drum Flash");
                            ui.label(format!(
                                "v{} --- build {}",
                                env!("CARGO_PKG_VERSION"),
                                BUILD_ID
                            ));
                            ui.separator();

                            draw_top_bar(ui, setter, &params_for_ui);
                            draw_song_bar(ui, state);
                            draw_preset_bar(ui, &pattern_for_ui, &params_for_ui, setter, state);
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
                            ui.label("La grille edite le pattern joue en temps reel.");
                        });
                });
        },
    )
}

// ---------------------------------------------------------------------------------------------------------------
// Top bar: Master Volume / Swing / Groove / Choke
// ---------------------------------------------------------------------------------------------------------------
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

// ---------------------------------------------------------------------------------------------------------------
// Song mode placeholder: P1..P8 slots
// ---------------------------------------------------------------------------------------------------------------
fn draw_song_bar(ui: &mut egui::Ui, state: &mut EditorUIState) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Song").strong());
        ui.add_space(4.0);
        if ui.button("--").clicked() && state.selected_pattern_slot > 0 {
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
        if ui.button("-").clicked() && state.selected_pattern_slot < 7 {
            state.selected_pattern_slot += 1;
        }
    });
}

// ---------------------------------------------------------------------------------------------------------------
// Presets / Random / Export MIDI
// ---------------------------------------------------------------------------------------------------------------
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
        if ui.button("Random Random").clicked() {
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

        let drag_btn = egui::Button::new("Drag").sense(egui::Sense::click_and_drag());
        let drag_response = ui.add(drag_btn);
        if drag_response.clicked() || drag_response.drag_started() {
            let bpm = params.bpm.value();
            match export_midi_to_documents(pattern, bpm)
                .and_then(|path| start_external_midi_drag(&path).map(|_| path))
            {
                Ok(path) => {
                    nih_log!("MIDI drag helper started from: {}", path.display());
                    state.last_midi_export_path = Some(path.display().to_string());
                    state.last_midi_export_error = None;
                }
                Err(e) => {
                    nih_log!("MIDI drag failed: {}", e);
                    state.last_midi_export_path = None;
                    state.last_midi_export_error = Some(e.to_string());
                }
            }
        }
        drag_response.on_hover_text("Open external MIDI drag handle");

        if let Some(path) = &state.last_midi_export_path {
            if ui.button("Copy Path").clicked() {
                ui.ctx().copy_text(path.clone());
            }
            ui.label(RichText::new("Exported").size(10.0));
        } else if state.last_midi_export_error.is_some() {
            ui.label(
                RichText::new("Export failed")
                    .size(10.0)
                    .color(Color32::RED),
            );
        }
    });
}

// ---------------------------------------------------------------------------------------------------------------
// Generator parameters
// ---------------------------------------------------------------------------------------------------------------
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

// ---------------------------------------------------------------------------------------------------------------
// Pattern grid with per-row Hum/Push/Len
// ---------------------------------------------------------------------------------------------------------------
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

    // Page navigation + Follow toggle
    ui.horizontal(|ui| {
        ui.label(RichText::new("Page:").strong());
        for page in 0..4 {
            let is_active = state.current_page == page;
            let btn = egui::Button::new(format!("{}", page + 1))
                .min_size(Vec2::new(28.0, 22.0))
                .fill(if is_active {
                    Color32::from_rgb(56, 132, 255)
                } else {
                    Color32::from_rgb(40, 40, 40)
                });
            if ui.add(btn).clicked() {
                state.current_page = page;
            }
        }
        ui.add_space(16.0);
        let follow_btn = egui::Button::new(if state.follow_mode { "Follow ON" } else { "Follow OFF" })
            .min_size(Vec2::new(80.0, 22.0))
            .fill(if state.follow_mode {
                Color32::from_rgb(50, 150, 50)
            } else {
                Color32::from_rgb(80, 80, 80)
            });
        if ui.add(follow_btn).clicked() {
            state.follow_mode = !state.follow_mode;
        }
    });
    ui.add_space(4.0);

    // Follow mode: auto-switch page based on playhead
    if state.follow_mode {
        let master_step = current_step.load(Ordering::Relaxed) as usize;
        let target_page = master_step / 16;
        if target_page < 4 {
            state.current_page = target_page;
        }
    }

    let page_offset = state.current_page * 16;

    egui::Grid::new("pattern-grid")
        .spacing(Vec2::new(4.0, 4.0))
        .show(ui, |ui| {
            // Header row --- use exact same widths as instrument rows below
            let header_item = |ui: &mut egui::Ui, text: &str, width: f32| {
                ui.add_sized(
                    Vec2::new(width, 20.0),
                    egui::Label::new(RichText::new(text).strong().size(10.0)),
                );
            };
            header_item(ui, "", 32.0);        // instrument label
            header_item(ui, "Vol", 40.0);     // volume slider
            header_item(ui, "M", 24.0);       // mute
            header_item(ui, "S", 24.0);       // solo
            header_item(ui, "T", 24.0);       // test
            // Steps container with tighter spacing (showing steps of current page)
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for local_step in 0..16 {
                    let global_step = page_offset + local_step;
                    let is_current = (current_step.load(Ordering::Relaxed) as usize) == global_step;
                    let text = if local_step % 4 == 0 {
                        RichText::new(format!("{}", global_step + 1)).strong().size(10.0)
                    } else {
                        RichText::new(format!("{}", global_step + 1)).size(10.0)
                    };
                    let label = if is_current {
                        RichText::new(text.text())
                            .strong()
                            .color(Color32::YELLOW)
                            .size(10.0)
                    } else {
                        text
                    };
                    ui.add_sized(
                        Vec2::new(20.0, 20.0),
                        egui::Label::new(label),
                    );
                }
            });
            header_item(ui, "Hum", 40.0);
            header_item(ui, "Push", 45.0);
            header_item(ui, "Len", 35.0);
            ui.end_row();

            // Instrument rows
            for inst in 0..DrumVoice::COUNT {
                let row = &mixer_rows[inst];
                let track_len = lengths[inst].value() as usize;

                // Instrument label (clickable)
                let label_btn = egui::Button::new(
                    RichText::new(crate::instrument_registry::INSTRUMENTS[inst].label)
                        .monospace()
                        .size(11.0),
                )
                .min_size(Vec2::new(32.0, 22.0))
                .fill(if state.selected_instrument == inst {
                    Color32::from_rgb(56, 132, 255)
                } else {
                    Color32::from_rgb(28, 28, 28)
                });
                if ui.add(label_btn).clicked() {
                    state.selected_instrument = inst;
                }

                // Volume par lane
                let inst_state = &sound_settings.instruments[inst];
                let mut lane_vol = f32::from_bits(inst_state.volume.load(Ordering::Relaxed));
                let vol_slider = LocalParamSlider::new(&mut lane_vol, 0.0..=2.0)
                    .with_width(40.0)
                    .without_value();
                if ui.add(vol_slider).changed() {
                    inst_state.volume.store(lane_vol.to_bits(), Ordering::Relaxed);
                    sound_settings.bump_version();
                }

                // Mute / Solo / Test
                draw_bool_toggle(ui, setter, row.mute, "M", "Mute");
                let solo_clicked = draw_bool_toggle(ui, setter, row.solo, "S", "Solo");
                if solo_clicked && params.auto_edit.value() {
                    state.selected_instrument = inst;
                }
                let test_btn = egui::Button::new("T").min_size(Vec2::new(24.0, 20.0));
                if ui.add(test_btn).on_hover_text("Test").clicked() {
                    voice_test_triggers[inst].store(true, Ordering::Relaxed);
                }

                // 16 steps of current page (tight horizontal container)
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for local_step in 0..16 {
                    let global_step = page_offset + local_step;
                    let active = pattern.is_active(global_step, inst);
                    let is_current = current_steps[inst].load(Ordering::Relaxed) as usize == global_step;
                    let beyond_len = global_step >= track_len;
                    let has_plock = plock.masks.is_active(inst, global_step);

                    let plock_mask = if has_plock {
                        plock.field_masks.get(inst, global_step)
                    } else {
                        0
                    };
                    let all_bits = (1u64 << crate::plock::FIELD_COUNT) - 1;
                    let is_snapshot = has_plock && plock_mask == all_bits;

                    if beyond_len {
                        ui.allocate_space(Vec2::new(20.0, 20.0));
                    } else {
                        let bg = if active && has_plock {
                            if is_snapshot {
                                Color32::from_rgb(220, 50, 50) // rouge snapshot + active
                            } else {
                                Color32::from_rgb(255, 140, 0) // orange link/mixed + active
                            }
                        } else if active {
                            Color32::from_rgb(56, 132, 255)
                        } else if has_plock {
                            if is_snapshot {
                                Color32::from_rgb(160, 30, 30) // rouge fonce snapshot
                            } else {
                                Color32::from_rgb(180, 100, 0) // orange fonce link/mixed
                            }
                        } else if is_current {
                            Color32::from_rgb(48, 48, 48)
                        } else {
                            Color32::from_rgb(28, 28, 28)
                        };

                        let block_color = if local_step < 4 {
                            Color32::from_rgb(32, 32, 32)
                        } else if local_step < 8 {
                            Color32::from_rgb(40, 40, 40)
                        } else if local_step < 12 {
                            Color32::from_rgb(32, 32, 32)
                        } else {
                            Color32::from_rgb(40, 40, 40)
                        };

                        let btn = egui::Button::new(if active { "X" } else { "." })
                            .min_size(Vec2::new(20.0, 20.0))
                            .fill(if active || is_current || has_plock {
                                bg
                            } else {
                                block_color
                            })
                            .stroke(egui::Stroke::NONE);

                        let response = ui.add(btn);
                        if response.clicked() {
                            toggle_step_for_ui(pattern, global_step, inst);
                            if params.auto_edit.value() {
                                state.selected_instrument = inst;
                            }
                        }
                        response.context_menu(|ui| {
                            draw_plock_menu(ui, plock, sound_settings, params, setter, inst, global_step, state);
                        });
                    }
                }
                });

                // Hum / Push / Len (compact sliders)
                ui.add(widgets::ParamSlider::for_param(hums[inst], setter).with_width(40.0));
                ui.add(widgets::ParamSlider::for_param(pushes[inst], setter).with_width(45.0));
                ui.add(widgets::ParamSlider::for_param(lengths[inst], setter).with_width(35.0));

                ui.end_row();
            }
        });
}

// ---------------------------------------------------------------------------------------------------------------
// Sound Panel (always visible, tabbed by instrument)
// ---------------------------------------------------------------------------------------------------------------
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
        for (i, label) in crate::instrument_registry::INSTRUMENTS
            .iter()
            .map(|d| d.label)
            .enumerate()
        {
            let selected = state.selected_instrument == i;
            let btn = egui::Button::new(RichText::new(label).monospace().size(11.0))
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
        mut attack,
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

    // ------ Dev Tools: Preset Dumps ------
    ui.collapsing("Dev: Preset Dumps", |ui| {
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut state.dump_name_input);
            if ui.button("Dump").clicked() {
                let instrument = &crate::instrument_registry::INSTRUMENTS[state.selected_instrument];
                let mut specials = Vec::new();
                for def in instrument.special_params {
                    if let Some(param) = params.special_param(state.selected_instrument, def.special_index) {
                        specials.push(param.value());
                    } else {
                        specials.push(0.0);
                    }
                }
                let algo = params.algos()[state.selected_instrument].value() as u8;
                // Skip Analog for instruments that don't use it
                let algo = params.algos()[state.selected_instrument].value() as u8;
                // Skip Analog for instruments that don't use it
                let standards = if matches!(state.selected_instrument, 2 | 3 | 7 | 8 | 10 | 12) {
                    // HiHat, OpenHiHat, Ride, Cymbal, Perc1, Zap - use 0.0 as placeholder
                    [freq, decay, vol, filt, attack, release, decay_curve, release_curve, hold, filter_env_amount, filter_env_decay, 0.0, stereo]
                } else {
                    [freq, decay, vol, filt, attack, release, decay_curve, release_curve, hold, filter_env_amount, filter_env_decay, analog, stereo]
                };
                
                let dump = preset_dumps::PresetDump {
                    name: state.dump_name_input.clone(),
                    instrument_idx: state.selected_instrument,
                    instrument_label: instrument.label.to_string(),
                    standards,
                    algo,
                    specials,
                };
                if let Err(e) = preset_dumps::dump_preset(&dump) {
                    eprintln!("Dump failed: { }", e);
                }
            }
        });
        let dumps = preset_dumps::list_dumps();
        if !dumps.is_empty() {
            ui.label("Existing dumps:");
            for info in dumps {
                ui.horizontal(|ui| {
                    ui.label(format!("{} - {}", info.instrument_label, info.name));
                    if ui.button("Load").clicked() {
                        if let Ok(dump) = preset_dumps::load_dump(&info.path) {
                            state.selected_instrument = dump.instrument_idx;
                            let target_inst = &sound_settings.instruments[dump.instrument_idx];
                            store_field(target_inst, crate::instrument_registry::StandardField::Freq, dump.standards[0]);
                            store_field(target_inst, crate::instrument_registry::StandardField::Decay, dump.standards[1]);
                            store_field(target_inst, crate::instrument_registry::StandardField::Volume, dump.standards[2]);
                            store_field(target_inst, crate::instrument_registry::StandardField::FilterFreq, dump.standards[3]);
                            store_field(target_inst, crate::instrument_registry::StandardField::Attack, dump.standards[4]);
                            store_field(target_inst, crate::instrument_registry::StandardField::Release, dump.standards[5]);
                            store_field(target_inst, crate::instrument_registry::StandardField::DecayCurve, dump.standards[6]);
                            store_field(target_inst, crate::instrument_registry::StandardField::ReleaseCurve, dump.standards[7]);
                            store_field(target_inst, crate::instrument_registry::StandardField::Hold, dump.standards[8]);
                             store_field(target_inst, crate::instrument_registry::StandardField::FilterEnvAmount, dump.standards[9]);
                             store_field(target_inst, crate::instrument_registry::StandardField::FilterEnvDecay, dump.standards[10]);
                             // Skip Analog for instruments that don't use it
                             let is_analog_fixed = matches!(
                                 dump.instrument_idx,
                                 2 | 3 | 7 | 8 | 10 | 12  // HiHat, OpenHiHat, Ride, Cymbal, Perc1, Zap
                             );
                             if !is_analog_fixed {
                                 store_field(target_inst, crate::instrument_registry::StandardField::Analog, dump.standards[11]);
                             }
                             store_field(target_inst, crate::instrument_registry::StandardField::Stereo, dump.standards[12]);
                            let algo_param = params.algos()[dump.instrument_idx];
                            setter.set_parameter(algo_param, dump.algo as i32);
                            let inst_def = &crate::instrument_registry::INSTRUMENTS[dump.instrument_idx];
                            for (i, def) in inst_def.special_params.iter().enumerate() {
                                if let Some(param) = params.special_param(dump.instrument_idx, def.special_index) {
                                    if i < dump.specials.len() {
                                        setter.set_parameter(param, dump.specials[i]);
                                    }
                                }
                            }
                        }
                    }
                    if ui.button("Delete").clicked() {
                        let _ = preset_dumps::delete_dump(&info.path);
                    }
                });
            }
        }
    });
    ui.add(egui::Separator::default().spacing(8.0));

    // ------ Volume global de l'instrument ------
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Volume").strong().size(14.0));
            ui.add_space(8.0);
            let slider = LocalParamSlider::new(&mut vol, 0.0..=2.0).with_width(180.0);
            if ui.add(slider).changed() {
                store_field(inst, crate::instrument_registry::StandardField::Volume, vol);
                changed = true;
            }
        });
    });
    ui.add(egui::Separator::default().spacing(12.0));

    // Data-driven grouped Sound Panel
    let instrument = &crate::instrument_registry::INSTRUMENTS[state.selected_instrument];
    let standard_defs = instrument.standard_params;
    let special_defs = instrument.special_params;

    for family in [
        crate::instrument_registry::ParamFamily::Osc,
        crate::instrument_registry::ParamFamily::Env,
        crate::instrument_registry::ParamFamily::Filter,
        crate::instrument_registry::ParamFamily::Saturation,
        crate::instrument_registry::ParamFamily::Output,
    ] {
        ui.group(|ui| {
            ui.label(RichText::new(family.label()).strong().size(13.0));
            ui.separator();

            ui.horizontal(|ui| {
                // Left column: params
                ui.vertical(|ui| {
                    // Standard params for this family
                    for def in standard_defs.iter().filter(|d| d.family == family) {
                            ui.horizontal(|ui| {
                                let label_text = if def.field == crate::instrument_registry::StandardField::FilterFreq {
                                    let ft = crate::instrument_registry::filter_type_label(state.selected_instrument);
                                    format!("{} ({})", def.label, ft)
                                } else {
                                    def.label.to_string()
                                };
                                ui.label(label_text);
                                
                                // Frequency display mode switch for Kick and B8
                                let is_bass_drum = state.selected_instrument == 0 || state.selected_instrument == 11;
                                if def.field == crate::instrument_registry::StandardField::Freq && is_bass_drum {
                                    let freq_mode_param = if state.selected_instrument == 0 {
                                        &params.freq_mode_kick
                                    } else {
                                        &params.freq_mode_bassdrum808
                                    };
                                    let mut freq_in_notes = freq_mode_param.value();
                                    if ui.checkbox(&mut freq_in_notes, "Notes").changed() {
                                        setter.set_parameter(freq_mode_param, freq_in_notes);
                                        // Snap frequency to exact note when switching to Notes mode
                                        if freq_in_notes {
                                            let ratio = instrument.freq_display_ratio;
                                            let snapped_note = freq_to_note(freq * ratio).round();
                                            freq = note_to_freq(snapped_note) / ratio;
                                            store_field(inst, crate::instrument_registry::StandardField::Freq, freq);
                                            changed = true;
                                        }
                                    }
                                }
                                
                                // Check if we're in note display mode for bass drums
                                let freq_in_notes = if is_bass_drum && def.field == crate::instrument_registry::StandardField::Freq {
                                    let freq_mode_param = if state.selected_instrument == 0 {
                                        &params.freq_mode_kick
                                    } else {
                                        &params.freq_mode_bassdrum808
                                    };
                                    freq_mode_param.value()
                                } else {
                                    false
                                };
                                
                                 match (&def.widget, def.field) {
                                (crate::instrument_registry::ParamWidget::Slider { min, max, logarithmic, suffix }, field) => {
                                    // Special case: frequency in note mode for bass drums
                                    if freq_in_notes && field == crate::instrument_registry::StandardField::Freq {
                                        let ratio = instrument.freq_display_ratio;
                                        let note_val = freq_to_note(freq * ratio).round();
                                        ui.label(RichText::new(note_name(note_val)).monospace().size(14.0));
                                        if ui.button("-").clicked() {
                                            let new_note = (note_val - 1.0).max(0.0);
                                            freq = note_to_freq(new_note) / ratio;
                                            store_field(inst, field, freq);
                                            changed = true;
                                        }
                                        if ui.button("+").clicked() {
                                            let new_note = (note_val + 1.0).min(127.0);
                                            freq = note_to_freq(new_note) / ratio;
                                            store_field(inst, field, freq);
                                            changed = true;
                                        }
                                    } else {
                                        let value: &mut f32 = match field {
                                            crate::instrument_registry::StandardField::Freq => &mut freq,
                                            crate::instrument_registry::StandardField::Decay => &mut decay,
                                            crate::instrument_registry::StandardField::Volume => &mut vol,
                                            crate::instrument_registry::StandardField::FilterFreq => &mut filt,
                                            crate::instrument_registry::StandardField::Attack => &mut attack,
                                            crate::instrument_registry::StandardField::Release => &mut release,
                                            crate::instrument_registry::StandardField::DecayCurve => &mut decay_curve,
                                            crate::instrument_registry::StandardField::ReleaseCurve => &mut release_curve,
                                            crate::instrument_registry::StandardField::Hold => &mut hold,
                                            crate::instrument_registry::StandardField::FilterEnvAmount => &mut filter_env_amount,
                                            crate::instrument_registry::StandardField::FilterEnvDecay => &mut filter_env_decay,
                                            crate::instrument_registry::StandardField::Analog => &mut analog,
                                            crate::instrument_registry::StandardField::Stereo => &mut stereo,
                                        };
                                        let slider = LocalParamSlider::new(value, *min..=*max)
                                            .logarithmic(*logarithmic)
                                            .with_width(120.0);
                                        if let Some(s) = suffix {
                                            // Note: LocalParamSlider doesn't support suffix in the same way
                                            // We'll add it to the label or handle it separately if needed
                                        }
                                        if ui.add(slider).changed() {
                                            store_field(inst, field, *value);
                                            changed = true;
                                        }
                                    }
                                }
                                (crate::instrument_registry::ParamWidget::Checkbox, field) => {
                                    let value: &mut f32 = match field {
                                        crate::instrument_registry::StandardField::Stereo => &mut stereo,
                                        _ => &mut stereo,
                                    };
                                    let mut checked = *value >= 0.5;
                                    if ui.checkbox(&mut checked, "").changed() {
                                        *value = if checked { 1.0 } else { 0.0 };
                                        store_field(inst, field, *value);
                                        changed = true;
                                    }
                                }
                            }
                        });
                    }

                    // Special params for this family
                    for def in special_defs.iter().filter(|d| d.family == family) {
                        if let Some(param) = params.special_param(state.selected_instrument, def.special_index) {
                            ui.horizontal(|ui| {
                                ui.label(def.label);
                                // Boolean toggle for on/off switches (min=0, max=1)
                                if def.min == 0.0 && def.max == 1.0 && def.label.to_lowercase().contains("pre-filter") {
                                    let mut checked = param.value() >= 0.5;
                                    if ui.checkbox(&mut checked, "").changed() {
                                        setter.set_parameter(param, if checked { 1.0 } else { 0.0 });
                                    }
                                // Saturation Type: show combobox with names instead of number slider
                                } else if def.label.to_lowercase().contains("saturation type") {
                                    let current_val = param.value() as i32;
                                    let type_names = ["None", "SoftClip", "Valve", "Transistor", "HardClip", "Tape"];
                                    let current_name = type_names.get(current_val as usize).unwrap_or(&"None");
                                    egui::ComboBox::from_id_salt(def.name)
                                        .width(100.0)
                                        .selected_text(*current_name)
                                        .show_ui(ui, |ui| {
                                            for (idx, name) in type_names.iter().enumerate() {
                                                if ui.selectable_label(idx as i32 == current_val, *name).clicked() {
                                                    setter.set_parameter(param, idx as f32);
                                                }
                                            }
                                        });
                                // Cymbal Noise Type: show combobox with names
                                } else if def.label.to_lowercase().contains("noise type") {
                                    let current_val = param.value() as i32;
                                    let type_names = ["White", "Pink", "Brown", "Blue"];
                                    let current_name = type_names.get(current_val as usize).unwrap_or(&"White");
                                    egui::ComboBox::from_id_salt(def.name)
                                        .width(100.0)
                                        .selected_text(*current_name)
                                        .show_ui(ui, |ui| {
                                            for (idx, name) in type_names.iter().enumerate() {
                                                if ui.selectable_label(idx as i32 == current_val, *name).clicked() {
                                                    setter.set_parameter(param, idx as f32);
                                                }
                                            }
                                        });
                                } else {
                                    ui.add(widgets::ParamSlider::for_param(param, setter).with_width(120.0));
                                }
                            });
                        }
                    }

                    // Algorithm selector inside OSC family
                    if family == crate::instrument_registry::ParamFamily::Osc {
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
                    }

                    // Mix checkbox inside OUTPUT family
                    if family == crate::instrument_registry::ParamFamily::Output {
                        let mix_param = params.mixes()[state.selected_instrument];
                        ui.horizontal(|ui| {
                            ui.label("Mix");
                            let mut mix = mix_param.value();
                            if ui.add(egui::Checkbox::new(&mut mix, "")).changed() {
                                setter.set_parameter(mix_param.into(), mix);
                            }
                        });
                    }

                    // Legend inside ENV family (under the params)
                    if family == crate::instrument_registry::ParamFamily::Env {
                        let has_attack = standard_defs.iter().any(|d| d.field == crate::instrument_registry::StandardField::Attack);
                        let has_hold = standard_defs.iter().any(|d| d.field == crate::instrument_registry::StandardField::Hold);
                        let has_release = standard_defs.iter().any(|d| d.field == crate::instrument_registry::StandardField::Release);
                        ui.horizontal(|ui| {
                            let legend = |ui: &mut egui::Ui, color: Color32, text: &str| {
                                ui.label(RichText::new("- ").color(color));
                                ui.label(text);
                            };
                            if has_attack {
                                legend(ui, Color32::from_rgb(255, 220, 80), "A");
                            }
                            if has_hold {
                                legend(ui, Color32::from_rgb(140, 220, 255), "H");
                            }
                            legend(ui, Color32::from_rgb(100, 180, 255), "D");
                            if has_release {
                                legend(ui, Color32::from_rgb(180, 120, 255), "R");
                            }
                        });
                    }
                });

                // Right column: graphs
                match family {
                    crate::instrument_registry::ParamFamily::Env => {
                        let has_release = standard_defs.iter().any(|d| d.field == crate::instrument_registry::StandardField::Release);
                        draw_amp_envelope(ui, attack, decay, decay_curve, hold, release, release_curve, has_release);
                    }
                    crate::instrument_registry::ParamFamily::Filter => {
                        let has_filter_env = standard_defs.iter().any(|d| d.field == crate::instrument_registry::StandardField::FilterEnvAmount);
                        if has_filter_env {
                            draw_filter_envelope(ui, decay_curve, filter_env_decay);
                        }
                    }
                    _ => {}
                }
            });
        });
    }

    if changed {
        sound_settings.bump_version();
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------------------------------
fn store_field(
    inst: &crate::sound_settings::InstrumentSettingsState,
    field: crate::instrument_registry::StandardField,
    value: f32,
) {
    use crate::instrument_registry::StandardField;
    match field {
        StandardField::Freq => inst.frequency.store(value.to_bits(), Ordering::Relaxed),
        StandardField::Decay => inst.decay.store(value.to_bits(), Ordering::Relaxed),
        StandardField::Volume => inst.volume.store(value.to_bits(), Ordering::Relaxed),
        StandardField::FilterFreq => inst.filter_freq.store(value.to_bits(), Ordering::Relaxed),
        StandardField::Attack => inst.attack.store(value.to_bits(), Ordering::Relaxed),
        StandardField::Release => inst.release.store(value.to_bits(), Ordering::Relaxed),
        StandardField::DecayCurve => inst.decay_curve.store(value.to_bits(), Ordering::Relaxed),
        StandardField::ReleaseCurve => inst.release_curve.store(value.to_bits(), Ordering::Relaxed),
        StandardField::Hold => inst.hold.store(value.to_bits(), Ordering::Relaxed),
        StandardField::FilterEnvAmount => inst
            .filter_env_amount
            .store(value.to_bits(), Ordering::Relaxed),
        StandardField::FilterEnvDecay => inst
            .filter_env_decay
            .store(value.to_bits(), Ordering::Relaxed),
        StandardField::Analog => inst.analog.store(value.to_bits(), Ordering::Relaxed),
        StandardField::Stereo => inst.stereo.store(value.to_bits(), Ordering::Relaxed),
    }
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

fn algo_combo(ui: &mut egui::Ui, setter: &ParamSetter, param: &IntParam, algo_names: &[&str]) {
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
) -> bool {
    let enabled = param.value();
    let button = egui::Button::new(short_label)
        .min_size(Vec2::new(24.0, 20.0))
        .fill(if enabled {
            Color32::from_rgb(56, 132, 255)
        } else {
            Color32::from_rgb(36, 36, 36)
        });

    let response = ui.add(button).on_hover_text(hover_label);
    let clicked = response.clicked();
    if clicked {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, !enabled);
        setter.end_set_parameter(param);
    }
    clicked
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

#[cfg(target_os = "windows")]
fn start_external_midi_drag(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let helper = find_midi_drag_helper().ok_or("MIDI drag helper not found")?;
    Command::new(helper).arg(path).spawn()?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn find_midi_drag_helper() -> Option<PathBuf> {
    const HELPER_NAME: &str = "drum-pattern-midi-drag-helper.exe";

    if let Ok(path) = std::env::var("DRUM_FLASH_MIDI_DRAG_HELPER") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }

    if let Ok(common_files) = std::env::var("CommonProgramFiles") {
        let path = PathBuf::from(common_files)
            .join("VST3")
            .join("drum-pattern-vst.vst3")
            .join("Contents")
            .join("x86_64-win")
            .join(HELPER_NAME);
        if path.is_file() {
            return Some(path);
        }
    }

    let local_bundle = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("build")
        .join("drum-pattern-vst.vst3")
        .join("Contents")
        .join("x86_64-win")
        .join(HELPER_NAME);
    if local_bundle.is_file() {
        return Some(local_bundle);
    }

    None
}

#[cfg(not(target_os = "windows"))]
fn start_external_midi_drag(_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    Err("MIDI drag helper is only implemented on Windows".into())
}

// ---------------------------------------------------------------------------------------------------------------
// Plock context menu
// ---------------------------------------------------------------------------------------------------------------
fn draw_plock_menu(
    ui: &mut egui::Ui,
    plock: &PlockState,
    sound_settings: &SoundSettingsState,
    params: &DrumFlashParams,
    setter: &ParamSetter,
    instrument: usize,
    step: usize,
    _state: &mut EditorUIState,
) {
    use crate::plock::{FIELD_COUNT, SPECIAL_FIELD_START};

    ui.label(
        RichText::new(format!(
            "Plock {} --- Step {}",
            crate::instrument_registry::INSTRUMENTS[instrument].label,
            step + 1
        ))
        .strong(),
    );
    ui.separator();

    let inst = &sound_settings.instruments[instrument];
    let global = inst.load();

    let has_plock = plock.masks.is_active(instrument, step);

    // ------ Creation ------
    if !has_plock {
        ui.label("Create plock:");
        if ui.button("-- Link to global").clicked() {
            plock.masks.set_active(instrument, step, true);
        }
        if ui.button("Snapshot Snapshot current settings").clicked() {
            let mut special = [0.0f32; 32];
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
                attack: global.4,
                release: global.5,
                decay_curve: global.6,
                release_curve: global.7,
                hold: global.8,
                filter_env_amount: global.9,
                filter_env_decay: global.10,
                analog: global.11,
                stereo: global.12,
                algo,
                special,
            };
            plock.set_settings(instrument, step, &settings);
        }
        return;
    }

    // ------ Mode indicator ------
    let mask = plock.field_masks.get(instrument, step);
    let all_bits = if FIELD_COUNT >= 64 {
        0xFFFFFFFFFFFFFFFFu64
    } else {
        (1u64 << FIELD_COUNT) - 1
    };
    let mode_text = if mask == 0 {
        "-- Linked to global"
    } else if mask == all_bits {
        "Snapshot Full snapshot"
    } else {
        "-- Mixed"
    };
    ui.label(RichText::new(mode_text).small());
    ui.separator();

    // ------ Helpers ------
    let draw_slider = |ui: &mut egui::Ui,
                       label: &str,
                        value: &mut f32,
                        range: std::ops::RangeInclusive<f32>,
                        log: bool,
                        field: usize| {
        let overridden = plock.field_masks.is_set(instrument, step, field);
        let label_text = if overridden {
            RichText::new(label).strong()
        } else {
            RichText::new(label).weak()
        };
        let (changed, reset) = ui
            .horizontal(|ui| {
                ui.label(label_text);
                let slider = LocalParamSlider::new(value, range.clone())
                    .logarithmic(log)
                    .with_width(120.0);
                let response = ui.add(slider);
                let c = response.changed();
                let r = overridden && ui.small_button("Undo").clicked();
                (c, r)
            })
            .inner;
        if changed {
            plock.set_field(instrument, step, field, *value);
        }
        if reset {
            plock.field_masks.clear(instrument, step, field);
        }
    };

    // ------ Standard fields ------
    let get_global_value = |field: crate::instrument_registry::StandardField| -> f32 {
        match field {
            crate::instrument_registry::StandardField::Freq => global.0,
            crate::instrument_registry::StandardField::Decay => global.1,
            crate::instrument_registry::StandardField::Volume => global.2,
            crate::instrument_registry::StandardField::FilterFreq => global.3,
            crate::instrument_registry::StandardField::Attack => global.4,
            crate::instrument_registry::StandardField::Release => global.5,
            crate::instrument_registry::StandardField::DecayCurve => global.6,
            crate::instrument_registry::StandardField::ReleaseCurve => global.7,
            crate::instrument_registry::StandardField::Hold => global.8,
            crate::instrument_registry::StandardField::FilterEnvAmount => global.9,
            crate::instrument_registry::StandardField::FilterEnvDecay => global.10,
            crate::instrument_registry::StandardField::Analog => global.11,
            crate::instrument_registry::StandardField::Stereo => global.12,
        }
    };

    let is_bass_drum_plock = instrument == 0 || instrument == 11;
    let freq_in_notes_plock = if is_bass_drum_plock {
        let freq_mode_param = if instrument == 0 {
            &params.freq_mode_kick
        } else {
            &params.freq_mode_bassdrum808
        };
        freq_mode_param.value()
    } else {
        false
    };

    // Note display toggle for bass drums in plock
    if is_bass_drum_plock {
        let freq_mode_param = if instrument == 0 {
            &params.freq_mode_kick
        } else {
            &params.freq_mode_bassdrum808
        };
        let mut freq_in_notes = freq_mode_param.value();
        if ui.checkbox(&mut freq_in_notes, "Notes").changed() {
            setter.set_parameter(freq_mode_param, freq_in_notes);
        }
    }

    // Data-driven standard params
    let inst_def = &crate::instrument_registry::INSTRUMENTS[instrument];
    for def in inst_def.standard_params {
        let field_index = def.field.plock_field_index();
        let mut value = if plock.field_masks.is_set(instrument, step, field_index) {
            plock.values.get(instrument, step, field_index)
        } else {
            get_global_value(def.field)
        };

        // Special case: frequency in note mode for bass drums
        if def.field == crate::instrument_registry::StandardField::Freq && freq_in_notes_plock {
            let ratio = inst_def.freq_display_ratio;
            let overridden = plock.field_masks.is_set(instrument, step, field_index);
            let note_val = freq_to_note(value * ratio).round();
            let label_text = if overridden {
                RichText::new(format!("{}: {}", def.label, note_name(note_val))).strong()
            } else {
                RichText::new(format!("{}: {}", def.label, note_name(note_val))).weak()
            };
            ui.horizontal(|ui| {
                ui.label(label_text);
                if ui.button("-").clicked() {
                    let new_note = (note_val - 1.0).max(0.0);
                    value = note_to_freq(new_note) / ratio;
                    plock.set_field(instrument, step, field_index, value);
                }
                if ui.button("+").clicked() {
                    let new_note = (note_val + 1.0).min(127.0);
                    value = note_to_freq(new_note) / ratio;
                    plock.set_field(instrument, step, field_index, value);
                }
                if overridden && ui.small_button("Undo").clicked() {
                    plock.field_masks.clear(instrument, step, field_index);
                }
            });
            continue;
        }

        match &def.widget {
            crate::instrument_registry::ParamWidget::Slider { min, max, logarithmic, .. } => {
                draw_slider(ui, def.label, &mut value, *min..=*max, *logarithmic, field_index);
            }
            crate::instrument_registry::ParamWidget::Checkbox => {
                let overridden = plock.field_masks.is_set(instrument, step, field_index);
                let label_text = if overridden {
                    RichText::new(def.label).strong()
                } else {
                    RichText::new(def.label).weak()
                };
                let (changed, reset) = ui
                    .horizontal(|ui| {
                        ui.label(label_text);
                        let mut checked = value >= 0.5;
                        let c = ui.add(egui::Checkbox::new(&mut checked, "")).changed();
                        if c {
                            value = if checked { 1.0 } else { 0.0 };
                        }
                        let r = overridden && ui.small_button("Undo").clicked();
                        (c, r)
                    })
                    .inner;
                if changed {
                    plock.set_field(instrument, step, field_index, value);
                }
                if reset {
                    plock.field_masks.clear(instrument, step, field_index);
                }
            }
        }
    }

    // ------ Algo ------
    let mut algo_val = if plock.field_masks.is_set(instrument, step, 13) {
        plock.values.get(instrument, step, 13) as u8
    } else {
        params.algos()[instrument].value() as u8
    };
    let algo_overridden = plock.field_masks.is_set(instrument, step, 13);
    let algo_label = if algo_overridden {
        RichText::new("Algo").strong()
    } else {
        RichText::new("Algo").weak()
    };
    let mut algo_val_f32 = algo_val as f32;
    let (algo_changed, algo_reset) = ui
        .horizontal(|ui| {
            ui.label(algo_label);
            let slider = LocalParamSlider::new(&mut algo_val_f32, 0.0..=3.0)
                .with_width(120.0);
            let c = ui.add(slider).changed();
            let r = algo_overridden && ui.small_button("Undo").clicked();
            (c, r)
        })
        .inner;
    if algo_changed {
        algo_val = algo_val_f32.round() as u8;
    }
    if algo_changed {
        plock.set_field(instrument, step, 13, algo_val as f32);
    }
    if algo_reset {
        plock.field_masks.clear(instrument, step, 13);
    }

    // ------ Special params ------
    let special_defs = crate::instrument_registry::special_params(instrument);
    for def in special_defs {
        if def.special_index >= 8 {
            continue;
        }
        let field = SPECIAL_FIELD_START + def.special_index;
        let mut value = if plock.field_masks.is_set(instrument, step, field) {
            plock.values.get(instrument, step, field)
        } else {
            params
                .special_param(instrument, def.special_index)
                .map(|p| p.value())
                .unwrap_or(def.default)
        };
        let overridden = plock.field_masks.is_set(instrument, step, field);
        let label_text = if overridden {
            RichText::new(def.label).strong()
        } else {
            RichText::new(def.label).weak()
        };
        let (changed, reset) = ui
            .horizontal(|ui| {
                ui.label(label_text);
                let log = def.min > 0.0 && def.max / def.min >= 20.0;
                let slider = LocalParamSlider::new(&mut value, def.min..=def.max)
                    .logarithmic(log)
                    .with_width(120.0);
                let c = ui.add(slider).changed();
                let r = overridden && ui.small_button("Undo").clicked();
                (c, r)
            })
            .inner;
        if changed {
            plock.set_field(instrument, step, field, value);
        }
        if reset {
            plock.field_masks.clear(instrument, step, field);
        }
    }

    ui.separator();
    if ui.button("-' Clear plock").clicked() {
        plock.clear(instrument, step);
    }
}
