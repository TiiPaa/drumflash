# Flash Drum — Pads (bitmap)

Sprites des cellules du séquenceur, bakés depuis le rendu CSS de référence (thème Skeuo).
**69 sprites** : 9 pas simples + 60 cellules fusionnées.

| Fichier | Contenu |
|---|---|
| `atlas-pads.png` | l'atlas — 1600×840 points, baké ×4 (soit 6400×3360 px) |
| `atlas-pads.json` | le manifeste : `sprites[nom] = [x, y, w, h, ew, eh]` |

## Lire le manifeste

Les rects sont en **px CSS = points egui** (échelle 1) : multiplier par `scale` (4) pour obtenir les pixels dans le PNG.

Chaque rect **inclut une marge transparente de 8 points** (`bleed`) sur les 4 côtés, pour que les ombres et le glow soient capturés. L'élément utile est **centré** dans le rect et mesure `ew × eh`.

Pour dessiner : prendre le rect logique de la cellule, **l'agrandir de `bleed`** (mis à l'échelle si besoin), et y peindre le sprite entier.

```
uv = (x / 1600, y / 840) → ((x + w) / 1600, (y + h) / 840)
```

## Pas simple — 9 sprites

Taille utile **44 × 26 points**.

| Nom | État |
|---|---|
| `pad-off` | éteint |
| `pad-off-beat` | éteint, 1er pas d'un temps (nuance plus claire) |
| `pad-hit` | actif (bleu) |
| `pad-hit-link` | actif + p-lock **link** (orange) |
| `pad-hit-snap` | actif + p-lock **snapshot** (rouge) |
| `pad-hit-seq` | actif + p-lock **séquenceur** (violet) |
| `pad-off-link` | éteint, p-lock link conservé |
| `pad-off-snap` | éteint, p-lock snapshot conservé |
| `pad-off-seq` | éteint, p-lock séquenceur conservé |

## Cellule fusionnée — 60 sprites

Une cellule fusionnée est **un seul pad allongé continu** couvrant N pas — jamais N cellules côte à côte — et de **même hauteur qu'un pas simple** (26 points).

Sa largeur utile vaut `N × 44 + (N − 1) × 3` : elle recouvre aussi les gaps entre les pas.

Elle est **toujours active** : on ne la désactive pas, on la supprime. D'où **4 statuts seulement**, sans variante éteinte :

`fuse-<statut>-<N>` avec statut ∈ `hit`, `hit-link`, `hit-snap`, `hit-seq` et **N de 2 à 16**.

Exemples : `fuse-hit-3`, `fuse-hit-snap-8`, `fuse-hit-seq-16`.

## Overlays — non bakés, à dessiner par-dessus

1. **Playhead** — contour blanc 75 %, épaisseur 2, offset −1 (rayon d'angle 4).
2. **Chiffre de pulses** sur une cellule fusionnée — mono 600, 10 px, blanc, centré.
3. **Hors-longueur** (pas au-delà de la longueur du pattern) — teinte alpha 28 % sur le sprite.

## Géométrie de la grille

Pas : **44 × 26** points, **gap de 3** points entre cellules. Rayon d'angle : 4 points.

---

Pack complet (UI, code Rust, specs, pages de bake) : voir le pack `flash-drum-source`.
