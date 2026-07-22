# Tokens — tailles, rayons, espacements, typographie

## Fenêtre

- Taille fixe : **1480 × 900** (pas de redimensionnement)

## Hauteurs & tailles standard

| Token | Valeur | Usage |
|---|---|---|
| `HEADER_H` | 44 px | barre d'en-tête |
| `CTL_HEIGHT` | 26 px | boutons, selects, sliders pill |
| `LANE_H` | 24 px | hauteur d'une lane de grille |
| `STEP_H` | 21 px | hauteur d'une cellule step |
| `TAG_SIZE` | 21 px | tags M / S |
| Panneau bas | 210 px | Generator / Song |
| Popup settings | 200-220 px | min-max width |
| Popup p-lock | 280-350 px | min-max width |

## Colonnes de la grille (par lane)

| Colonne | Largeur |
|---|---|
| grip (reorder) | 14 px |
| nom de lane | 50 px |
| volume | 56 px |
| M / S | ~69 px (3 × 21 + gaps) |
| Hum / Push / Len | 44 / 44 / 35 px |
| gap inter-colonnes | 7 px |
| steps | le reste (16 cellules, min 18 px/cellule, gap 3 px) |

## Rayons & gaps

| Token | Valeur | Usage |
|---|---|---|
| `RADIUS_PANEL` | 9 px | panneaux, popups |
| `RADIUS_CTL` | 5 px | petits contrôles, cellules (4 px en pratique) |
| `GAP_TIGHT` | 3 px | espacement serré (steps, M/S) |
| Boutons | 5-6 px | corner radius |
| Bordures | 1 px (hairline) | pas de glow ni d'ombre portée |

## Sound Editor

| Token | Valeur | Usage |
|---|---|---|
| `EDITOR_LABEL_W` | 138 px | colonne labels (alignés à gauche) |
| `EDITOR_PARAMS_W` | 340 px | colonne paramètres (sliders) |
| `EDITOR_VALUE_W` | 52 px | colonne valeur mono à droite |
| Header | 42 px | titre + nom du slot |
| Onglets | 45 px | Sound / Track |
| Track slider | 6 px hauteur | fill accent, handle r5.5 au hover (r4 pour les mini-sliders) |

## Typographie — IBM Plex

Faces embarquées : **IBM Plex Sans** (400/500/600/700) et **IBM Plex Mono** (400/500/600).

Convention : **chiffre = mono, mot = sans**.

| Usage | Famille | Tailles courantes |
|---|---|---|
| Titre app (FLASH DRUM) | sans_bold | 15 px |
| Titres sections | sans_sb (600) | 10.5-13 px |
| Labels contrôles | sans_med (500) | 10.5-11.5 px |
| Valeurs / compteurs / codes | mono, mono_med, mono_sb | 9.5-13 px |
| Texte secondaire | sans_med | 10-10.5 px |

Pas de faux-gras (`strong()` interdit) : utiliser les familles pondérées `sans_med` / `sans_sb` / `sans_bold` / `mono_med` / `mono_sb`.

## Règles visuelles (déjà appliquées dans le code)

- **Pas de blur ni d'ombre portée** (egui ne les gère pas) : aplats + bordures 1 px nettes uniquement.
- **Hover** : léger éclaircissement (`p_hover`) ou lerp vers l'accent, jamais de halo.
- **Playhead** : anneau blanc intérieur pulsé sur la cellule courante (pas de colonne pleine).
- **P-lock actif = cellule pleine** (orange sound / violet sequencer), jamais de bordure seule.
- **Hauteur de grille constante** : les 14 lanes sont toujours rendues (actives + vides) pour que les panneaux du bas ne bougent jamais.
