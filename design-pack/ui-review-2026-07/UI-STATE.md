# UI State — Flash Drum (implémentation actuelle, build 20260721-170521)

Fenêtre fixe **1480 × 900**. Layout : header en haut, colonne gauche (pattern bank + grille + panneau bas), colonne droite (Sound Editor).

---

## 1. Header (hauteur 44 px)

Fond `panel`, bordure basse `line`. De gauche à droite :

- **FLASH DRUM** (sans bold 15, blanc) + `v0.2.0 · <build_id>` (mono 9.5, `ink3`)
- **Master** — slider pill 172 px (label + valeur mono)
- **Swing** — slider pill 172 px
- **Groove** — select (Straight / Swing16 / Shuffle / MPC)
- **Seq** — segmented Internal / Ext MIDI. Passer en Ext MIDI coupe automatiquement MIDI Pat.
- **Choke** / **Auto-Edit** / **MIDI Pat** — toggles LED (MIDI Pat désactivé en Ext MIDI)
- **Settings** — bouton texte à droite → popup Settings

Séparateurs verticaux 1 px (`line`) entre les groupes.

## 2. Pattern bank bar

- **Export** — chip → export `.mid` dans `Documents/Flash Drum/exports`
- **Drag** — chip → helper de drag-and-drop MIDI vers le DAW
- **Save** — bouton qui clignote bleu en mode save ; ensuite cliquer un slot P1-P8
- **P1-P8** — 8 slots 30×26 px. Slot chargé : fond `p_active` + anneau vert (`green`). Occupé : `panel2`. Vide : `bg`. `P{n}*` si modifications non sauvées. Clic droit sur occupé = copier, sur vide = coller.
- **Clr** — efface steps + plocks + fusions (double confirmation, rouge clignotant)
- Indicateur `Exported` / `Export failed` + Copy Path.

## 3. Page bar (au-dessus de la grille)

