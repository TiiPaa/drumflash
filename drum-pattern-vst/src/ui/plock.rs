//! P-lock menus: sound plocks, fusion morph, sequencer plocks, popup.

use crate::plock::PlockState;
use crate::sequencer::{
    pattern::MorphDirection,
    FusedGroup, MorphTarget, SharedPattern,
};
use crate::sound_settings::SoundSettingsState;
use crate::synthesis::{self, DrumVoice, VoiceSettings};
use crate::ui::editor_state::*;
use crate::ui::fmt::*;
use crate::ui::grid::{fusion_morph_state, preserve_step_active_from_plock_popup};
use crate::ui::local_param_slider::LocalParamSlider;
use crate::ui::menus::*;
use crate::ui::theme::*;
use crate::ui::widgets::*;
use crate::DrumFlashParams;
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, Color32, RichText, Vec2};

fn draw_plock_menu(
    ui: &mut egui::Ui,
    pattern: &SharedPattern,
    plock: &PlockState,
    sound_settings: &SoundSettingsState,
    params: &DrumFlashParams,
    _setter: &ParamSetter,
    instrument: usize,
    step: usize,
    step_was_active: bool,
    state: &mut EditorUIState,
) {
    use crate::plock::{FIELD_COUNT, SPECIAL_FIELD_START};

    #[allow(non_snake_case)] let ACCENT: Color32 = PL_LINK();
    // `instrument` is a SLOT index (plock storage is per slot); registry and
    // special-param lookups go through the voice index of the slot's kind.
    let voice_idx = schema_voice_idx(params, instrument);
    let inst_def = &crate::instrument_registry::INSTRUMENTS[voice_idx];
    let title = format!("Plock {}", inst_def.label);

    preserve_step_active_from_plock_popup(pattern, state, instrument, step, step_was_active);

    plock_menu_frame(ui, ACCENT, |ui| {
        if plock_menu_header(ui, &title, step, ACCENT) {
            state.plock_popup = None;
        }

        let inst = &sound_settings.instruments[instrument];
        let global = inst.load();
        let has_plock = plock.masks.is_active(instrument, step);

        // ------ Creation ------
        if !has_plock {
            ui.label(
                RichText::new("Create Plock")
                    .font(f_sans_sb(10.0))
                    .color(INK2()),
            );
            ui.add_space(6.0);
            if plock_menu_action_row(ui, "Link to Global", ACCENT).clicked() {
                plock.masks.set_active(instrument, step, true);
            }
            if plock_menu_action_row(ui, "Snapshot Current Settings", ACCENT).clicked() {
                // Specials are stored per slot alongside the standard settings.
                let special = inst.load_specials();
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
            if let Some(ref entry) = state.plock_clipboard {
                if entry.instrument == instrument {
                    if plock_menu_action_row(ui, "Paste Plock", ACCENT).clicked() {
                        plock.masks.set_active(instrument, step, true);
                        plock
                            .field_masks
                            .set_raw(instrument, step, entry.field_mask);
                        for (field, &value) in entry.values.iter().enumerate() {
                            plock.values.set(instrument, step, field, value);
                        }
                    }
                }
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
            "Linked to Global"
        } else if mask == all_bits {
            "Full Snapshot"
        } else {
            "Mixed"
        };
        ui.horizontal(|ui| {
            ui.label(RichText::new("Mode").font(f_sans_med(10.0)).color(INK3()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(mode_text)
                        .font(f_mono_med(10.0))
                        .color(ACCENT),
                );
            });
        });
        ui.add_space(8.0);

        // ------ Volume (most used, shown first) ------
        {
            let vol_field = crate::instrument_registry::StandardField::Volume.plock_field_index();
            let mut vol_value = if plock.field_masks.is_set(instrument, step, vol_field) {
                plock.values.get(instrument, step, vol_field)
            } else {
                global.2
            };
            let overridden = plock.field_masks.is_set(instrument, step, vol_field);
            let vol_response = plock_menu_row(
                ui,
                "Volume",
                ACCENT,
                overridden,
                Some(&format!("{:.2}", vol_value)),
                |ui| {
                    ui.add(
                        LocalParamSlider::new(&mut vol_value, 0.0..=2.0)
                            .with_width(120.0)
                            .without_value()
                            .reset_value(global.2),
                    )
                },
            );
            if vol_response.changed() {
                plock.masks.set_active(instrument, step, true);
                plock.field_masks.set(instrument, step, vol_field);
                plock.set_field(instrument, step, vol_field, vol_value);
            }
        }

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

        let is_bass_drum_plock = matches!(voice_idx, 0 | 11);
        let freq_in_notes_plock =
            is_bass_drum_plock && sound_settings.instruments[instrument].freq_mode();

        // Note display toggle for bass drums in plock (per-slot freq mode)
        if is_bass_drum_plock {
            let freq_in_notes = freq_in_notes_plock;
            let label = if freq_in_notes { "Notes" } else { "Hz" };
            plock_menu_row(ui, "Display", ACCENT, false, None, |ui| {
                if plock_menu_action_row(ui, label, ACCENT).clicked() {
                    sound_settings.instruments[instrument].set_freq_mode(!freq_in_notes);
                    sound_settings.bump_version();
                }
                ui.allocate_response(Vec2::new(1.0, 1.0), egui::Sense::hover())
            });
        }

        for def in inst_def.standard_params {
            if def.field == crate::instrument_registry::StandardField::Volume {
                continue;
            }
            let field_index = def.field.plock_field_index();
            let mut value = if plock.field_masks.is_set(instrument, step, field_index) {
                plock.values.get(instrument, step, field_index)
            } else {
                get_global_value(def.field)
            };
            let overridden = plock.field_masks.is_set(instrument, step, field_index);

            // Special case: frequency in note mode for bass drums
            if def.field == crate::instrument_registry::StandardField::Freq && freq_in_notes_plock {
                let ratio = inst_def.freq_display_ratio;
                let note_val = freq_to_note(value * ratio).round();
                let label = format!("{} {}", def.label, note_name(note_val));
                plock_menu_row(ui, &label, ACCENT, overridden, None, |ui| {
                    let mut changed = false;
                    if ui.small_button("-").clicked() {
                        let new_note = (note_val - 1.0).max(0.0);
                        value = note_to_freq(new_note) / ratio;
                        changed = true;
                    }
                    if ui.small_button("+").clicked() {
                        let new_note = (note_val + 1.0).min(127.0);
                        value = note_to_freq(new_note) / ratio;
                        changed = true;
                    }
                    if changed {
                        plock.set_field(instrument, step, field_index, value);
                    }
                    ui.allocate_response(Vec2::new(1.0, 1.0), egui::Sense::hover())
                });
                continue;
            }

            match &def.widget {
                crate::instrument_registry::ParamWidget::Slider {
                    min,
                    max,
                    logarithmic,
                    ..
                } => {
                    let value_text = format_value_for_plock(def.field, value, *min, *max);
                    let row_response = plock_menu_row(
                        ui,
                        def.label,
                        ACCENT,
                        overridden,
                        Some(&value_text),
                        |ui| {
                            ui.add(
                                LocalParamSlider::new(&mut value, *min..=*max)
                                    .logarithmic(*logarithmic)
                                    .with_width(120.0)
                                    .without_value()
                                    .reset_value(get_global_value(def.field)),
                            )
                        },
                    );
                    if row_response.changed() {
                        plock.masks.set_active(instrument, step, true);
                        plock.field_masks.set(instrument, step, field_index);
                        plock.set_field(instrument, step, field_index, value);
                    }
                }
                crate::instrument_registry::ParamWidget::Checkbox => {
                    plock_menu_row(ui, def.label, ACCENT, overridden, None, |ui| {
                        let mut checked = value >= 0.5;
                        let response = ui.add(egui::Checkbox::new(
                            &mut checked,
                            RichText::new("").font(f_sans_med(10.0)),
                        ));
                        if response.changed() {
                            value = if checked { 1.0 } else { 0.0 };
                        }
                        response
                    });
                    if overridden {
                        plock.set_field(instrument, step, field_index, value);
                    }
                }
            }
        }

        // ------ Algo ------
        {
            let algo_count = crate::instrument_registry::algo_count(voice_idx);
            if algo_count > 1 {
                if let Some(voice) = DrumVoice::from_index(voice_idx) {
                    let algos = synthesis::algos_for(voice);
                    let algo_names: Vec<&str> = algos.iter().map(|a| a.name).collect();

                    let mut algo_val = if plock.field_masks.is_set(instrument, step, 13) {
                        plock.values.get(instrument, step, 13) as u8
                    } else {
                        params.algos()[instrument].value() as u8
                    };
                    let algo_overridden = plock.field_masks.is_set(instrument, step, 13);

                    let max_algo = (algo_count - 1) as u8;
                    if algo_val > max_algo {
                        algo_val = max_algo;
                    }

                    let mut selected = algo_val as usize;
                    let current_name = algo_names.get(selected).copied().unwrap_or("?");
                    let _algo_response = plock_menu_row(
                        ui,
                        "Algo",
                        ACCENT,
                        algo_overridden,
                        Some(current_name),
                        |ui| {
                            let (response, picked) =
                                styled_select(ui, "plock_algo", selected, &algo_names, 120.0);
                            if let Some(picked) = picked {
                                selected = picked;
                            }
                            response
                        },
                    );
                    if selected != algo_val as usize {
                        plock.masks.set_active(instrument, step, true);
                        plock.field_masks.set(instrument, step, 13);
                        plock.set_field(instrument, step, 13, selected as f32);
                    }
                }
            }
        }

        // ------ Special params ------
        let special_defs = crate::instrument_registry::special_params(voice_idx);
        for def in special_defs {
            if def.special_index >= 8 {
                continue;
            }
            let field = SPECIAL_FIELD_START + def.special_index;
            if field
                == crate::instrument_registry::StandardField::Attack.plock_field_index()
            {
                continue;
            }
            let value = if plock.field_masks.is_set(instrument, step, field) {
                plock.values.get(instrument, step, field)
            } else {
                inst.special_value(def.special_index)
            };
            let overridden = plock.field_masks.is_set(instrument, step, field);
            let label_lower = def.label.to_lowercase();

            let new_value = if label_lower.contains("saturation type") {
                let (_, picked) = plock_menu_enum_row(
                    ui,
                    def.label,
                    ACCENT,
                    overridden,
                    value,
                    &[
                        "None",
                        "SoftClip",
                        "Valve",
                        "Transistor",
                        "HardClip",
                        "Tape",
                    ],
                    def.name,
                );
                picked
            } else if label_lower.contains("noise type") {
                let (_, picked) = plock_menu_enum_row(
                    ui,
                    def.label,
                    ACCENT,
                    overridden,
                    value,
                    &["White", "Pink", "Brown", "Blue"],
                    def.name,
                );
                picked
            } else if label_lower.contains("click type") {
                let (_, picked) = plock_menu_enum_row(
                    ui,
                    def.label,
                    ACCENT,
                    overridden,
                    value,
                    &["Soft", "Medium", "Hard"],
                    def.name,
                );
                picked
            } else {
                let mut value = value;
                let log = def.min > 0.0 && def.max / def.min >= 20.0;
                let value_text = format_value_for_plock_special(value, def.min, def.max);
                let response =
                    plock_menu_row(ui, def.label, ACCENT, overridden, Some(&value_text), |ui| {
                        ui.add(
                            LocalParamSlider::new(&mut value, def.min..=def.max)
                                .logarithmic(log)
                                .with_width(120.0)
                                .without_value()
                                .reset_value(def.default),
                        )
                    });
                if response.changed() {
                    Some(value)
                } else {
                    None
                }
            };

            if let Some(new_value) = new_value {
                let clamped = new_value.clamp(def.min, def.max);
                plock.masks.set_active(instrument, step, true);
                plock.field_masks.set(instrument, step, field);
                plock.set_field(instrument, step, field, clamped);
            }
        }

        // ------ Actions ------
        ui.add_space(8.0);
        if plock_menu_action_row(ui, "Copy Plock", ACCENT).clicked() {
            let field_mask = plock.field_masks.get_raw(instrument, step);
            let mut values = Vec::with_capacity(crate::plock::FIELD_COUNT);
            for field in 0..crate::plock::FIELD_COUNT {
                values.push(plock.values.get(instrument, step, field));
            }
            state.plock_clipboard = Some(SinglePlockClipboard {
                instrument,
                field_mask,
                values,
            });
        }
        if let Some(ref entry) = state.plock_clipboard {
            if entry.instrument == instrument {
                if plock_menu_action_row(ui, "Paste Plock", ACCENT).clicked() {
                    plock.masks.set_active(instrument, step, true);
                    plock
                        .field_masks
                        .set_raw(instrument, step, entry.field_mask);
                    for (field, &value) in entry.values.iter().enumerate() {
                        plock.values.set(instrument, step, field, value);
                    }
                }
            }
        }
        if plock_menu_action_row(ui, "Clear Plock", DANGER()).clicked() {
            plock.clear(instrument, step);
        }
    });
}

fn draw_morph_target_action_buttons(
    ui: &mut egui::Ui,
    new_fusions: &mut Vec<FusedGroup>,
    fusion_index: usize,
    field_index: usize,
    pattern: &SharedPattern,
    instrument: usize,
) {
    let Some(group) = new_fusions.get(fusion_index) else {
        return;
    };
    let Some(target) = group.morph_targets[..group.morph_count as usize]
        .iter()
        .find(|t| t.field == field_index as u8)
    else {
        return;
    };
    let current = target.direction;
    let label = match current {
        MorphDirection::Target => "Target",
        MorphDirection::Source => "Source",
    };
    let mut new_dir = None;
    let mut delete = false;
    if ui.small_button(label).clicked() {
        new_dir = Some(match current {
            MorphDirection::Target => MorphDirection::Source,
            MorphDirection::Source => MorphDirection::Target,
        });
    }
    if ui.small_button("X").clicked() {
        delete = true;
    }
    if let Some(dir) = new_dir {
        if let Some(g) = new_fusions.get_mut(fusion_index) {
            g.set_morph_target_direction(field_index, dir);
            pattern.store_fusions(instrument, &new_fusions);
        }
    }
    if delete {
        if let Some(g) = new_fusions.get_mut(fusion_index) {
            g.remove_morph_target(field_index);
            pattern.store_fusions(instrument, &new_fusions);
        }
    }
}

fn draw_fusion_morph_menu(
    ui: &mut egui::Ui,
    pattern: &SharedPattern,
    sound_settings: &SoundSettingsState,
    params: &DrumFlashParams,
    _setter: &ParamSetter,
    instrument: usize,
    fusion_index: usize,
    step: usize,
    state: &mut EditorUIState,
) {
    use crate::plock::SPECIAL_FIELD_START;

    #[allow(non_snake_case)] let ACCENT: Color32 = PL_LINK();
    // `instrument` is a SLOT index; schema lookups use the slot's voice index.
    let voice_idx = schema_voice_idx(params, instrument);
    let inst_def = &crate::instrument_registry::INSTRUMENTS[voice_idx];
    let title = format!("Morph {}", inst_def.label);
    let global = sound_settings.instruments[instrument].load();

    let mut new_fusions = pattern.load_fusions(instrument);
    if new_fusions.get(fusion_index).is_none() {
        return;
    }

    plock_menu_frame(ui, ACCENT, |ui| {
        if plock_menu_header(ui, &title, step, ACCENT) {
            state.plock_popup = None;
        }

        // Mode indicator
        let mode_text = if new_fusions
            .get(fusion_index)
            .map(|g| g.morph_count)
            .unwrap_or(0)
            == 0
        {
            "Off"
        } else {
            "On"
        };
        ui.horizontal(|ui| {
            ui.label(RichText::new("Mode").font(f_sans_med(10.0)).color(INK3()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(mode_text)
                        .font(f_mono_med(10.0))
                        .color(ACCENT),
                );
            });
        });
        ui.add_space(8.0);

        // Disable morphing
        if plock_menu_action_row(ui, "Disable Morphing", DANGER()).clicked() {
            if let Some(g) = new_fusions.get_mut(fusion_index) {
                g.morph_count = 0;
                g.morph_targets = [MorphTarget::default(); 4];
                pattern.store_fusions(instrument, &new_fusions);
                state.plock_popup = None;
            }
        }
        ui.add_space(8.0);

        // Volume
        {
            let vol_field = crate::instrument_registry::StandardField::Volume.plock_field_index();
            let (mut vol_value, is_target) = fusion_morph_state(
                &new_fusions,
                fusion_index,
                vol_field,
                params,
                sound_settings,
                instrument,
                step,
            );
            vol_value = vol_value.clamp(0.0, 2.0);
            let vol_response = plock_menu_row(
                ui,
                "Volume",
                ACCENT,
                is_target,
                Some(&format!("{:.2}", vol_value)),
                |ui| {
                    let slider = ui.add(
                        LocalParamSlider::new(&mut vol_value, 0.0..=2.0)
                            .with_width(76.0)
                            .without_value()
                            .reset_value(global.2),
                    );
                    if is_target {
                        draw_morph_target_action_buttons(
                            ui,
                            &mut new_fusions,
                            fusion_index,
                            vol_field,
                            pattern,
                            instrument,
                        );
                    }
                    slider
                },
            );
            if vol_response.changed() {
                if let Some(g) = new_fusions.get_mut(fusion_index) {
                    g.set_morph_target(vol_field, vol_value);
                    pattern.store_fusions(instrument, &new_fusions);
                }
            }
        }

        // Standard fields (per-slot freq display mode)
        let is_bass_drum = matches!(voice_idx, 0 | 11);
        let freq_in_notes = is_bass_drum && sound_settings.instruments[instrument].freq_mode();

        // Note display toggle for bass drums
        if is_bass_drum {
            let label = if freq_in_notes { "Notes" } else { "Hz" };
            plock_menu_row(ui, "Display", ACCENT, false, None, |ui| {
                let response = plock_menu_action_row(ui, label, ACCENT);
                if response.clicked() {
                    sound_settings.instruments[instrument].set_freq_mode(!freq_in_notes);
                    sound_settings.bump_version();
                }
                response
            });
        }

        for def in inst_def.standard_params {
            if def.field == crate::instrument_registry::StandardField::Volume {
                continue;
            }
            let field_index = def.field.plock_field_index();
            let (mut value, is_target) = fusion_morph_state(
                &new_fusions,
                fusion_index,
                field_index,
                params,
                sound_settings,
                instrument,
                step,
            );

            // Special case: frequency in note mode for bass drums
            if def.field == crate::instrument_registry::StandardField::Freq && freq_in_notes {
                let ratio = inst_def.freq_display_ratio;
                let note_val = freq_to_note(value * ratio).round();
                let label = format!("{} {}", def.label, note_name(note_val));
                plock_menu_row(ui, &label, ACCENT, is_target, None, |ui| {
                    let mut changed = false;
                    if ui.small_button("-").clicked() {
                        let new_note = (note_val - 1.0).max(0.0);
                        value = note_to_freq(new_note) / ratio;
                        changed = true;
                    }
                    if ui.small_button("+").clicked() {
                        let new_note = (note_val + 1.0).min(127.0);
                        value = note_to_freq(new_note) / ratio;
                        changed = true;
                    }
                    if is_target {
                        draw_morph_target_action_buttons(
                            ui,
                            &mut new_fusions,
                            fusion_index,
                            field_index,
                            pattern,
                            instrument,
                        );
                    }
                    if changed {
                        if let Some(g) = new_fusions.get_mut(fusion_index) {
                            g.set_morph_target(field_index, value);
                            pattern.store_fusions(instrument, &new_fusions);
                        }
                    }
                    ui.allocate_response(Vec2::new(1.0, 1.0), egui::Sense::hover())
                });
                continue;
            }

            match &def.widget {
                crate::instrument_registry::ParamWidget::Slider {
                    min,
                    max,
                    logarithmic,
                    ..
                } => {
                    let value_text = format_value_for_plock(def.field, value, *min, *max);
                    value = value.clamp(*min, *max);
                    let row_response =
                        plock_menu_row(ui, def.label, ACCENT, is_target, Some(&value_text), |ui| {
                            let slider = ui.add(
                                LocalParamSlider::new(&mut value, *min..=*max)
                                    .logarithmic(*logarithmic)
                                    .with_width(76.0)
                                    .without_value()
                                    .reset_value(match def.field {
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
                                    }),
                            );
                            if is_target {
                                draw_morph_target_action_buttons(
                                    ui,
                                    &mut new_fusions,
                                    fusion_index,
                                    field_index,
                                    pattern,
                                    instrument,
                                );
                            }
                            slider
                        });
                    if row_response.changed() {
                        if let Some(g) = new_fusions.get_mut(fusion_index) {
                            g.set_morph_target(field_index, value);
                            pattern.store_fusions(instrument, &new_fusions);
                        }
                    }
                }
                crate::instrument_registry::ParamWidget::Checkbox => {
                    plock_menu_row(ui, def.label, ACCENT, is_target, None, |ui| {
                        let mut checked = value >= 0.5;
                        let response = ui.add(egui::Checkbox::new(
                            &mut checked,
                            RichText::new("").font(f_sans_med(10.0)),
                        ));
                        if response.changed() {
                            value = if checked { 1.0 } else { 0.0 };
                            if let Some(g) = new_fusions.get_mut(fusion_index) {
                                g.set_morph_target(field_index, value);
                                pattern.store_fusions(instrument, &new_fusions);
                            }
                        }
                        if is_target {
                            draw_morph_target_action_buttons(
                                ui,
                                &mut new_fusions,
                                fusion_index,
                                field_index,
                                pattern,
                                instrument,
                            );
                        }
                        response
                    });
                }
            }
        }

        // Special params (continuous only â€” discrete params can't be morphed).
        // Also skip any special param whose plock field overlaps a standard field
        // (e.g. special_index 4 â†’ field 18 which is also Attack).
        let standard_field_indices: std::collections::HashSet<usize> = inst_def
            .standard_params
            .iter()
            .map(|def| def.field.plock_field_index())
            .collect();
        let special_defs = crate::instrument_registry::special_params(voice_idx);
        for def in special_defs {
            if def.special_index >= 8 || !def.continuous {
                continue;
            }
            let field = SPECIAL_FIELD_START + def.special_index;
            if field
                == crate::instrument_registry::StandardField::Attack.plock_field_index()
                || standard_field_indices.contains(&field)
            {
                continue;
            }
            let (mut value, is_target) = fusion_morph_state(
                &new_fusions,
                fusion_index,
                field,
                params,
                sound_settings,
                instrument,
                step,
            );
            value = value.clamp(def.min, def.max);
            let log = def.min > 0.0 && def.max / def.min >= 20.0;
            let value_text = format_value_for_plock_special(value, def.min, def.max);
            let special_response =
                plock_menu_row(ui, def.label, ACCENT, is_target, Some(&value_text), |ui| {
                    let slider = ui.add(
                        LocalParamSlider::new(&mut value, def.min..=def.max)
                            .logarithmic(log)
                            .with_width(76.0)
                            .without_value()
                            .reset_value(def.default),
                    );
                    if is_target {
                        draw_morph_target_action_buttons(
                            ui,
                            &mut new_fusions,
                            fusion_index,
                            field,
                            pattern,
                            instrument,
                        );
                    }
                    slider
                });
            if special_response.changed() {
                if let Some(g) = new_fusions.get_mut(fusion_index) {
                    g.set_morph_target(field, value);
                    pattern.store_fusions(instrument, &new_fusions);
                }
            }
        }
    });
}

