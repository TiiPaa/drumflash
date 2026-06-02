# Flash Drum — Design System (egui)

> Tokens visuels et catalogue de widgets réutilisables.
> Ce fichier est la **source de vérité** pour toutes les décisions visuelles.
> Tout ce qui apparaît dans l'interface doit pouvoir se construire à partir de ces briques.
>
> 🔒 = convention figée. 🔓 = ajustable pendant le développement.

---

## 1. Palette — Couleurs 🔒

Toutes les couleurs sont définies en RGBA u8 (0–255).

### Fonds & surfaces

| Token | Valeur | Usage |
|-------|--------|-------|
| `BG` | `#0a0a0f` (10,10,15) | Fond global de la fenêtre |
| `PANEL` | `#141419` (20,20,25) | Fond des panneaux/cartes |
| `PANEL2` | `#1c1c24` (28,28,36) | Fond des éléments interactifs inactifs, tags, boutons secondaires |
| `PANEL_HOVER` | `#242430` (36,36,48) | Hover sur PANEL2 |
| `PANEL_ACTIVE` | `#2a2a38` (42,42,56) | État actif/pressé sur PANEL2 |

### Lignes & bordures

| Token | Valeur | Usage |
|-------|--------|-------|
| `LINE` | `#2a2a35` (42,42,53) | Bordures de panneaux, séparateurs, grilles fines |
| `LINE2` | `#3a3a48` (58,58,72) | Bordures d'éléments interactifs (boutons, tags) |
| `DIVIDER` | `#1f1f28` (31,31,40) | Séparateurs internes subtils |

### Accent & états

| Token | Valeur | Usage |
|-------|--------|-------|
| `BLUE` | `#4a9eff` (74,158,255) | Accent principal : playhead, pas actif, sélection, valeurs numériques editables |
| `BLUE_DIM` | `#4a9eff80` (74,158,255,128) | Variante semi-transparente de BLUE |
| `BLUE_GLOW` | `#4a9eff40` (74,158,255,64) | Halo/glow autour des éléments accentués |
| `GREEN` | `#4ade80` (74,222,128) | Toggle ON, état positif |
| `RED` | `#f87171` (248,113,113) | Erreur, mute, accent négatif |
| `AMBER` | `#fbbf24` (251,191,36) | Accent secondaire (rare, pour hiérarchie) |

### Texte

| Token | Valeur | Usage |
|-------|--------|-------|
| `INK` | `#e8e8f0` (232,232,240) | Texte principal (titres, labels) |
| `INK2` | `#9ca3af` (156,163,175) | Texte secondaire (valeurs, unités, désactivé) |
| `INK3` | `#6b7280` (107,114,128) | Texte tertiaire (indices, placeholders) |
| `INK_FAINT` | `#4b5563` (75,85,99) | Texte très atténué (en-têtes de grille, pas inactifs) |
| `INK_BLUE` | `#4a9eff` (74,158,255) | Texte en accent (valeurs numériques éditées, step courant) |

### États des steps (séquenceur)

| État | Fond | Bordure | Texte |
|------|------|---------|-------|
| `OFF` | `PANEL2` | `LINE` | — |
| `HIT` | `PANEL2` | `BLUE` 1.5px | — |
| `ACCENT` | `BLUE` | `BLUE` 1.5px | — |
| `PLAYHEAD` (sur OFF) | `PANEL2` | `white_a(140)` 1.5px inset | — |
| `PLAYHEAD` (sur HIT) | `BLUE_DIM` | `white_a(140)` 1.5px inset | — |
| `PLAYHEAD` (sur ACCENT) | `BLUE` | `white_a(255)` 2px inset | — |

> `white_a(alpha)` = `(255,255,255,alpha)`

---

## 2. Typographie 🔒

Police : **system-ui** stack (Segoe UI, Roboto, SF Pro, etc.).
Monospace pour les valeurs numériques : **Consolas, monospace**.

| Token | Taille | Poids | Couleur | Usage |
|-------|--------|-------|---------|-------|
| `H1` | 20 px | Bold (700) | `INK` | Titre principal ("Flash Drum") |
| `H2` | 14 px | SemiBold (600) | `INK` | Titres de panneaux, noms de section |
| `H3` | 12 px | SemiBold (600) | `INK2` | Sous-sections, labels de groupe |
| `BODY` | 11 px | Regular (400) | `INK` | Texte courant, labels de paramètres |
| `BODY2` | 11 px | Regular (400) | `INK2` | Valeurs par défaut, unités |
| `MONO` | 11 px | Regular (400) | `INK2` | Valeurs numériques, BPM, ms, Hz |
| `MONO_EDIT` | 11 px | Regular (400) | `INK_BLUE` | Valeur en cours d'édition |
| `MONO_FAINT` | 10 px | Regular (400) | `INK_FAINT` | Labels de grille (1, 5, 9, 13…) |
| `TAG` | 10 px | SemiBold (600) | `INK2` | Tags M/S, chips presets |
| `TAG_ACTIVE` | 10 px | SemiBold (600) | `INK` | Tags actifs |
| `BTN` | 11 px | SemiBold (600) | `INK` | Texte de bouton |
| `BTN_PRIMARY` | 11 px | SemiBold (600) | `#fff` | Texte de bouton primaire (fond BLUE) |

