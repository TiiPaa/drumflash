//! skeuo_theme.rs — Flash Drum, thème "Skeuo" pour egui.
//! Généré depuis SPEC-COMPUTED.md (computed styles du rendu navigateur, 1 px CSS = 1 pt egui).
//! Compagnon : skeuo_widgets.rs (dessin) + png/ (textures bakées).
//! Cible : egui >= 0.29. Sur 0.26–0.28 remplacer `CornerRadius` par `Rounding`.

#![allow(dead_code)]
use egui::Color32;

// ---------- Surfaces ----------
pub const WINDOW_BG_TOP: Color32 = Color32::from_rgb(42, 43, 48);   // fond fenêtre (dégradé vertical)
pub const WINDOW_BG_BOT: Color32 = Color32::from_rgb(30, 31, 35);
pub const HEADER_TOP: Color32 = Color32::from_rgb(61, 62, 68);      // barre 44 pt
pub const HEADER_MID: Color32 = Color32::from_rgb(43, 44, 49);      // stop 60%
pub const HEADER_BOT: Color32 = Color32::from_rgb(38, 39, 43);
pub const PANEL_BORDER: Color32 = Color32::from_rgb(18, 18, 21);
pub const WELL_TOP: Color32 = Color32::from_rgb(28, 29, 33);        // puits de grille (seqwrap)
pub const WELL_BOT: Color32 = Color32::from_rgb(33, 34, 38);
pub const KEYCAP_BORDER: Color32 = Color32::from_rgb(23, 23, 27);

// ---------- Texte ----------
pub const INK: Color32 = Color32::from_rgb(233, 234, 238);        // valeurs
pub const INK_LABEL: Color32 = Color32::from_rgb(169, 171, 180);  // labels de contrôle
pub const INK_HEAD: Color32 = Color32::from_rgb(143, 145, 155);   // titres de section / en-têtes grille
pub const INK_KEYCAP: Color32 = Color32::from_rgb(201, 203, 211); // texte keycap repos
pub const INK_FAINT: Color32 = Color32::from_rgb(84, 85, 94);     // tags M/S/T repos
pub const BRAND: Color32 = Color32::from_rgb(223, 225, 232);

// ---------- Accents ----------
pub const BLUE: Color32 = Color32::from_rgb(87, 190, 255);        // fill sliders (haut)
pub const BLUE_D: Color32 = Color32::from_rgb(47, 134, 196);      // fill sliders (bas), keycap bleu haut
pub const BLUE_PRESSED_BOT: Color32 = Color32::from_rgb(26, 84, 128);
pub const BLUE_BORDER: Color32 = Color32::from_rgb(13, 44, 68);
pub const AMBER_TOP: Color32 = Color32::from_rgb(201, 122, 30);   // segmented actif
pub const AMBER_BOT: Color32 = Color32::from_rgb(138, 78, 12);
pub const AMBER_BORDER: Color32 = Color32::from_rgb(61, 36, 4);
pub const AMBER_TEXT: Color32 = Color32::from_rgb(255, 232, 200);
pub const GEN_TOP: Color32 = Color32::from_rgb(255, 202, 85);     // GENERATE
pub const GEN_MID: Color32 = Color32::from_rgb(224, 154, 24);     // stop 55%
pub const GEN_BOT: Color32 = Color32::from_rgb(201, 134, 14);
pub const GEN_BORDER: Color32 = Color32::from_rgb(90, 60, 4);
pub const GEN_TEXT: Color32 = Color32::from_rgb(58, 37, 0);

// ---------- P-locks (états de pad — textures png/pad-*.png) ----------
pub const SEQPL: Color32 = Color32::from_rgb(168, 85, 247);       // violet seq-plock
pub const PLAYHEAD_STROKE: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 150);

// ---------- LCD (ADSR) ----------
pub const LCD_BG_EDGE: Color32 = Color32::from_rgb(14, 24, 16);
pub const LCD_BG_MID: Color32 = Color32::from_rgb(19, 32, 26);
pub const LCD_BORDER: Color32 = Color32::from_rgb(6, 10, 7);
pub const LCD_LEGEND: Color32 = Color32::from_rgb(90, 138, 104);
pub const ADSR_ATTACK: Color32 = Color32::from_rgb(251, 191, 36);
pub const ADSR_DECAY: Color32 = Color32::from_rgb(74, 158, 255);
pub const ADSR_RELEASE: Color32 = SEQPL;

// ---------- Géométrie ----------
pub const R_MICRO: f32 = 3.0;   // tags M/S/T, piste+capuchon slider
pub const R_PAD: f32 = 4.0;     // pads du grid, touche lane, écran LCD
pub const R_KEYCAP: f32 = 5.0;  // pages, slots, chips, selects, segmented, GENERATE, puits, blocs Song
pub const R_PLATE: f32 = 7.0;   // plaques, popups

pub const H_HEADER: f32 = 44.0;
pub const H_KEYCAP: f32 = 26.0;   // tous les boutons de barre
pub const H_ROW: f32 = 24.0;      // ligne de grille (contenu 21 + respiration)
pub const PAD_SIZE: egui::Vec2 = egui::Vec2::new(27.0, 21.0); // pad (largeur = colonne flex ~27)
pub const LANE_NAME: egui::Vec2 = egui::Vec2::new(52.0, 21.0);
pub const TAG_SIZE: f32 = 17.0;
pub const SLIDER_TRACK_H: f32 = 5.0;
pub const SLIDER_KNOB: egui::Vec2 = egui::Vec2::new(12.0, 19.0);
pub const MINISLD: egui::Vec2 = egui::Vec2::new(56.0, 5.0);
pub const SWITCH: egui::Vec2 = egui::Vec2::new(34.0, 18.0);
pub const LED_D: f32 = 8.0;
pub const ADSR_PANEL: egui::Vec2 = egui::Vec2::new(200.0, 124.0);
pub const GAP_CELL: f32 = 3.0;
pub const GAP_ROW: f32 = 7.0;

// ---------- Typo (IBM Plex, à charger via FontDefinitions) ----------
// Sans 400/500/600/700 + Mono 400/500/600. Convention : chiffres = Mono, mots = Sans.
pub const FS_VALUE: f32 = 11.0;    // Mono 500
pub const FS_LABEL: f32 = 11.5;    // Sans 500
pub const FS_KEYCAP: f32 = 11.0;   // Sans 600 (Mono 10.5 pour pages/slots)
pub const FS_SECTION: f32 = 10.0;  // Sans 600, ls 0.5
pub const FS_LANE: f32 = 10.0;     // Mono 600
pub const FS_STEPLAB: f32 = 9.0;   // Mono 500