/// Fusion group actions (Morphing / Edit Fusion Steps / Delete Fusion),
/// shared by the sound-plock and sequencer-plock popup branches.
fn draw_fusion_group_menu(
    ui: &mut egui::Ui,
    pattern: &SharedPattern,
    params: &DrumFlashParams,
    inst: usize,
    idx: usize,
    group: FusedGroup,
    step: usize,
    popup: &mut PlockPopup,
    state: &mut EditorUIState,
) {
    plock_menu_frame(ui, PL_LINK(), |ui| {
        if plock_menu_header(
            ui,
            &format!("Fusion {}-{}", group.start_cell + 1, group.end_cell + 1),
            step,
            PL_LINK(),
        ) {
            state.plock_popup = None;
        }

        let morph_active = group.morph_count > 0;
        let morph_label = if morph_active {
            let morphable =
                crate::instrument_registry::morphable_fields(schema_voice_idx(params, inst));
            let names: Vec<&str> = group.morph_targets[..group.morph_count as usize]
                .iter()
                .map(|t| {
                    morphable
                        .iter()
                        .find(|f| f.field_index == t.field as usize)
                        .map(|f| f.label)
                        .unwrap_or("?")
                })
                .collect();
            format!("Morphing ({})", names.join(", "))
        } else {
            "Morphing".to_string()
        };
        if plock_menu_action_row(ui, &morph_label, PL_LINK()).clicked() {
            popup.morph_menu = true;
            state.plock_popup = Some(*popup);
        }
        if plock_menu_action_row(ui, "Edit Fusion Steps", PL_LINK()).clicked() {
            state.fusion_editing = Some((inst, idx));
            state.fusion_edit_steps = group.step_count;
            state.fusion_edit_focus_request = true;
            state.plock_popup = None;
        }
        if plock_menu_action_row(ui, "Delete Fusion", DANGER()).clicked() {
            let mut new_fusions = pattern.load_fusions(inst);
            if idx < new_fusions.len() {
                new_fusions.remove(idx);
                pattern.store_fusions(inst, &new_fusions);
            }
            state.mark_pattern_dirty();
            state.plock_popup = None;
        }
    });
}

