# Skeuo — design retenu (spec de portage egui)

Thème **hardware réaliste** : faceplate métal brossé, panneaux vissés, pads caoutchouc
rétroéclairés, faders à capuchon strié, keycaps biseautés, écran LCD. Glow volontairement
**très contenu** (3-4 px, alphas ≤ .3). Réf. pixel : `index.html` (thème par défaut) +
`assets/fd-skeuo.css`. Géométrie inchangée (`TOKENS.md`) ; rayons : panneaux 7, contrôles 5,
pads/noms 4, tags 3, LCD 4.

## 1. Matières (recettes egui)

Tout se rend en aplats + `Mesh` — aucune texture bitmap nécessaire.

| Matière | Recette |
|---|---|
| **Faceplate** (fond app) | dégradé vertical `#2a2b30 → #1e1f23` |
| **Métal brossé** (panneaux) | dégradé `#323338 → #28292d` + stries horizontales : lignes 1px `blanc α .014` tous les 3px (boucle de `hline`) |
| **Plaque vissée** | bord `#121215`, liseré haut `blanc α .09`, liseré bas `noir α .4`, ombre portée 3px ; **vis** = cercle Ø9 radial (`#6a6c74 → #3a3b41 → #242429`, highlight 35%/30%) aux coins haut-gauche + bas-droit |
| **Puits de grille** | fond `#1c1d21 → #212226`, inner shadow (2 lignes noires α .35/.18 en haut), r5, marge 8 |
| **Keycap** (bouton) | dégradé 3 arrêts `#4a4b52 → #38393f (55%) → #333439`, bord `#17171b`, liseré haut `blanc α .16`, creux bas (`noir α .35` inset), ombre portée 2px |
| **Keycap enfoncé** | dégradé de la couleur d'état assombrie (ex. bleu `#2f86c4 → #1e5f92 → #1a5480`), inner shadow 2px, glow externe 3px α .2, texte clair ombré 1px noir |
| **Pad caoutchouc** (cellule off) | dégradé `#313237 → #28292d` (beat : `#37383e → #2c2d32`), bord `#131317`, liseré haut `blanc α .07`, creux bas |
| **Pad allumé** | **radial** centré 50 %/30 % : cœur clair → couleur → foncé (voir §2), bord foncé assorti, glow 4px α .22-.25, reflet haut `blanc α .45-.5` |
| **Fader** | glissière h5 `#121317 → #1c1d21` inner shadow + fill `#57beff → #2f86c4` ; **capuchon** 12×19 r3 : 3 bandes horizontales `#5c5d65 / #2e2f34 / #4a4b52` (ligne de préhension), bord `#101014`, ombre 2px |
| **LED** (toggle) | éteinte : radial gris `#5a5b62 → #3a3b41` inset ; allumée : radial `#c4ecff → #4ab6ff → #1e6ea0`, glow 3px α .5 |
| **LCD** (ADSR) | fond vert-noir 3 arrêts `#0e1810 → #13201a → #0e1810`, bord `#060a07`, inner shadow 8px, teinte interne verte α .04 ; légende `#5a8a68` |
| **Labels gravés** | texte `#8f919b` + ombre `0 -1px noir α .7` + `0 1px blanc α .08` |
| **GENERATE** | ambre `#ffca55 → #e09a18 → #c9860e`, bord `#5a3c04`, texte `#3a2500` reflet blanc |

## 2. Pads — couleurs d'état (radial cœur → couleur → assombri)

| État | Cœur | Couleur | Bas | Bord |
|---|---|---|---|---|
| Note (hit) | `#9adcff` | `#4ab6ff` | `#1e6ea0` | `#0d3a5c` |
| P-lock link | `#ffd9a0` | `#ff9a2e` | `#b05e0a` | `#4a2a04` |
| P-lock snapshot | `#ffb0a4` | `#f04a3a` | `#a02618` | `#4a0f08` |
| Seq p-lock | `#dcbaff` | `#b06aff` | `#6a35b0` | `#2c1050` |
| Off + link / snap / seq | dégradés sombres teintés (`#3a2d1c`, `#3a211c`, `#32283e`) | | | bord teinté |
| Playhead | contour blanc α .75, 2px, inset | | | |

Radial en egui : 3-4 cercles concentriques du clair au foncé, ou petit `Mesh` radial —
à 21px de haut, 3 cercles suffisent.

## 3. Tokens texte / accent

ink `#e9eaee` · ink2 `#a9abb4` · ink3 `#787a84` · faint `#54555e` ·
accent `#4ab6ff` (foncé `#2f86c4`) · link `#ff9a2e` · snap `#f04a3a` · seq `#b06aff` ·
mute `#ffc23d` · solo `#5fd98a` · GENERATE ambre.

## 4. Règles de sobriété (🔒)

- Glow max 4px, alpha ≤ .3 — l'allumage se lit dans le **pad**, pas autour.
- Aucune rotation/inclinaison ; alignements de `TOKENS.md` inchangés.
- Texte des états actifs : ombre gravée simple, jamais de halo lumineux.
- Vis uniquement sur les panneaux de premier niveau (2 par plaque, coins opposés).
