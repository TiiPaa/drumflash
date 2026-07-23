# SPEC — cotes mesurées (computed styles, rendu réel navigateur)

> Relevé automatique par `getComputedStyle` sur `index.html` (thème Skeuo, fenêtre 1480×800, échelle 1.0 → px = points egui). C'est la **vérité terrain** : en cas de doute entre un doc et cette page, cette page gagne.
> Couleurs en `rgb()` prêtes pour `Color32::from_rgb`.

## Cadre
| Élément | Taille | Fond | Notes |
|---|---|---|---|
| `.app` (fenêtre) | 1480 × 800 | linear-gradient(#2a2b30 → #1e1f23) vertical | — |
| `.bar` (header) | 1480 × 44 | linear-gradient(#3d3e44, #2b2c31 60%, #26272b) | pad 0 14 · gap 14 · shadow: inset 0 1 0 rgba(255,255,255,.12) + 0 2 4 rgba(0,0,0,.5) |
| Marque `FLASH DRUM` | Sans 700 15px | #dfe1e8 | letter-spacing 0.3 |

## Colonne gauche
| Élément | Taille | Radius | Fond / notes |
|---|---|---|---|
| `.pagebar` / `.patbar` | 883 × 26 | — | gap 12 / 8 |
| `.pg` (page, actif) | 28 × 26 | 5 | linear-gradient(#2f86c4, #1e5f92 60%, #1a5480), bord #0d2c44, enfoncé (inset 0 2 4 rgba(0,0,0,.5)) + glow bleu 3px 20% |
| `.pslot` (slot Px, actif) | 30 × 26 | 5 | idem `.pg` |
| `.qbtn` / `.pbtn` / `.chip` (keycap repos) | h 26 | 5 | linear-gradient(#4a4b52, #38393f 55%, #333439), bord #17171b, inset 0 1 0 rgba(255,255,255,.16) + inset 0 -2 3 rgba(0,0,0,.35) + 0 2 3 rgba(0,0,0,.45) |
| `.tgl` (toggle LED, repos) | h 26, pad 0 12, gap 7 | 5 | keycap ; LED Ø8, radial(circle 40% 35%, #5a5b62, #3a3b41), inset 0 1 1 rgba(0,0,0,.6) |
| `.seg button` (segmented ACTIF, ambre) | h 26, pad 0 12 | 5 | linear-gradient(#c97a1e, #96560e 60%, #8a4e0c), bord #3d2404, texte #ffe8c8, enfoncé + glow ambre |
| `.genbtn` (GENERATE) | h 26, pad 0 16 | 5 | linear-gradient(#ffca55, #e09a18 55%, #c9860e), bord #5a3c04, texte #3a2500, ls 0.66 |
| `.seqwrap` (puits grille) | 867 × 451 | 5 | linear-gradient(#1c1d21 → #212226), bord #121215, inset 0 2 6 rgba(0,0,0,.6) + inset 0 -1 0 rgba(255,255,255,.05), pad 10 12 |

## Grille (1 ligne : gap 7)
| Élément | Taille | Radius | Fond / notes |
|---|---|---|---|
| `.seq__name` (touche lane) | 52 × 21 | 4 | linear-gradient(#45464d → #35363c), bord #17171b, Mono 600 10 #c9cbd3 |
| `.minisld` (Vol/Hum/Push) | 56 × 5 | 5 (=pilule) | creux #16171b, inset 0 1 2 rgba(0,0,0,.8) ; fill linear-gradient(#57beff → #2f86c4) |
| `.tag` (M/S/T) | 17 × 17 | 3 | linear-gradient(#404148 → #33343a), bord #17171b, texte repos #54555e |
| **`.step` (pad)** | **27 × 21** (col. flex) | **4** | bord 1px. Mesuré ici à l'état **snapshot rouge** : radial(circle 50% 30%, #ffb0a4, #f04a3a 45%, #a02618), bord #4a0f08, glow 4px 22% + inset 0 1 1 rgba(255,255,255,.45) + inset 0 -2 3 rgba(0,0,0,.3). Tous les états → `assets/fd-skeuo.css` §pads + textures `png/pad-*.png` |
| `.seq__steplab` (n° de pas) | Mono 500 9 | — | #a9abb4 (beat) / atténué |
| Labels d'en-tête (Vol/Hum/…) | Sans 600 9.5 | — | #8f919b, ls 0.19 |

## Éditeur (droite)
| Élément | Taille | Radius | Notes |
|---|---|---|---|
| `.selbox` (dropdown) | 97 × 26 (170 dans l'éditeur) | 5 | keycap ; Mono 500 11 |
| `.sld` (piste slider) | h 5 | 3 | creux linear-gradient(#121317 → #1c1d21), inset 0 1 2 rgba(0,0,0,.9) |
| `.sld__fill` | h 5 | 3 | linear-gradient(#57beff → #2f86c4) |
| `.sld__knob` (capuchon) | 12 × 19 | 3 | 3 bandes horiz. : #5c5d65 0–42%, #2e2f34 42–58%, #4a4b52 58–100% ; bord #101014 ; 0 2 4 rgba(0,0,0,.6) + inset 0 1 0 rgba(255,255,255,.2) |
| `.sw` (switch) | 34 × 18 | 10 (=pilule) | repos : fond #3a3b41, bord #4a4b52 |
| `.edsec__h` (titre section) | Sans 600 10 | — | #8f919b, ls 0.5 |
| `.ctl__label` | Sans 500 11.5/13.8 | — | #a9abb4 |
| `.ctl__val` | Mono 500 11 | — | #e9eaee, ls −0.22 |
| `.adsr` (écran LCD) | 200 × 124 | 4 | linear-gradient vert #0e1810 / #13201a / #0e1810, bord #060a07, inset 0 2 8 rgba(0,0,0,.8) |
| `.sblock` (bloc Song) | pad 7 7 6 | 5 | linear-gradient(#3d3e44 → #303137), keycap shadow |

## Typographie — rappel
IBM Plex Sans (UI) + IBM Plex Mono (valeurs/IDs). **À embarquer** dans le binaire (`egui::FontDefinitions`) — licence OFL, téléchargement : github.com/IBM/plex (releases). Poids utilisés : Sans 400/500/600/700, Mono 400/500/600.

## Hiérarchie des documents
1. **`SPEC-COMPUTED.md`** (ce fichier) — cotes mesurées, prioritaire.
2. `RADIUS.md` — radius exhaustifs par élément.
3. `SKEUO.md` — recettes matières (dégradés, états on/off de chaque contrôle).
4. `assets/fd-skeuo.css` + `assets/fd-base.css` — source de vérité pixel complète (tous les états).
5. `png/` — cible visuelle (`reference-full-ui.png`) + textures de pads prêtes à l'emploi.