pub fn draw_plock_popup(
    ctx: &egui::Context,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    pattern: &SharedPattern,
    sound_settings: &SoundSettingsState,
    plock: &PlockState,
    state: &mut EditorUIState,
) {
    let mut popup = match state.plock_popup {
        Some(p) => p,
        None => return,
    };

    let area_id = egui::Id::new("plock_popup");
    let response = egui::Area::new(area_id)
        .kind(egui::UiKind::Menu)
        .order(egui::Order::Foreground)
        .fixed_pos(popup.screen_pos)
        .sense(egui::Sense::click())
        .show(ctx, |ui| {
            // Outer border: draw a slightly larger rounded rect behind the panel.
            let content_response = egui::Frame::NONE
                .fill(P_ACTIVE())
                .corner_radius(RADIUS_PANEL)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.set_min_width(260.0);
                    ui.set_max_width(350.0);

                    let inst = popup.instrument;
                    let step = popup.step;
                    let fusions = pattern.load_fusions(inst);
                    let fusion_info = fusions.iter().enumerate().find(|(_, g)| {
                        (g.start_cell as usize) <= step && step <= (g.end_cell as usize)
                    });

                    if state.sequencer_mode {
                        if let Some((idx, group)) = fusion_info {
                            if popup.morph_menu {
                                draw_fusion_morph_menu(
                                    ui,
                                    pattern,
                                    sound_settings,
                                    params,
                                    setter,
                                    inst,
                                    idx,
                                    step,
                                    state,
                                );
                            } else {
                                // Fused cell: fusion actions (morph/edit/delete)
                                // on top, the seq-plock menu below â€” same as
                                // the sound-plock branch.
                                draw_fusion_group_menu(
                                    ui, pattern, params, inst, idx, *group, step, &mut popup,
                                    state,
                                );
                                ui.separator();
                                draw_sequencer_plock_menu(
                                    ui,
                                    pattern,
                                    params,
                                    setter,
                                    inst,
                                    step,
                                    popup.step_was_active,
                                    state,
                                    true,
                                );
                            }
                        } else {
                            draw_sequencer_plock_menu(
                                ui,
                                pattern,
                                params,
                                setter,
                                inst,
                                step,
                                popup.step_was_active,
                                state,
                                false,
                            );
                        }
                    } else {
                        if let Some((idx, group)) = fusion_info {
                            if popup.morph_menu {
                                draw_fusion_morph_menu(
                                    ui,
                                    pattern,
                                    sound_settings,
                                    params,
                                    setter,
                                    inst,
                                    idx,
                                    step,
                                    state,
                                );
                            } else {
                                draw_fusion_group_menu(
                                    ui, pattern, params, inst, idx, *group, step, &mut popup,
                                    state,
                                );
                                ui.separator();

                                // Also show the source-step plock menu below.
                                draw_plock_menu(
                                    ui,
                                    pattern,
                                    plock,
                                    sound_settings,
                                    params,
                                    setter,
                                    inst,
                                    step,
                                    popup.step_was_active,
                                    state,
                                );
                            }
                        } else {
                            draw_plock_menu(
                                ui,
                                pattern,
                                plock,
                                sound_settings,
                                params,
                                setter,
                                inst,
                                step,
                                popup.step_was_active,
                                state,
                            );
                        }
                    }
                })
                .response;

            let border_rect = content_response.rect.expand2(egui::Vec2::new(1.0, 1.0));
            ui.painter().rect_stroke(
                border_rect,
                RADIUS_PANEL + 1.0,
                egui::Stroke::new(1.0, LINE2()),
                egui::StrokeKind::Inside,
            );
            content_response
        })
        .response;

    // Close popup on click outside.
    if response.clicked_elsewhere() {
        state.plock_popup = None;
    }

    // Close popup on click in the popup border/padding (consume the click so it
    // does not pass through to the step cell underneath).
    if state.plock_popup.is_some() && response.clicked() {
        state.plock_popup = None;
    }
}

