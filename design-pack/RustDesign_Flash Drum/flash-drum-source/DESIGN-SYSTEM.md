# Flash Drum — Design System (egui) · v2

> Aligné sur l'alpha 0.1.0 + le design pack dev. Briques atomiques et stables :
> tokens + widgets + visus. Cible : `theme.rs` (constantes) + `widgets.rs` (Painter).
> **Fenêtre : 1480 × 800 px, fixe.** Échelle 1.0 (px maquette = points egui).
> Réf. pixel : `variation-a.html` + `assets/fd-base.css` + `assets/fd-core.js`.
> Assemblage des écrans → `LAYOUT.md`.

---

## 1. Palette (= `design_system.rs`, recalée sur le build)

```rust
// theme.rs — surfaces
pub const BG:        Color32 = Color32::from_rgb(10, 10, 15);
pub const PANEL:     Color32 = Color32::from_rgb(20, 20, 25);
pub const PANEL2:    Color32 = Color32::from_rgb(28, 28, 36);
pub const P_HOVER:   Color32 = Color32::from_rgb(36, 36, 48);
pub const P_ACTIVE:  Color32 = Color32::from_rgb(42, 42, 56);
pub const LINE:      Color32 = Color32::from_rgb(42, 42, 53);
pub const LINE2:     Color32 = Color32::from_rgb(58, 58, 72);
pub const DIVIDER:   Color32 = Color32::from_rgb(31, 31, 40);

// accents
pub const BLUE:      Color32 = Color32::from_rgb(74, 158, 255);   // primaire / step actif
pub const BLUE_D:    Color32 = Color32::from_rgb(47, 111, 208);
pub const GREEN:     Color32 = Color32::from_rgb(74, 222, 128);   // solo / slot occupé / play
pub const RED:       Color32 = Color32::from_rgb(248, 113, 113);  // record / erreur
pub const AMBER:     Color32 = Color32::from_rgb(251, 191, 36);   // mute

// p-locks (cœur produit — voir §6)
pub const PL_LINK:     Color32 = Color32::from_rgb(255, 140, 0);  // link (actif)
pub const PL_LINK_DIM: Color32 = Color32::from_rgb(180, 100, 0);  // link (pas inactif)
pub const PL_SNAP:     Color32 = Color32::from_rgb(220, 50, 50);  // snapshot (actif)
pub const PL_SNAP_DIM: Color32 = Color32::from_rgb(160, 30, 30);  // snapshot (pas inactif)
pub const SEQPL:       Color32 = Color32::from_rgb(168, 85, 247); // seq-plock (violet)

// texte
pub const INK:       Color32 = Color32::from_rgb(232, 232, 240);
pub const INK2:      Color32 = Color32::from_rgb(156, 163, 175);
pub const INK3:      Color32 = Color32::from_rgb(107, 114, 128);
pub const FAINT:     Color32 = Color32::from_rgb(75, 85, 99);

pub fn blue_glow(a: u8) -> Color32 { Color32::from_rgba_unmultiplied(74, 158, 255, a) }
pub fn white_a(a: u8)   -> Color32 { Color32::from_white_alpha(a) }
```

> Pas de gradients : aplats + ombres/glow subtils (le brief le demande explicitement).

---

## 2. Typographie

IBM Plex Sans (UI) + IBM Plex Mono (valeurs/IDs/labels de step) à embarquer via `FontDefinitions`.
Convention : **chiffre = mono, mot = sans.**

> **Norme de casse (🔒) : Title Case partout** — labels de groupe (`Page`, `Len`, `Patterns`, `P-Lock Mode`, `Seq`), en-têtes de section éditeur (`Oscillator`, `Envelope`, `Filter`, `Saturation`, `Output`, `Sample`, `Pitch`, `Modulation`), en-têtes de colonne (`Vol`, `Hum`, `Push`, `Len`), labels de contrôle (`Click Type`, `Saturation Amount`…). Pas de capitales forcées (`text-transform`) ni de sentence-case. Exceptions : acronymes propres (`MIDI`, `LP`/`HP`, `CC`) et les micro-tags de piste M/S/T. Labels de groupe en **sans 600 10.5px, INK3**, alignés sur une largeur fixe (~84px) pour que la 1re commande de chaque rangée s'aligne verticalement.

