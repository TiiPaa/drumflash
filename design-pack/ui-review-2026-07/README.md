# Flash Drum — Pack revue UI (juillet 2026)

Pack destiné au designer pour revoir l'interface **telle qu'implémentée aujourd'hui** dans le plugin VST3 (build `20260721-170521`, v0.2.0). La maquette d'origine (`../Flash_Drum_design_11062026/`) reste la référence visuelle initiale ; ce dossier documente **ce qui est réellement codé**, avec les écarts et évolutions depuis juin.

## Contenu

| Fichier | Rôle |
|---|---|
| `UI-STATE.md` | Inventaire complet de l'UI implémentée : zones, contrôles, interactions, états visuels. |
| `SKINS.md` | Les 3 skins (Dark / Midnight / Ember) avec toutes les valeurs RGB par token. |
| `skins.json` | Les mêmes palettes en JSON exploitable (import outil design). |
| `TOKENS.md` | Tailles, rayons, espacements, typographie (IBM Plex). |
| `screenshots/` | Captures de l'UI réelle (à régénérer — voir ci-dessous). |

## Évolutions majeures depuis la maquette de juin

- **Skins** : l'UI est entièrement thémée à l'exécution. Sélecteur `Skin` dans le popup Settings (⚙ en haut à droite), persisté dans `Documents/Flash Drum/config.json`. 3 skins : **Dark** (palette d'origine), **Midnight**, **Ember**.
- **14 slots modulaires** : la grille affiche toujours 14 lanes (hauteur fixe). Les lanes vides montrent une pastille `+N` → popup de choix d'instrument (11 voix). Lanes actives : ajout, suppression (menu lane), changement de kind (onglet Track).
- **Sequencer p-locks** (mode violet) en plus des sound p-locks (orange) : probabilité, stutter, condition, micro-timing par step. Sélecteur `P-Lock Mode : Sound / Sequencer` sous la grille.
- **Fusion morph** : morphing de paramètres sur les cellules fusionnées (menu Morph).
- **Song editor** : 16 blocks avec pattern + répétitions, copier/coller/dupliquer/clear, toujours en boucle.
- **Page bar** : boutons Page 1-4 + LED de lecture, Follow, Len (16-64), ×2, dropdown Preset (Clear All / Preset 4 / Preset 12 avec popup de confirmation).
- **Drag & drop de steps** (appui long sur une cellule pour déplacer step + plocks) et **réordonnancement de lanes** par la poignée gauche.
- **Settings popup** : Default Analog, Global MIDI Channel, Skin.

## Captures d'écran à fournir

Fenêtre fixe **1480 × 900**. À recapturer dans Studio One (les anciennes `screenshots/ui01-04.png` à la racine sont obsolètes) :

1. `ui-dark-full.png` — UI complète skin Dark, pattern rempli, page 1.
2. `ui-midnight-full.png` — même vue en skin Midnight.
3. `ui-ember-full.png` — même vue en skin Ember.
4. `ui-plock-sound.png` — popup p-lock Sound ouvert sur une step active.
5. `ui-plock-seq.png` — popup p-lock Sequencer ouvert (mode violet).
6. `ui-fusion.png` — une fusion sélectionnée + box d'édition.
7. `ui-song-tab.png` — panneau bas onglet Song.
8. `ui-track-tab.png` — Sound Editor onglet Track.
9. `ui-settings.png` — popup Settings (avec sélecteur Skin).
10. `ui-add-module.png` — popup Add Module sur une lane vide.

Déposer les PNG dans `screenshots/` de ce dossier.
