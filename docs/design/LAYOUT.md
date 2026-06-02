# Flash Drum — Layout & assemblage (egui)

> À transmettre au dev **maintenant**. Ce fichier décrit *où vont les choses* et
> *quels paramètres expose chaque instrument*.
> Prérequis : le toolkit de `DESIGN-SYSTEM.md` (tokens + widgets).
>
> **Comment lire ce fichier — deux niveaux :**
>
> - 🔒 **INVARIANT** — décisions d'**architecture**. À respecter dès maintenant, même si
>   l'app n'est pas finie. Coder à l'encontre d'un 🔒 = travail qui devra être jeté/refait.
>   Ce sont les garde-fous qui empêchent d'ajouter des features incompatibles avec le design final.
> - 🔓 **AJUSTABLE** — détail (largeurs au pixel, présence de telle colonne…). Indicatif :
>   peut évoluer avec les fonctionnalités sans rien casser. À figer en fin de parcours.
>
> Code de référence pixel : `variation-a.html`.

---

## 0. Les invariants en un coup d'œil (à lire en premier)

| 🔒 | Décision d'architecture |
|----|--------------------------|
| 1 | **2 colonnes** : à gauche séquenceur + générateur, à droite **l'éditeur de son**. |
| 2 | L'**éditeur est un panneau dynamique** : son contenu se **reconstruit** selon l'instrument sélectionné. **Ne jamais coder les paramètres en dur dans le layout.** |
| 3 | **Sélection d'instrument unique et partagée** entre la ligne du séquenceur et l'éditeur (une seule source d'état `selected_instrument`). |
| 4 | **Séparation données ↔ rendu** : le schéma de paramètres (`schema_for(cat)`) est une donnée ; le rendu ne fait que parcourir ce schéma. Ajouter un instrument = ajouter une donnée, pas du code de layout. |
| 5 | Le **séquenceur** = grille `N_instruments × 16 pas`, états par pas `off / hit / accent`, avec **playhead** piloté par le tempo. |

> Tant que ces 5 points sont respectés, tu peux ajouter mixer, effets, arrangeur, etc.
> sans rendre le travail incompatible avec le design validé (Option A).

---

## 1. Layout général (Option A — 2 colonnes) 🔒 structure / 🔓 cotes

Fenêtre **1400 × 880**. Barre haute → corps (2 colonnes).

```
┌─────────────────────────────────────────────────────────────┐
│  BARRE  (~50)                                                 │
├──────────────────────────────┬──────────────────────────────┤
│  COLONNE GAUCHE  (850)        │  COLONNE DROITE  (~550)       │
│  ┌─ panneau Sequencer ─────┐  │  ┌─ onglets instruments ───┐ │
│  │  grille 13 × 16          │  │  └──────────────────────────┘ │
│  └──────────────────────────┘  │  ┌─ éditeur dynamique ──────┐ │
│  ┌─ panneau Generator ─────┐  │  │  sections + ADSR          │ │
│  └──────────────────────────┘  │  └──────────────────────────┘ │
└──────────────────────────────┴──────────────────────────────┘
```

| Zone | Valeur | |
|---|---|---|
| Fenêtre | 1400 × 880 | 🔓 |
| Barre haute | padding 10 (V) × 16 (H), bordure basse 1 px LINE, fond ≈ PANEL | 🔓 |
| Séparateur vertical (vbar) | 1 × 24, LINE | 🔓 |
| Gap items barre | 16 (`GAP_MD` ×2) | 🔓 |
| Colonne gauche | largeur **850**, padding 14×16, gap vertical 12, bordure droite 1 px LINE | 🔒 *gauche = séq.+gén.* / 🔓 *largeur* |
| Colonne droite | reste (≈ 550), fond PANEL | 🔒 *droite = éditeur* / 🔓 *largeur* |
| Panneau (carte) | fond PANEL, bordure 1 px LINE, radius 10 (`RADIUS_PANEL`) | 🔓 |
| En-tête de panneau | padding 9×13, bordure basse 1 px LINE | 🔓 |
| Intérieur séquenceur | padding 12×13 | 🔓 |
| Onglets instruments | 7 colonnes, gap 4, padding 13×14, bordure basse 1 px LINE | 🔓 |
| Intérieur éditeur | padding 6 (haut) 16 (côtés) 14 (bas) | 🔓 |

> 🔒 **L'ossature** (barre en haut ; gauche = séquenceur+générateur ; droite = éditeur) est le contrat.
> 🔓 Toutes les **cotes** ci-dessus (largeurs, paddings, radius) peuvent s'ajuster.

---

## 2. Barre haute — composition (gauche → droite)

`FLASH DRUM` `v0.2` │ ▶ │ `124 BPM` │ Master(slider) │ Swing(slider) │ Swing 16th(select) │ Choke(toggle LED) Auto-Edit(toggle LED) │ →(droite) P1..P8

- **Play** : 30×30, radius 7, bordure LINE2, fond PANEL2, glyphe BLUE. Actif : fond BLUE, glyphe #fff.
- **Sliders d'entête** : largeur totale ~184 (label auto + track + valeur).
- **Toggles LED** Choke / Auto-Edit : voir widget dans DESIGN-SYSTEM §4.
- **Pavés P1..P8** : 30×26, radius 5, bordure LINE2, fond PANEL2 ; actif fond BLUE texte #fff. Alignés à droite (`with_layout(Layout::right_to_left)`).

---

## 3. Séquenceur — assemblage de la grille

1 ligne d'en-tête + **13 lignes d'instrument**. Ordre :
`BD, SD, HH, OH, T1, T2, T3, CL, RD, CY, S6, B8, P1`.

### Une ligne (gauche → droite)
`[Nom 38]  [Vol mini-slider 54]  [M][S]  [16 steps · flex]  [Hum 46][Push 46][Len 46]`

| Élément | Dimensions |
|---|---|
| Hauteur de ligne | 26 (en-tête 18) |
| Gap vertical entre lignes | 3 (`GAP_TIGHT`) |
| Gap horizontal entre colonnes | 8 (`GAP_MD`) |
| Nom instrument | 38×22 |
| Mini-slider Vol | largeur 54 |
| Tags M / S | 18×18 chacun, gap 3 |
| Zone steps | flex, 16 cellules réparties également : largeur = (zone − 15×3) / 16, hauteur 22, gap 3 |
| Hum / Push / Len | 46 chacune — Hum = mini-slider grisé ; Push = « 0 ms » ; Len = « 16 » (mono 10 INK2) |

### Labels de step (en-tête)
Mono 9 `FAINT` ; temps forts (1,5,9,13) en `INK2`. Sous le playhead : label courant en `BLUE`.

### Playhead — pilotage
```
ms_par_pas = 60_000.0 / bpm / 4.0   // double-croches, 16 pas = 1 mesure 4/4
// à 124 BPM ≈ 121 ms/pas
```
Avancer `current_step` sur l'horloge (idéalement dérivée du thread audio), puis `ctx.request_repaint()`. Rendu : voir cellule de step (DESIGN-SYSTEM §4) — contour `white_a(140)` ép. 1.5 inset.

---

## 4. Éditeur dynamique — schéma par type d'instrument 🔒🔒🔒

> ⚠️ **LE point le plus structurant de toute l'app.** C'est l'invariant n°2 + n°4.
> Si une seule chose doit être respectée dès maintenant, c'est celle-ci :
> **l'éditeur ne contient aucun paramètre codé en dur.** Il *parcourt* un schéma de données.
> Coder « les sliders du kick » directement dans le layout = exactement le piège qui rend
> tout incompatible quand tu ajoutes un moteur de son.

Sélectionner un instrument reconstruit l'éditeur avec **un jeu de paramètres propre à sa catégorie**.

Sections **communes** à tous : **Level**, **Envelope** (+ADSR), **Filter**, **Saturation**, **Output**.
La section « source » dépend de la catégorie :

| Catégorie | Instruments | Section source : titre + contrôles | Filtre |
|---|---|---|---|
| `kick`   | BD          | **Oscillator** : Frequency (0–200), Click, Algorithm (Sine/Triangle/Saw/Square) | LP |
| `tom`    | T1 T2 T3    | **Oscillator** : Frequency, Click, Algorithm, **Pitch Bend** | LP |
| `snare`  | SD          | **Body + Noise** : Tone Freq, Noise Mix, Snap, Body (select) | LP |
| `hat`    | HH OH       | **Metal / Noise** : Tone, Decay, Color, Shimmer | **HP** |
| `cymbal` | RD CY       | **Metal / Noise** : Tone, Decay, Color, Shimmer | **HP** |
| `clap`   | CL          | **Clap Engine** : Pitch, Spread, Count | LP |
| `perc`   | S6 B8 P1    | **Source** : Pitch, Decay, Noise | LP |

### Disposition d'une section
Titre de section (widget) + corps en grille **N colonnes**, gap **10 (V) × 22 (H)**.
La section Envelope porte en plus le graphe ADSR (widget, DESIGN-SYSTEM §6) sous ses sliders.

### Structure de données suggérée (source de vérité)
```rust
pub enum CtlKind { Slider, Select, Switch }

pub struct ParamSpec {
    pub label: &'static str,
    pub key:   &'static str,   // identifiant du paramètre moteur
    pub kind:  CtlKind,
    pub min: f32, pub max: f32, pub step: f32, pub default: f32,
    pub unit: &'static str,
    pub options: &'static [&'static str], // pour Select
}

pub struct Section {
    pub title: &'static str,
    pub cols:  u8,
    pub items: &'static [ParamSpec],
    pub has_adsr: bool,
}

// fn schema_for(cat: Category) -> &'static [Section]
```

> Les **plages, pas et valeurs par défaut** exacts de chaque paramètre sont dans
> `assets/fd-data.js` → fonction `schemaFor(cat)` + objet `params`.
> Je peux te l'exporter directement en table Rust (`&[ParamSpec]` par catégorie) si tu veux
> brancher la source de vérité sans la retranscrire à la main — demande-le.

---

## 5. Générateur (panneau bas gauche)

En-tête « Generator · Probabilistic ». Corps : rangée de chips presets + sliders + bouton.

- Chips : `Rock` `Funk` `Disco` `Clear` `⟳ Random`(accent) `Export MIDI` `Drag`.
- Sliders/selects : Type (select), Style (select), Mix, Density, Variation.
- Bouton **GENERATE** pleine largeur (widget, DESIGN-SYSTEM §4).

---

*Fin du layout. À assembler avec le toolkit de `DESIGN-SYSTEM.md`. Référence pixel : `variation-a.html`.*
