// =============================================================================
// Flash Drum — egui layout reference  (eframe / egui ~0.27)
// =============================================================================
// Ce fichier N'EST PAS le code de production : c'est une RÉFÉRENCE DE LAYOUT qui
// reproduit la maquette `index.html` au pixel près, pour régler les problèmes
// d'alignement en egui (mode immédiat).
//
// Règle d'or qui résout 90 % des décalages :
//   -> Les largeurs de colonnes sont des CONSTANTES partagées.
//      L'en-tête du séquenceur ET chaque ligne de piste utilisent EXACTEMENT
//      les mêmes largeurs. Si une seule diverge, toute la grille se décale.
//
// Brancher ensuite chaque cellule sur ton modèle (Lane / Step / params).
// Tokens couleur = DESIGN-SYSTEM.md §1.  Dimensions = ci-dessous (= fd-base.css).
// =============================================================================

use eframe::egui::{
    self, Color32, FontId, Pos2, Rect, Rounding, Sense, Stroke, Vec2,
};

// -----------------------------------------------------------------------------
// 1. TOKENS (cf. DESIGN-SYSTEM.md §1)
// -----------------------------------------------------------------------------
pub mod theme {
    use eframe::egui::Color32;
    pub const BG: Color32 = Color32::from_rgb(10, 10, 15);
    pub const PANEL: Color32 = Color32::from_rgb(20, 20, 25);
    pub const PANEL2: Color32 = Color32::from_rgb(28, 28, 36);
    pub const P_HOVER: Color32 = Color32::from_rgb(36, 36, 48);
    pub const P_ACTIVE: Color32 = Color32::from_rgb(42, 42, 56);
    pub const LINE: Color32 = Color32::from_rgb(42, 42, 53);
    pub const LINE2: Color32 = Color32::from_rgb(58, 58, 72);
    pub const DIVIDER: Color32 = Color32::from_rgb(31, 31, 40);

    pub const BLUE: Color32 = Color32::from_rgb(74, 158, 255);
    pub const BLUE_D: Color32 = Color32::from_rgb(47, 111, 208);
    pub const GREEN: Color32 = Color32::from_rgb(74, 222, 128);
    pub const RED: Color32 = Color32::from_rgb(248, 113, 113);
    pub const AMBER: Color32 = Color32::from_rgb(251, 191, 36);

    pub const PL_LINK: Color32 = Color32::from_rgb(255, 140, 0);
    pub const PL_LINK_DIM: Color32 = Color32::from_rgb(180, 100, 0);
    pub const PL_SNAP: Color32 = Color32::from_rgb(220, 50, 50);
    pub const PL_SNAP_DIM: Color32 = Color32::from_rgb(160, 30, 30);
    pub const SEQPL: Color32 = Color32::from_rgb(168, 85, 247);

    pub const INK: Color32 = Color32::from_rgb(232, 232, 240);
    pub const INK2: Color32 = Color32::from_rgb(156, 163, 175);
    pub const INK3: Color32 = Color32::from_rgb(107, 114, 128);
    pub const FAINT: Color32 = Color32::from_rgb(75, 85, 99);

    pub const STEP_OFF: Color32 = Color32::from_rgb(27, 27, 34);   // #1b1b22
    pub const STEP_BEAT: Color32 = Color32::from_rgb(35, 35, 44);  // #23232c

    pub fn blue_glow(a: u8) -> Color32 { Color32::from_rgba_unmultiplied(74, 158, 255, a) }
    pub fn white_a(a: u8) -> Color32 { Color32::from_white_alpha(a) }
}

// -----------------------------------------------------------------------------
// 2. DIMENSIONS (= fd-base.css / variation-a.html, au pixel)
// -----------------------------------------------------------------------------
pub mod dims {
    /// Fenêtre fixe.
    pub const WIN_W: f32 = 1480.0;
    pub const WIN_H: f32 = 800.0;

    /// Header.
    pub const HEADER_H: f32 = 44.0;
    pub const HEADER_PAD_X: f32 = 14.0;
    pub const HEADER_GAP: f32 = 14.0;

