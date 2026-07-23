# Graphite — design retenu (spec de portage egui)

Direction « luxe / pro sobre » (réf. Geist 2 / Battery / Fabfilter) : charbon usiné en
**dégradés verticaux 2 tons**, grille dans un **puits encastré**, accents **cyan** discrets,
glow minimal. Réf. pixel : `index.html` (thème par défaut) + `assets/fd-graphite.css`.
Géométrie inchangée (`TOKENS.md`) sauf les rayons ci-dessous.

## 1. Tokens

Le thème étend le schéma de `skins.json` : les surfaces deviennent des **paires de dégradé**
`[top, bottom]` (vertical). Valeurs prêtes à l'emploi : `graphite-tokens.json`.

| Rôle | Valeur |
|---|---|
| bg (app) | dégradé `#15181d → #0e1014` |
| header | `#252932 → #1a1d23`, bord bas `#05060a`, liseré haut `rgba(255,255,255,.06)` |
| panneau | `#20242b → #181b21`, bord `#0b0c10`, liseré haut blanc 6 % |
| puits de grille (seqwrap) | fond `rgba(0,0,0,.28)` + inner shadow, r3, marge 6 |
| contrôle (bouton/select/segment) | `#2c313c → #21252d`, bord `#12141a`, liseré haut blanc 7 % |
| contrôle actif | `#4fd0f2 → #2a90b0`, bord `#1a6480`, texte `#04222e`, glow 5px α.18 |
| toggle LED actif | fond contrôle normal, bord `#2a90b0`, **pas de glow** |
| GENERATE | dégradé accent, glow 6px α.16 |
| accent (blue) | `#3ec1e8` / foncé `#2a90b0` |
| texte | ink `#e6eaf0` · ink2 `#9aa4b2` · ink3 `#6b7482` · faint `#4a515d` |
| cellule off / beat | `#1d2129→#161920` / `#232833→#1a1e26`, bord `#0d0f13` |
| cellule active | `#5bd6f5 → #2a90b0`, glow 4px α.20, liseré haut blanc 35 % |
| p-lock sound | `#ffab3d → #e07800`, glow 4px α.18 |
| p-lock snapshot | `#f25555 → #b52222`, glow 4px α.18 |
| seq p-lock | `#c07bff → #8a3fd0`, glow 4px α.20 |
| cell_current | `#333947` · envelope_bg `#0a0c10` (écran, inner shadow) |
| slider track | `#0d0f13` creusé (inner shadow), h7 |
| slider fill | `#57d4f4 → #2fa4c6`, sans glow |
| poignée slider | Ø13, radial `#ffffff → #b8c4d2`, ombre portée 2px |
| popups | `#262b34 → #1c2027`, bord `#0b0c10`, ombre large |

## 2. Rayons (remplacent RADIUS_* pour ce thème)

Panneaux **4** · popups **5** · contrôles/boutons/selects/nom de lane **3** · cellules **2** ·
tags M/S/T **3** · ADSR/puits **3**.

## 3. Adaptation egui (les « interdits » deviennent faisables)

- **Dégradé vertical 2 tons** : `Mesh` 4 sommets avec couleurs par sommet
  (`top,top,bottom,bottom`) — helper `gradient_rect(painter, rect, top, bottom)`. Pas de
  texture nécessaire.
- **Liseré haut** (`inset 0 1px 0 blanc α`) : ligne 1px en haut du rect, blanc à l'alpha donné.
- **Puits encastré / inner shadow** : fond plus sombre + 2 lignes empilées en haut
  (noir α .35 puis .18) — illusion suffisante à 100 %.
- **Glow** : 1–2 rects arrondis translucides derrière l'élément (expand +2 px puis +4 px,
  alpha .10/.08 de la couleur d'accent). Jamais plus — le thème est volontairement calme.
- **Poignée radiale** : 2 cercles concentriques (blanc, puis `#b8c4d2` décalé) suffisent.
- **Ombres de panneaux** : optionnelles ; 1 rect noir α .25 expand +6 sous le panneau, ou rien.

## 4. Intégration au système de skins

Deux options :
1. **Skin étendu** : ajouter des tokens `*_top`/`*_bottom` (fallback : skins plats existants
   dupliquent la valeur) + champs `radius_override`. Graphite devient le 4e skin.
2. **Thème v2 par défaut** : Graphite remplace Dark, les skins plats restent disponibles.

Recommandé : option 1 — `graphite-tokens.json` suit ce schéma étendu.
