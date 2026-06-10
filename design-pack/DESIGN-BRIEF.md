# Flash Drum — Design Pack pour Designer UI

## Vue d'ensemble

Flash Drum est un plugin VST3 de drum sequencer en Rust (framework `nih-plug` + `egui`).
La fenêtre du plugin fait **1480×800px** (fixe, non redimensionnable).

L'interface actuelle est fonctionnelle mais graphiquement basique (widgets egui standards). Le but est de la moderniser avec un design cohérent et professionnel.

---

## Structure de l'interface

### Layout général (1480×800)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ HEADER BAR (hauteur ~40px)                                                  │
│ [FLASH DRUM v0.1.0 · build] | Vol | Swing | Groove | [Seq] [Choke] [Auto-E] │
├─────────────────────────────────────────────────────────────────────────────┤
│ PATTERN BANK BAR (~35px)                                                    │
│ [Save] [Load] P1 P2 P3 P4 P5 P6 P7 P8 | Clear | Generate | Random | Drag    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  MAIN CONTENT                                                               │
│  ┌──────────────────────┐ ┌─────────────────────────────────────────────┐  │
│  │                      │ │                                             │  │
│  │   SEQUENCER GRID     │ │   SOUND EDITOR / PATTERN EDITOR            │  │
│  │   (colonne gauche)   │ │   (colonne droite)                         │  │
│  │   largeur: ~580px    │ │   largeur: ~850px                          │  │
│  │                      │ │                                             │  │
│  │   13 instruments     │ │   Paramètres de synthèse par instrument    │  │
│  │   16 steps visibles  │ │   ou éditeur de pattern/song               │  │
│  │   (pages 1-4)        │ │                                             │  │
│  │                      │ │                                             │  │
│  └──────────────────────┘ └─────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 1. Header Bar

**Contenu actuel (gauche → droite) :**
- **Brand** : "FLASH DRUM" (gras, 15px) + version/build (monospace, 10px, gris)
- **Master Volume** : slider `-inf dB` à `+6.0 dB` (largeur 80px)
- **Swing** : slider 0-100% (largeur 80px)
- **Groove Type** : combo (Straight / Swing16 / Shuffle / MPC)
- **Toggles** : [Seq] [Choke] [Auto-Edit] [Song] — checkboxes

**Problèmes :**
- Trop d'éléments, manque d'hiérarchie visuelle
- Les toggles sont des checkboxes basiques
- Pas d'indicateur visuel de playback (play/stop/record)

---

## 2. Pattern Bank Bar

**Contenu actuel :**
- **Save / Load** : boutons pour sauvegarder/charger des patterns
- **Slots P1-P8** : boutons de slot avec indicateur d'occupation (point coloré)
- **Clear** : efface la grille courante
- **Generate** : génère un pattern aléatoire
- **Random** : randomise les paramètres
- **Drag** : export MIDI drag & drop

**Couleurs actuelles des slots :**
- Slot vide : fond sombre, texte gris
- Slot occupé : fond légèrement plus clair, point vert/bleu
- Slot actif (chargé) : bordure/bouton en surbrillance

---

## 3. Sequencer Grid (colonne gauche, ~580px)

### Structure
- **13 lignes** = 13 instruments (Kick, Snare, HiHat, OpenHH, Tom1, Tom2, Tom3, Clap, Ride, Cymbal, Snare606, BassDrum808, Perc1)
- **16 colonnes** visibles par page (4 pages = 64 steps max)
- **Hauteur cellule** : ~22px
- **Largeur cellule** : ~20px + 6px gap

### Labels d'instruments (colonne de gauche)
- Texte court : "BD", "SD", "HH", "OH", "T1", "T2", "T3", "CP", "RD", "CY", "S6", "B8", "P1"
- Largeur : ~35px

### Contrôles par lane (à droite des labels)
- **Len** : longueur de la lane (DragValue 1-64)
- **Vol** : volume local (slider dB)
- **Push** : timing push/pull
- **Humanize** : humanisation

### États visuels des cellules

**Mode Sound (par défaut) :**
| État | Couleur | Texte |
|------|---------|-------|
| Step vide (pair) | `rgb(32, 32, 32)` | "." |
| Step vide (impair) | `rgb(40, 40, 40)` | "." |
| Step actif | `rgb(56, 132, 255)` | "X" |
| Step actif + plock snapshot | `rgb(220, 50, 50)` | "X" |
| Step actif + plock link | `rgb(255, 140, 0)` | "X" |
| Step inactif + plock snapshot | `rgb(160, 30, 30)` | "." |
| Step inactif + plock link | `rgb(180, 100, 0)` | "." |
| Step courant (playback) | `rgb(48, 48, 48)` | "." ou "X" |