    /// Corps : colonne droite (éditeur) fixe, gauche prend le reste.
    pub const EDITOR_W: f32 = 568.0;
    pub const COL_L_PAD: f32 = 14.0;
    pub const COL_L_GAP: f32 = 16.0; // espace vertical entre modules

    /// Largeur fixe des labels de groupe (Page / Patterns / P-Lock Mode…)
    /// -> garantit que la 1re commande de chaque rangée s'aligne verticalement.
    pub const GROUP_LABEL_W: f32 = 84.0;

    /// Système de contrôles coordonné : tout "chrome" fait 26 de haut, r6.
    pub const CTL_H: f32 = 26.0;
    pub const CTL_RADIUS: f32 = 6.0;

    // --- Séquenceur : colonnes (CRITIQUE pour l'alignement) ---
    pub const SEQ_ROW_H: f32 = 24.0;
    pub const SEQ_HEAD_H: f32 = 16.0;
    pub const SEQ_GAP: f32 = 7.0; // gap horizontal entre TOUTES les colonnes
    pub const COL_GRIP: f32 = 14.0;
    pub const COL_NAME: f32 = 34.0;
    pub const COL_VOL: f32 = 56.0;
    pub const TAG_W: f32 = 17.0;
    pub const TAG_GAP: f32 = 3.0;
    pub const COL_MST: f32 = TAG_W * 3.0 + TAG_GAP * 2.0; // M S T = 57
    pub const COL_EXTRA: f32 = 44.0; // Hum / Push / Len (chacune)
    pub const STEP_GAP: f32 = 3.0;
    pub const STEP_H: f32 = 21.0;
    pub const STEP_RADIUS: f32 = 4.0;
    pub const STEPS: usize = 16; // pas visibles par page
    pub const TAG_RADIUS: f32 = 4.0;

    /// Largeur consommée à GAUCHE de la grille de pas (colonnes fixes + gaps).
    /// grip + name + vol + mst, séparés par SEQ_GAP, plus le gap avant les pas.
    pub fn left_block_w() -> f32 {
        COL_GRIP + SEQ_GAP + COL_NAME + SEQ_GAP + COL_VOL + SEQ_GAP + COL_MST + SEQ_GAP
    }
    /// Largeur consommée à DROITE (Hum/Push/Len), gap inclus avant chacune.
    pub fn right_block_w() -> f32 {
        (SEQ_GAP + COL_EXTRA) * 3.0
    }
    /// Largeur d'UNE cellule de pas pour une largeur de ligne donnée.
    /// Header et lignes DOIVENT appeler cette même fonction.
    pub fn step_cell_w(row_w: f32) -> f32 {
        let grid_w = row_w - left_block_w() - right_block_w();
        (grid_w - STEP_GAP * (STEPS as f32 - 1.0)) / STEPS as f32
    }
}

// -----------------------------------------------------------------------------
// 3. HELPERS DE CONTRÔLE COORDONNÉS  (hauteur 26, r6, états hover/on)
// -----------------------------------------------------------------------------

fn mono(sz: f32) -> FontId { FontId::monospace(sz) }
fn sans(sz: f32) -> FontId { FontId::proportional(sz) }

/// Rectangle peint + texte centré, sensible au clic. Brique de tous les boutons.
fn chrome_button(ui: &mut egui::Ui, w: f32, label: &str, on: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, dims::CTL_H), Sense::click());
    let hovered = resp.hovered();
    let (fill, stroke_col, text_col) = if on {
        (theme::BLUE, theme::BLUE, Color32::WHITE)
    } else if hovered {
        (theme::PANEL2, theme::BLUE, theme::INK)
    } else {
        (theme::PANEL2, theme::LINE2, theme::INK2)
    };
    let p = ui.painter();
    p.rect(rect, Rounding::same(dims::CTL_RADIUS), fill, Stroke::new(1.0, stroke_col));
    p.text(rect.center(), Align2_CENTER(), label, sans(11.0), text_col);
    resp
}