| Élément | Police | px | Graisse | Couleur |
|---|---|---|---|---|
| Marque | Sans | 15 | 700 | #fff |
| Build/version | Mono | 9.5 | 500 | INK3 |
| Label de contrôle | Sans | 11.5 | 500 | INK2 |
| Valeur | Mono | 11 | 500 | INK |
| Titre de section éditeur | Mono | 10 | 600 | INK3 (UPPER) |
| Nom instrument (éditeur) | Sans | 13 | 700 | #fff |
| ID instrument / page / step | Mono | 9–11 | 600 | INK2 / FAINT |

---

## 3. Primitives

> ⚠️ **Valeurs du thème plat d'origine — périmées pour Skeuo.** Pour le thème retenu, utiliser `RADIUS.md` (pads = 4 px, keycaps = 5 px, plaques = 7 px) et `SPEC-COMPUTED.md`.

```rust
pub const RADIUS_CTL: f32 = 5.0;   // boutons, cellules, selects
pub const RADIUS_PANEL: f32 = 9.0;
pub const RADIUS_PILL: f32 = 7.0;  // toggles, page buttons
pub const STROKE_HAIR: f32 = 1.0;
pub const STROKE_CURVE: f32 = 2.0;
pub const GAP_TIGHT: f32 = 3.0;    // cellules / lignes
pub const GAP_SM: f32 = 4.0;
pub const GAP_MD: f32 = 8.0;
pub const GAP_LG: f32 = 10.0;
```

---

## 4. Widgets

> **Système de contrôles coordonné (🔒) :** tout contrôle de "chrome" (bouton, sélecteur, segment, toggle, onglet, slot, chip) partage : **hauteur 26px**, **rayon 6px**, **bordure 1px LINE2**, fond PANEL2, texte INK2. Label-mot = **600 11px sans** ; label numérique/code = **600 10.5px mono**. **Hover** = bordure BLUE + texte INK ; **Actif/on** = fond BLUE + #fff + bordure BLUE. Transition .14s. Exceptions assumées : micro-tags M/S/T de la grille (17×17), cellules de pas, et la rangée « + Add module » (variante pointillée). États (repos / hover / actif / désactivé) : tableau §7.

- **Slider** : track h6 r6 PANEL2 (creux), fill BLUE, poignée Ø11 `#eef2f8` (apparaît au hover). Label gauche ~138, valeur droite (mono).
- **Freq** (custom) : label + petit toggle **Notes** (Hz ↔ nom de note) + slider + valeur. Note = `60+12·log2(hz/440)` mappé sur la gamme tempérée.
- **Switch** : 34×18 r10 ; off pastille gauche INK3 ; on fond `blue_glow(64)` + bordure BLUE, pastille droite BLUE.
- **Select** : h24, PANEL2, bord LINE2, chevron ▾ INK3 ; menu P_ACTIVE, option hover = BLUE/#fff.
- **Toggle LED** (header) : pilule h26 r7, LED Ø7 ; actif = bord BLUE + fond `blue_glow` + LED BLUE glow.
- **Knob** (réserve) : Ø44, arc r18 ép.4, course 135°→405°, drag vertical sensibilité /140.

### Step cell — voir §6 (états p-lock). Mini-slider lane : h6 r5, fill BLUE (ou FAINT pour Hum).

---

## 5. ADSR — visualisation

Cadre bord LINE r7 fond #0c0c11 padding 6, hauteur dessin ≥ 96. **Inline à droite des sliders ENV** (layout horizontal, largeur fixe ~200).