**Mode Sequencer (toggle violet) :**
- Seq-plock actif : `rgb(168, 85, 187)` (violet)
- Step actif + seq-plock : `rgb(168, 85, 247)`

**Step Fusion (fusion de cellules) :**
- Groupe fusionné : bordure bleue, fond légèrement différent
- Cellule de départ : affiche le nombre de pulses (ex: "3")
- Cellules internes : vides, bordure fine

### Pagination
- Boutons [1] [2] [3] [4] sous la grille
- Page active : fond bleu
- Page en lecture : LED rouge sous le bouton
- Slider "Len" global : 1-64 steps (à côté des boutons de page)
- Boutons rapides : 16 / 32 / 48 / 64
- Bouton "×2" : double la longueur (grisé si >32)

### Interactions
- **Clic gauche** : toggle step on/off
- **Clic droit** : menu contextuel (plock sound ou seq-plock selon mode)
- **Shift+clic** : sélection plage pour Step Fusion
- **Double-clic** sur fusion : édition inline du nombre de pulses

---

## 4. Sound Editor (colonne droite, ~850px)

### Structure
- **Titre** : nom de l'instrument courant (ex: "Kick")
- **Onglets** : [Sound] [Pattern] [Song] (si Song mode activé)
- **ScrollArea vertical** pour les paramètres

### Sections de paramètres (par famille)

**OSC (Oscillateur) :**
- Frequency / Note (avec switch Hz/Notes pour BD)
- Algo (combo : Sine/Square/FM/etc.)
- Special params (varie par instrument)

**ENV (Enveloppe) :**
- Attack, Hold, Decay, Release
- Decay Curve, Release Curve
- Visualisation graphique AHDSR (enveloppe amplitude + filtre)

**FILTER :**
- Filter Frequency
- Filter Env Amount
- Filter Env Decay

**OUTPUT :**
- Volume (slider principal en haut)
- Pan / Stereo width
- Analog (drift)
- Saturation (Type, Amount, Mix, Output Gain, Pre-Filter)

### Visualisations
- **Enveloppe amplitude** : graphique interactif AHDSR
- **Enveloppe filtre** : graphique interactif AD
- Les graphs sont à droite des paramètres (layout horizontal)

---

## 5. Menus contextuels (clic droit sur cellule)

### Menu Plock Sound
```
Plock Kick — Step 5
─────────────────────
Mode: Link to global
─────────────────────
[Volume slider]          ← EN PREMIER (le plus utilisé)
─────────────────────
Freq      [────●───] Undo
Decay     [──●────] Undo
Filter    [────●──] Undo
Attack    [●──────] Undo
Release   [──●────] Undo
... (tous les params)
─────────────────────
[Copy Plock] [Paste Plock]
```

### Menu Plock Sequencer
```
Seq Plock Kick — Step 5
─────────────────────
Probability  [────●──] Undo
Stutter      [─●─────] Undo
Condition    [Always ▼] Undo
Micro-timing [────●──] Undo
```

---

## Palette de couleurs actuelle

```rust
bg:           rgb(10, 10, 15)      // Fond global très sombre
panel:        rgb(20, 20, 25)      // Panneaux
panel2:       rgb(28, 28, 36)      // Panneaux secondaires
panel_hover:  rgb(36, 36, 48)      // Hover
panel_active: rgb(42, 42, 56)      // Actif
line:         rgb(42, 42, 53)      // Lignes
line2:        rgb(58, 58, 72)      // Lignes secondaires
divider:      rgb(31, 31, 40)      // Séparateurs
blue:         rgb(74, 158, 255)    // Bleu principal (steps actifs)
blue_dim:     rgba(74, 158, 255, 128)
blue_glow:    rgba(74, 158, 255, 64)
green:        rgb(74, 222, 128)    // Vert (validation)
red:          rgb(248, 113, 113)   // Rouge (erreurs, snapshot plock)
amber:        rgb(251, 191, 36)    // Ambre/orange (link plock)
ink:          rgb(232, 232, 240)   // Texte principal
ink2:         rgb(156, 163, 175)   // Texte secondaire
ink3:         rgb(107, 114, 128)   // Texte tertiaire
ink_faint:    rgb(75, 85, 99)      // Texte très faint
ink_blue:     rgb(74, 158, 255)    // Texte bleu
```