/// Toggle "pastille LED" (header) : Choke / Auto-Edit / Enabled…
fn toggle_led(ui: &mut egui::Ui, label: &str, on: bool) -> egui::Response {
    // largeur auto : LED 7 + gap 7 + texte
    let text_w = ui.fonts(|f| f.layout_no_wrap(label.to_string(), sans(11.0), theme::INK).size().x);
    let w = 12.0 + 7.0 + 7.0 + text_w + 12.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, dims::CTL_H), Sense::click());
    let (fill, stroke_col, text_col) = if on {
        (theme::blue_glow(40), theme::BLUE, theme::INK)
    } else {
        (theme::PANEL2, theme::LINE2, theme::INK2)
    };
    let p = ui.painter();
    p.rect(rect, Rounding::same(dims::CTL_RADIUS), fill, Stroke::new(1.0, stroke_col));
    let led_c = Pos2::new(rect.left() + 12.0 + 3.5, rect.center().y);
    p.circle_filled(led_c, 3.5, if on { theme::BLUE } else { theme::FAINT });
    p.text(
        Pos2::new(led_c.x + 3.5 + 7.0, rect.center().y),
        Align2_LEFT_CENTER(),
        label,
        sans(11.0),
        text_col,
    );
    resp
}

/// Segmented control (Sound|Sequencer, Internal|Ext MIDI, Generator|Song…).
/// Renvoie l'index cliqué, sinon None. `accent` = couleur du segment actif.
fn segmented(ui: &mut egui::Ui, options: &[&str], selected: usize, accent: Color32) -> Option<usize> {
    let mut clicked = None;
    let seg_w = 92.0; // largeur par segment (ajuster au besoin)
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let total = seg_w * options.len() as f32;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(total, dims::CTL_H), Sense::hover());
        let p = ui.painter();
        p.rect_stroke(rect, Rounding::same(dims::CTL_RADIUS), Stroke::new(1.0, theme::LINE2));
        for (i, opt) in options.iter().enumerate() {
            let seg_rect = Rect::from_min_size(
                Pos2::new(rect.left() + seg_w * i as f32, rect.top()),
                Vec2::new(seg_w, dims::CTL_H),
            );
            let id = ui.id().with(("seg", i));
            let resp = ui.interact(seg_rect, id, Sense::click());
            let on = i == selected;
            if on {
                let txt_col = if accent == theme::PL_LINK { Color32::from_rgb(26, 18, 6) } else { Color32::WHITE };
                p.rect_filled(seg_rect.shrink(1.0), Rounding::same(dims::CTL_RADIUS - 1.0), accent);
                p.text(seg_rect.center(), Align2_CENTER(), *opt, sans(11.0), txt_col);
            } else {
                p.text(seg_rect.center(), Align2_CENTER(), *opt, sans(11.0), theme::INK2);
            }
            if i > 0 {
                p.line_segment(
                    [Pos2::new(seg_rect.left(), rect.top() + 4.0), Pos2::new(seg_rect.left(), rect.bottom() - 4.0)],
                    Stroke::new(1.0, theme::LINE2),
                );
            }
            if resp.clicked() { clicked = Some(i); }
        }
    });
    clicked
}

/// Label de groupe à largeur fixe (Page / Patterns / P-Lock Mode…).
/// => c'est CE composant qui aligne verticalement la 1re commande des rangées.
fn group_label(ui: &mut egui::Ui, text: &str) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(dims::GROUP_LABEL_W, dims::CTL_H), Sense::hover());
    ui.painter().text(
        Pos2::new(rect.left(), rect.center().y),
        Align2_LEFT_CENTER(),
        text,
        sans(10.5),
        theme::INK3,
    );
}

// alias lisibles pour Align2 (egui::Align2)
#[allow(non_snake_case)]
fn Align2_CENTER() -> egui::Align2 { egui::Align2::CENTER_CENTER }
#[allow(non_snake_case)]
fn Align2_LEFT_CENTER() -> egui::Align2 { egui::Align2::LEFT_CENTER }

// -----------------------------------------------------------------------------
// 4. SÉQUENCEUR — alignement header ⇄ lignes (LE point sensible)
// -----------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
pub enum PlockKind { None, Link, Snapshot }

#[derive(Clone, Copy)]
pub struct StepView {
    pub hit: bool,
    pub plock: PlockKind,
    pub seq: bool,
    pub beat: bool,    // 1er pas d'un temps (i % 4 == 0)
    pub playing: bool, // sous la tête de lecture
    pub in_len: bool,  // dans la longueur du pattern (sinon grisé)
}

