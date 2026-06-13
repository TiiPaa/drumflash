# Flash Drum — UI Redesign : passation & notes d'implémentation

> À lire **avant toute modification de l'UI**. Synthèse des décisions, des pièges déjà rencontrés
> (à ne pas reproduire) et de l'état d'avancement. Source de vérité visuelle : la maquette du designer.

---

## 0. Périmètre (lire en premier)

- **Refonte VISUELLE** du plugin existant à **13 voix fixes**, rendu à l'identique de la maquette.
  Ce n'est **pas** la refonte modulaire (lanes assignables / moteurs Sample / Sample FX / MIDI Out /
  song arranger) — ça, c'est une **phase B ultérieure**. On dessine contre les 13 instruments actuels.
- **Transport (play/stop/rec) retiré du header volontairement** (un VST suit le transport de l'hôte),
  même si `LAYOUT.md` le marque 🔒. **Ne pas le réintroduire.**
- `src/ui/schema.rs` et `src/ui/engine_registry.rs` ont été **supprimés** (stubs morts/désync). Ne pas
  les ressusciter ; le futur schéma modulaire dérivera de `fd-data.js` + `instrument_registry.rs`.

## 1. Source de vérité & règles de fidélité

- **La maquette rendue est la vérité pixel** : `design-pack/Flash_Drum_design_11062026/flash-drum-source/`
  → `index.html` + `assets/fd-base.css` + `assets/fd-core.js`. Là où `DESIGN-SYSTEM.md`/`LAYOUT.md`
  contredisent le CSS/JS rendu, **la maquette gagne**.
- Tokens couleur dans `src/ui/theme.rs` (alignés au hex de `fd-base.css`). **Les utiliser**, ne jamais
  coder une couleur en dur.
- **Polices** : 7 faces IBM Plex embarquées. Utiliser les helpers de graisse `f_sans` / `f_sans_med` /
  `f_sans_sb` / `f_sans_bold` / `f_mono` / `f_mono_med` / `f_mono_sb` (dans `theme.rs`).
  **Jamais de faux-gras via `.strong()`** — toujours la vraie famille de graisse.
- Convention **« chiffre = mono, mot = sans »** : valeurs/nombres/IDs en mono, mots en sans.
- **Title Case** partout. Système de contrôle coordonné : h26, r6, bordure 1px LINE2, fond PANEL2,
  hover = bordure BLUE + texte INK, actif = fond BLUE + #fff.

## 2. Pièges DÉJÀ rencontrés — NE PAS REPRODUIRE

### Rendu egui
- **egui n'a pas de flou.** Ne PAS imiter un `box-shadow`/glow par un rectangle translucide agrandi :
  sur des cellules adjacentes (charley en doubles-croches) les halos à bords durs se chevauchent et
  **bavent**. → Aplats vifs + bordure nette 1px. (Rejeté par l'utilisateur sur la grille.)
- **Largeurs flex dans des rangées partagées.** Un slider dont la piste prend `ui.available_width()`
  **mange l'espace d'un voisin inline** : ça a fait **disparaître le graphe ADSR** (ENV/Filter) et
  étiré les sliders. → Quand une section a un graphe inline (ENV, Filter-avec-env), **contraindre la
  largeur de la colonne de params** (`ui.set_max_width(...)`) pour réserver ~196px + un gap de 16px au
  graphe ; le slider flexe alors dans la colonne contrainte. Les rangées isolées (ex. Volume) doivent
  être contraintes à la **même largeur** que les sections, sinon incohérence.
- **`ui.add_sized(W, Label)` CENTRE le label.** Pour des labels de formulaire alignés à gauche, utiliser
  `allocate_ui_with_layout(size, Layout::left_to_right(Align::Center), …)` — voir le helper `editor_label`.
- **Masquer les sections vides.** Sauter une famille de params sans contenu pour l'instrument courant
  (pas de titre orphelin — ex. Saturation sur l'OpenHiHat).
- `egui::CornerRadius` / `egui::Margin` ont des champs entiers (u8/i8) selon l'API ;
  `Color32::from_rgba_unmultiplied` **n'est pas `const`** → pour une `const`, utiliser
  `from_rgba_premultiplied` avec des valeurs prémultipliées.

### Build / PowerShell (Windows, PS 5.1)
- **Toujours lancer `build.ps1 -Install` EN CLAIR, au premier plan.** Ne **jamais** le piper ni rediriger :
  `2>&1 | …` ET `2>$null` le cassent — PS 5.1 emballe le stderr de cargo en `NativeCommandError` et avorte
  le pipeline. Pire : un `build.ps1 … 2>$null` lancé en arrière-plan a fait tourner **deux `cargo`
  concurrents bloqués sur le verrou du dossier build** (0 % CPU pendant 32 min). Si un build semble figé :
  `Get-Process cargo,rustc` (vérifier CPU/StartTime), tuer, relancer en clair.
- **Studio One verrouille la DLL installée.** Il doit être **complètement fermé** avant `-Install`, sinon la
  copie échoue en `Accès refusé`. (`cargo check`/`cargo build` vers `target/` marchent SO ouvert ; seule
  l'install système exige SO fermé.)
- **Chemins absolus.** Le répertoire courant peut dériver : passer `--manifest-path "E:\…\drum-pattern-vst\Cargo.toml"`
  et appeler `build.ps1` par son chemin absolu.
- Après un changement de comportement : build+install puis **entrée CHANGELOG** avec le build ID.

### Boucle de validation
- L'UI egui **ne se screenshote pas en headless**. Après chaque changement visuel :
  `build.ps1 -Install` (SO fermé) → l'utilisateur ouvre Studio One → capture → on itère.
  Travailler **par zone**, vérifier au `cargo check` (rapide) avant l'install release.

## 3. État d'avancement (fait)

- Nettoyage ~1300 lignes de code mort ; suppression `schema.rs` + `engine_registry.rs`.
- Polices IBM Plex multi-graisses + `Visuals` egui globales (coins r6, bordures, hover).
- **Header** (transport retiré, Master/Swing sliders pilule, Seq segmented LED, Choke/Auto-Edit LED).
- **Grille** (états de cellule plats + bordures, anneau playhead, tags M/S/T, poignée de drag, en-têtes).
- **Page-bar** (slider Len fin, Follow plein, LED page).
- **Sound Editor** (rangées slider/switch, en-tête, layout de sections à largeur contrainte, graphe ADSR
  espacé, labels alignés à gauche, sections vides masquées, Mix en switch).
- **Select stylé** (`.selbox`) appliqué aux anciens `ComboBox` : Saturation Type, Click Type, Noise Type,
  Algorithm, Groove, Generator type/A/B.
- **Graphe ADSR** passé au modèle maquette (3 segments colorés, grille, labels A/D/S/R dans le cadre).
- **Grille** : tooltips custom Hum/Push, reset double-clic Hum/Push, switch P-Lock Mode custom.
- **Playhead Push/Pull** : playhead alignée sur `current_step` global. Push/Pull décale le timing audio, pas la grille visuelle.

## 4. Reste à faire (par priorité)

1. **Panneau bas Patterns / Generator** (slots P1-8, chips Export/Drag MIDI, segmented Generator|Song).
2. **Menus clic-droit p-lock** → style `.plk` (largeur 284, P_ACTIVE, r9, ombre), Volume en tête, ↺ undo
    par rangée, toggle de mode Sound=orange / Sequencer=violet.
3. **Menu page Copy/Paste/Clear** → à **recâbler sur la page-bar** (helpers conservés sous
    `#[allow(dead_code)]` : `clear_page_fusions_for_ui`, `replace_page_fusions_for_ui`).
4. **Animations** (.14s hover/toggle) — basse priorité (demande utilisateur).
5. **Nettoyage** : adopter `widgets::StyledButton` pour le hover des boutons chrome ; supprimer le code mort
    restant (`design_system.rs` non câblé, `StyledButton`/`SegmentedControl` si non adoptés) ; remplacer
    `allocate_ui_at_rect` (déprécié) par `allocate_new_ui`.

## 5. Repères de cotes (maquette, fenêtre 1480×800)

- Corps : `grid-template-columns: 1fr 568px` (colonne droite = Sound Editor, 568).
- Sound Editor : padding scroll 14px ; label de rangée 138px ; piste slider h6 r6 ; valeur mono à droite.
- Section ENV/Filter : colonne params contrainte + gap 16 + graphe ~196px (cadre #0c0c11, r7).
- Menu p-lock : 284px. Cellule de step : h21 r4, gap 3, 16 colonnes/page.