---

## 3. Espacement & formes 🔓

### Gaps (échelle de base 4 px)

| Token | Valeur | Usage |
|-------|--------|-------|
| `GAP_XS` | 2 px | Gap interne très serré |
| `GAP_SM` | 4 px | Gap entre widgets proches (chips, tags) |
| `GAP_MD` | 8 px | Gap standard entre éléments |
| `GAP_LG` | 12 px | Gap entre blocs (padding interne panneaux) |
| `GAP_XL` | 16 px | Padding de panneau, gap colonnes |
| `GAP_XXL` | 24 px | Séparateurs majeurs |

### Radius

| Token | Valeur | Usage |
|-------|--------|-------|
| `RADIUS_SM` | 4 px | Petits éléments (tags, chips) |
| `RADIUS_MD` | 7 px | Boutons standards, toggles |
| `RADIUS_PANEL` | 10 px | Panneaux/cartes |
| `RADIUS_LG` | 12 px | Grandes surfaces (éditeur) |

### Hauteurs de ligne (UI)

| Token | Valeur | Usage |
|-------|--------|-------|
| `ROW_SM` | 18 px | En-tête de grille, labels |
| `ROW_MD` | 22 px | Steps, mini-sliders |
| `ROW_LG` | 26 px | Lignes d'instrument, boutons |
| `ROW_XL` | 30 px | Boutons de transport |

---

## 4. Catalogue de widgets 🔒

### 4.1 Bouton standard

```
Fond: PANEL2
Bordure: 1px LINE2
Radius: RADIUS_MD (7)
Padding: 6×14
Texte: BTN

Hover: fond PANEL_HOVER
Active/Pressé: fond PANEL_ACTIVE, bordure LINE

Variante primaire:
  Fond: BLUE
  Texte: BTN_PRIMARY
  Hover: fond légèrement plus clair (~#5aadff)
  Active: fond légèrement plus foncé (~#3d8de6)
```

### 4.2 Bouton icône (carré)

```
Taille: ROW_XL × ROW_XL (30×30)
Fond: PANEL2
Bordure: 1px LINE2
Radius: RADIUS_MD (7)
Glyphe: centre, couleur INK2

État actif (ex: Play en cours):
  Fond: BLUE
  Glyphe: #fff
```

### 4.3 Toggle LED

```
Indicateur: cercle 8×8, radius 4
  OFF: fond LINE
  ON: fond GREEN (ou BLUE selon le contexte)

Label: BODY à droite, couleur INK2

Layout: [LED] [Label] — gap GAP_SM (4)
```

### 4.4 Tag (M/S, chips presets)

```
Taille: auto × 18 (ou fixe 30×26 pour P1..P8)
Fond: PANEL2
Bordure: 1px LINE2
Radius: RADIUS_SM (4 ou 5)
Padding: 2×8 (ou 4×10)
Texte: TAG

Hover: fond PANEL_HOVER
Actif/sélectionné:
  Fond: BLUE
  Texte: TAG_ACTIVE (ou #fff)
  Bordure: BLUE
```

### 4.5 Slider standard

```
Track (fond): hauteur 4, radius 2, couleur LINE
Fill (valeur): même hauteur, couleur BLUE
Handle: 12×12, cercle, couleur #fff, bordure 1px LINE2

Label: BODY au-dessus ou à gauche
Valeur: MONO à droite du slider

Hover handle: glow BLUE_GLOW (rayon 4)
Drag: curseur change, valeur affichée en MONO_EDIT
```

### 4.6 Mini-slider (ligne de séquenceur)

```
Largeur: 54
Hauteur: 22
Track: hauteur 3, radius 1.5, LINE
Fill: BLUE (si actif) ou LINE (si à 0)
Sans handle (clic direct ou drag horizontal)

Grisé (désactivé):
  Fill: LINE
  Opacité réduite (0.5)
```

### 4.7 Cellule de step (séquenceur)

```
Hauteur: ROW_MD (22)
Largeur: flex (zone steps / 16 − gap)
Fond: voir états des steps §1
Bordure: 1px selon état
Radius: RADIUS_SM (4)

Playhead (overlay):
  Bordure: white_a(140), 1.5px, inset (offset 1.5)
  Si accent: white_a(255), 2px
```