```
padX=12 padY=12 ; gx=w-2padX ; gy=h-2padY ; topY=padY ; baseY=h-padY
a=max(.02,attack) d=max(.05,decay) r=max(.05,release) ; tot=a+d+r
xA=padX+gx·(a/tot) ; xD=padX+gx·((a+d)/tot) ; xR=padX+gx ; susY=topY+gy·0.62
```
3 segments ép.2 : **Attaque** droite AMBER `(padX,baseY)→(xA,topY)` · **Decay** courbe BLUE → susY · **Release** courbe SEQPL(violet) → baseY.
Courbe = polyligne 40 pts, `x` linéaire, `y = t^(1+k)` avec `k = curve/2` (curve ∈ [0,8]).
Grille : 5 verticales `white_a(13)`. Légende : `■A ■D ■R` (amber/bleu/violet).

---

## 6. P-lock — le cœur produit (états de cellule) 🔒

Une cellule = un pas. **Les deux vocabulaires de p-lock sont des modes d'affichage mutuellement exclusifs** (toggle `Sound` / `Sequencer`) : on ne cumule JAMAIS les couleurs Sound et Sequencer sur la même cellule. `paintCell` applique **une seule classe d'état explicite** par cellule (pas de combinateur d'attribut/ancêtre — robuste au rendu et à l'export).

### Mode Sound (affichage des p-locks de synthèse)
| Pas | P-lock | Classe | Fond | Bord |
|---|---|---|---|---|
| off (pair/impair) | — | `step` / `is-beat` | `#1b1b22` / `#23232c` | LINE |
| **on** | none | `st-hit` | BLUE | BLUE + glow |
| **on** | link | `st-link` | PL_LINK | PL_LINK + glow orange |
| **on** | snapshot | `st-snap` | PL_SNAP | PL_SNAP + glow rouge |
| off | link | `st-link-off` | `#241a08` | PL_LINK_DIM |
| off | snapshot | `st-snap-off` | `#241010` | PL_SNAP_DIM |

### Mode Sequencer (affichage des seq-plocks — violet)
Les notes restent **normales** (bleu, comme en mode Sound) ; seuls les pas porteurs d'un seq-plock changent de couleur (violet). Les couleurs Sound (link/snapshot) ne sont simplement pas affichées.
| Pas | Classe | Fond | Bord |
|---|---|---|---|
| hit, sans seq-plock | `st-hit` | BLUE | BLUE + glow (note normale) |
| hit, avec seq-plock | `st-seqhit` | SEQPL | SEQPL + glow violet |
| off, avec seq-plock | `st-seqoff` | `#1c1230` | SEQPL_DIM |

**Playhead** : contour `white_a(150)` 1.5px inset ; pas vide sous playhead → `#30303c`.
**Fusion** (groupe de pulses) : cellule de départ fond `#14223a` bord BLUE + nombre de pulses centré ; cellules internes `#0f1828` bord pointillé.

> ⚠️ **Règle d'or** : Sound et Sequencer ne s'accumulent pas — c'est l'un OU l'autre selon le mode.
> Côté egui : `match (mode, hit, plock, seqlock)` → une seule couleur de remplissage. Clic droit = menu d'édition (LAYOUT §5).

---

## 7. États interactifs

| Élément | Repos | Hover | Actif/On | Désactivé |
|---|---|---|---|---|
| Nom instrument | PANEL2 / INK2 | P_HOVER / INK | BLUE / #fff | — |
| Step | voir §6 | — | voir §6 | opacité .28 (hors longueur) |
| Tag M / S / T | FAINT | — | AMBER / GREEN / BLUE | — |
| Onglet, page, slot, chip | PANEL2 / INK2 | bord BLUE / INK | BLUE / #fff | bord faded |
| Toggle LED | LED FAINT | INK | bord+LED BLUE | — |
| Bouton ×2 | PANEL2 | bord BLUE | — | opacité .4 (len>32) |

---

## 8. Animations (frame par frame)

| Effet | egui |
|---|---|
| Hover | `ctx.animate_value_with_time(id, target, 0.12)` |
| Toggle / LED | `…, 0.16` |
| Step playback | glow pulsé : alpha sinusoïdal sur la cellule courante, `request_repaint()` |

> Pilotage du playhead (tempo, pages, Follow) → `LAYOUT.md`.

*Réf. pixel : `assets/fd-base.css`.*