fn draw_sequencer_plock_menu(
    ui: &mut egui::Ui,
    pattern: &SharedPattern,
    params: &DrumFlashParams,
    _setter: &ParamSetter,
    instrument: usize,
    step: usize,
    step_was_active: bool,
    state: &mut EditorUIState,
    stutter_disabled: bool,
) {
    use crate::plock::{SequencerStepParams, StepCondition};

    #[allow(non_snake_case)] let ACCENT: Color32 = SEQPL();
    // `instrument` is a SLOT index; the label comes from the slot's voice schema.
    let inst_def = &crate::instrument_registry::INSTRUMENTS[schema_voice_idx(params, instrument)];
    let title = format!("Seq Plock {}", inst_def.label);

    preserve_step_active_from_plock_popup(pattern, state, instrument, step, step_was_active);

    plock_menu_frame(ui, ACCENT, |ui| {
        if plock_menu_header(ui, &title, step, ACCENT) {
            state.plock_popup = None;
        }

        let seq_plock = &params.seq_plock_state.state;
        let has_seq_plock = seq_plock.is_active(instrument, step);
        let current = seq_plock.get(instrument, step).unwrap_or_default();
        let mut changed_this_frame = false;

        // Mode indicator
        ui.horizontal(|ui| {
            ui.label(RichText::new("Mode").font(f_sans_med(10.0)).color(INK3()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mode_text = if has_seq_plock { "Active" } else { "Inactive" };
                ui.label(
                    RichText::new(mode_text)
                        .font(f_mono_med(10.0))
                        .color(ACCENT),
                );
            });
        });
        ui.add_space(8.0);

        // Solo â€” mutes every other lane ONLY while the playhead sits on this
        // cell (its step, or the whole span of a fused cell). Per-cell toggle;
        // remove it by toggling the same cell off. Turning it off clears the
        // whole seq-plock when solo was the only thing set, so a solo-only cell
        // does not linger as an empty (paramless) seq-plock.
        {
            let solo_now = current.solo;
            let solo_response = plock_menu_row(ui, "Solo", ACCENT, solo_now, None, |ui| {
                ui.add(crate::ui::widgets::ToggleSwitch::new(solo_now))
            });
            if solo_response.clicked() {
                if solo_now {
                    seq_plock.set_solo(instrument, step, false);
                    if let Some(p) = seq_plock.get(instrument, step) {
                        let no_other_params = p.probability == 1.0
                            && p.stutter_count == 1
                            && p.condition == StepCondition::Always
                            && p.microtiming_ms == 0.0;
                        if no_other_params {
                            seq_plock.clear(instrument, step);
                        }
                    }
                } else {
                    seq_plock.set_solo(instrument, step, true);
                }
                changed_this_frame = true;
            }
        }
        ui.add_space(4.0);

        // Probability
        {
            let mut prob = current.probability;
            let prob_text = format!("{:.0}%", prob * 100.0);
            let prob_response = plock_menu_row(
                ui,
                "Probability",
                ACCENT,
                has_seq_plock,
                Some(&prob_text),
                |ui| {
                    ui.add(
                        LocalParamSlider::new(&mut prob, 0.0..=1.0)
                            .with_width(86.0)
                            .without_value()
                            .reset_value(1.0),
                    )
                },
            );
            if prob_response.changed() {
                seq_plock.set_probability(instrument, step, prob);
                changed_this_frame = true;
            }
        }

        // Stutter
        if stutter_disabled {
            if has_seq_plock && current.stutter_count != 1 {
                let mut fixed = current;
                fixed.stutter_count = 1;
                seq_plock.set(instrument, step, &fixed);
            }
            plock_menu_row(ui, "Stutter", ACCENT, false, None, |ui| {
                ui.label(
                    RichText::new("disabled on fusion")
                        .font(f_sans_med(10.0))
                        .color(INK3()),
                );
                ui.allocate_response(Vec2::new(1.0, 1.0), egui::Sense::hover())
            });
        } else {
            let mut stutter = current.stutter_count.max(1) as f32;
            let stutter_text = format!("{}x", stutter as i32);
            let stutter_response = plock_menu_row(
                ui,
                "Stutter",
                ACCENT,
                has_seq_plock && current.stutter_count != 1,
                Some(&stutter_text),
                |ui| {
                    ui.add(
                        LocalParamSlider::new(&mut stutter, 1.0..=16.0)
                            .with_width(86.0)
                            .without_value()
                            .reset_value(1.0),
                    )
                },
            );
            if stutter_response.changed() {
                let new_stutter = stutter.round() as u8;
                seq_plock.set_stutter(instrument, step, new_stutter);
                changed_this_frame = true;
            }
        }

        // Microtiming (nudge): shifts the whole cell (stutter/fusion pulses
        // included) by -100..+100 ms around its step boundary.
        {
            let mut nudge = current.microtiming_ms.clamp(-100.0, 100.0);
            let nudge_text = if nudge.abs() < 0.5 {
                "0 ms".to_string()
            } else {
                format!("{:+.0} ms", nudge)
            };
            let nudge_response = plock_menu_row(
                ui,
                "Nudge",
                ACCENT,
                has_seq_plock && current.microtiming_ms != 0.0,
                Some(&nudge_text),
                |ui| {
                    ui.add(
                        LocalParamSlider::new(&mut nudge, -100.0..=100.0)
                            .with_width(86.0)
                            .without_value()
                            .reset_value(0.0),
                    )
                },
            );
            if nudge_response.changed() {
                seq_plock.set_microtiming(instrument, step, nudge);
                changed_this_frame = true;
            }
        }

        // Condition
        ui.add_space(8.0);
        ui.label(RichText::new("Condition").font(f_sans_sb(10.0)).color(INK2()));
        ui.add_space(6.0);

        let all_conditions = StepCondition::all();
        let grid_id = format!("condition_grid_{}_{}", instrument, step);
        let available_w = ui.available_width();
        let button_w = (available_w - 16.0) / 3.0;
        egui::Grid::new(grid_id)
            .num_columns(3)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                for (idx, cond) in all_conditions.iter().copied().enumerate() {
                    let selected = current.condition == cond;
                    let text_color = if selected { ACCENT } else { INK2() };
                    let fill = if selected { PANEL2() } else { PANEL2() };
                    let stroke_color = if selected { ACCENT } else { LINE2() };
                    if ui
                        .add_sized(
                            Vec2::new(button_w.max(1.0), 26.0),
                            egui::Button::new(
                                RichText::new(cond.label())
                                    .font(f_sans_med(9.5))
                                    .color(text_color),
                            )
                            .fill(fill)
                            .stroke(egui::Stroke::new(1.0, stroke_color))
                            .corner_radius(RADIUS_CTL),
                        )
                        .clicked()
                    {
                        seq_plock.set_condition(instrument, step, cond);
                        changed_this_frame = true;
                    }
                    if (idx + 1) % 3 == 0 {
                        ui.end_row();
                    }
                }
            });

        // Actions
        ui.add_space(8.0);
        if has_seq_plock || changed_this_frame {
            if plock_menu_action_row(ui, "Clear Seq Plock", DANGER())
                .clicked()
            {
                seq_plock.clear(instrument, step);
            }
        } else {
            if plock_menu_action_row(ui, "Create Seq Plock", ACCENT).clicked() {
                seq_plock.set(instrument, step, &SequencerStepParams::default());
            }
        }
    });
}