pub struct LaneView<'a> {
    pub tag: &'a str,
    pub selected: bool,
    pub empty: bool,
    pub mute: bool,
    pub solo: bool,
    pub trig: bool,
    pub vol: f32,       // 0..1
    pub hum: f32,       // 0..1
    pub push_ms: i32,
    pub len: u32,
    pub steps: [StepView; dims::STEPS],
}

/// Mode d'affichage des cellules (couleurs mutuellement exclusives).
#[derive(Clone, Copy, PartialEq)]
pub enum SeqMode { Sound, Sequencer }

/// En-tête de grille. Utilise EXACTEMENT les mêmes largeurs que `lane_row`.
fn seq_header(ui: &mut egui::Ui, row_w: f32, page_base: usize) {
    let cell_w = dims::step_cell_w(row_w);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = dims::SEQ_GAP;
        col_spacer(ui, dims::COL_GRIP);                  // grip
        col_spacer(ui, dims::COL_NAME);                  // name
        head_label(ui, dims::COL_VOL, "Vol");            // vol
        head_mst(ui);                                    // M S T
        // labels de pas
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = dims::STEP_GAP;
            for i in 0..dims::STEPS {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(cell_w, dims::SEQ_HEAD_H), Sense::hover());
                let n = page_base + i + 1;
                let col = if i % 4 == 0 { theme::INK2 } else { theme::FAINT };
                ui.painter().text(rect.center(), Align2_CENTER(), n.to_string(), mono(9.0), col);
            }
        });
        head_label(ui, dims::COL_EXTRA, "Hum");
        head_label(ui, dims::COL_EXTRA, "Push");
        head_label(ui, dims::COL_EXTRA, "Len");
    });
}

/// Une ligne de piste. `row_w` = largeur dispo identique à l'en-tête.
fn lane_row(ui: &mut egui::Ui, lane: &LaneView, mode: SeqMode, row_w: f32) -> egui::Response {
    let cell_w = dims::step_cell_w(row_w);
    let mut row_resp = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = dims::SEQ_GAP;

        // grip (drag pour réordonner — gérer dnd côté appelant)
        let (g, _) = ui.allocate_exact_size(Vec2::new(dims::COL_GRIP, dims::SEQ_ROW_H), Sense::drag());
        ui.painter().text(g.center(), Align2_CENTER(), "⠿", mono(12.0), theme::FAINT);

        // name
        let (nrect, nresp) = ui.allocate_exact_size(Vec2::new(dims::COL_NAME, 21.0), Sense::click());
        let (fill, txt) = if lane.selected {
            (theme::BLUE, Color32::WHITE)
        } else {
            (theme::PANEL2, if lane.empty { theme::FAINT } else { theme::INK2 })
        };
        let stroke = if lane.empty && !lane.selected {
            Stroke::new(1.0, theme::LINE2) // pointillé idéalement : approx plein ici
        } else {
            Stroke::NONE
        };
        ui.painter().rect(nrect, Rounding::same(5.0), fill, stroke);
        ui.painter().text(nrect.center(), Align2_CENTER(), lane.tag, mono(11.0), txt);
        row_resp = Some(nresp);

        // vol (mini-slider)
        minisld(ui, dims::COL_VOL, lane.vol, theme::BLUE);

        // M S T
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = dims::TAG_GAP;
            tag(ui, "M", lane.mute, theme::AMBER);
            tag(ui, "S", lane.solo, theme::GREEN);
            tag(ui, "T", lane.trig, theme::BLUE);
        });

        // pas
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = dims::STEP_GAP;
            for s in &lane.steps {
                step_cell(ui, cell_w, s, mode, lane.empty);
            }
        });

        // Hum / Push / Len
        minisld(ui, dims::COL_EXTRA, lane.hum, theme::FAINT);
        extra_num(ui, format!("{}{} ms", if lane.push_ms > 0 { "+" } else { "" }, lane.push_ms), lane.push_ms == 0);
        extra_num(ui, lane.len.to_string(), false);
    });
    row_resp.unwrap()
}