- **Page** : boutons 1-4 (actif = fill `blue`, LED de lecture sous la page en cours), Follow
- **Len** : drag value 1-64 (master length) + libellé "NN steps"
- **×2** : double la longueur en copiant les steps
- **Preset** : dropdown → Clear All / Preset 4 / Preset 12, avec popup de confirmation (Warning rouge)
- Sous la page bar : **P-Lock Mode** segmented `Sound` (orange) / `Sequencer` (violet) + hint "Right-click a step to edit its p-lock" + **Fusion box** à droite (édition du nombre de steps d'une fusion sélectionnée)

## 4. Grille — 14 lanes (hauteur constante, 24 px/lane)

Colonnes par lane : **grip** 14 (drag reorder + indicateur), **nom** 50 (clic = sélection, double-clic = renommer dans Track, clic droit = menu Copy Lane / Paste Lane / Paste Grid / Clear Lane / Randomize / Delete), **Vol** 56 (mini-slider), **M / S** tags (amber / green, fonds `mute_fill` / `solo_fill`), **16 steps** (4 pages max 64), **Hum** 44 (mini-slider %), **Push** 44 (mini-slider ms), **Len** 35 (drag, lock par lane).

Lane vide : fond assombri + pastille **+N** cliquable → popup Add Module (11 instruments).

### États des cellules step

| État | Rendu |
|---|---|
| Vide (temps fort) | `cell_empty_beat` |
| Vide (autres) | `cell_empty_off` |
| Active | plein `blue` |
| Sound p-lock | plein `pl_link` (orange) |
| Seq p-lock | plein `seqpl` (violet) |
| P-lock sur step inactive | fond sombre + bordure `pl_link_dim` / `pl_snap_dim` |
| Au-delà de la longueur de lane | `cell_disabled` + bordure noire pointillée |
| Tête de lecture | anneau blanc intérieur pulsé (alpha 120-200) |
| Step courante (inactive) | `cell_current` |
| Fusion start | bloc continu sur N cellules + compteur |
| Sélection fusion | bordure `blue`, fond `fusion_fill` |
| Drag source | pulse blanc ; Drag target : bordure `drag_target` |

### Interactions

- **Clic gauche** : toggle step
- **Clic droit** : popup p-lock (Sound ou Sequencer selon le mode) — interdit en Song Mode ou Follow ON
- **Shift + clic** : sélection de fusion ; **double-clic** : éditer la fusion (steps)
- **Appui long** sur step active : drag pour déplacer (step + sound/seq plocks)
- **Drag de la poignée** : réordonner les lanes (toutes les données suivent : steps, plocks, settings, routing)

## 5. Panneau bas (210 px) — Generator | Song

Segmented **Generator** (bleu) / **Song** (orange) + meta à droite.

- **Generator** : Presets (Rock / Funk / Disco / ⟳ Random), combo algorithme (Classic / Euclidean / Markov / Probabilistic), styles A et B, sliders Mix A/B + Density + Variation, bouton **GENERATE** (bleu).
- **Song** : checkbox Song Mode + Clear All (confirm). 16 blocks 64 px : dropdown pattern (P1-P8 ou --) + drag repeat ×1-×64. Menu clic droit : Copy / Paste / Duplicate / Clear. Block courant = bleu.

## 6. Sound Editor (colonne droite)

Header : **Sound Editor** (bold 13) + `Slot N - <nom>` (mono, `ink3`). Onglets **Sound** / **Track** (pleine largeur, actif = bleu).

### Onglet Sound

- **Volume** en tête (slider pleine largeur, range 0-2)
- Sections data-driven, **masquées si vides** : Oscillator, Envelope, Analog, Filter, Saturation, Output
- Labels 138 px à gauche, sliders (track 6 px, fill bleu, handle au hover, double-clic = reset), valeur mono à droite (52 px)
- **Kick / BassDrum808** : fréquence en Hz ou Notes (toggle Hz/Note, boutons +/-)
- Graphes inline à droite : enveloppe ADSR (famille Envelope), enveloppe filtre (famille Filter)
- Selects typés : Saturation Type, Noise Type, Click Type
- **Algorithm** (combo) dans la famille Oscillator si > 1 algo
- Dev only (debug) : section Preset Dumps

### Onglet Track

- **Name** (champ 6 caractères), **Instrument** (combo 11 kinds — change le kind, reset les settings du slot)
- **Routing** : checkbox Main + Out (No Aux / Out 1-14, exclusif par slot)
- **MIDI** : Channel (global, affichage) + Note (drag 0-127)
- **Sequencing** : Length (drag + lock)

## 7. Popups (foreground, fermeture au clic extérieur)

| Popup | Contenu |
|---|---|
| **P-lock Sound** | Link to Global / Snapshot Current Settings / Paste, indicateur de mode (Linked / Full Snapshot / Mixed), sliders par champ (Volume, Freq, Decay, …, specials, algo), Copy / Paste / Clear Plock |
| **P-lock Sequencer** | Create / Clear Seq Plock, Probability, Stutter (1-16), Condition (grille 3×N), micro-timing |
| **Fusion morph** | Morphing (cibles), Edit Fusion Steps, Delete Fusion ; sliders Target/Source par champ morphable |
| **Page menu** | Copy / Paste (confirm overwrite) / Clear (confirm) |
| **Add Module** | liste des 11 instruments pour la lane vide |
| **Settings** | Default Analog (slider), Global MIDI Channel (drag 1-16), **Skin** (Dark / Midnight / Ember) |
| **Lane preset warning** | Apply / Cancel (rouge) |

Chrome : fond `p_active`, barre d'accent 3 px en haut, coins `radius_panel` (9 px).

## 8. Skins

3 skins runtime (voir `SKINS.md` / `skins.json`) : **Dark** (défaut = palette d'origine), **Midnight** (bleu nuit), **Ember** (chaud/ambré). Changement immédiat via Settings, persistance `Documents/Flash Drum/config.json` (`skin`).
