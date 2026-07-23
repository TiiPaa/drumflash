# PNG — cibles visuelles & textures pour egui

Réponse aux deux demandes de l'intégrateur : **captures du rendu réel** + **le look sous forme d'images**.

## Contenu

| Fichier | Usage |
|---|---|
| `reference-full-ui.png` | **La cible.** L'UI complète en Skeuo, rendu navigateur, 2× (2960×1600). À garder ouverte pendant l'intégration. |
| `sprite-sheet.png` | Planche annotée de tous les états (3×) — vue d'ensemble. |
| `pad-*.png` | **Textures de pads, 4× (176×104 pour un pad 44×26).** Un fichier par état : `off`, `off-beat`, `hit`, `hit-link`, `hit-snap`, `off-link`, `off-snap`, `fuse-start` (sans le chiffre — le "3" est du texte à dessiner par-dessus), `fuse-mid`, `seq-hit`, `seq-off`. |
| `slider.png`, `minislider.png`, `switch-on.png` | Références des contrôles (le slider se dessine plutôt en vectoriel, voir plus bas). |

## Intégration egui recommandée

1. **Pads = textures.** Charger chaque `pad-*.png` via `egui::include_image!` / `Image::new`, afficher à la taille de la cellule (`Image::fit_to_exact_size`). Les PNG sont à 4× → nets à toutes les tailles raisonnables. Fond transparent inutile : les pads sont opaques, coins arrondis inclus dans l'image.
   - Si la taille des cellules doit varier fortement : demander un export 9-slice, mais à taille quasi fixe (fenêtre 1480×800 fixe) l'étirement simple suffit.
   - Overlays à dessiner par-dessus la texture : contour playhead (`Stroke` blanc 2px), chiffre de pulses sur `fuse-start`.
2. **Sliders / switches = vectoriel.** Pas besoin de textures : creux = 2 rects empilés (ombre interne simulée par un rect sombre 1px en haut), remplissage plat bleu, poignée = cercle clair + `Stroke` sombre. `slider.png` sert de référence de proportions.
3. **Panneaux / boutons** : garder le rendu plat actuel d'egui, en calant couleurs et rayons sur `reference-full-ui.png` — l'essentiel de l'effet "machine" vient des **pads**, pas des panneaux.

## Régénération

Ces PNG sont exportés depuis `skeuo-sprites.html` (racine du projet design). Toute retouche du thème (`assets/fd-skeuo.css`) → me redemander un export.