**Thème général :** Dark mode, fond presque noir, accents bleus.

---

## Éléments graphiques nécessaires

### Icônes / Boutons
- **Play / Stop / Pause** : indicateurs de transport
- **Save / Load** : disquette / dossier
- **Generate / Random / Clear** : dés / aléatoire / croix
- **Drag MIDI** : icône de drag
- **Step Fusion** : icône de groupe/link
- **Plock indicators** : petites icônes pour snapshot vs link

### Widgets custom souhaités
- **Step cells** : plus visuelles (formes, ombres, états animés)
- **Sliders** : style minimaliste avec valeur en hover
- **Toggles** : switches au lieu de checkboxes
- **Combo boxes** : style moderne
- **Enveloppe visualization** : plus belle, avec poignées interactives
- **LEDs** : indicateurs de playback (page active, step courant)

### Animations / Feedback
- **Step courant** : pulse léger ou glow pendant la lecture
- **Plock actif** : micro-indicateur sur la cellule (point coloré)
- **Hover** : transition douce
- **Drag & drop** : feedback visuel

---

## Problèmes UX identifiés

1. **Header bar trop chargée** — manque d'hiérarchie
2. **Checkboxes pour toggles** — devraient être des switches
3. **Grid visuellement plate** — manque de profondeur, d'ombres
4. **Sound Editor scroll** — fonctionne mais pas très élégant
5. **Menus contextuels** — basiques, pourraient être plus riches
6. **Pas d'icônes** — tout est texte
7. **Indicateurs de playback** — LED rouge sous page = pas très visible
8. **Labels d'instruments** — texte court sur fond sombre, pas très lisible
9. **Fusion steps** — bordures visibles mais pas très esthétiques
10. **Volume en dB** — affichage correct mais slider standard

---

## Contraintes techniques

- Framework : **egui** (Rust immediate-mode GUI)
- Pas de chargement d'images/textures externes (simplifie le déploiement)
- Tout doit être dessiné avec les primitives egui (rectangles, cercles, texte, lignes)
- Fenêtre fixe 1480×800
- Pas d'allocations dynamiques dans l'audio thread
- Performance : 60fps minimum

---

## Inspirations possibles

- **Elektron Digitakt / Analog Rytm** : grille de steps, parameter locks
- **Ableton Drum Rack** : layout instrument + paramètres
- **Native Instruments Battery** : cellules visuelles
- **Roland TR-8S** : esthétique hardware
- **Xfer Nerve** : éditeur de patterns moderne

---

## Fichiers sources UI

- `drum-pattern-vst/src/ui.rs` — Interface principale (~3300 lignes)
- `drum-pattern-vst/src/ui/design_system.rs` — Design system (palette, typographie)
- `drum-pattern-vst/src/ui/local_param_slider.rs` — Slider custom
- `drum-pattern-vst/src/ui/envelope_viz.rs` — Visualisation d'enveloppes
- `drum-pattern-vst/src/ui/schema.rs` — Schéma de données UI (non utilisé actuellement)

---

## Notes pour le designer

1. **Le plugin est un instrument** — l'UX doit être rapide et intuitive pour la performance live
2. **Les parameter locks sont le cœur du produit** — ils doivent être très visibles et accessibles
3. **La grille de steps est l'élément principal** — elle doit être parfaitement lisible
4. **Dark theme obligatoire** — c'est un standard audio/DJ
5. **Pas de gradients complexes** — egui gère mieux les aplats avec ombres subtiles
6. **Animations subtiles** — pulse, glow, transitions douces (pas de flash)
7. **Accessibilité** — contrastes suffisants, tailles de texte lisibles

---

## Livrables attendus

Le designer doit fournir :

1. **Maquettes haute fidélité** — écrans principaux (grid, sound editor, menus)
2. **Spécifications de couleurs** — palette complète avec codes hex
3. **Spécifications de typographie** — polices, tailles, graisses
4. **Composants UI** — boutons, sliders, toggles, cellules, menus
5. **États interactifs** — hover, active, disabled, playback
6. **Assets vectoriels** — si icônes nécessaires (format SVG ou dessin egui)
7. **Guide d'animation** — timings, easing, comportements

---

*Pack généré le 2026-06-10 — Build actuel : 20260610-082223*