/// Une cellule de pas — UNE seule classe d'état (modes exclusifs).
fn step_cell(ui: &mut egui::Ui, w: f32, s: &StepView, mode: SeqMode, lane_empty: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, dims::STEP_H), Sense::click());
    let base = if s.beat { theme::STEP_BEAT } else { theme::STEP_OFF };
    let (fill, stroke_col) = match mode {
        SeqMode::Sequencer => {
            if s.seq && s.hit { (theme::SEQPL, theme::SEQPL) }
            else if s.seq { (base, theme::SEQPL) }
            else if s.hit { (theme::BLUE, theme::BLUE) }
            else { (base, theme::LINE) }
        }
        SeqMode::Sound => match (s.hit, s.plock) {
            (true, PlockKind::Link) => (theme::PL_LINK, theme::PL_LINK),
            (true, PlockKind::Snapshot) => (theme::PL_SNAP, theme::PL_SNAP),
            (true, PlockKind::None) => (theme::BLUE, theme::BLUE),
            (false, PlockKind::Link) => (Color32::from_rgb(36, 26, 8), theme::PL_LINK_DIM),
            (false, PlockKind::Snapshot) => (Color32::from_rgb(36, 16, 16), theme::PL_SNAP_DIM),
            (false, PlockKind::None) => (base, theme::LINE),
        },
    };
    let p = ui.painter();
    let mut f = fill;
    if lane_empty || !s.in_len {
        f = f.linear_multiply(0.30); // grisé : lane vide ou pas hors-longueur
    }
    p.rect(rect, Rounding::same(dims::STEP_RADIUS), f, Stroke::new(1.0, stroke_col));
    if s.playing {
        p.rect_stroke(rect.shrink(0.5), Rounding::same(dims::STEP_RADIUS), Stroke::new(1.5, theme::white_a(150)));
    }
    resp
}

// --- petits helpers de cellule ---
fn col_spacer(ui: &mut egui::Ui, w: f32) { ui.allocate_exact_size(Vec2::new(w, dims::SEQ_ROW_H), Sense::hover()); }
fn head_label(ui: &mut egui::Ui, w: f32, t: &str) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, dims::SEQ_HEAD_H), Sense::hover());
    ui.painter().text(rect.center(), Align2_CENTER(), t, sans(9.5), theme::INK3);
}
fn head_mst(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = dims::TAG_GAP;
        for t in ["M", "S", "T"] { head_label(ui, dims::TAG_W, t); }
    });
}
fn tag(ui: &mut egui::Ui, label: &str, on: bool, accent: Color32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(dims::TAG_W, dims::TAG_W), Sense::click());
    let (fill, stroke, txt) = if on { (accent, accent, Color32::from_rgb(26, 18, 6)) }
                              else { (theme::PANEL2, theme::LINE2, theme::FAINT) };
    ui.painter().rect(rect, Rounding::same(dims::TAG_RADIUS), fill, Stroke::new(1.0, stroke));
    ui.painter().text(rect.center(), Align2_CENTER(), label, mono(9.0), txt);
    resp
}
fn minisld(ui: &mut egui::Ui, w: f32, frac: f32, fill: Color32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 6.0), Sense::click_and_drag());
    let p = ui.painter();
    p.rect_filled(rect, Rounding::same(3.0), theme::PANEL2);
    let mut fr = rect;
    fr.set_width(rect.width() * frac.clamp(0.0, 1.0));
    p.rect_filled(fr, Rounding::same(3.0), fill);
    resp
}
fn extra_num(ui: &mut egui::Ui, text: String, dim: bool) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(dims::COL_EXTRA, dims::SEQ_ROW_H), Sense::hover());
    let col = if dim { theme::INK3 } else { theme::INK2 };
    ui.painter().text(rect.center(), Align2_CENTER(), text, mono(10.0), col);
}

// -----------------------------------------------------------------------------
// 5. ASSEMBLAGE GÉNÉRAL  (header / colonne gauche / éditeur)
// -----------------------------------------------------------------------------
pub struct FlashDrumApp {
    pub seq_mode: SeqMode,
    pub bottom_tab: usize,  // 0 = Generator, 1 = Song
    pub seq_source: usize,  // 0 = Internal, 1 = Ext MIDI
    pub page: usize,        // 0..3
    pub len: u32,
    pub follow: bool,
    pub choke: bool,
    pub auto_edit: bool,
    pub lanes: Vec<()>,     // -> remplacer par ton Vec<Lane>
}