### 4.8 Select (dropdown)

```
Fond: PANEL2
Bordure: 1px LINE2
Radius: RADIUS_SM (4)
Padding: 5×10
Texte: BODY
Flèche: ▼, INK2, 8px

Menu déroulant:
  Fond: PANEL
  Bordure: 1px LINE
  Radius: RADIUS_SM
  Item hover: PANEL_HOVER
  Item sélectionné: BLUE (texte #fff)
```

### 4.9 Champ numérique éditable

```
Fond: transparent (ou PANEL2 si focus)
Texte: MONO
Focus: bordure basse 1px BLUE, texte MONO_EDIT

Double-clic ou clic + pause : sélection texte
Entrée : validation
Escape : annulation
```

### 4.10 En-tête de panneau

```
Hauteur: auto (padding 9×13)
Fond: PANEL (même que le panneau) ou légèrement différent
Bordure basse: 1px LINE
Texte: H2
Icône/label optionnel à gauche
```

---

## 5. Layout helpers (egui) 🔒

Conventions pour construire les zones sans coder en dur :

### Panneau (carte)

```rust
// Équivalent visuel
frame.fill(PANEL)
     .stroke(Stroke::new(1.0, LINE))
     .rounding(RADIUS_PANEL)
     .show(ui, |ui| {
         // contenu avec padding GAP_LG (12) / GAP_XL (16)
     });
```

### Barre de séparation verticale

```
Largeur: 1
Hauteur: 24 (ou auto)
Couleur: LINE
```

### Ligne d'instrument (séquenceur)

```
Hauteur: ROW_LG (26)
Gap vertical: GAP_TIGHT (3)
Alignement: gauche, tous les éléments sur baseline
```

### Grille de paramètres (éditeur)

```
Gap horizontal: 22
Gap vertical: 10
Alignement: top pour toutes les cellules
```

---

## 6. Widgets composites 🔒

### 6.1 Graphe ADSR

```
Taille: largeur = parent, hauteur = 80
Fond: PANEL2 (ou transparent)
Bordure: 1px LINE (optionnel)
Radius: RADIUS_SM

Grille de fond: lignes pointillées, LINE, opacité 0.3
Courbe d'enveloppe: ligne 2px, BLUE
Points A/D/S/R: poignées 8×8, cercle, #fff + bordure LINE2
  Drag horizontal sur les poignées
  Valeurs affichées en MONO à côté de chaque point

Axes: labels temp (ms) en bas, amp (0–1) à gauche — INK_FAINT
```

### 6.2 Section d'éditeur (collapsible)

```
En-tête: H3 + flèche ▶/▼ (INK2)
Corps: grille de paramètres, gap 10×22
Séparateur bas: 1px DIVIDER

État replié: hauteur = en-tête uniquement
État déployé: hauteur = auto
```

### 6.3 Onglets d'instruments (colonne droite, en haut)

```
Layout: 7 colonnes, gap GAP_SM (4)
Largeur: (parent − 6×gap) / 7
Hauteur: 26

État inactif:
  Fond: PANEL2
  Texte: TAG
  Bordure basse: 1px LINE

État actif/sélectionné:
  Fond: BLUE
  Texte: #fff
  Bordure basse: 1px BLUE

Ordre: BD, SD, HH, OH, T1, T2, T3, CL, RD, CY, S6, B8, P1
  → 7 visibles à la fois, scroll horizontal si nécessaire
```

---

## 7. Animation & interaction 🔓

| Interaction | Comportement |
|-------------|--------------|
| Hover | transition instantanée (pas d'animation CSS, egui est immédiat) |
| Drag slider | valeur affichée en temps réel, curseur changes |
| Playhead | avance par pas discrets, pas de smooth scroll entre steps |
| Sélection instrument | reconstruction immédiate de l'éditeur (pas de transition) |
| Toggle | changement instantané, LED s'allume/éteint |
| Focus champ texte | bordure basse apparaît, texte passe en INK_BLUE |

---

## 8. Checklist d'implémentation egui

Pour chaque widget, vérifier :

- [ ] Les couleurs viennent de la palette §1 (pas de hardcodage)
- [ ] Les tailles utilisent les tokens §3
- [ ] Le texte utilise les tokens de typo §2
- [ ] Les états (hover, active, disabled) sont tous définis
- [ ] Le widget est réutilisable (pas de logique métier inline)

---

*Ce document est complémentaire à `LAYOUT.md`. LAYOUT décrit **où** vont les choses ; DESIGN-SYSTEM décrit **à quoi** elles ressemblent.*