impl eframe::App for FlashDrumApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Fond global.
        ctx.style_mut(|s| s.visuals.panel_fill = theme::BG);

        // --- HEADER (hauteur fixe 44) ---
        egui::TopBottomPanel::top("header")
            .exact_height(dims::HEADER_H)
            .frame(egui::Frame::none().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(dims::HEADER_PAD_X, 0.0)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = dims::HEADER_GAP;
                    ui.label(egui::RichText::new("FLASH DRUM").color(Color32::WHITE).font(sans(15.0)).strong());
                    // … Master / Swing / Groove (sliders) …
                    if let Some(i) = segmented(ui, &["Internal", "Ext MIDI"], self.seq_source, theme::BLUE) { self.seq_source = i; }
                    if toggle_led(ui, "Choke", self.choke).clicked() { self.choke = !self.choke; }
                    if toggle_led(ui, "Auto-Edit", self.auto_edit).clicked() { self.auto_edit = !self.auto_edit; }
                });
            });

        // --- ÉDITEUR : panneau DROIT fixe (568), AVANT le central ---
        egui::SidePanel::right("editor")
            .exact_width(dims::EDITOR_W)
            .resizable(false)
            .frame(egui::Frame::none().fill(theme::PANEL))
            .show(ctx, |ui| {
                ui.add_space(11.0);
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    ui.label(egui::RichText::new("Sound Editor").color(Color32::WHITE).font(sans(13.0)).strong());
                });
                // onglets instruments (grille) + scroll des sections de l'éditeur
                egui::ScrollArea::vertical().show(ui, |_ui| {
                    // for section in schema_for_engine(lane.engine) { render_section(ui, section); }
                });
            });

        // --- COLONNE GAUCHE : CentralPanel prend le reste ---
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::BG).inner_margin(egui::Margin::same(dims::COL_L_PAD)))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = dims::COL_L_GAP;

                // 1) Page / Length bar — label fixe 84 => alignement vertical
                ui.horizontal(|ui| {
                    group_label(ui, "Page");
                    for i in 0..4 {
                        let enabled = i < (self.len as usize).div_ceil(dims::STEPS);
                        ui.add_enabled_ui(enabled, |ui| {
                            if chrome_button(ui, 28.0, &(i + 1).to_string(), i == self.page).clicked() { self.page = i; }
                        });
                    }
                    // … Follow, Len slider, presets 16/32/48/64, ×2 …
                });

                // 2) Séquenceur — header puis lignes, MÊME row_w partout
                let row_w = ui.available_width();
                seq_header(ui, row_w, self.page * dims::STEPS);
                ui.add_space(3.0);
                // for lane in &self.lanes { lane_row(ui, &lane.view(), self.seq_mode, row_w); }

                // 3) P-Lock Mode bar — label fixe 84
                ui.horizontal(|ui| {
                    group_label(ui, "P-Lock Mode");
                    let sel = if self.seq_mode == SeqMode::Sound { 0 } else { 1 };
                    let accent = if sel == 0 { theme::PL_LINK } else { theme::SEQPL };
                    if let Some(i) = segmented(ui, &["Sound", "Sequencer"], sel, accent) {
                        self.seq_mode = if i == 0 { SeqMode::Sound } else { SeqMode::Sequencer };
                    }
                });

                // 4) Patterns bar — label fixe 84 (=> "Save" s'aligne sous "Sound"/page 1)
                ui.horizontal(|ui| {
                    group_label(ui, "Patterns");
                    let _ = chrome_button(ui, 56.0, "Save", false);
                    // … slots P1..P8 …
                });

                // 5) Panneau partagé Generator | Song
                ui.horizontal(|ui| {
                    if let Some(i) = segmented(ui, &["Generator", "Song"], self.bottom_tab, theme::PL_LINK) { self.bottom_tab = i; }
                });
                // if self.bottom_tab == 0 { generator_pane(ui) } else { song_pane(ui) }
            });
    }
}
