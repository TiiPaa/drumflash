# Changelog

## 2026-07-09 — Clear Grid confirmé remplace Clear Lane (build 20260709-182258)

**Build:** `20260709-182258`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants), `cargo test` OK (138 + 85 tests), `build.ps1 -Install` OK (Studio One fermé pour l'installation)

### Changements
- **[116] `Clear Lane` remplacé par `Clear Grid` avec confirmation.**
  - `src/ui.rs` : l'entrée destructive du menu contextuel ne supprime plus le module/lane.
  - Premier clic : le menu passe de `Clear Grid` à `Confirm Clear Grid?`.
  - Deuxième clic : efface les steps, fusions, sound plocks et seq plocks de la lane active.
  - Les données non-grid restent intactes : instrument, réglages sonores, algo, Hum/Push/Len, lock Len, routing, mute/solo/mix et note MIDI.
  - `Clear Grid` n'est disponible que sur une lane active ; les lanes vides gardent seulement `Paste Lane`.

### À tester dans Studio One (build 20260709-182258)
1. Clic droit sur une lane active → `Clear Grid` : au premier clic, rien ne doit être effacé et le menu doit demander `Confirm Clear Grid?`.
2. Cliquer `Confirm Clear Grid?` : tous les steps de cette lane doivent disparaître et la lane ne doit plus déclencher de hits.
3. Vérifier que la lane reste active avec le même instrument, les mêmes réglages Sound, le même routing, la même note MIDI, les mêmes Hum/Push/Len et les mêmes états mute/solo/mix.
4. Sur une lane avec fusions, sound plocks et seq plocks → `Clear Grid` confirmé : recréer des steps aux mêmes positions ne doit pas faire réapparaître les anciennes fusions/plocks.
5. Clic droit sur une lane vide : `Clear Grid` ne doit pas être proposé ; `Paste Lane` doit rester disponible si un clipboard existe.

---

## 2026-07-09 — Clear Lane dans le menu contextuel (build 20260709-181427)

**Build:** `20260709-181427`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants), `cargo test` OK (138 + 85 tests), `build.ps1 -Install` OK

### Changements
- **[116] Ajout de `Clear Lane` dans le menu contextuel d'une lane active.**
  - `src/ui.rs` : action rouge sous `Copy Lane` / `Paste Lane` / `Paste Grid`.
  - `Clear Lane` désactive le slot dans `track-layout-v1`, ce qui remet immédiatement la rangée en lane vide.
  - Nettoyage complet des données cachées du slot : steps, fusions, sound plocks, seq plocks.
  - Remise à l'état neutre des contrôles par lane : mute off, solo off, mix on, algo 0, Hum 0, Push 0, Len 16, lock Len off.
  - Le routing du slot est réinitialisé via `TrackSlot::inactive()` ; le clipboard de lane reste disponible pour pouvoir clear puis coller ailleurs.
  - Après clear, la sélection UI bascule vers la prochaine lane active, ou reste sur le slot vidé si aucune lane active ne reste.

### À tester dans Studio One (build 20260709-181427)
1. Clic droit sur une lane active → `Clear Lane` : la rangée doit devenir vide (`+N` / Empty) et ne plus jouer de son.
2. Sur une lane avec steps, fusions, sound plocks et seq plocks → `Clear Lane`, puis recréer une lane au même slot : aucun ancien step/plock/fusion ne doit réapparaître.
3. Mettre une lane en mute/solo, changer Hum/Push/Len et activer lock Len, puis `Clear Lane` : en recréant une lane au même slot, ces contrôles doivent être revenus à l'état neutre.
4. Faire `Copy Lane`, puis `Clear Lane` sur la source, puis `Paste Lane` ailleurs : le clipboard doit encore coller la lane copiée.
5. Effacer la dernière lane active : l'UI doit rester stable, afficher le slot vide sélectionné, sans crash ni décalage des panneaux.

---

## 2026-07-09 — Copier/coller lanes + cleanup Analog HH/OH (build 20260709-180637)

**Build:** `20260709-180637`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants), `cargo test` OK, `build.ps1 -Install` OK (Studio One fermé pour l'installation)

### Changements
- **[116] Copier/coller une lane depuis le menu contextuel de la grille.**
  - `src/ui.rs` : ajout d'un clipboard de lane en mémoire (`LaneClipboardData`) et des actions `Copy Lane`, `Paste Lane`, `Paste Grid` sur les lanes actives.
  - Les lanes vides acceptent `Paste Lane`, ce qui active le slot cible avec l'instrument du clipboard.
  - `Paste Lane` copie instrument, settings sonores complets, algo, steps, fusions, sound plocks, seq plocks, Humanize, Push/Pull, Len et lock Len.
  - `Paste Grid` remplace `Paste Params` et copie uniquement les steps on/off de la grille sur une lane active cible.
  - `Paste Grid` ne change pas l'instrument, les settings sonores, l'algo, les fusions, les plocks, Humanize, Push/Pull, Len, lock Len, routing, mute/solo/mix ni la note MIDI du slot cible.
  - Routing, Main/Out, note MIDI source personnalisée, mute/solo/mix ne sont pas copiés pour éviter les effets de bord dans une session Studio One.
- **Snapshot de settings par slot pour le clipboard.**
  - `src/sound_settings.rs` : ajout de `SoundSettings` + `get_settings_for_slot()` / `set_settings_for_slot()` pour copier les standards, specials et le mode Hz/Notes sans repasser par les anciens params legacy.
- **Correction de l'implémentation précédente dangereuse.**
  - Le collage des steps ne passe plus par une sérialisation de `Pattern` ni par un remplacement de rangée global ; il ne modifie que le bit du slot cible dans chaque step.
  - Le fichier `shared_pattern_clipboard.rs` a été retiré et `Pattern` reste non sérialisé.
- **Cleanup Analog HH/OH.**
  - Le drift timing HiHat / OpenHiHat ne crée plus de délai silencieux avant le hit (`timing_delay_samples` reste à 0 au trigger).
  - Les logs de debug `println!` ajoutés pendant l'itération Analog ont été retirés.

### À tester dans Studio One (build 20260709-180637)
1. Clic droit sur une lane active (ex. BD) → `Copy Lane`, puis clic droit sur une lane vide (`+N`) → `Paste Lane` : le slot doit s'activer avec le même instrument, le même son et la même séquence.
2. Sur une lane source avec fusions, sound plocks, seq plocks, Hum/Push/Len et Len lock, faire `Copy Lane` puis `Paste Lane` vers une autre lane : tous ces éléments doivent suivre la copie.
3. Sur une lane active cible d'un instrument différent, faire `Paste Grid` : seuls les steps on/off doivent être remplacés ; l'instrument, le son, l'algo, les fusions/plocks, Hum/Push/Len, routing, mute/solo/mix et note MIDI de la cible doivent rester inchangés.
4. Clic droit sur une lane vide : vérifier que `Paste Params` n'existe plus et que `Paste Grid` n'est pas proposé ; seul `Paste Lane` doit permettre d'activer la lane depuis le clipboard.
5. Vérifier que `Paste Lane` ne recopie pas le routing Main/Out ni mute/solo/mix depuis la source : le slot cible ne doit pas déplacer le son vers une sortie inattendue ni hériter d'un état mute/solo.
6. Sur HiHat et OpenHiHat, jouer une pattern dense avec `Analog` élevé : les hits ne doivent pas être retardés/silencieux au départ (régression possible du drift timing), et le tone doit toujours varier par hit.

---

## 2026-07-09 — Analog : tone drift sur HiHat / OpenHiHat / Clap / Ride / Cymbal (build 20260709-141013)

**Build:** `20260709-141013`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants), `cargo test` OK (138 + 85 tests), `build.ps1 -Install` OK (Studio One fermé pour l’installation)

### Changements
- **[115] Le paramètre `Analog` module désormais le tone des instruments non-tonaux.**
  - `src/synthesis/dsp.rs` : ajout de `ToneDrift` avec une profondeur configurable par instrument (0.0 = déterministe, 1.0 = maximum).
  - `src/synthesis/hihat.rs` / `open_hihat.rs` : `Analog` décale le centre du peaking filter (`Tone`) de **±25 %** par hit.
  - `src/synthesis/ride.rs` : `Analog` décale la fréquence de base des oscillateurs inharmoniques (`Frequency`) de **±7.5 %** par hit.
  - `src/synthesis/clap.rs` : `Analog` décale le highpass cutoff de **±25 %** par hit.
  - `src/synthesis/cymbal.rs` : `Analog` décale le highpass cutoff de **±25 %** avant la modulation shimmer.
  - Anti-click : pas de reset de phase, filtre ou générateur de bruit ; la dérive est échantillonnée au trigger et appliquée via les setters de fréquence existants.
  - Tests ajoutés : `tone_drift_is_deterministic_at_zero_and_varies_at_full` (dsp.rs), `test_hihat_analog_affects_tone`, `test_open_hihat_analog_affects_tone`, `test_ride_analog_affects_tone`, `test_clap_analog_affects_tone`, `test_cymbal_analog_affects_tone`.

### À tester dans Studio One (build 20260709-141013)
1. **HiHat** : monter `Analog` à fond et jouer une pattern dense → le pic métallique doit varier clairement d’un hit à l’autre (régression précédente : inaudible avec ±7.5 %).
2. **OpenHiHat** : idem, vérifier que le tone varie.
3. **Ride** : `Analog` à fond → le timbre métallique doit varier (conservé à ±7.5 %).
4. **Clap** : `Analog` à fond → le tone/la brillance doit varier par hit.
5. **Cymbal** : `Analog` à fond → le haut du spectre (cutoff) doit fluctuer.
6. **Regression** : mettre `Analog` à 0 sur ces instruments → chaque hit doit être identique en tone.
7. Vérifier que les instruments tonaux (Kick, Snare, Tom, Kick808, Perc1, Snare606) conservent leur comportement `AnalogDrift` sans régression.
8. Recharger une session sauvegardée avant cette build : le slider `Analog` reste restauré et s’applique au tone des non-tonaux.

---

**Build:** `20260709-123527`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants), `cargo test` OK (132 + 79 tests), `build.ps1 -Install` OK (Studio One fermé pour l’installation)

### Changements
- **[115] Nouvelle section `Analog` dans le Sound Editor, positionnée entre `Envelope` et `Filter`.**
  - `src/instrument_registry.rs` :
    - Ajout de `ParamFamily::Analog`.
    - Déplacement du champ `StandardField::Analog` de `ParamFamily::Output` vers `ParamFamily::Analog` dans toutes les listes standard des 13 instruments.
  - `src/ui.rs` :
    - `sound_family_sections()` affiche désormais la section `Analog` après `Envelope` et avant `Filter`.
    - Ordre final des familles : `Osc`, `Env`, `Analog`, `Filter`, `Sat`, `Output`.
  - `src/synthesis/mod.rs` / `src/synthesis/voice.rs` : `AnalogDrift` reste piloté par le paramètre `analog` pour les instruments tonaux (Kick, Snare, Tom, Kick808, Perc1, Snare606).
  - Les instruments non tonaux (HiHat, OpenHiHat, Clap, Ride, Cymbal, Zap) affichent le slider `Analog` mais ne l’appliquent pas encore en synthèse (pas d’implémentation `AnalogDrift` sur ces voix).
  - Tests de régression existants : `cargo test` OK (132 + 79 tests), `cargo check` OK, pas de nouveaux tests spécifiques ajoutés pour cette refonte.

### À tester dans Studio One (build 20260709-123527)
1. Sélectionner une lane Kick (ou Snare, Tom, Kick808, Perc1, Snare606) → onglet `Sound` : vérifier que la section `Analog` apparaît entre `Envelope` et `Filter` (et non plus dans `Output`).
2. Sur la même lane tonal, monter `Analog` à fond et jouer une pattern rapide : chaque hit doit légèrement varier en hauteur/niveau/temps (drift analogique audible).
3. Sélectionner une lane HiHat (ou OpenHiHat, Clap, Ride, Cymbal, Zap) → onglet `Sound` : vérifier que `Analog` est aussi visible entre `Envelope` et `Filter`.
4. Sur une lane non tonale, bouger le slider `Analog` : le son ne doit pas changer pour l’instant (comportement attendu, pas de drift implémenté sur ces voix).
5. Vérifier que la section `Output` n’affiche plus le slider `Analog` (il n’y reste que `Volume`, `Stereo`, `Main`, `Out`, etc.).
6. Créer un plock sur un step Kick/Snare : vérifier que `Analog` est proposé dans le menu Plock (Snapshot/Link/Morph) et que sa valeur s’applique.
7. Créer une Fusion sur une lane Kick/Snare : vérifier que `Analog` est proposé dans le menu Morph et que le morph entre la valeur globale et la cible s’entend.
8. Recharger une session sauvegardée avant cette build : le paramètre `Analog` doit être restauré à sa valeur globale et le slider doit être visible dans la nouvelle section.
9. Vérifier que les autres instruments conservent leur ordre de sections habituel et que `Env` reste juste avant `Analog`.

---

**Build:** `20260709-121611`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants), `cargo test` OK (132 + 79 tests), `build.ps1 -Install` OK (Studio One fermé pour l’installation)

### Changements
- **[114] Refonte complète du panneau sonore HiHat / OpenHiHat.**
  - `src/instrument_registry.rs` :
    - `Frequency` → `Tone` (range 100–20000 Hz).
    - `Filter` → `Cutoff` (range 100–20000 Hz).
    - Ajout des paramètres spéciaux : `Noise Type` (White/Pink/Brown/Blue), `Resonance` (0.1–10.0), `Shimmer` (0.0–1.0).
    - Suppression de l’algorithme `Bright` (`algo_count: 1`).
  - `src/synthesis/dsp.rs` : ajout de l’enum `NoiseSource` pour sélectionner le type de bruit sans allocation.
  - `src/synthesis/hihat.rs` / `open_hihat.rs` :
    - Remplacement des générateurs `WhiteNoise` fixes par `NoiseSource`.
    - Le peaking filter utilise désormais `settings.resonance` à la place de Q=2.0 fixe.
    - Ajout d’un chemin shimmer parallèle (bruit **bleu** high-pass à 8 kHz, mixé selon `Shimmer` avec un gain de 2.0).
    - Suppression de la branche `algo == 1` (Bright).
  - `src/synthesis/settings/hihat.rs` / `open_hihat.rs` : mapping des nouveaux special params `special[5]` (noise type), `special[6]` (resonance), `special[7]` (shimmer).
  - `src/synthesis/mod.rs` : mise à jour des valeurs par défaut de `VoiceSettings::hihat()` / `open_hihat()` (`resonance = 2.0`).
  - `src/synthesis/special_params.rs` : `HIHAT_ALGOS` réduit à `[Standard]`.

### À tester dans Studio One (build 20260709-121611)
1. Sélectionner la lane HiHat (HH) → onglet `Sound` : vérifier que les paramètres sont `Tone`, `Cutoff`, `Resonance`, `Noise Type`, `Shimmer` (plus d’algorithme).
2. Sélectionner la lane OpenHiHat (OH) → idem, vérifier que les labels sont identiques et qu’il n’y a pas de dropdown Algorithme.
3. Bouger `Tone` de 100 à 20000 Hz sur HH : le pic métallique doit se déplacer clairement dans les graves/aigus.
4. Bouger `Cutoff` : vérifier que l’on comprend la relation avec `Tone` (Cutoff enlève les basses, Tone pousse une bande).
5. Bouger `Resonance` de 0.1 à 10 : à 0.1 le pic doit être très large/doux, à 10 très aigu et crécelle.
6. Changer `Noise Type` (White / Pink / Brown / Blue) : le timbre doit changer de manière audible (White = standard, Brown = sombre, Blue = très aigu).
7. Monter `Shimmer` à 1.0 : un bruit d’air/souffle aigu doit apparaître clairement par-dessus le HiHat (régression à surveiller : rester inaudible).
8. Jouer une pattern avec HH et OH : vérifier qu’il n’y a plus de dropdown Algorithme et que le son est proche de l’ancien `Standard` par défaut.
9. Recharger une session sauvegardée avec des réglages HiHat : les valeurs doivent être restaurées (champs persistants inchangés, seuls les labels et les ranges ont changé).
10. Vérifier que les autres instruments (Kick, Snare, Tom, etc.) conservent encore leurs labels et ranges d’origine.

---

## 2026-07-09 — Clarification HiHat : `Frequency` renommé en `Tone` (build 20260709-101109)

**Build:** `20260709-101109`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants), `cargo test` OK (132 + 79 tests), `build.ps1 -Install` OK (Studio One fermé pour l’installation)

### Changements
- **[114] Clarification du paramètre HiHat / OpenHiHat.**
  - `src/instrument_registry.rs` : création de `HIHAT_STD` et `OPENHIHAT_STD` où le label de `StandardField::Freq` est "Tone" au lieu de "Frequency".
  - Les instruments HiHat et OpenHiHat utilisent désormais ces listes dédiées ; le champ persistant reste `frequency` (pas de migration nécessaire, seul le label UI change).
  - `src/synthesis/hihat.rs` : documentation du rôle de `frequency` (centre du filtre peaking, Q=2, gain=6 dB) et des deux algorithmes (`Standard` vs `Bright`).
  - `src/synthesis/open_hihat.rs` : documentation similaire du peaking filter et du partage des algos avec le HiHat fermé.
  - `src/synthesis/special_params.rs` : commentaire structuré sur les algos HiHat (Standard/Bright).

### À tester dans Studio One (build 20260709-101109)
1. Sélectionner la lane HiHat (HH) → onglet `Sound` : le premier paramètre doit s’appeler `Tone` et non plus `Frequency`.
2. Sélectionner la lane OpenHiHat (OH) → onglet `Sound` : idem, le premier paramètre doit être `Tone`.
3. Jouer une pattern avec des notes HH/OH et bouger le knob `Tone` : on doit entendre clairement le pic métallique se déplacer dans les aigus/graves (pas de silence ou d’effet absent).
4. Passer l’algorithme de `Standard` à `Bright` sur HH ou OH : le son doit devenir plus brillant/saturation légèrement accentuée (régression à surveiller : son identique entre les deux algos).
5. Vérifier que les autres instruments (Kick, Snare, Tom, etc.) conservent encore leur label `Frequency`.
6. Recharger une session sauvegardée avec des réglages HiHat : les valeurs `Tone` doivent être restaurées (le champ persistant est inchangé).

---

## 2026-07-08 — Song Editor : finition, blocks vides assombris, Clear All avec confirmation (build 20260708-185335)

**Build:** `20260708-185335`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants), `cargo test` OK (132 + 79 tests), `build.ps1 -Install` OK (Studio One fermé pour l’installation)

### Changements
- **[112] Polish du Song Editor.**
  - `src/ui.rs` : les widgets pattern/répétition sont maintenant encadrés avec une marge de 2 px à l’intérieur du block, ce qui évite que les bordures débordent quand le block est sélectionné.
  - `src/ui.rs` : les blocks sans pattern ont un fond assombri (`Color32::from_rgb(18, 18, 24)`).
  - `src/ui.rs` : suppression du bouton `Reset`.
  - `src/ui.rs` : `Clear All` demande maintenant une confirmation via un état `song_clear_confirm` (le bouton devient rouge "Confirm?" après le premier clic).
  - Menu contextuel `Copy / Paste / Duplicate / Clear` conservé.

### À tester dans Studio One (build 20260708-185335)
1. Ouvrir l’onglet `Song` → vérifier que les widgets ne débordent plus du contour du block sélectionné.
2. Vérifier que les blocks vides sont plus sombres que les blocks occupés.
3. Confirmer que le bouton `Reset` a disparu.
4. Cliquer sur `Clear All` → le bouton doit devenir rouge `Confirm?` ; cliquer une seconde fois vide la song.
5. Clic droit sur un block → `Copy / Paste / Duplicate / Clear` fonctionnent toujours.
6. Éditer pattern/répétition directement dans les blocks et lire la song pour vérifier le comportement de boucle.

---

## 2026-07-08 — Song Editor : panneau agrandi, édition directe dans les blocks (build 20260708-183824)

**Build:** `20260708-183824`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants + 2 nouveaux `allocate_ui_at_rect` dans la grille), `cargo test` OK (132 + 79 tests), `build.ps1 -Install` OK

### Changements
- **[112] Suite de la refonte du Song Editor.**
  - `src/ui.rs` : hauteur du panneau Song/Generator passée de 180 px à 210 px.
  - Suppression de la rangée d’inspection `Step X` / dropdown Pattern / `Rpt`.
  - Chaque block est maintenant éditable directement : partie supérieure = `ComboBox` de pattern, partie inférieure = `DragValue` de répétition (`xN`).
  - La sélection du block (stroke bleue) est conservée ; le clic sur le fond d’un block le sélectionne.
  - Menu contextuel (clic droit) conservé : `Copy / Paste / Duplicate / Clear`.

### À tester dans Studio One (build 20260708-183824)
1. Ouvrir l’onglet `Song` → vérifier que le panneau est plus haut (210 px) et que les 16 blocks ne sont plus tronqués.
2. Cliquer sur la partie supérieure d’un block → un dropdown permet de choisir le pattern (`P1`–`P8` ou vide).
3. Cliquer/glisser sur la partie inférieure d’un block → régler le nombre de répétitions (`x1`–`x64`).
4. Vérifier qu’il n’y a plus de rangée `Step X` / `Rpt` sous l’en-tête Song.
5. Vérifier que la lecture en mode Song boucle correctement sur les blocks remplis et revient au début sur un block vide.
6. Clic droit sur un block → `Copy / Paste / Duplicate / Clear` fonctionnent.

---

## 2026-07-08 — Song Editor : 16 blocks fixes, mode Song via checkbox, loop implicite (build 20260708-182802)

**Build:** `20260708-182802`
**Validation:** `cargo fmt` OK, `cargo check` OK (39 warnings UI préexistants), `cargo test` OK (132 + 79 tests), `build.ps1 -Install` OK (Studio One fermé pour l’installation)

### Changements
- **[112] Refonte du Song Editor.**
  - `src/pattern_bank.rs` : ajout de `SONG_BLOCKS = 16` et `SongSequence::default().length = 16`.
  - `src/lib.rs` : la longueur du mode song est plafonnée à 16 blocks ; quand le prochain block est vide, la song revient au début ; en fin de song, elle boucle toujours ; le paramètre `loop_enabled` n’est plus utilisé.
  - `src/ui.rs` : l’onglet `Generator | Song` ne fait plus que changer la vue (`bottom_panel_tab` dans `EditorUIState` sérialisé).
  - `src/ui.rs` : ajout d’une checkbox `Song Mode` dans le panneau Song pour activer/désactiver le mode song.
  - Suppression du bouton `Loop` et du paramètre `Len` du mode song.
  - Grille désormais une seule rangée de 16 blocks, chaque cellule affiche le pattern en haut (`P1`) et le nombre de répétitions en bas (`x4`), ou `--` si vide.
  - `Clear All` et `Duplicate` ne touchent plus que les 16 premiers blocks.

### À tester dans Studio One (build 20260708-182802)
1. Ouvrir l’onglet `Song` → vérifier que la grille affiche 16 blocks en une ligne.
2. Cocher `Song Mode` → la lecture doit suivre la song (et non plus le pattern courant).
3. Décocher `Song Mode` → la lecture revient au pattern classique.
4. Remplir les blocks 1-3 avec des patterns, régler leurs répétitions, puis laisser le block 4 vide → en lecture, la song doit boucler sur les 3 premiers blocks et ne jamais avancer au-delà.
5. Remplir le block 16 → en fin de song, elle doit repartir au block 1 automatiquement.
6. Vérifier que le bouton `Loop` a disparu et que `Len` n’est plus présent dans le panneau Song.
7. Clic droit sur un block → `Copy / Paste / Duplicate / Clear` doivent fonctionner sans dépasser 16 blocks.
8. Passer à l’onglet `Generator` puis revenir à `Song` : le panneau doit revenir à la vue Song, et la checkbox doit refléter l’état réel du mode song.

---

## 2026-07-08 — Song Editor : fixes UI dropdown / repeat / couleur / hauteur de ligne (build 20260708-171322)

**Build:** `20260708-171322`
**Validation:** `cargo fmt` OK, `cargo check` OK (39 warnings UI préexistants), `cargo test` OK (132 + 79 tests), `build.ps1 -Install` OK

### Changements
- **[112] Fix UI Song Editor.**
  - `src/ui.rs` : le dropdown de pattern de l’inspecteur utilisait `ui.selectable_label(...).clicked()` et mettait à jour le bank à l’intérieur du `show_ui` ; il est remplacé par `ui.selectable_value(&mut slot, ...)` avec mise à jour du bank après fermeture du popup.
  - Le nombre de répétitions est maintenant affiché dans la cellule de la grille sous la forme `P1x4` (uniquement si `repeat > 1`).
  - Le step courant utilise le bleu `BLUE` à la place du rouge vif, avec le texte en `INK` (blanc) pour rester lisible.
  - Les cellules sont dimensionnées avec `ui.add_sized(Vec2::new(cell_w, cell_h), btn)` et `cell_h` passe à 18 px, ce qui empêche les rangées 2-3-4 d’être absorbées/coupées par la mise en page.
  - Le contexte du bouton de cellule propose `Copy / Paste / Duplicate / Clear`.

### À tester dans Studio One (build 20260708-171322)
1. Ouvrir l’onglet `Song`, sélectionner une step, ouvrir le dropdown `Pattern` et choisir un pattern occupé → la case de la grille affiche `P1` et le step joue ce pattern.
2. Régler `Rpt` à 3 ou plus → la grille affiche `P1x3`, et la lecture répète le pattern 3 fois.
3. Lancer la lecture en mode Song et regarder le step courant → le fond est bleu et le texte blanc reste lisible (pas de rouge).
4. Remplir des steps au-delà de la 16e (rangées 2, 3, 4) → toutes les cases 4×16 sont visibles et alignées.
5. Faire un clic droit sur une cellule → `Copy / Paste / Duplicate / Clear` fonctionnent comme avant.

---

## 2026-07-08 — Song Editor : répétitions par step, inspecteur et workflow retravaillé (build 20260708-164626)

**Build:** `20260708-164626`
**Validation:** `cargo fmt` OK, `cargo check` OK (39 warnings UI préexistants), `cargo test` OK (132 + 79 tests), `build.ps1 -Install` OK

### Changements
- **[112] Rework du Song Editor.**
  - `src/pattern_bank.rs` : ajout du champ `repeats: [u8; 64]` dans `SongSequence` avec `#[serde(default)]` pour compatibilité `pattern-bank-v1`.
  - `src/lib.rs` : le moteur audio reste sur le step courant pendant `repeats` boucles de pattern avant d’avancer ; ajout de `song_repeat_counter` et `last_song_position` pour détecter les resets UI.
  - `src/ui.rs` : panneau Song/Generator passé de 144 px à 180 px ; nouvelle rangée d’inspection avec dropdown `P1-P8` / vide, compteur de répétitions, `Copy` / `Paste` / `Dup` / `Clear`.
  - La grille 4×16 reste : clic gauche sélectionne la step, clic droit menu `Copy / Paste / Duplicate / Clear`.
  - Boutons globaux `Reset` (remet la song à 0) et `Clear All`.
  - Suppression du toggle `Song Enabled` redondant ; le suivi du playhead song fonctionne dès que l’onglet `Song` est actif.
  - Reset automatique de `song_position` quand on quitte le mode Song ou quand le transport s’arrête.
- **Tests ajoutés.**
  - `song_sequence_repeat_clamps_and_defaults`, `pattern_bank_legacy_load_without_repeats_defaults_to_one`, `pattern_bank_persistence_roundtrips_song` mis à jour avec les répétitions.

### À tester dans Studio One (build 20260708-164626)
1. Basculer sur l’onglet `Song` → vérifier que le panneau est plus haut (180 px) et affiche la rangée d’inspection au-dessus de la grille.
2. Sélectionner une step, choisir un pattern dans le dropdown, régler `Rpt` à 4 → lire la song : le pattern doit boucler 4 fois avant de passer à la step suivante.
3. Remplir plusieurs steps avec des répétitions différentes, activer `Loop` et lire → la chaîne avance au bon rythme.
4. Cliquer `Reset` pendant la lecture → la song repart de la step 1 (la position audio se reset au prochain process).
5. Passer de l’onglet `Song` à `Generator` puis revenir à `Song` → la position de lecture doit être remise à 0.
6. Faire un clic droit sur une cellule de la grille → le menu doit proposer `Copy / Paste / Duplicate / Clear`.
7. Copier une step, coller sur une autre → le pattern et le nombre de répétitions doivent être transposés.
8. Vérifier que les songs sauvegardées avant cette build se chargent toujours (test de compatibilité `pattern-bank-v1`).
9. Vérifier que les répétitions sont conservées après sauvegarde/recharge du projet Studio One.

---

## 2026-07-08 — Grille : alignement précis de l’indicateur de drop (build 20260708-162542)

**Build:** `20260708-162542`
**Validation:** `cargo fmt` OK, `cargo check` OK (37 warnings UI préexistants), `cargo test` OK (130 + 79 tests), `build.ps1 -Install` OK

### Changements
- **[108] Correction du décalage du trait de drop.**
  - Le trait est maintenant dessiné à la limite exacte de la lane cible : en haut de la ligne visée, ou en bas de la dernière ligne pour un drop après la fin.
  - `compute_reorder_gap()` retourne un index de gap `0..14` ; `handle_lane_reorder_drop()` le clampe à l’index slot valide `0..13`.
  - `draw_lane_reorder_indicator()` utilise le haut de la ligne cible au lieu du milieu de l’intervalle, corrigeant notamment le positionnement à la fin de la grille.

### À tester dans Studio One (build 20260708-162542)
1. Glisser une lane vers le haut de la grille → le trait doit apparaître exactement au-dessus de la première ligne quand le curseur est dans la moitié supérieure de cette ligne.
2. Glisser une lane vers le bas de la grille → le trait doit descendre au bas de la dernière ligne quand on dépasse son centre, et non rester coincé au milieu de l’intervalle précédent.
3. Déplacer le curseur lentement entre deux lanes → le trait doit basculer nettement au bord supérieur de la lane cible, au même emplacement où la lane sera insérée.
4. Relâcher quand le trait est sur le bord supérieur d’une lane → la lane doit être insérée juste avant cette ligne.
5. Vérifier que les données (steps, plocks, volume, routing, etc.) suivent toujours la lane déplacée.

---

## 2026-07-08 — Grille : feedback visuel de drop pour le drag-reorder (build 20260708-161106)

**Build:** `20260708-161106`
**Validation:** `cargo fmt` OK, `cargo check` OK (37 warnings UI préexistants), `cargo test` OK (130 + 79 tests), `build.ps1 -Install` OK

### Changements
- **[108] Trait indicateur de position de drop pendant le drag-reorder des lanes.**
  - `src/ui.rs` : ajout de `compute_reorder_gap()` et `draw_lane_reorder_indicator()`.
  - Le trait bleu est dessiné dans l’intervalle entre deux lanes (ou au-dessus/en-dessous des extrémités) pendant qu’une poignée est traînée.
  - La position de drop est calculée à partir du centre vertical de chaque lane : le pointeur au-dessus du centre d’une lane déplace la ligne juste au-dessus, en dessous juste au-dessous.
  - `handle_lane_reorder_drop()` utilise maintenant cette logique gap-based pour déterminer l’index cible.

### À tester dans Studio One (build 20260708-161106)
1. Sur le layout 4 lanes, cliquer-glisser la poignée d’une lane → un trait bleu doit apparaître entre les lanes au fur et à mesure du déplacement du curseur.
2. Déplacer le curseur lentement d’une lane à l’autre → le trait doit basculer de manière nette au milieu de l’intervalle entre deux lanes.
3. Relâcher la poignée quand le trait est entre deux lanes → la lane doit être insérée à l’emplacement indiqué par le trait, pas sur la lane survolée.
4. Vérifier que les données (steps, plocks, volume, routing, etc.) suivent toujours la lane déplacée.

---

## 2026-07-07 — Grille : boutons de longueur stables quand `Len < 10` (build 20260707-174844)

**Build:** `20260707-174844`
**Validation:** `cargo fmt` OK, `cargo check` OK (37 warnings UI préexistants), `cargo test` OK (127 + 76 tests), `build.ps1 -Install` OK

### Changements
- **[107] Correction du déplacement des boutons `16/32/48/64` quand l'indicateur `Len` passe à un chiffre.**
  - `src/ui.rs` : l'indicateur `N steps` est maintenant dessiné dans un rectangle alloué en taille exacte.
  - Le nombre est formaté sur 2 digits (` 9`, `10`) et le texte `steps` garde une position fixe.
  - Les boutons de longueur ne dépendent plus de la largeur réelle du texte `9 steps` vs `10 steps`.

### À tester dans Studio One (build 20260707-174844)
1. Descendre `Len` global de `10` à `9` → les boutons `16`, `32`, `48`, `64` ne doivent plus bouger horizontalement.
2. Remonter `Len` de `9` à `10` → les boutons doivent rester exactement en place.
3. Tester `Len` `1`, `8`, `9`, `10`, `16` → l'indicateur change, mais le groupe de boutons reste stable.
4. Vérifier que les boutons `16`, `32`, `48`, `64` et `x2` restent cliquables et fonctionnels.

---

## 2026-07-07 — Grille : largeur stable quand `Len < 10` (build 20260707-174302)

**Build:** `20260707-174302`
**Validation:** `cargo fmt` OK, `cargo check` OK (37 warnings UI préexistants), `cargo test` OK (127 + 76 tests), `build.ps1 -Install` OK

### Changements
- **[107] Correction du décalage UI quand `Len` global passe sous 10.**
  - `src/ui.rs` : la zone complète `Len` de la page-bar réserve désormais une largeur fixe.
  - L'affichage `N steps` a aussi une sous-zone fixe, donc `9 steps` ne réduit plus le bloc par rapport à `10 steps`.
  - La grille conserve la même largeur et le reste de l'interface ne doit plus bouger lors du passage `10 -> 9` ou `9 -> 10`.

### À tester dans Studio One (build 20260707-174302)
1. Dans la page-bar, descendre `Len` global de `10` à `9` avec le slider → le bloc grille ne doit pas rétrécir et aucun panneau ne doit se décaler.
2. Remonter `Len` global de `9` à `10` → aucune expansion/saut horizontal ne doit apparaître.
3. Tester aussi `Len` `1`, `8`, `16`, `32` → la page-bar doit rester stable et les cellules hors longueur doivent toujours s'afficher correctement.
4. Vérifier que les boutons `16`, `32`, `48`, `64` et `x2` fonctionnent toujours.

---

## 2026-07-07 — Grille : contraste inactif renforcé + lanes non activées (build 20260707-173031)

**Build:** `20260707-173031`
**Validation:** `cargo fmt` OK, `cargo check` OK (37 warnings UI préexistants), `cargo test` OK (127 + 76 tests), `build.ps1 -Install` OK

### Changements
- **[107] Contraste renforcé entre cellules actives et inactives.**
  - `src/ui.rs` : l'état disabled utilise maintenant un fond beaucoup plus sombre (`10,10,14`) et une bordure pointillée noire plus épaisse.
  - Les cellules hors longueur et les cellules des lanes non activées partagent désormais le même rendu inactif.
  - Aucun changement audio ou interaction : hors longueur / lanes vides restent non cliquables et non jouées.

### À tester dans Studio One (build 20260707-173031)
1. Mettre `Len` global à `16`, aller page 2 → les cellules hors longueur doivent fortement contraster avec les cellules actives de la page 1.
2. Sur une lane active, régler `Length` individuel à `8` dans `Track` → les steps 9-16 doivent être clairement inactifs, avec fond très sombre + pointillés épais.
3. Regarder une lane non activée (`+N`) → ses cellules doivent avoir le même design inactif que les cellules hors longueur.
4. Cliquer sur une cellule inactivée ou hors longueur → elle ne doit pas s'activer ni déclencher de note.

---

## 2026-07-07 — Grille : pointillés hors longueur plus visibles (build 20260707-171944)

**Build:** `20260707-171944`
**Validation:** `cargo fmt` OK, `cargo check` OK (37 warnings UI préexistants), `cargo test` OK (127 + 76 tests), `build.ps1 -Install` OK

### Changements
- **[107] Retouche visuelle des cellules hors longueur.**
  - `src/ui.rs` : les segments du contour pointillé passent à 5 px avec un trait 2 px.
  - La couleur du pointillé est assombrie pour rendre l'état hors longueur beaucoup plus évident.
  - Aucun changement de logique : ces cellules restent non cliquables et non jouées.

### À tester dans Studio One (build 20260707-171944)
1. Mettre `Len` global à `16`, aller page 2 → les pointillés hors longueur doivent être nettement plus gros et plus sombres qu'avant.
2. Sur une lane active, régler `Length` individuel à `8` dans `Track` → les steps 9-16 doivent être immédiatement identifiables comme hors longueur.
3. Cliquer sur une cellule pointillée hors longueur → elle ne doit pas s'activer ni déclencher de note.
4. Vérifier une lane vide (`+N`) → elle doit rester grisée normalement, sans pointillés sombres.

---

## 2026-07-07 — Grille : cellules hors longueur en pointillé (build 20260707-165324)

**Build:** `20260707-165324`
**Validation:** `cargo fmt` OK, `cargo check` OK (37 warnings UI préexistants), `cargo test` OK (127 + 76 tests), `build.ps1 -Install` OK

### Changements
- **[107] Les cellules hors longueur sont maintenant visuellement distinctes.**
  - `src/ui.rs` : `draw_step_cell_v2()` accepte un état `dashed_border` séparé de `enabled`.
  - Les cellules dont `global_step >= lane_length` conservent le fond désactivé existant mais remplacent la bordure continue par un contour segmenté.
  - Les cellules de slots vides restent avec leur rendu désactivé classique, sans pointillés.

### À tester dans Studio One (build 20260707-165324)
1. Mettre `Len` global à `16`, aller page 2 → les cellules visibles hors longueur doivent être grisées avec bordure pointillée, et ne doivent pas être cliquables.
2. Sur une lane active, régler `Length` individuel à `8` dans `Track` → les steps 9-16 de cette lane doivent apparaître pointillés, sans toucher aux autres lanes.
3. Remettre la lane en `Follow pattern length` → les pointillés doivent suivre à nouveau le `Len` global.
4. Vérifier une lane vide (`+N`) → elle doit rester grisée normalement, sans nouvelle bordure pointillée.

---

## 2026-07-07 — Track tab : retrait Humanize / Push-Pull (build 20260707-164821)

**Build:** `20260707-164821`
**Validation:** `cargo fmt` OK, `cargo check` OK (37 warnings UI préexistants), `cargo test` OK (127 + 76 tests), `build.ps1 -Install` OK

### Changements
- **[106] `Humanize` et `Push/Pull` retirés de l'onglet `Track`.**
  - `src/ui.rs` : suppression des deux lignes de sliders dans la section `Sequencing` du Track tab.
  - Les contrôles restent présents dans la grille, conformément au retour utilisateur.
  - Le tooltip de l'onglet `Track` est ajusté : `Instrument type, MIDI note, routing, length`.

### À tester dans Studio One (build 20260707-164821)
1. Ouvrir l'onglet `Track` sur une lane active → les lignes `Humanize` et `Push/Pull` ne doivent plus apparaître.
2. Vérifier que `Length` est toujours présent et modifiable dans `Track`.
3. Vérifier sur la grille que les mini-sliders `Hum` et `Push` sont toujours visibles et fonctionnels.
4. Modifier `Hum`/`Push` depuis la grille puis lancer la lecture → le comportement audio doit rester identique à avant.

---

## 2026-07-07 — Generator : HiHats différenciés par style (build 20260707-163927)

**Build:** `20260707-163927`
**Validation:** `cargo fmt` OK, `cargo test` OK (127 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[113] Les rôles HiHat ne sont plus quasi identiques entre styles.**
  - `src/generator/styles.rs` : chaque style a maintenant une signature HiHat dédiée.
  - Rock : 8ths + ghost 16ths légers.
  - Funk : offbeat 8ths + ghost notes.
  - Techno / Metal / Disco : 16ths droits.
  - Hip-Hop : sparse/swung.
  - Jazz : 8ths + skip-beat accents.
  - Latin : pattern syncopé type clave.
  - Trap : 8ths avec rolls 16ths très probables.
  - Reggae : one-drop sparse.
- **Test de régression ajouté.**
  - `src/generator/mod.rs` : `hihat_roles_are_style_specific` vérifie qu’au moins 8 signatures HiHat distinctes existent et verrouille des anchors représentatifs (`Funk`, `Latin`, `Reggae`).

### À tester dans Studio One (build 20260707-163927)
1. Layout 4 lanes `Kick / Snare / HiHat / Tom` → `GENERATE` en `Rock` → HiHat majoritairement en 8ths, avec peu de 16ths.
2. Même layout → `Funk` → HiHat sur les offbeats/contretemps, différent du Rock.
3. `Techno` puis `Disco` → HiHat très droit en 16ths, régulier.
4. `Hip-Hop` → HiHat plus sparse/swung, pas le même tapis 8ths que Rock.
5. `Latin` → HiHat syncopé type clave, accents irréguliers.
6. `Trap` → HiHat plus dense, avec beaucoup de 16ths/roll feel.
7. `Reggae` → HiHat sparse one-drop, surtout steps 2/6/10/14.

---

## 2026-07-07 — Générateur : mapping par `track_layout` et variations sur duplicates (build 20260707-161620)

**Build:** `20260707-161620`
**Validation:** `cargo test` OK (126 + 76 tests), `cargo check` OK (40 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[MG-10] Le générateur s’adapte désormais au `track_layout` courant.**
  - `src/generator/mod.rs` : `generate()` accepte un `AtomicTrackLayout` et appelle `remap_roles_to_slots()` après la génération des rôles legacy.
  - Les rôles musicaux (Kick, Snare, HiHat, OpenHH, Tom, Clap, Ride, Cymbal, Snare606, 808 Kick, Perc1) sont assignés aux slots actifs selon leur `TrackInstrumentKind::drum_voice_index()`, et non plus selon leur index de rangée.
  - Les slots vides/inactifs restent silencieux après `GENERATE`.
- **Gestion des duplicates.**
  - Jusqu’à 3 slots `Tom` répartissent naturellement les rôles legacy `Tom1/Tom2/Tom3`.
  - Pour toute autre duplication de kind (ex. deux Kicks) ou un 4e Tom, une variation déterministe est appliquée (shift de phase + éclaircissement/ajouts proportionnels au paramètre `Variation`).
- **Tests de régression ajoutés dans `src/generator/mod.rs`.**
  - `generate_maps_kick_to_kick_slot_not_opens_hh` : en layout 4 lanes par défaut, le slot 3 (Tom) ne reçoit plus le rôle OpenHH.
  - `generate_uses_distinct_tom_roles_for_multiple_toms` : 3 slots Tom produisent des patterns différents.
  - `generate_varies_duplicate_kick_slots` : deux slots Kick avec `Variation=1.0` ne sont pas identiques.
  - `generate_leaves_empty_slots_silent` : les slots inactifs restent vides.

### À tester dans Studio One (build 20260707-161620)
1. **Layout 4 lanes par défaut** (Kick/Snare/HiHat/Tom) → cliquer `GENERATE` (style Rock, density 0.8) → le Tom (lane 4) doit jouer uniquement en fin de mesure (steps 14-15), pas les offbeats d’OpenHH.
2. **Layout legacy 13 voix** → `GENERATE` → chaque instrument reçoit son rôle attendu (Kick sur 1/3, Snare sur 2/4, HiHat en 8e, OpenHH sur offbeats, Toms en fill, etc.).
3. **Deux slots Kick** → `GENERATE` avec `Variation > 0` → les deux lanes Kick ont des patterns différents (pas de copie conforme).
4. **Trois slots Tom** → `GENERATE` → les 3 lanes Tom ont des fills distincts (Tom1/Tom2/Tom3).
5. **Slot vide** → après `GENERATE`, il reste vide (pas de notes parasites).
6. **Tester les 4 modes de générateur** (Probabilistic, Markov, Euclidean, Classic) sur le layout 4 lanes : le Kick/Snare/HiHat/Tom doivent tous recevoir un pattern cohérent avec le style choisi.

---

## 2026-07-07 — Morphing : correction généralisée à tous les instruments (build 20260707-155108)

**Build:** `20260707-155108`
**Validation:** `cargo test` OK (122 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[118] Généralisation du fix morph à tous les instruments.**
  - `src/ui.rs` : popup Morph élargi de 284 px à 350 px et sliders réduits de 104 px à 96 px, afin d’éviter que les longs labels (`Saturation Output Gain`, `bassdrum808_saturation_amount`, etc.) ne poussent le slider hors du cadre et ne perdent l’interaction au relâchement.
  - Clamp systématique des valeurs morph affichées/stockées à `[min, max]` pour Volume, les champs standard sliders et les paramètres spéciaux continus.
- **Cohérence `morphable_fields()` avec le menu Morph.**
  - `src/instrument_registry.rs` : les champs standard de type checkbox (ex. `Stereo`) sont désormais inclus dans `morphable_fields()` avec `min=0.0, max=1.0`, puisque le menu Morph les permet déjà. Évite que la Fusion box affiche `?` à la place de `Stereo` quand celui-ci est une cible de morph.
- **Test de régression ajouté.**
  - `src/lib.rs` : `morphable_fields_include_checkbox_standard_fields` vérifie que chaque champ standard de chaque instrument est présent dans `morphable_fields()` et que les checkbox ont les bonnes bornes.

### À tester dans Studio One (build 20260707-155108)
1. **Tom** : Fusion → Morph → `Saturation Amount` / `Saturation Mix` : valeur fixée au relâchement, `×` visible, ré-ouverture conservée.
2. **Kick / Snare / 808 Kick** : Fusion → Morph → `Saturation Amount`, `Saturation Mix`, et si visible `Saturation Output Gain` (selon l’instrument) : même comportement stable.
3. **HiHat / OpenHiHat / Ride / Cymbal / Snare606 / Perc1** : vérifier que les paramètres continus spéciaux (shimmer, saturation, width, etc.) peuvent être morphés sans retour à la valeur de base.
4. **Kick / Snare606 / 808 Kick / Perc1** : Fusion → Morph → `Stereo` (checkbox) : cocher/décocher, fermer le menu, ré-ouvrir → l’état cible est conservé et la Fusion box affiche `M: Stereo` au lieu de `M: ?`.
5. **Tous instruments** : vérifier visuellement qu’aucun slider du menu Morph ne dépasse de la fenêtre, même après avoir défini une cible (apparition du `×`).

---

## 2026-07-07 — Morphing Tom : popup plus large + clamp morph (build 20260707-153553)

**Build:** `20260707-153553`
**Validation:** `cargo test` OK (121 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK après fermeture de Studio One

### Changements
- **[118] Correction du retour à la valeur de base pour `Saturation Amount` et `Saturation Mix` dans le menu Morph.**
  - `src/ui.rs` : largeur max du popup `plock_menu_frame` augmentée de 284 px à 320 px, car les longs labels `Saturation Amount` / `Saturation Mix` poussaient le slider hors du cadre et faisaient perdre l’interaction au relâchement.
  - Clamp systématique de la valeur morph affichée et stockée à `[min, max]` pour Volume, les champs standard et les specials continus.
- **TODO.md** : ajout de l’item [118] et marquage comme corrigé.

### À tester dans Studio One (build 20260707-153553)
1. Sur une lane Tom, créer une Fusion de plusieurs steps (ex. F 1-4) → ouvrir son menu Morph.
2. Régler `Saturation Amount` sur une valeur autre que 0, relâcher le slider → la valeur reste affichée et le petit `×` apparaît (cible enregistrée).
3. Régler `Saturation Mix` sur une valeur autre que 1.0, relâcher → idem, pas de retour à 1.0.
4. Fermer/ré-ouvrir le menu Morph de la même Fusion → les deux valeurs cibles sont conservées.
5. Lancer la lecture → le morph entre la valeur globale et la cible doit s’entendre sur les pulses de la fusion.
6. Vérifier que les autres paramètres continus du menu Morph (Volume, Freq, Decay, etc.) conservent aussi leur cible au relâchement.

---

## 2026-07-07 — Morphing Tom : params discrets exclus + conflit Attack/saturation évités (build 20260707-151057)

**Build:** `20260707-151057`
**Validation:** `cargo test` OK (121 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **Menu Morph : les paramètres spéciaux discrets ne sont plus proposés.**
  - `src/ui.rs` : le menu morph exclut désormais les specials `!continuous` (ex. `Saturation Type`, `Saturation Pre-Filter`).
  - Ces paramètres étant indexés (pas interpolables), les afficher comme des sliders faisait revenir leur valeur à la base au relâchement.
- **Évite les conflits entre champs standard et spéciaux dans la liste morphable.**
  - `src/ui.rs` + `src/instrument_registry.rs` : un special dont l'index de champ plock entre en collision avec un champ standard (cas connu : `Attack` utilise le champ 18, qui est aussi `SPECIAL_FIELD_START + 4` pour `Saturation Output Gain` / `Saturation Pre-Filter` / `Saturation Type` selon l'instrument) est ignoré dans le menu morph.
  - Cela empêche deux sliders de partager le même champ et de s'écraser mutuellement.
- **Lecture fraîche de l'état morph à chaque ligne.**
  - Remplacement de la closure `morph_state` qui capturait un `group` copié au début du menu par une fonction `fusion_morph_state` qui relit `new_fusions[fusion_index]` à chaque appel. Évite que la valeur affichée ne reprenne un état obsolète.
- **Tests de régression ajoutés.**
  - `src/lib.rs` : vérification que `morphable_fields()` n'a pas d'indices en double, n'inclut pas de params discrets, et n'overlappe pas les champs standard.

---

## 2026-07-07 — Fusion box : centrage vertical du contenu d’édition (build 20260707-142118)

**Build:** `20260707-142118`
**Validation:** `cargo test` OK (118 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[104] Centrage vertical du contenu de la Fusion box en mode édition.**
  - `src/ui.rs` : le layout interne passe explicitement à `left_to_right(Align::Center)`.
  - Les boutons `Del` et `×` sont maintenant alloués dans des emplacements de 18 px de hauteur pour être alignés avec le TextEdit.
  - Objectif : tous les éléments de la ligne d’édition (`F x-y`, `Steps:`, champ, `M: …`, `Del`, `×`) sont sur la même ligne de base / centrés verticalement dans le bloc.

---

## 2026-07-07 — Fusion box : marge interne réduite de 4 px à 3 px (build 20260707-140953)

**Build:** `20260707-140953`
**Validation:** `cargo test` OK (118 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[104] Marge interne de la Fusion box réduite de 4 px à 3 px.**
  - `src/ui.rs` : `inner_margin(4.0)` → `inner_margin(3.0)` et `inner_size` calculé avec `box_size - 6.0` au lieu de `box_size - 8.0`.
  - Objectif : laisser 1 px de plus de chaque côté pour le trait extérieur du cadre, afin que le contenu en mode édition (TextEdit + petits boutons) ne déborde pas visuellement et ne fasse plus sauter la ligne de 1–2 px.

---

## 2026-07-07 — Fusion box : hauteur de ligne verrouillée à 28 px (build 20260707-140333)

**Build:** `20260707-140333`
**Validation:** `cargo test` OK (118 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[104] La ligne `P-Lock Mode / Fusion` est maintenant clampée à exactement 28 px de haut.**
  - `src/ui.rs` : `ui.set_height(28.0)` remplacé par `ui.set_min_size(..., 28.0)` + `ui.set_max_size(..., 28.0)` sur le `horizontal` parent.
  - Conséquence : le passage idle ↔ édition d’une fusion ne peut plus faire pousser/rétrécir la ligne, même si les widgets internes (TextEdit, petits boutons) ont des tailles naturelles différentes.

---

## 2026-07-07 — Validation installée : Fusion box, Plock Frequency, preset Tom (build 20260707-135525)

**Build:** `20260707-135525`
**Validation:** `cargo test` OK (118 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[104][105][111] Build final installé après fermeture de Studio One.**
  - Aucun changement de code supplémentaire ; ce build consolide les corrections déjà documentées dans les builds `20260707-125743`, `20260707-113932` et `20260707-120216`.
  - Installation réussie dans `C:\Program Files\Common Files\VST3\drum-pattern-vst.vst3`.

---

## 2026-07-07 — Fusion box : allocation de taille exacte, plus de saut d’interface (build 20260707-125743)

**Build:** `20260707-125743`
**Validation:** `cargo test` OK (118 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[104] La Fusion box utilise maintenant `allocate_exact_size` (380×28 px).**
  - `src/ui.rs` : le rectangle externe est alloué avec une taille fixe, indépendamment du contenu idle ou édition.
  - Le contenu est dessiné dans ce rectangle via `allocate_ui_at_rect`, avec une taille interne min/max verrouillée.
  - Conséquence : la hauteur de la ligne P-Lock Mode/Fusion reste identique ; la Pattern Bank et le Bottom Panel ne bougent plus quand on entre/sort de l’édition d’une Fusion.

---

## 2026-07-07 — Fusion box : suppression du micro-saut idle/édition (build 20260707-124135)

**Build:** `20260707-124135`
**Validation:** `cargo test` OK (118 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[104] Fix du décalage de 2-3 px entre idle et édition de la Fusion box.**
  - `src/ui.rs` : la zone interne de la Fusion box est maintenant contrainte à une taille max égale à sa taille min (20 px de contenu), empêchant les boutons d’agrandir la hauteur en mode édition.
  - Boutons `Del` et `×` passés en `small_button` pour tenir dans l’espace fixe.

---

## 2026-07-07 — Placement Fusion box : même ligne que P-Lock Mode (build 20260707-121720)

**Build:** `20260707-121720`
**Validation:** `cargo test` OK (118 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[104] Fusion box sur la même ligne que le sélecteur P-Lock Mode.**
  - `src/ui.rs` : la Fusion box (380 px) est dessinée à droite de la barre `P-Lock Mode | Sound/Sequencer` au lieu d’occuper une ligne dédiée sous la grille.
  - Réduction de la largeur de la box de 720 px à 380 px ; labels et boutons compactés (`F x-y`, `M: ...`, `Del`, `×`).
  - Conséquence : la Pattern Bank et le Bottom Panel ne sont plus décalés vers le bas par la Fusion box.

---

## 2026-07-07 — Ajustement Tom : fréquence par défaut à 196 Hz (build 20260707-120216)

**Build:** `20260707-120216`
**Validation:** `cargo test` OK (118 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[111] Fréquence du Tom par défaut fixée à 196 Hz.**
  - `instrument_registry.rs` : Tom1 (lane `Tom` par défaut) passe de 150 Hz à **196 Hz** ; Tom2 reste à 150 Hz, Tom3 à 100 Hz.
  - `synthesis/mod.rs` : `VoiceSettings::tom1()` aligné sur 196 Hz.

---

## 2026-07-07 — Preset Tom retravaillé (build 20260707-115036)

**Build:** `20260707-115036`
**Validation:** `cargo test` OK (118 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[111] Preset Tom plus musical/utilisable dès création de lane.**
  - `instrument_registry.rs` : ajustement des `sound_settings_default` des 3 voix Tom et du défaut du paramètre spécial *Stick Attack* (0.5 → 0.3).
  - `synthesis/mod.rs` : alignement de `VoiceSettings::tom1/2/3()` sur les nouveaux défauts du registre.
  - Nouvelle famille Tom :
    - **Tom1** (utilisé par la lane `Tom` par défaut) : 150 Hz, decay 0.35 s, volume 0.7, filter 600 Hz, release 0.25 s.
    - **Tom2** : 200 Hz, decay 0.30 s, volume 0.7, filter 650 Hz, release 0.20 s.
    - **Tom3** : 100 Hz, decay 0.45 s, volume 0.7, filter 500 Hz, release 0.35 s.
  - Objectif : moins aigu et plus audible que l’ancien défaut Tom1 à 300 Hz / volume 0.5.

---

## 2026-07-07 — Plock sound : vérification Frequency > 0 + tests de régression (build 20260707-113932)

**Build:** `20260707-113932`
**Validation:** `cargo test` OK (118 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **Investigation du retour [105] : `Frequency` à 0 par défaut dans le menu plock sound.**
  - Le code actuel (`ST-7` + `reset_slot_to_defaults`) initialise correctement la fréquence globale de chaque slot depuis les défauts du registre ; le menu plock sound affiche déjà la valeur globale courante quand aucun override n’est actif.
  - Aucun instrument du registre n’a de fréquence par défaut à 0 (Kick 60, Snare 220, HiHat 1000, OpenHH 300, Tom 120/200/120, Clap 1000, Ride 3000, Cymbal 5000, Snare606 220, BassDrum808 50, Perc1 800).
- **Ajout de tests de régression dans `sound_settings.rs`.**
  - `default_frequency_is_nonzero_for_every_instrument_kind` : pour chaque kind, un slot actif obtient la fréquence par défaut attendue et elle est strictement positive.
  - `duplicate_slots_keep_nonzero_default_frequency` : deux slots B8 ont chacun la fréquence par défaut 50 Hz et sont indépendants.

---

## 2026-07-07 — Ajustement visuel : flash `T` en AMBER (build 20260707-111442)

**Build:** `20260707-111442`
**Validation:** `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **Le flash visuel du bouton `T` passe du bleu à l’ambre.**
  - L’indicateur d’activité MIDI externe utilise maintenant `AMBER` avec du texte noir, plus harmonieux avec les pastilles rouge (`M`) et verte (`S`).

---

## 2026-07-07 — Ext MIDI : playhead gelée, flash T, swing exporté (build 20260707-103907)

**Build:** `20260707-103907`
**Validation:** `cargo test` OK (116 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **En mode Ext MIDI, la tête de lecture interne est masquée.**
  - Quand `Seq` est sur `Ext MIDI`, `current_step`/`current_steps` sont stockés avec une valeur hors plage (`u32::MAX`) au lieu de suivre le transport hôte.
  - Aucune cellule de grille n’est donc surlignée comme "en cours" ; le plugin se contente de répondre aux notes MIDI entrantes.
- **Le bouton `T` (Test) de chaque lane clignote quand la lane est déclenchée par MIDI externe.**
  - Le thread audio lève un drapeau atomique par slot à la réception d’un `NoteOn` correspondant.
  - L’UI lit ce drapeau et allume le `T` en bleu pendant ~100 ms.
- **L’export MIDI (fichier + drag) applique maintenant le swing/groove.**
  - `midi_export.rs` reçoit `swing` et `groove_type` ; les steps impairs sont décalés selon l’algorithme actif (`Swing16`, `Shuffle`, `MPC`).
  - Test ajouté : un step 1 avec Swing16 +50 % est exporté à 160 ticks au lieu de 120.

---

## 2026-07-07 — Restauration du drag & drop MIDI + export 14 slots (build 20260707-094444)

**Build:** `20260707-094444`
**Validation:** `cargo test` OK (115 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **Restaure le drag & drop MIDI dans Studio One.**
  - Le bouton `Drag` est de retour dans la barre Patterns (à gauche, à côté de `Export`).
  - Le helper Windows (`drum-pattern-midi-drag-helper.exe`) est lancé au clic ; il faut ensuite cliquer-glisser la petite fenêtre `Flash Drum MIDI Drag` vers Studio One.
- **Corrige l’export MIDI pour les 14 slots et les notes personnalisées.**
  - `midi_export.rs` itère sur `0..MAX_TRACKS` au lieu des 13 voix legacy.
  - Il lit `track_layout.midi_note_for_slot(slot)` pour chaque slot actif, donc une note MIDI modifiée dans l’onglet `Track` est respectée.
  - Test ajouté : le 14e slot avec une note personnalisée est bien exporté.

---

## 2026-07-06 — UI routing : `No Aux` remplace `Main` dans la liste `Out` (build 20260706-192033)

**Build:** `20260706-192033`
**Validation:** `cargo test` OK (114 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- Dans l'onglet `Track`, la liste `Out` n'affiche plus `Main`.
- L'état sans sortie auxiliaire dédiée est maintenant libellé `No Aux`.
- Le switch `Main` reste le seul contrôle pour envoyer ou retirer la lane du Main Mix.

---

## 2026-07-06 — Fix init synth layout : slot Tom réactivé en OpenHH (build 20260706-190624)

**Build:** `20260706-190624`
**Validation:** `cargo test` OK (114 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `cargo test mapped_aux_output_idx` dans `vendor/nih-plug` OK (3 tests), `build.ps1 -Install` OK

### Changements
- **Corrige une cause interne du Tom qui pouvait sonner comme un HH après réactivation/routing.**
  - `DrumFlashVst::initialize()` initialisait encore le synthé avec le layout legacy 13 voix (`slot 3 = OpenHH`).
  - Si `last_slot_kinds` indiquait déjà le layout modulaire (`slot 3 = Tom`), le process ne réinitialisait pas ce slot, et la lane Tom pouvait garder physiquement une voix OpenHH.
  - Le synthé est maintenant initialisé directement avec le `track-layout` courant et `last_slot_kinds` est aligné sur ce layout.
- Test ajouté : le layout modulaire par défaut initialise bien le slot 4 en `Tom`, pas en `OpenHiHat`.

---

## 2026-07-06 — Fix VST3 sparse aux outputs Studio One (build 20260706-185857)

**Build:** `20260706-185857`
**Validation:** `cargo test` OK (113 + 75 tests), `cargo check` OK (37 warnings UI préexistants), `cargo test mapped_aux_output_idx` dans `vendor/nih-plug` OK (3 tests), `build.ps1 -Install` OK

### Changements
- **Corrige le bug profond où une lane routée vers `Out 2` pouvait sortir comme une autre lane selon les sorties activées dans Studio One.**
  - Le wrapper VST3 vendored mémorise maintenant les bus audio activés via `activateBus()`.
  - Pendant `process()`, les buffers auxiliaires compactés fournis par l'hôte sont remappés vers leurs vrais indices `Out N` au lieu d'être supposés en préfixe `Out 1..N`.
  - Cas ciblé : Studio One active une sortie sparse (`Main + Out 2` sans `Out 1`) ; le premier buffer aux reçu est maintenant mappé vers `Out 2`, pas `Out 1`.
- La validation défensive des buffers aux utilise le même mapping sparse.
- Tests vendor ajoutés : fallback préfixe sans info d'activation, `Main + Out 2`, et sorties sparse multiples.

---

## 2026-07-06 — Fix routing : sortie auxiliaire exclusive par lane (build 20260706-175157)

**Build:** `20260706-175157`
**Validation:** `cargo test` OK (113 + 75 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **Corrige le cas où un Tom routé vers `Out 2` pouvait sembler sonner comme un HH.**
  - L'assignation `Track > Out` est maintenant exclusive : si une lane prend `Out N`, toute autre lane déjà routée vers ce même `Out N` repasse en sortie auxiliaire `Main`/aucune aux dédiée.
  - Objectif : éviter qu'un ancien HH ou autre slot reste caché sur le même bus auxiliaire et masque la lane qu'on vient d'assigner.
- Tests ajoutés : exclusivité d'un `Out N` entre slots, et non-régression quand une lane repasse sur `Main`.

---

## 2026-07-06 — Renommage des sorties DAW en `Out 1..14` (build 20260706-173427)

**Build:** `20260706-173427`
**Validation:** `cargo test` OK (111 + 73 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- Les noms de ports auxiliaires exposés au DAW sont maintenant génériques : `Out 1` à `Out 14`.
- Suppression des anciens noms de bus hérités (`Kick`, `Snare`, `Hi-Hat`, `Open HH`, etc.) dans `OUTPUT_PORT_NAMES`.
- Le routing audio reste celui de la build précédente : chaque slot est envoyé vers la sortie choisie dans `Track > Out`.

---

## 2026-07-06 — Fix routing Track : sorties auxiliaires par slot (build 20260706-172704)

**Build:** `20260706-172704`
**Validation:** `cargo test` OK (111 + 73 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **Corrige la régression où changer `Track > Out` donnait l'impression de changer le son/instrument.**
  - Le moteur audio lit maintenant le routing `TrackRouting` par slot au lieu d'envoyer implicitement `slot N -> Out N`.
  - `Main` suit la case `Main` du slot ; `Out N` suit le sélecteur `Out` du même slot.
  - Plusieurs lanes routées vers le même `Out` sont additionnées au lieu de s'écraser.
- Le helper d'écriture aux reste défensif : bus inactifs, mono, incomplets ou trop courts sont ignorés sans panic ni écriture invalide.
- Tests ajoutés/ajustés : écriture aux inactive/incomplète ignorée, sortie stéréo valide accumulée.

---

## 2026-07-06 — Fix multi-out : écriture aux défensive pendant activation/désactivation DAW (build 20260706-141836)

**Build:** `20260706-141836`
**Validation:** `cargo test` OK (111 + 73 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **Corrige le P0 [117] : distorsion lors de l'activation/désactivation d'une sortie dans le DAW.**
  - L'écriture des sorties auxiliaires ne suppose plus que chaque bus fourni par l'hôte contient toujours 2 canaux non vides.
  - Les bus aux inactifs, mono, incomplets ou transitoirement vides sont ignorés au lieu d'être indexés par `channels[0][sample_idx]` / `channels[1][sample_idx]`.
  - Objectif : éviter les écritures dans des buffers invalides/stale pendant les changements d'activation de sorties Studio One.
- Tests ajoutés : sorties aux inactives/incomplètes ignorées sans panic, sortie stéréo valide écrite correctement.

---

## 2026-07-05 — Fix song-mode : reset step 0 après pattern de longueur différente (build 20260705-150850)

**Build:** `20260705-150850`
**Validation:** `cargo test` OK (109 + 73 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **Corrige le song-mode quand les patterns n'ont pas tous la même longueur.**
  - `load_pattern_from_slot()` applique maintenant la longueur chargée côté audio immédiatement, sans attendre que l'UI applique `pending_pattern_length`.
  - Après une transition song réussie, le séquenceur est redémarré à step 0 au bloc suivant avec les longueurs de lanes recalculées.
  - La resynchro continue à la timeline absolue du DAW est désactivée pendant le song-mode, pour éviter qu'elle recale la tête au milieu du nouveau pattern.
- Tests ajoutés : longueur audio immédiate après load et redémarrage step 0 après changement de longueur.

---

## 2026-07-05 — AUDIT-1 : PatternBank non bloquant sur thread audio (build 20260705-132937)

**Build:** `20260705-132937`
**Validation:** `cargo test` OK (107 + 72 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **Thread audio : les accès `PatternBank` ne bloquent plus.**
  - Les chemins save/load pattern et song-mode utilisent `try_lock()` au lieu de `lock()` dans `process()`.
  - Si l'UI détient temporairement le lock, la demande save/load est conservée et retentée au bloc audio suivant.
  - En song-mode, un wrap de pattern n'est consommé que lorsque la lecture du bank et le chargement du slot ont réellement réussi ; en cas de contention, le changement est retenté sans bloquer le callback audio.
- Test ajouté : `pattern_bank_actions_return_busy_instead_of_blocking_when_locked` vérifie que save/load retournent `Busy` quand la banque est déjà verrouillée.
- TODO audit mis à jour : [AUDIT-1] `try_lock()` + report save/load/song cochés ; la phase optionnelle double-buffer/SPSC reste ouverte.

---

## 2026-07-05 — ST-7 : instances par slot complètes + onglets Sound/Track + picker instrument (build 20260705-122315)

**Build:** `20260705-122315`
**Validation:** `cargo test` OK (106 + 72 tests, dont 3 nouveaux tests persistance/migration), `build.ps1 -Install` OK

### Changements
- **ST-7 — Special params par slot (fix "le Click Type de la lane 5 change celui de la lane 1").**
  - `special[32]` + mode Hz/Notes stockés PAR SLOT dans `SoundSettingsState`, seedés depuis les défauts du registre.
  - Persistance : `sound-settings-v2` format v3 (46 floats/slot) ; les anciennes sessions sont migrées automatiquement depuis les params par voix (`needs_param_seed`, seed one-shot RT-safe dans `process()`).
  - Moteur : `voice_settings_for(slot, voice, …)` lit specials + algo par slot ; UI Sound Panel, menus plock/morph, Snapshot et morphing rebranchés sur les atomics par slot.
  - Ranges algo unifiés ("Slot N Algo", `max_algo_index()`) : un Kick sur n'importe quel slot peut changer d'algo ; fixe aussi les ranges 0..0 crashogènes (`algo_cymbal`, `algo_s13`).
  - ⚠️ Les special params ne sont plus automatisables par le DAW (ils restent plockables par step) ; les params legacy servent uniquement de source de migration.
- **Onglets refaits : `Sound Editor` | `Track` (retour utilisateur).**
  - Les boutons par instrument disparaissent — la lane éditée se choisit en cliquant dans la grille ; l'en-tête affiche toujours "Slot N - nom".
  - Onglet Track complet : type d'instrument, note MIDI, routing Main/Out, **Humanize, Push/Pull, Length** (mêmes params que les mini-sliders de lane) ; message dédié si le slot est vide.
- **Choix de l'instrument à la création (retour utilisateur).**
  - Cliquer la pastille `+N` d'une lane vide ouvre un menu avec les 11 instruments ; le slot est créé avec le kind choisi (plus de Kick imposé).
- **Fix : le lock de longueur de lane était indexé par voix côté UI** (`draw_track_length_control`) alors que l'audio le lit par slot — aligné sur le slot.
- Docs : `AGENTS.md` (nouvelle section "Per-slot instances"), `CLAUDE.md` (invariant mis à jour), `ADDING_AN_INSTRUMENT.md` (étapes params specials marquées obsolètes).

---

## 2026-07-04 — Défaut 4 lanes + grille à hauteur fixe 14 rangées (build 20260704-195335)

**Build:** `20260704-195335`
**Validation:** `cargo test` OK (103 + 72 tests — un test de migration supprimé volontairement), `build.ps1 -Install` OK

### Changements
- **Nouveau défaut : 4 lanes (BD/SD/HH/Tom)** — décision produit 2026-07-04.
  - `TrackLayoutState::default_layout()` retourne le template modulaire 4 slots au lieu des 13 voix legacy.
  - Suppression de la migration `is_buggy_four_track_template` (elle re-transformait tout layout 4 lanes exact en 13 lanes au rechargement — incompatible avec le nouveau défaut).
  - ⚠️ Compat : les songs sauvegardées AVANT l'existence de `track-layout-v1` s'ouvriront avec 4 lanes (leurs patterns des autres instruments restent stockés mais inactifs). Les songs avec un layout sauvegardé conservent leur layout.
- **Grille à hauteur constante : 14 rangées toujours rendues** (règle UI : aucune ligne conditionnelle qui décale les zones).
  - Les slots inactifs sont rendus comme lanes vides stylées ; la pastille `+N` de chaque lane vide active CE slot (curseur main + tooltip).
  - Suppression de la rangée `+ Add Module` (elle apparaissait/disparaissait et décalait les panneaux du bas).
  - Les panneaux sous la grille (P-Lock mode, patterns, generator/song) ne bougent plus jamais.
- Nettoyage : `visible_lane_count()` et `draw_add_module_row_v2()` supprimés ; activation factorisée dans `activate_slot(slot_idx)`.
- **Limitation connue rendue plus visible par le défaut 4 lanes :** les générateurs de patterns supposent encore les rôles legacy par rangée (rangée 4 = OpenHH, etc.) alors que la lane 4 du template est un Tom — voir [MG-10].

---

## 2026-07-04 — La pastille `+N` de la lane vide active le slot (build 20260704-174006)

**Build:** `20260704-174006`
**Validation:** `cargo test` OK (104 + 73 tests), `build.ps1 -Install` OK

### Changements
- **Corrige "quand je clique sur +14 rien ne se passe" (rapporté par test S1).**
  - La lane vide affichait une pastille `+14` qui s'illuminait au survol mais n'était pas cliquable (`Sense::hover` seulement) — le seul bouton actif était la rangée `+ Add Module` en dessous.
  - La pastille `+N` est maintenant cliquable (curseur main, tooltip "Activate this slot") et déclenche la même activation que `+ Add Module`.
  - Logique d'activation factorisée dans `activate_next_free_slot()` (layout + reset des settings du slot + sélection).

---

## 2026-07-04 — Fix trigger : settings et plocks appliqués par slot (build 20260704-173043)

**Build:** `20260704-173043`
**Validation:** `cargo test` OK (104 + 73 tests), `build.ps1 -Install` OK

### Changements
- **Corrige "la freq de la lane 1 change celle de la lane 14" (rapporté par test S1).**
  - `voice_settings_at_step()` lisait les settings standards (`sound_settings_state.instruments[...]`) et les plocks (`plock_state.get_settings(...)`) avec l'index de **voix** alors que ces stockages sont par **slot**.
  - À chaque trigger, un slot dupliqué (ex : 2e Kick) se voyait réappliquer les settings ET les plocks du premier slot du même kind, écrasant le push par slot correct fait en début de bloc.
  - Fix : `voice_settings_at_step(slot_idx, voice_idx, step)` — settings et plocks par slot, schéma/special params par voix. Trois appelants corrigés (séquenceur, MIDI thru, test triggers).
- **Limitation restante (ST-7, connue) :** les special params (Click, Saturation, mode Hz/Notes) restent des paramètres nih-plug par voix legacy — physiquement partagés entre deux slots du même kind, dans l'UI comme dans le moteur.

---

## 2026-07-04 — Stabilisation modular grid 14 slots (build 20260704-165252)

**Build:** `20260704-165252`
**Validation:** `cargo check` OK, `cargo test` OK (104 + 73 tests), `build.ps1 -Install` OK

### Changements
- **Corrige le crash Studio One à l'ajout de la 14e piste (ST-1).**
  - `EditorUIState.fusion_selection_start` était encore taillé à 13 (`DrumVoice::COUNT`) mais indexé par slot (0..14) dans la boucle de grille → index out of bounds dès le rendu de la lane 14. Passé à `MAX_TRACKS`.
- **Corrige deux crashs latents des menus plock sur la lane 14 (ST-2).**
  - `INSTRUMENTS[slot_idx]` (13 entrées) dans les menus Plock / Morph / Seq Plock, et `DrumVoice::from_index(slot).expect(...)` dans le dropdown Algo.
  - Les menus résolvent maintenant le schéma via l'index de voix dérivé du kind du slot (`schema_voice_idx`), le stockage plock reste indexé par slot.
- **Corrige le son défectueux d'un slot ajouté (ST-3).**
  - `SoundSettingsState::reset_slot_to_defaults()` n'était jamais appelé : un slot activé via `+ Add Module` gardait les réglages d'init de la voix legacy de même index (ex : un Kick au slot 5 jouait avec des réglages de Tom).
  - Reset aux défauts du kind à l'activation et au changement d'instrument dans l'onglet TRK.
- **Sépare index de slot et index de voix dans le Sound Editor (ST-4).**
  - `selected_instrument` est désormais un index de slot (0..14) ; le schéma de paramètres (registre, special params, filter label, checks Kick/B8, liste d'algos) est dérivé du kind du slot.
  - Changer le type dans l'onglet TRK ne fait plus sauter la sélection sur un autre slot (= le "impossible de choisir le type" du test S1).
  - Les onglets du Sound Editor listent les slots actifs (labels par kind, tooltip avec numéro de slot) au lieu des 13 voix fixes.
- **Aligne la longueur de lane UI sur le moteur audio.**
  - `effective_lane_length_for_ui` utilise l'index de slot (comme `raw_lengths` / `lane_length_locks` côté audio) au lieu de l'index de voix.
- Reste à valider dans Studio One (ST-6) : ajout jusqu'à 14 pistes, clic droit lane 14, changement de type via TRK, son correct, Out 14 audible. Le layout "4 lanes par défaut" observé reste à éclaircir (ST-5).

---

## 2026-07-02 — MG-7a.2: activate 14th track slot + Track tab (build 20260702-215053)

**Build:** `20260702-215053`
**Validation:** `cargo check` OK, `cargo test` OK (104 + 73 tests), `build.ps1 -Install` OK

### Changements
- **Active le bouton `+ Add Module` : le slot 14 devient une piste fonctionnelle.**
  - Le bouton active le premier slot inactif avec l'instrument par défaut Kick (réassignable via l'onglet Track).
  - Le séquenceur, le moteur audio et les sorties auxiliaires itèrent maintenant sur `MAX_TRACKS = 14` slots.
  - `AUX_OUT_COUNT` passe à 14 ; le bus 14 est nommé `Out 14`.
  - Le `DrumSynthesizer` réinitialise automatiquement un slot dont le `kind` change dans `track-layout-v1`.
- **Ajoute un onglet `TRK` (Track) dans le Sound Editor.**
  - Affiche le slot sélectionné et permet de changer son instrument (Kick, Snare, HiHat, ...).
  - Permet de réguler le routing Main / Out et la note MIDI du slot.
- **Persistance et migration.**
  - Le champ de pattern DAW passe de `pattern-v4` à `pattern-v5` (14 rangées d'instruments).
  - Migration automatique depuis `pattern-v4` (13 instruments) et `pattern-v3` (13 instruments + fusion legacy).
  - `track-layout-v1` reste le champ de persistance de la disposition ; l'état par défaut reste la migration legacy 13 voix.
- **Rupture de compatibilité volontaire pour les projets Studio One existants.**
  - Le nombre de sorties stéréo auxiliaires change (13 → 14) : les projets sauvegardés devront réaffecter leurs bus aux.
  - L'identité VST3 (`DrumFlashPlugin1`) est volontairement conservée pour ne pas casser l'insert du plugin lui-même.

---

## 2026-07-01 — Fix plugin fixed height after empty modular lane (build 20260701-230011)

**Build:** `20260701-230011`
**Validation:** `cargo check` OK, `cargo test` OK (103 + 73 tests), `build.ps1 -Install` OK

### Changements
- Augmente la taille fixe de l'éditeur VST de `1480x800` à `1480x900`.
- Corrige le bas de l'interface masqué après l'ajout visuel du slot 14 vide et de la rangée `+ Add Module`.
- Aucun changement audio, VST3, routing ou persistance DAW.

---

## 2026-07-01 — Modular UI checkpoint 5: visual empty slot and Add Module placeholder (build 20260701-205643)

**Build:** `20260701-205643`
**Validation:** `cargo check` OK, `cargo test` OK (103 + 73 tests), `build.ps1 -Install` OK

### Changements
- **Ajoute un checkpoint visuel sûr pour MG-7a.**
  - Affiche le slot 14 comme lane vide stylée sous les 13 lanes legacy actives.
  - Ajoute une rangée `+ Add Module` sous les lanes.
  - Le bouton reste volontairement visuel/inactif dans ce checkpoint : aucune activation de piste, aucune mutation de `track-layout-v1`, aucun changement audio, VST3 ou DAW state.
- Prépare le prochain checkpoint qui pourra activer l'ajout de module de façon contrôlée.

---

## 2026-07-01 — Fix individual lane length beyond global length (build 20260701-201011)

**Build:** `20260701-201011`
**Validation:** `cargo check` OK, `cargo test` OK (103 + 73 tests), `build.ps1 -Install` OK, validation Studio One OK

### Changements
- **Corrige le comportement de `Len` individuel sur les lanes.**
  - Une lane lockée utilise maintenant sa propre longueur brute `1..64`, même si elle dépasse la longueur globale du pattern.
  - Le séquenceur accepte les longueurs par piste jusqu'à 64 au lieu de les re-clamper sur la longueur globale.
  - La grille UI et le playhead par lane utilisent la longueur effective de la lane, ce qui rend les pas au-delà de la longueur globale visibles/editables pour une lane lockée.
- Aucun changement de topologie VST3, de bus audio ou d'identité plugin.

---

## 2026-07-01 — Modular UI checkpoint 4: extracted slot-aware lane renderer (build 20260701-183243)

**Build:** `20260701-183243`
**Validation:** `cargo check` OK, `cargo test` OK (73 tests), `build.ps1 -Install` OK

### Changements
- **Refactor structurel sans changement visible.**
  - Extrait le rendu d'une lane dans `draw_legacy_slot_lane_v2(...)`.
  - La fonction reçoit explicitement `slot_idx` et `voice_idx`, ce qui prépare l'affichage de slots actifs/inactifs sans mélanger index de slot et index de voix.
  - Corrige au passage la condition d'édition fusion pour comparer contre `slot_idx`.
- Aucun changement de topologie VST3, de bus audio, de pattern, de plocks ou de persistance DAW.

---

## 2026-07-01 — Modular UI checkpoint 3: slot-to-voice bridge in grid loop (build 20260701-175321)

**Build:** `20260701-175321`
**Validation:** `cargo check` OK, `cargo test` OK (73 tests), `build.ps1 -Install` OK

### Changements
- **Prépare la grille aux vraies lanes modulaires sans changement visible.**
  - Ajoute les helpers `visible_legacy_lane_count()` et `legacy_voice_idx_for_slot()`.
  - La boucle de grille itère maintenant sur `slot_idx`, puis dérive le `voice_idx` legacy (`slot_idx == voice_idx` tant que l'UI reste en 13 lanes fixes).
  - Pattern, plocks, fusions et sélection utilisent progressivement `slot_idx`; labels et paramètres automatisables restent indexés par `voice_idx` legacy.
- Aucun changement de topologie VST3, de bus audio, de pattern, de plocks ou de persistance DAW.

---

## 2026-07-01 — Fix silent lanes after modular layout checkpoint (build 20260701-174700)

**Build:** `20260701-174700`
**Validation:** `cargo check` OK, `cargo test` OK (73 tests), `build.ps1 -Install` OK

### Changements
- **Corrige les lanes silencieuses à partir de la 5e lane.**
  - Cause : le layout modulaire par défaut n'activait que 4 slots (`BD/SD/HH/Tom`) alors que l'UI affiche encore les 13 lanes fixes.
  - `TrackLayoutState::default_layout()` revient temporairement au layout legacy 13 voix tant que l'UI modulaire complète n'est pas prête.
  - Les états `track-layout-v1` déjà sauvegardés avec le template 4 slots buggué sont automatiquement migrés vers `from_legacy_13()` au chargement.
- Ajoute un test de compat pour détecter le template 4 slots buggué.
- Aucun changement de topologie VST3 ou de bus audio.

---

## 2026-07-01 — Modular UI checkpoint 2: grid interactions select track slot (build 20260701-173832)

**Build:** `20260701-173832`
**Validation:** `cargo check` OK, `cargo test` OK (72 tests), `build.ps1 -Install` OK

### Changements
- **Étend la sélection `selected_track_slot` aux interactions de grille/lane restantes, toujours sur les 13 lanes fixes.**
  - Lane volume, Humanize, Push/Pull et Length sélectionnent désormais le slot concerné quand l'utilisateur interagit.
  - Double-clic fusion, shift-clic fusion et clic droit p-lock sélectionnent aussi le slot concerné.
  - Les actions déjà gouvernées par Auto-Edit conservent leur comportement existant, mais passent par `select_legacy_track()`.
- Aucun changement de topologie VST3, de bus audio, de pattern, de plocks ou de persistance DAW.

---

## 2026-07-01 — Modular UI checkpoint 1: non-breaking selected track alias (build 20260701-172602)

**Build:** `20260701-172602`
**Validation:** `cargo check` OK, `cargo test` OK (72 tests), `build.ps1 -Install` OK

### Changements
- **Reprise prudente de la grille modulaire après rollback Studio One.**
  - Ajoute `EditorUIState::selected_track_slot` avec `#[serde(default)]` pour ne pas casser l'état d'éditeur existant.
  - Synchronise `selected_track_slot` avec `selected_instrument` sur les 13 lanes fixes actuelles.
  - Remplace les chemins de sélection UI par un helper central `select_legacy_track()`.
- Aucun changement de topologie VST3, de bus audio, de pattern, de plocks ou de persistance DAW.
- Sécurise le loader debug de preset dumps contre un index instrument hors bornes.

---

## 2026-07-01 — Rollback to last stable pre-crash code path (build 20260701-171707)

**Build:** `20260701-171707`
**Validation:** `cargo check` OK, `cargo test` OK (72 tests), `build.ps1 -Install` OK

### Changements
- **Rollback des changements non commités de la grille modulaire qui faisaient encore crasher Studio One au lancement.**
  - Retour au code du commit stable `edb1ef8` pour les sources Rust du plugin.
  - Conserve les fondations déjà commités : modèle track 14 slots, `track-layout-v1`, audio interne 14 slots avec topologie VST3 compatible 13 sorties auxiliaires.
  - Retire les changements UI/interaction non stabilisés : `+ Add module`, empty lanes stylées, sélection canonique `selected_track_slot`, onglets Sound/Track, menus plock slot/voice-aware, solos par slot.
- `TODO.md` rouvre les tâches modular-grid UI/interaction et le fix “new tracks silent / solo / interactions track-based”.
- Les entrées de build `20260701-162641`, `20260701-163806`, `20260701-164653`, `20260701-170135` et `20260701-170950` sont à considérer comme **supplantées par ce rollback** pour la validation Studio One.

---

## 2026-07-01 — Restore Studio One bus compatibility after modular-grid crash (build 20260701-170950)

**Build:** `20260701-170950`
**Validation:** `cargo check` OK, `cargo test` OK (72 tests), `build.ps1 -Install` OK

### Changements
- **Restaure la compatibilité de topologie VST3 avec les anciennes instances Studio One.**
  - Le plugin garde 14 slots internes pour la grille modulaire.
  - Les sorties auxiliaires VST3 exposées repassent à 13 bus stéréo, comme l'identité VST3 existante.
  - Changer 13 → 14 bus avec le même `VST3_CLASS_ID` était probablement la cause du crash au restore Studio One.
- Ajoute des garde-fous sur les derniers accès `slot_idx`/`voice_idx` dangereux dans `voice_settings_for` et `voice_settings_at_step`.
- Le menu Track / Aux Out n'expose plus `Out 14` tant que l'identité VST3 reste celle de la ligne compatible.

---

## 2026-07-01 — Fix Studio One startup crash: slot/voice index confusion in `voice_settings_at_step` (build 20260701-170135)

**Build:** `20260701-170135`
**Validation:** `cargo check` OK, `cargo test` OK (72 tests), `build.ps1 -Install` OK

### Changements
- **Corrige une confusion d'index slot/voice qui provoquait un panic dans l'audio thread.**
  - `voice_settings_at_step` prend maintenant explicitement `slot_idx` *et* `voice_idx`.
  - `sound_settings.instruments[slot_idx]` est lu par slot (14 slots), tandis que `voice_settings_for(voice_idx, ...)` et `INSTRUMENTS[voice_idx]` restent indexés par `DrumVoice` (13 voix).
  - Avant ce fix, un slot d'index 13 actif passait `slot_idx = 13` à `voice_settings_for`, causant un accès hors limites sur `INSTRUMENTS[13]` (taille 13) et un crash `EXCEPTION_STACK_BUFFER_OVERRUN` à travers l'ABI VST3.
- Mise à jour des trois appelants dans `process()` pour transmettre les deux indices correctement.

---

## 2026-07-01 — Fix Studio One startup crash: TrackLayoutState + plock popup compat (build 20260701-164653)

**Build:** `20260701-164653`
**Validation:** `cargo check` OK, `cargo test` OK (72 tests), `build.ps1 -Install` OK, Studio One 7 launches without crash

### Changements
- **Corrige le crash au lancement de Studio One.**
  - `TrackLayoutState` implémente maintenant `Deserialize` manuellement et accepte un `Vec<TrackSlot>` de n'importe quelle taille, remplissant/tronquant à `MAX_TRACKS = 14` slots.
  - Cela répare la désérialisation de l'état DAW qui contenait encore 13 slots (ancien format).
  - `PlockPopup.slot_idx` et `SinglePlockClipboard.slot_idx` acceptent l'alias serde `instrument` pour la compatibilité avec l'état de l'éditeur sauvegardé avant le renommage.

---

## 2026-07-01 — Fix Studio One startup crash (build 20260701-163806)

**Build:** `20260701-163806`
**Validation:** `cargo check` OK, `cargo test` OK (72 tests), `build.ps1 -Install` OK, Studio One 7 launches without crash

### Changements
- **Corrige le crash au lancement de Studio One.**
  - `EditorUIState::fusion_selection_start` passe d'un tableau de taille fixe (`[Option<usize>; MAX_TRACKS]`) à un `Vec<Option<usize>>` initialisé à 14 entrées.
  - Cela évite l'erreur de désérialisation serde lorsque l'état précédent de l'éditeur contenait un tableau de 13 éléments (ancien format).

---

## 2026-07-01 — All UI interactions are track-based (build 20260701-162641)

**Build:** `20260701-162641`
**Validation:** `cargo check` OK, `cargo test` OK (72 tests), `build.ps1 -Install` OK

### Changements
- **Toutes les interactions de la grille sont maintenant associées au track slot, pas au type d'instrument.**
  - Clic sur le nom d'une lane, Mute, Solo, Test, clic sur une step : ciblent le `slot_idx` sélectionné.
  - `selected_track_slot` est la sélection canonique ; `selected_instrument` n'est que le type de l'instrument du slot.
- **Sound Editor édite le son du slot actif.**
  - `sound_settings.instruments[slot_idx]` est lu/écrit au lieu de `[selected_instrument]`.
- **Plock / fusion / seq-plock menus sont slot/voice-aware.**
  - `PlockPopup` stocke `slot_idx` ; `voice_idx` est dérivé du `track_layout` pour les métadonnées instrument.
  - `draw_plock_menu`, `draw_fusion_morph_menu`, `draw_sequencer_plock_menu` prennent séparément `slot_idx` et `voice_idx`.
  - `SinglePlockClipboard` stocke `slot_idx`.
- **Onglet Track restauré.**
  - Sélecteur d'instrument (`TrackInstrumentKind`) pour le slot actif.
  - Routing `Main` + `Aux Out` (`Out 1`..`Out 14`) par slot.
  - Réglage de la note MIDI par slot.
- **`LaneLengthLocks` passe à 14 bits (`AtomicU32`).**
  - Persistance `u32` à la place de `u16`.
- **Grille UI restaurée après revert accidentel.**
  - Itération sur `MAX_TRACKS = 14` slots, lanes vides stylisées, bouton `+ Add module` sous les lanes.

---

## 2026-07-01 — Solo per slot (build 20260701-155824)

**Build:** `20260701-155824`
**Validation:** `cargo check` OK, `cargo test` OK (103 tests), `build.ps1 -Install` OK

### Changements
- **Solo devient un paramètre par slot.**
  - Ajout de 14 params `solo_s00`..`solo_s13` dans `DrumFlashParams`.
  - `slot_solos()` expose les 14 params.
  - `seq_mutes` utilise `slot_solo_states[slot]` au lieu de `solo_states[voice_idx]`.
  - Le tag `S` de chaque lane contrôle le solo de ce slot uniquement.

---

## 2026-07-01 — Fix audio thread: triggers now per-slot (build 20260701-154857)

**Build:** `20260701-154857`
**Validation:** `cargo check` OK, `cargo test` OK (103 tests), `build.ps1 -Install` OK

### Changements
- **Corrigé le thread audio qui traitait les triggers par `DrumVoice` au lieu de par slot.**
  - La boucle interne itère maintenant `(slot_idx, trigger)` et déclenche uniquement ce slot.
  - Cela répare le silence sur les nouveaux tracks et la double activation des lanes de même instrument.
- **Hihat choke** et **stutter/fusion scheduling** mis à jour pour utiliser `slot_idx`.

---

## 2026-07-01 — Modular grid: pattern per slot + instrument selector (build 20260701-153855)

**Build:** `20260701-153855`
**Validation:** `cargo check` OK, `cargo test` OK (103 tests), `build.ps1 -Install` OK

### Changements
- **[MG-6] Pattern bank indexée par slot (14 pistes indépendantes).**
  - `Pattern`, `SharedPattern`, `PlockState`, `SequencerPlockState` passent de 13 voix legacy à 14 slots.
  - Le séquenceur émet des triggers par slot (`MAX_TRACKS = 14`) au lieu de `DrumVoice::COUNT`.
  - La grille UI utilise `slot_idx` pour lire/écrire les cellules, fusions, plocks et seq-plocks.
  - Les paramètres de piste (mute/solo/length/push/humanize) sont mappés slot → voix legacy.
- **Sélecteur d'instrument dans l'onglet Track.**
  - `draw_track_tab` propose un ComboBox pour changer `TrackInstrumentKind` (Kick, Snare, ...).
  - Le changement met à jour le layout, reset les `sound_settings` du slot au defaults de l'instrument, et bump la version du synthétiseur.
- **Migration de persistance `pattern-v4` (13 rows) → `pattern-v5` (14 slots).**
  - Ajout de `PatternStateV4` avec `LEGACY_INSTRUMENT_COUNT = 13` et `expand()`.
  - `filter_state` migre `pattern-v4`, `pattern-v3`, `pattern-v2`, `pattern-v1` et `st01..st16` vers `pattern-v5`.
- **Générateurs adaptés à 14 slots.**
  - `euclidean_params`, rotations et templates de style ont une entrée FX supplémentaire.
- **Export MIDI itère sur les 14 slots.**

---

## 2026-07-01 — UI: lanes vides stylisées + bouton +Add module sous les lanes (build 20260701-151829)

**Build:** `20260701-151829`
**Validation:** `cargo check` OK, `cargo test` OK (103 tests), `build.ps1 -Install` OK

### Changements
- **Bouton `+ Add module` déplacé sous les lanes.**
  - Suppression du bouton `+ Add Track` du header.
  - Ajout d'une rangée `+ Add module` en bas de la grille, pleine largeur.
- **Lanes vides stylisées (hauteur de grille fixe).**
  - `draw_empty_lane` dessine les 14 emplacements avec un style "placeholder" : bordures dashed, tags M/S/T grisés, cellules grisées, sliders muets.
  - La grille conserve toujours 14 rangées, quels que soient les pistes actives.

---

## 2026-07-01 — UI: grid modulaire + onglets Sound/Track (build 20260701-144428)

**Build:** `20260701-144428`
**Validation:** `cargo check` OK, `cargo test` OK (103 tests), `build.ps1 -Install` OK

### Changements
- **Grid modulaire (MG-7).**
  - `draw_grid_v2` itère sur les slots actifs du layout au lieu des 13 voix fixes.
  - Seules les pistes actives sont affichées (par défaut BD/SD/HH/Tom).
  - Sélection de piste via `selected_track_slot` ; `selected_instrument` reste synchronisé avec le `drum_voice_index` legacy.
- **Onglets Sound / Track dans le Sound Editor (MG-8).**
  - `SoundEditorTab` : `Sound` (panneau de synthèse actuel) / `Track` (contrôles de piste).
  - Onglet `Track` : nom de piste, type d'instrument, routing Main/Out 1..14, note MIDI.
- **Bouton `+ Add Track`.**
  - Active le premier slot inactif avec un Kick par défaut.
  - Met à jour `SoundSettingsState` avec les valeurs par défaut de l'instrument.
  - Bumper la version du layout pour forcer la réinitialisation du synthétiseur dans le thread audio.
- **Réinitialisation du synthétiseur sur changement de layout.**
  - `process()` surveille `track_layout.state.version` et réinitialise `DrumSynthesizer` si elle change.

---


## 2026-07-01 — Audio: moteur 14 slots + routing modulaire (build 20260701-093806)

**Build:** `20260701-093806`
**Validation:** `cargo check` OK, `cargo test` OK (103 tests), `build.ps1 -Install` OK

### Changements
- **Le moteur audio itère désormais sur les 14 slots actifs du layout.**
  - `process()` snapshot le `AtomicTrackLayout` au début de chaque buffer.
  - Les triggers du séquenceur (13 familles legacy) sont routés vers chaque slot actif de la famille correspondante.
  - Le mixage Main, les sorties auxiliaires et les événements MIDI sont émis par slot.
- **Routing par piste fonctionnel.**
  - `main_on` contrôle l'envoi dans le Main Mix.
  - `out_select` (`Main` / `Out 1..14`) route le signal vers la sortie auxiliaire choisie.
  - `AUX_OUT_COUNT` passe de 13 à 14 ; les noms de sorties deviennent génériques (`Out 1` .. `Out 14`).
- **Hi-hat choke adapté au modèle modulaire.**
  - Un trigger HiHat reset toutes les pistes OpenHiHat actives, quel que soit leur slot.
- **`initialize()` utilise le layout actif.**
  - Le synthétiseur est initialisé avec `TrackLayoutState::default_layout()` (BD/SD/HH/Tom) au lieu du legacy 13 voix.
  - Ajout de `AtomicTrackLayout::snapshot()` pour capturer le layout sans verrou.

---


## 2026-06-30 — Fix: crash au lancement du transport dans Studio One (build 20260630-201216)

**Build:** `20260630-201216`
**Validation:** `cargo check` OK, `cargo test` OK (103 tests), `build.ps1 -Install` OK

### Changements
- **Correction d'un crash immédiat au lancement de la lecture dans Studio One.**
  - `mix_gains` était encore dimensionné à 13 voix (`DrumVoice::COUNT`) alors que `voice_outputs` était passé à 14 slots (`MAX_TRACKS`).
  - L'index 13 provoquait un `index out of bounds` dans le mixage Main, qui tuait le plugin dès le premier échantillon.
  - `mix_gains` est maintenant un tableau de `MAX_TRACKS` ; les slots 0-12 suivent les paramètres `mix_*` existants, le slot 13 est silencieux par défaut.

---

## 2026-06-30 — Architecture: fondations du grid modulaire 14 slots (build 20260630-181506)

**Build:** `20260630-181506`
**Validation:** `cargo check` OK, `cargo test` OK (103 tests), `build.ps1 -Install` OK

### Changements
- **Nouveau modèle de tracks modulaires (`src/track.rs`).**
  - 14 slots internes fixes (`MAX_TRACKS = 14`), seuls les slots actifs sont visibles dans l'UI.
  - 11 types d'instruments : Kick, Snare, HiHat, OpenHiHat, Tom, Clap, Ride, Cymbal, Snare606, BassDrum808, Perc1.
  - `TrackLayoutState` persiste dans un nouveau champ DAW `track-layout-v1`.
  - Migration legacy 13 voix → 14 slots via `TrackLayoutState::from_legacy_13()`.
- **Adaptation de `SoundSettingsState` à 14 slots.**
  - Persistance `sound-settings-v2` compatible avec les anciens formats 12 et 13 champs.
  - Initialisation des slots selon le layout actif.
- **Adaptation de `DrumSynthesizer` à 14 instances indépendantes.**
  - `voices` allouées sur le heap (`Box<[Option<Box<DrumVoiceKind>>; MAX_TRACKS]>`) pour éviter le stack overflow.
  - `initialize_with_layout()` crée les voices selon le `TrackLayoutState`.
  - API passée de `DrumVoice` index à `slot_idx`.
- **Adaptation de `lib.rs` et `ui.rs` pour `MAX_TRACKS`.**
  - Tableaux `current_steps`, `voice_test_triggers`, `voice_outputs` passés à 14 slots.
  - Comportement actuel inchangé : le layout par défaut est le legacy 13 voix.

---

## 2026-06-30 — UX: bouton fermeture simplifié avec accent au clic (build 20260630-155543)

**Build:** `20260630-155543`
**Validation:** `cargo check` OK, `cargo test` OK (100 tests), `build.ps1 -Install` OK

### Changements
- **Refonte du bouton `×` de fermeture.**
  - Plus de fond ni de bordure au repos : juste une croix discrète en `INK3`.
  - Plus d’état hover inutile.
  - **Au clic maintenu : fond plein avec la couleur d’accent du menu** (orange pour Plock/Fusion, violet pour Seq Plock, etc.) et croix en `INK` blanc.
  - Le feedback est donc binaire et très visible : rien au repos, couleur d’accent sous le doigt.

---

## 2026-06-30 — Fix: création de fusions cassée + migration pattern-v4 (build 20260630-145727)

**Build:** `20260630-145727`
**Validation:** `cargo check` OK, `cargo test` OK (100 tests), `build.ps1 -Install` OK

### Changements
- **Correction de la régression qui empêchait de créer des cellules fusionnées.**
  - La détection d’ancien format dans `unpack_fusion` était trop large : une fusion sans morphing (champ `field` par défaut = 255) positionnait le bit 24, ce qui faisait croire à l’ancien format.
  - Résultat : `is_valid` échouait sur les données décodées comme anciennes, et la fusion disparaissait.
  - `unpack_fusion` ne décode maintenant que le nouveau layout ; l’ancien format est migré au niveau de l’état DAW.
- **Passage du champ de persistance de `pattern-v3` à `pattern-v4`.**
  - `filter_state` migre automatiquement `pattern-v3` vers `pattern-v4` en préservant la géométrie des fusions existantes (les données de morphing corrompues sont ignorées).
  - Les migrations `pattern-v2`, `pattern-v1` et legacy `st01..st16` pointent maintenant vers `pattern-v4`.
- **Tests ajoutés :**
  - round-trip `SharedPattern` avec et sans morphing ;
  - migration `pattern-v3` → `pattern-v4` avec conservation de la géométrie.

---

## 2026-06-30 — Fix: corruption des valeurs de morphing dans les fusions (build 20260630-144304)

**Build:** `20260630-144304`
**Validation:** `cargo check` OK, `cargo test` OK (97 tests), `build.ps1 -Install` OK

### Changements
- **Correction d’un bug critique d’encodage binaire des fusions.**
  - Dans l’ancien layout 3×`u64`, `end_value` du premier target était shifté de 40 bits, ce qui ne laissait que 24 bits dans le `u64` — les 8 bits de poids fort du `f32` étaient perdus.
  - Conséquence : une valeur comme `Frequency = 300.0` devenait un nombre dénormal proche de zéro après sauvegarde/recharge, d’où le "reset à zéro" constaté.
  - Le 3ème target subissait une troncature similaire, ce qui expliquait les comportements erratiques avec plusieurs cibles.
- **Nouveau layout binaire compact sur 3×`u64`.**
  - Stocke correctement la géométrie de la fusion + 4 cibles de morphing (`field` 8 bits + `end_value` 32 bits chacune).
  - Bit de validité déplacé pour éviter toute ambiguïté avec l’ancien format.
- **Migration automatique des anciennes fusions.**
  - Les fusions encodées avec l’ancien format sont reconnues : la géométrie des cellules fusionnées est conservée, mais les données de morphing corrompues sont ignorées (morphing désactivé sur ces groupes).
- **Tests unitaires ajoutés** pour valider le round-trip 1 à 4 cibles et la migration depuis l’ancien format.

---

## 2026-06-30 — Feature: morphing multi-cibles parallèles sur les fusions (build 20260630-123315)

**Build:** `20260630-123315`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Les fusions supportent maintenant jusqu'à 4 cibles de morphing en parallèle.**
  - Le menu contextuel de clic droit sur une cellule fusionnée permet d'ajouter, modifier et supprimer plusieurs cibles de morphing.
  - Chaque cible possède un paramètre (`morph_field`) et une valeur de fin (`morph_end_value`).
  - Les cibles actives sont appliquées simultanément lors de chaque pulse de la fusion.
  - L'interpolation reste linéaire de la valeur courante (globale ou plock) vers la valeur de fin sur la durée de la fusion.
- **Modèle de données refactoré.**
  - `FusedGroup` et `TriggerResult` utilisent un tableau fixe `[MorphTarget; 4]` piloté par `morph_count`.
  - `SharedPattern` stocke les fusions dans 3 slots `AtomicU64` par groupe (`FUSION_SLOT_COUNT = 3`) pour encoder 4 cibles.
- **Persistance DAW et pattern bank mises à jour.**
  - Format `pattern-v3` inchangé au niveau du champ, mais la taille des données fusion augmente (`INSTRUMENT_COUNT * MAX_FUSIONS * FUSION_SLOT_COUNT * 8`).
  - Migration automatique des anciennes fusions mono-cible (`unpack_fusion_legacy`) vers le format multi-cibles.
  - La pattern bank sauvegarde et restaure correctement les fusions multi-cibles.
- **L'UI du menu contextuel reflète les cibles multiples** avec un bouton **Add Morph Target** jusqu'à 4 cibles maximum.

---

## 2026-06-30 — Feature: morphing accessible depuis le menu contextuel des fusions (build 20260630-120230)

**Build:** `20260630-120230`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Le morphing des cellules fusionnées est maintenant accessible par clic droit, avec une présentation identique au menu p-lock.**
  - Clic droit sur une cellule fusionnée : menu contextuel avec **Morphing**, **Edit Fusion Steps**, **Delete Fusion**.
  - Sélection de **Morphing** : affichage des paramètres continus sous forme de lignes avec slider + valeur (Volume, Frequency, Decay, Filter, Attack, Release, curves, Analog, Stereo, special params continus).
  - Le paramètre actuellement morphé est surligné (label en couleur d'accent).
  - Déplacement d'un slider : définit immédiatement `morph_field` et `morph_end_value`.
  - Bouton **Disable Morphing** pour désactiver (`morph_field = 255`).
  - Toggle **Display Notes/Hz** conservé pour les bass drums (Kick / BassDrum808).
- **Le menu p-lock de la cellule source reste disponible** en dessous des actions fusion.

---

## 2026-06-29 — Feature: morphing par pulse sur les cellules fusionnées (build 20260629-160624)

**Build:** `20260629-160624`
**Validation:** `cargo check` OK, `cargo test` OK (153 tests), `build.ps1 -Install` OK

### Changements
- **Morphing par pulse sur les fusions (Step Fusion).**
  - Dans la boîte d'édition d'une fusion, un select **Morph** permet de choisir un paramètre continu (Volume, Frequency, Decay, Filter, Attack, Release, curves, Analog, Stereo, et les special params continus comme saturation amount/mix/output gain).
  - Un slider **End** définit la valeur cible à atteindre au dernier pulse.
  - L'interpolation est linéaire de la valeur actuelle (globale ou plock) vers la valeur de fin, appliquée à chaque pulse.
  - Les paramètres discrets (type d'algo, type de saturation, pre-filter, mode stéréo…) ne sont pas proposés.
- **Stockage des fusions étendu à `u64`.**
  - `FusedGroup` contient maintenant `morph_field` et `morph_end_value`.
  - `SharedPattern.fusions` passe de `AtomicU32` à `AtomicU64`.
- **Persistance DAW des fusions implémentée.**
  - Nouveau champ `pattern-v3` qui persiste les step masks + les fused groups.
  - Migration automatique depuis `pattern-v2` (masks uniquement) et `pattern-v1` / legacy `st01..st16`.
  - La pattern bank sauvegarde et restaure aussi les fusions (`fusion_bytes`).

---

## 2026-06-24 — Fix: dropdown Algo dynamique dans le menu p-lock (build 20260624-171823)

**Build:** `20260624-171823`
**Validation:** `cargo check` OK, `cargo test` OK (153 tests), `build.ps1 -Install` OK

### Changements
- **Le slider Algo du menu p-lock était fixe 0→3 et affichait un chiffre.**
  - Il est remplacé par un dropdown qui liste seulement les algorithmes disponibles pour l'instrument courant.
  - Le nom de l'algorithme est affiché (ex: `Sine`, `Square`, `FM`) au lieu de son index.
- **La ligne Algo est masquée quand l'instrument n'a qu'un seul algorithme.**
  - Concerné : Cymbal, Snare606, BassDrum808.
- **Les valeurs de plock existantes hors plage sont clampées** vers l'index valide le plus proche au moment de l'affichage.

---

## 2026-06-23 — Fix: suppression du slider Frequency inactif sur le Clap (build 20260623-163320)

**Build:** `20260623-163320`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK (après fermeture de Studio One)

### Changements
- **Le slider Frequency (onglet OSC) du Clap ne faisait rien** car le synthé Clap n'utilise que `filter_freq` (le filtre passe-bande HP/LP).
- Le Clap utilise maintenant `NO_FREQ_STD` (comme le Cymbal) : plus de slider Frequency inutile.
- `ClapSettings` n'expose plus `frequency` pour éviter toute confusion.
- Vérification des autres instruments non tonaux :
  - HiHat / OpenHiHat : Frequency contrôle le peaking filter → utilisé.
  - Ride : Frequency contrôle les oscillateurs inharmoniques → utilisé.
  - Cymbal : n'avait déjà pas de slider Frequency → cohérent.
  - Seul le Clap avait ce problème.

---

## 2026-06-23 — Feature: saturation ajoutée à HiHat, OpenHiHat, Clap, Ride, Cymbal (build 20260623-153112)

**Build:** `20260623-153112`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Saturation complète pour les 5 instruments qui n'en avaient pas.**
  - HiHat, OpenHiHat, Ride : 5 paramètres saturation (type, amount, mix, output gain, pre-filter) en `special[0..4]`.
  - Clap : echo reste en `special[0]`, saturation en `special[1..5]`.
  - Cymbal : shimmer/noise restent en `special[0..2]`, saturation en `special[3..7]`.
  - `DrumFlashParams` expose 25 nouveaux `FloatParam` (5 × 5 instruments).
  - Chaque voix DSP initialise un `SaturationConfig`, l'applique sur le signal de sortie, et réagit aux changements via `set_special_param`.

---

## 2026-06-23 — Fix: resync du séquenceur quand `pattern_length` change (build 20260623-151154)

**Build:** `20260623-151154`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Le séquenceur est resynchronisé avec le transport hôte dès que `pattern_length` change.**
  - Avant, changer `Len` pendant la lecture (ex: 16 → 48) ne mettait pas à jour `loop_count` ni `beat_position` par rapport à la nouvelle longueur.
  - Cela pouvait créer un décalage permanent entre la page affichée et la page réellement lue, surtout avec des conditions de step dépendant du loop count.
  - `process()` détecte maintenant le changement de `master_length` et appelle `sync_to_host(position_beats)` pour recaler le séquenceur.

---

## 2026-06-23 — Fix: paste de page étend automatiquement la longueur du pattern (build 20260623-145953)

**Build:** `20260623-145953`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK (après fermeture de Studio One)

### Changements
- **Coller une page au-delà de la longueur actuelle étend automatiquement `pattern_length`.**
  - Avant, coller sur la page 2, 3 ou 4 avec `Len = 16` copiait bien les notes mais elles n'étaient pas jouées, ce qui donnait l'impression que l'ordre des pages ne se lisait pas.
  - Maintenant, après un `Paste Page`, si la page cible dépasse `pattern_length`, le paramètre `Len` est augmenté au multiple de 16 nécessaire (jusqu'à 64).
  - Cela concerne aussi le menu page Copy → Paste, pas seulement les presets/générateurs.

---

## 2026-06-23 — UX: confirmations page en lignes verticales (build 20260623-143214)

**Build:** `20260623-143214`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Layout des confirmations Paste/Clear revu.**
  - Au lieu de boutons Yes/No collés côte à côte, les confirmations affichent une ligne d'info puis deux boutons pleine largeur empilés : "Yes, overwrite" / "No, cancel" et "Yes, clear" / "No, cancel".
  - Le label d'info n'est plus un faux bouton inactif.

---

## 2026-06-23 — UX: menu page se ferme sur Copy (build 20260623-142847)

**Build:** `20260623-142847`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Clic sur "Copy" dans le menu page ferme immédiatement le popup.**

---

## 2026-06-23 — UX: menu page plus compact (build 20260623-142211)

**Build:** `20260623-142211`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Menu page réduit d'environ moitié.**
  - Nouveau `page_menu_frame` : min 130 px / max 150 px (vs 260/284 px pour les plocks).
  - Labels raccourcis : Copy / Paste / Clear, puis "Overwrite?" / "Clear?" + Yes / No en confirmation.
  - Header sans sous-titre "Step N".

---

## 2026-06-23 — Fix: synchronisation fusions lors du chargement de pattern (build 20260623-144724)

**Build:** `20260623-144724`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **`load_pattern_for_ui` et `load_pattern_for_ui_with_length` copient maintenant aussi les fusions.**
  - Avant, seuls les step masks étaient copiés ; les fusions de l'ancien pattern persistaient dans `SharedPattern`.
  - Le séquenceur audio pouvait donc jouer des fusions fantômes qui n'étaient plus visibles sur le grid après un preset / génération / clear.
  - Les fusions du `Pattern` source sont maintenant écrites dans `SharedPattern` pour chaque instrument.

---

## 2026-06-23 — UX: menu page restylé comme les menus p-lock (build 20260623-141809)

**Build:** `20260623-141809`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK (après fermeture de Studio One)

### Changements
- **Le menu contextuel des pages reprend le style `.plk` des menus p-lock.**
  - `plock_menu_frame` + `plock_menu_header` + `plock_menu_action_row` pleine largeur.
  - Barre d'accent bleue en haut, fond `P_ACTIVE`, bordure `LINE2`, radius 9.
  - Copy Page (bleu), Paste Page (orange `PL_LINK` si dispo, sinon grisé), Clear Page (rouge).
  - Les confirmations Paste/Clear apparaissent comme des lignes d'action dans le même menu.

---

## 2026-06-23 — UX: menu Copy/Paste/Clear sur les boutons de page (build 20260623-124600)

**Build:** `20260623-124600`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **[100y] Menu contextuel sur les boutons de page (1-4).**
  - Clic droit sur un numéro de page : ouvre un menu avec Copy Page / Paste Page / Clear Page.
  - Les fonctions Copy/Paste/Clear du pattern (P1-P8) restent intactes.
  - Copy Page : copie les triggers, les sound plocks et les fusions de la page dans `EditorUIState.page_clipboard`.
  - Paste Page : demande confirmation avant d'écraser la page cible.
  - Clear Page : demande confirmation avant de vider la page (triggers + plocks + fusions).
  - Popup maison `egui::Area` avec le style `.plk` (fond `P_ACTIVE`, bordure `LINE2`, radius 9).

---

## 2026-06-23 — UX: focus auto sur le champ step-count en édition fusion (build 20260623-122425)

**Build:** `20260623-122425`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Double-clic sur une fusion : le champ "Steps" reçoit le focus et son texte est sélectionné.**
  - Remplacement du `DragValue` par un `TextEdit` singleline pour permettre la sélection complète.
  - `EditorUIState.fusion_edit_focus_request` déclenché à l'ouverture de l'édition.
  - Focus + sélection `CCursorRange` de 0 à len appliqués sur le `TextEditOutput`.
- La valeur est parsée et clampée 1..64 à la perte de focus ou au changement.

---

## 2026-06-23 — UX: sortie auto du mode édition fusion (build 20260623-120806)

**Build:** `20260623-120806`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **[91] Sortie automatique du mode edit quand on clique en dehors de la cellule fusionnée.**
  - Lors d'un clic sur une autre cellule, si le clic ne porte pas sur le groupe fusionné en cours d'édition, l'édition est terminée avant de traiter le toggle.
  - Conserve le comportement si on reclique sur le même groupe fusionné (l'édition reste active).

---

## 2026-06-23 — Réinstallation du VST3 (build 20260623-113150)

**Build:** `20260623-113150`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Réinstallation du bundle VST3 après suppression.**
  - Aucun changement de code ; rebuild + install du dernier état source.
  - Bundle déployé dans `C:\Program Files\Common Files\VST3\drum-pattern-vst.vst3`.

---

## 2026-06-16 — Redesign UI: fusion couleur d'édition + texte centré (build 20260616-211439)

**Build:** `20260616-211439`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Fusion : la cellule de départ reprend sa couleur normale en sortie d'édition.**
  - `is_fusion_start` n'applique plus la couleur bleue foncée quand `is_editing` est actif ; seul le mode édition clignote.
  - Après fermeture de la boîte d'édition, le bloc fusionné redevient bleu standard.
- **Le nombre de triggers (`step_count`) est centré dans le bloc fusionné entier**, plus seulement dans la première cellule.

---

## 2026-06-16 — Redesign UI: rendu continu des cellules fusionnées (build 20260616-210639)

**Build:** `20260616-210639`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Les cellules fusionnées (Step Fusion) sont à nouveau rendues comme un bloc continu.**
  - `draw_step_cell_v2` étend le rectangle de la cellule de départ pour recouvrir l'ensemble du groupe fusionné, gaps compris.
  - Les cellules internes restent transparentes : pas de bordure ni de fond qui cassent le bloc.
  - L'indicateur "pulses" (`step_count`) est affiché sur la cellule de départ.
- **Le mode édition d'une fusion fait de nouveau clignoter l'ensemble du bloc.**
  - `is_editing` est recalculé depuis `state.fusion_editing` dans `draw_grid_v2`.
  - Toutes les cellules du groupe en édition pulsent en bleu de manière synchronisée.
- **Playhead sur une fusion restreint à la cellule exacte du curseur.**
  - `is_current` ne met plus l'anneau playhead sur toutes les cellules du groupe, seulement sur la cellule active.

---

## 2026-06-16 — Redesign UI: suppression undo par paramètre dans menus p-lock (build 20260616-203617)

**Build:** `20260616-203617`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Bouton "↺" (undo) retiré de chaque rangée de paramètre dans les menus p-lock.**
  - Il décalait les sliders et n'apportait pas assez de valeur par rapport aux actions globales `Clear Plock` / `Copy Plock`.
  - `plock_menu_row` passe de 7 à 6 arguments (suppression du callback `on_undo`).
  - Tous les appelants mis à jour : Volume, Display, Freq notes, standard params, Algo, specials, Probability, Stutter.

---

## 2026-06-15 — Redesign UI: menus p-lock bordure + cellule d'édition clignotante (build 20260615-165139)

**Build:** `20260615-165139`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Bordure fine claire (`LINE2`) ajoutée autour du menu p-lock.**
  - Stroke 1 px sur le `plock_menu_frame`, cohérent avec les autres panneaux.
- **Cellule en cours d'édition clignote.**
  - Quand un menu p-lock est ouvert, la step source pulse en bleu (fond interpolé + bordure `BLUE` 1.5 px).
  - Utilise `ctx.input(|i| i.time)` pour un clignotement sinusoïdal à 4 Hz.
  - `step_colors_v2` reçoit un paramètre `is_editing` ; `draw_grid_v2` passe l'état du popup.

---

## 2026-06-15 — Redesign UI: menus p-lock popup maison (build 20260615-160242)

**Build:** `20260615-160242`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Remplacement du `context_menu` egui par un popup maison.**
  - Le cadre noir venait de `Frame::menu` de egui, non contrôlable par `window_stroke`.
  - Le clic droit sur une step ouvre maintenant un `egui::Area` personnalisé avec notre propre `Frame::NONE` rempli `P_ACTIVE` r9.
  - Plus de bordure, plus d'ombre parasite.
  - État `plock_popup` dans `EditorUIState` avec fermeture au clic à l'extérieur.
  - Fusion "Edit/Delete" toujours disponible en mode Sound ; Sequencer inchangé.

---

## 2026-06-15 — Redesign UI: menus clic-droit p-lock reskinés (build 20260615-122058)

**Build:** `20260615-122058`
**Validation:** `cargo check` OK, `cargo test` OK (91 + 62 tests), `build.ps1 -Install` OK

### Changements
- **[100x] Menus clic-droit p-lock reskinés.**
  - Menu Sound (`draw_plock_menu`) utilise le frame `.plk` (fond `P_ACTIVE`, radius 9, barre d'accent orange `PL_LINK`, ombre).
  - Menu Sequencer (`draw_sequencer_plock_menu`) réécrit avec le même style, accent violet `SEQPL`.
  - Header "Seq Plock {instrument}" + "Step N", indicateur Mode Active/Inactive.
  - Probability et Stutter en rangées avec slider `LocalParamSlider` et valeur en ligne.
  - Grille Condition en 3 colonnes avec boutons stylisés (accent sélectionné).
  - Actions "Create Seq Plock" / "Clear Seq Plock" stylisées comme les actions Sound.
- Uniformisation des helpers `plock_menu_frame`, `plock_menu_header`, `plock_menu_row`, `plock_menu_action_row` partagés entre Sound et Sequencer.

---

## 2026-06-14 — Redesign UI: bloc Generator réorganisé (build 20260614-205742)

**Build:** `20260614-205742`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Bloc Generator refondu en 2 rangées alignées** (avant : 3 rangées tassées + labels décalés d'un cran, avec un label « B » orphelin sans combo).
  - Rangée 1 (moteur) : combo algorithme · **A**/**B** = styles à morpher · sliders pilule **Mix/Dens/Var** (slider du design system, fini les `ParamSlider` bruts) · **GENERATE** poussé à droite.
  - Rangée 2 (raccourcis) : Presets Rock/Funk/Disco + ⟳ Random.
- Labels corrigés (A = style primaire, B = style secondaire) ; selects `.selbox` + contrôles h26, cohérents avec header/éditeur. Import `ParamSlider` retiré (plus utilisé).
- **Sliders pilule** : la poignée Ø11 réserve désormais son rayon à chaque extrémité (`header_param_slider`) — plus de troncature à 0 %/100 % (corrige aussi le slider Len de la page-bar).
- **Labels complets** : « Mix · Densité · Variation » (largeurs ajustées pour garder des pistes lisibles).

---

## 2026-06-14 — Redesign UI: panneau Generator en 3 lignes propre (build 20260614-102408)

**Build:** `20260614-102408`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK après fermeture de Studio One

### Changements
- Panneau Generator retravaillé en **3 lignes** suite au feedback utilisateur.
  - Ligne 1 : `Generator` / `Type` / `A` / `B` avec les combos.
  - Ligne 2 : sliders `Mix` / `Density` / `Variation` avec labels alignés et noms complets.
  - Ligne 3 : bouton `GENERATE` à gauche, texte centré manuellement.
- Hauteur du bottom panel augmentée à `190 px` pour accueillir les 3 lignes.

## 2026-06-14 — Bouton GENERATE à la ligne + centré (build 20260614-095451)

**Build:** `20260614-095451`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Bouton `GENERATE` remis sur une ligne dédiée sous les contrôles Generator.
- Hauteur du bottom panel augmentée de 132 px à 168 px pour accueillir les deux lignes.
- Centrage manuel du texte `GENERATE` dans le bouton via `ui.painter().galley()`.

## 2026-06-14 — Fix bouton GENERATE invisible (build 20260614-092628)

**Build:** `20260614-092628`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Correction du bouton `GENERATE` invisible dans le panneau Generator.
  - Retour à une seule ligne horizontale pour éviter que le layout vertical ne dépasse la hauteur allouée au panel.
  - Le bouton est poussé à droite avec un `add_space` calculé après les sliders.
  - Réduction légère des largeurs de combos/sliders pour tenir dans la ligne.

## 2026-06-13 — Redesign UI: panneau Generator en deux lignes (build 20260613-210831)

**Build:** `20260613-210831`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK après fermeture de Studio One

### Changements
- Réécriture propre du layout du panneau Generator en deux lignes fixes.
  - Ligne 1 : `Generator` / `Type` / `A` / `B` + sliders `Mix` / `Dens` / `Var` sur une seule ligne horizontale.
  - Ligne 2 : bouton `GENERATE` déplacé en dessous, aligné à droite.
  - Espacements et largeurs de combos/sliders constants via constantes locales.

## 2026-06-13 — Redesign UI: alignement panneau Generator v2 (build 20260613-203430)

**Build:** `20260613-203430`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK après fermeture de Studio One

### Changements
- Correction du layout panneau Generator après screenshot utilisateur.
  - Suppression du sous-layout `right_to_left` qui compressait le bouton `GENERATE`.
  - Le bouton `GENERATE` est poussé à droite via `ui.add_space()` calculé dans le `horizontal` parent.
  - Le bloc de sliders (morph A/B + Mix/Dens/Var) est centré dans l'espace restant.
  - Largeur des combos A/B harmonisée à 92 px.

## 2026-06-13 — Redesign UI: alignement panneau Generator (build 20260613-193615)

**Build:** `20260613-193615`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK après fermeture de Studio One

### Changements
- Réalignement des sliders `Mix`, `Dens`, `Var` et du bouton `GENERATE` dans le panneau Generator.
  - Le bloc de paramètres est désormais centré dans l'espace disponible entre les combos A/B et le bouton.
  - Le bouton `GENERATE` est ancré à droite et sa largeur passe à 96 px pour matcher la maquette.
  - Espacements harmonisés via des constantes locales.

## 2026-06-13 — Correction régression Push/Pull (build 20260613-105028)

**Build:** `20260613-105028`
**Validation:** `cargo test` OK (91 lib + 62 standalone), `build.ps1 -Install` OK

### Changements
- `Sequencer::sync_to_host` recalcule `step_counter` depuis la timeline *shifted* (position hôte moins le décalage Push/Pull) au lieu de la timeline master.
  - Évite le décalage de phase qui apparaissait après un seek/loop quand une piste avait du Push/Pull.
  - Garde la polyrythmie et les conditions de step stables après resync.
- UI grille : la playhead reste sur `current_step` global et ne bouge plus quand on module Push/Pull.
  - Push/Pull décale uniquement le timing audio ; la grille visuelle reste alignée sur le transport hôte.
- Tests ajoutés/corrigés :
  - `test_push_pull_sync_to_host_preserves_phase` valide la stabilité après `sync_to_host` avec +30 ms.
  - `test_track_push_pull_stability` corrigé : applique réellement `push_pull_ms` au lieu de passer la valeur comme `swing`.

### Point d'attention résolu
- `[101]` Régression Push/Pull : le décalage audio ne doit plus devenir énorme après lecture/seek ; le reset double-clic à `0 ms` ramène bien à un comportement neutre. La tête de lecture visuelle reste stable quand on module Push/Pull.

---

## 2026-06-12 — Redesign UI: playhead indépendante du Push (build 20260612-210534)

**Build:** `20260612-210534`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- La playhead visuelle de la grille utilise désormais `current_step` global non décalé.
- Les valeurs `Push/Pull` continuent de décaler les déclenchements audio, mais ne déplacent plus l'anneau de lecture dans l'UI.
- Les `current_steps` par piste restent produits côté moteur pour la logique interne, mais ne pilotent plus l'affichage de la tête de lecture.

### Point d'attention
- Retour utilisateur fin de session : le comportement Push/Pull est devenu incorrect (décalage énorme, difficile à annuler). Reprise prioritaire consignée dans `TODO.md` sous `[101]`.

---

## 2026-06-12 — Redesign UI: double-clic reset Hum/Push (build 20260612-205837)

**Build:** `20260612-205837`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK après fermeture de Studio One

### Changements
- Double-clic sur les mini sliders `Hum` et `Push` : reset à la valeur par défaut du paramètre (`0%` / `0 ms`).
- Le tooltip custom affiche immédiatement la valeur resetée après double-clic.

---

## 2026-06-12 — Redesign UI: tooltip custom Hum/Push (build 20260612-205255)

**Build:** `20260612-205255`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Remplacement de `Response::on_hover_text()` par une bulle custom `Foreground` pour les mini sliders `Hum` et `Push`.
- La bulle est ancrée au-dessus du slider et reste visible au hover comme pendant le drag.
- Valeurs affichées : `Humanize: xx%` et `Push/Pull: +x ms`.

---

## 2026-06-12 — Redesign UI: tooltip Hum/Push corrigé (build 20260612-174601)

**Build:** `20260612-174601`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Correction du tooltip des mini sliders `Hum` / `Push` : suppression du tooltip vide qui masquait la valeur.
- `Push/Pull` affiche désormais explicitement l'unité `ms` dans le hover.

---

## 2026-06-12 — Redesign UI: valeurs Hum/Push en tooltip (build 20260612-173557)

**Build:** `20260612-173557`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK après fermeture de Studio One

### Changements
- Suppression du texte incrusté dans les mini sliders `Hum` et `Push`.
- Le hover affiche maintenant la valeur utile : `Humanize: xx%` ou `Push/Pull: +x`.
- Le tooltip générique seul (`Humanize`, `Push/Pull`) a été remplacé par la valeur formatée.

---

## 2026-06-12 — Redesign UI: Hum/Push + switch p-lock (build 20260612-164416)

**Build:** `20260612-164416`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Les colonnes `Hum` et `Push` affichent à nouveau leurs valeurs directement dans les mini sliders (`%` pour Hum, valeur signée pour Push).
- Sliders `Hum` et `Push` harmonisés sur la même couleur bleue.
- Remplacement du switch `P-Lock Mode` par un contrôle custom fiable et coordonné : `Sound` orange / `Sequencer` violet, hauteur 26 px, rayon 6, bordure `LINE2`.
- Suppression de helpers UI devenus morts après le recâblage (`segmented_control`, ancien mini slider param sans valeur).

---

## 2026-06-12 — Redesign UI: sliders constants + Note/Freq (build 20260612-162103)

**Build:** `20260612-162103`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Largeur de colonne paramètres fixée à `340 px` pour toutes les sections du Sound Editor : les sliders gardent désormais la même longueur avec ou sans graphe ENV/Filter.
- Extraction du rendu de piste slider pour partager exactement les mêmes dimensions entre les rangées.
- Remplacement de la checkbox `Notes` des bass drums par un mini sélecteur segmenté `Hz | Note` intégré à la rangée Frequency.
- Mode Note : contrôles `-` / note mono / `+` alignés dans la rangée, sans titre ni checkbox parasite.

---

## 2026-06-12 — Redesign UI: labels ADSR dans le graphe (build 20260612-151646)

**Build:** `20260612-151646`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Réintroduction des labels `A`, `D`, `S`, `R` directement dans le cadre du graphe ADSR.
- Les labels restent discrets en IBM Plex Mono Medium gris clair et sont clampés pour ne pas sortir du graphe.
- La légende externe sous les contrôles d'enveloppe reste supprimée.

---

## 2026-06-12 — Redesign UI: enveloppe ADSR sans légendes (build 20260612-150813)

**Build:** `20260612-150813`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK après fermeture de Studio One

### Changements
- Suppression des légendes A/H/D/R affichées sous les paramètres d'enveloppe dans le Sound Editor.
- Refonte du graphe d'amplitude en lecture ADSR simplifiée conforme à la maquette : attaque ambre, decay bleu, release violet.
- Ajout des 5 lignes verticales de grille `white_a(13)` dans le cadre du graphe.
- Suppression des lettres A/D/R/H dans le canvas : le graphe ne garde que les courbes et la grille.

---

## 2026-06-12 — Redesign UI: Select stylé maquette (build 20260612-145130)

**Build:** `20260612-145130`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Remplacement des `ComboBox` egui restants par un widget Select custom aligné sur `.selbox` : hauteur 26 px, fond `PANEL2`, bordure `LINE2`, hover bleu, texte courant en IBM Plex Mono Medium.
- Application aux selects Sound Editor : Saturation Type, Noise Type, Click Type et Algorithm.
- Application aux selects header/bas de page : Groove, Generator type, Style A et Style B.
- Menu déroulant custom : fond `P_ACTIVE`, bordure `LINE2`, options en IBM Plex Sans Medium, hover bleu + texte blanc.

---

## 2026-06-12 — Redesign UI: Sound Editor réorganisé + finitions (build 20260612-142330)

**Build:** `20260612-142330`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements (suite au retour : sliders trop longs, intitulés non alignés, graphes serrés)
- **Colonne de paramètres à largeur contrainte** par section : les sliders flexent dans cette colonne (uniformes, plus courts) au lieu de s'étirer sur toute la largeur.
- **Graphes d'enveloppe** : la colonne de params réserve désormais ~196 px + un gap de 16 px pour le graphe ENV/Filter → il n'est plus serré contre la droite ; cadre redessiné (fond #0c0c11, rayon 7, ~104 px de haut, remplit la largeur dispo).
- **Intitulés alignés** : tous les labels sur la même colonne de 138 px (Algorithm et Mix utilisaient avant un label nu non aligné).
- **Titres de section** : noms complets (Oscillator / Envelope / Filter / Saturation / Output) en sans 600 INK3 au lieu d'abréviations mono MAJUSCULES.
- **Mix** : ToggleSwitch aligné à droite (au lieu d'une checkbox egui brute).
- **Intitulés alignés à gauche** : colonne label 138 px rendue en `left_to_right` (avant centrés/flottants via `add_sized`).
- **Slider Volume** ramené à la largeur des sections (340 px) — fin de l'incohérence.
- **Sections vides masquées** : une famille sans paramètre pour l'instrument (ex. Saturation sur l'OpenHiHat) n'affiche plus de titre orphelin.

---

## 2026-06-12 — Redesign UI: Sound Editor (sliders / switches / en-tête) (build 20260612-114809)

**Build:** `20260612-114809`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Slider rows de l'éditeur** : piste fine à largeur **fixe (190 px)** sans bordure (aplat PANEL2 + fill bleu + poignée au survol), valeur mono à droite ; label sans 500. (Une piste *flex* a été abandonnée : elle consommait l'espace horizontal réservé au graphe d'enveloppe inline → sliders trop longs + graphes disparus.)
- **Padding du Sound Editor** : contenu du scroll encadré (14 px gauche/droite, 6 px haut) — les labels ne touchent plus le bord gauche.
- **Switch rows** : le ToggleSwitch est poussé au bord droit (space-between) ; label sans 500.
- **En-tête éditeur** : titre « Sound Editor » en blanc/bold ; nom d'instrument en mono ; bouton « Engine ▾ » inerte retiré (réservé à la future phase modulaire).

### À suivre (éditeur)
- Modèle de section (filet DIVIDER au lieu de `separator`, espacements), combos → Select stylé, ADSR inline réécrit (modèle 3 segments), toggle Notes en pilule.

---

## 2026-06-12 — Redesign UI: grille séquenceur + page-bar (build 20260612-104952)

**Build:** `20260612-104952`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Grille séquenceur** : cellules pleines vives + bordure nette par état (hit bleu, link orange, snapshot rouge, seq violet) — pas de glow externe (il bavait sur les pas adjacents, egui n'a pas de flou) ; playhead = anneau blanc inset dessiné par-dessus (conserve la bordure d'état) ; tags M/S/T avec texte lisible (T blanc sur bleu, M sur ambre, S sur vert) ; noms de lane sans bordure au repos (contour bleu si sélectionné), police mono 600 ; poignée de drag en matrice de points 2×3 ; en-têtes de colonnes M/S/T ; bordure des cellules fusion-mid en `BLUE_DIM` (50% bleu).
- **Page/Length bar** : slider Len en piste fine custom (`header_param_slider` bare-track) ; bouton Follow ON en bleu plein + texte blanc ; LED rouge sous la page en lecture, z-order corrigé (halo puis point) ; lecture « {n} steps » en deux runs (nombre mono 12 + unité sans 9.5).
- **`header_param_slider`** étendu (label/valeur optionnels) et réutilisé pour Master/Swing/Len.

---

## 2026-06-12 — Redesign UI: nettoyage migration + fondations design system (build 20260612-102825)

**Build:** `20260612-102825`
**Validation:** `cargo check` OK, `cargo test --no-run` OK, `build.ps1 -Install` OK

### Changements
- **Nettoyage migration** : suppression de ~1300 lignes de code mort (anciens `draw_grid`, `draw_top_bar`, `draw_song_bar`, `draw_generator_panel`, helpers volume-dB, `bool_checkbox`, `draw_bool_toggle`) + suppression des modules morts `src/ui/schema.rs` et `src/ui/engine_registry.rs`. Un seul chemin de rendu (`*_v2`) reste actif. Helpers du menu page Copy/Paste/Clear conservés sous `#[allow(dead_code)]` pour recâblage ultérieur.
- **Polices multi-graisses** : ajout des faces IBM Plex Sans Medium/SemiBold/Bold + Mono Medium/SemiBold dans `assets/fonts/`. `install_egui_fonts` enregistre des familles nommées par graisse (`sans_med/sb/bold`, `mono_med/sb`) → fin du faux-gras `.strong()`.
- **Visuals globales** : coins r6, bordures hairline (LINE/LINE2), hover bleu, sans expansion sur les widgets egui par défaut.
- **Header refait à la maquette** : transport ▶■● et toggle Song retirés ; sliders Master/Swing en pilule fine (fill bleu, poignée au survol, valeur mono à droite) ; Groove ; segmented Seq Internal/Ext MIDI avec LED ; Choke/Auto-Edit en pilules LED ; séparateurs 1px LINE.

### À suivre
- Propagation du langage visuel aux zones restantes : grille séquenceur, éditeur, page-bar, menus p-lock (284px), patterns/generator.

---

## 2026-06-12 — Redesign UI IBM Plex fonts (build 20260612-090421)

**Build:** `20260612-090421`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Ajout des assets `IBMPlexSans-Regular.ttf` et `IBMPlexMono-Regular.ttf` dans `drum-pattern-vst/assets/fonts/`.
- Chargement des polices via `egui::FontDefinitions` au demarrage de l'editeur.
- IBM Plex Sans devient la police proportionnelle prioritaire et IBM Plex Mono la police monospace prioritaire.

---

## 2026-06-11 — Redesign UI Sound Editor controls (build 20260611-201611)

**Build:** `20260611-201611`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Remplacement des sliders natifs visibles dans le Sound Editor par des rows custom : label fixe, piste arrondie, fill bleu, valeur mono à droite.
- Application du nouveau rendu aux paramètres standards et aux paramètres spéciaux, notamment Saturation Amount/Mix/Output Gain.
- Ajout de switches custom pour les booléens d'éditeur.
- Réduction de la hauteur du panneau Generator/Song de `136px` à `116px` pour limiter le vide en bas.

---

## 2026-06-11 — Redesign UI corrections clipping/pagebar (build 20260611-194657)

**Build:** `20260611-194657`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Ajout d'un clipping explicite sur la colonne gauche pour empêcher les panneaux de peindre dans le Sound Editor.
- Repeint du fond de colonne droite après la colonne gauche pour supprimer les traces de débordement.
- Remplacement du panneau Generator/Song par un panneau à rectangle fixe, header/body clippés.
- Correction de la pagebar : suppression du layout `right_to_left` qui décalait `Len`, ordre normal `Len · slider · steps · 16/32/48/64 · x2`.
- Generator compacté en deux lignes : presets puis contrôles, combobox plus étroits, sliders sans valeur inline.

---

## 2026-06-11 — Redesign UI reprise structurelle (build 20260611-184532)

**Build:** `20260611-184532`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Reprise du layout principal à partir du design pack `Flash_Drum_design_11062026` : body fixe en deux colonnes, gauche flexible et droite `568px`, bordure verticale `LINE`, padding gauche `14px`.
- Remplacement du rendu de grille basé sur `egui::Grid` par un séquenceur custom : lanes `24px`, tags M/S/T `17px`, steps `21px`, cellules sans texte `X`/`.` et couleurs p-lock/playhead conformes au design system.
- Page bar séparée au-dessus du séquenceur : pages 1-4, LED rouge de lecture, Follow, Len, presets `16/32/48/64`, `x2`.
- Header principal : remplacement du simple toggle `Seq` par un segment `Internal | Ext MIDI` branché sur `use_internal_sequencer`.
- Sound Editor : header et barre d'onglets avec zones fixes, onglets instruments sur une seule ligne au lieu de deux colonnes de tabs.
- Bottom panel Generator/Song : panneau encadré avec header séparé et contenu en dessous.

---

## 2026-06-11 — Redesign UI Phase 2a — Layout 2 colonnes (build 20260611-102814)

**Build:** `20260611-102814`
**Commits:** UI : refactor layout principal en 2 colonnes selon design pack v2

### Changements
- **Layout 2 colonnes** (design pack §1, LAYOUT.md) :
  - Colonne gauche : **912px**, padding 14h/11v, gap vertical 4px, bordure droite 1px `LINE`
  - Colonne droite : **568px**, fond `PANEL` (20,20,25), bordure gauche 1px `LINE`
  - Grid compact : spacing 2px, step buttons 18px, lignes 20px
  - Suppression des `ui.separator()` entre sections (remplacés par le gap)
  - Positionnement : `allocate_new_ui` avec rectangles fixes
- **Header** (design pack §2) :
  - Transport ▶ ■ ● ajoutés (green/red actif)
  - Brand + version + séparateurs DIVIDER
- **Page bar** (design pack §3) :
  - Boutons 1-4 : 26×20px, radius 6px, label mono 10.5px
  - Follow toggle : style coordonné (P_ACTIVE/PANEL2)
  - Len slider compact (50px)
  - Presets 16/32/48/64 : 26×20px boutons
  - ×2 : 26×20px bouton
- **P-lock mode bar** :
  - Label "P-Lock Mode" INK3 10.5px
  - Segmented Sound/Sequencer avec couleurs PL_LINK/SEQPL
  - Fusion box sur même ligne, 380px
- **Pattern Bank** :
  - Save button : 44×22px
  - Slots P1-P8 : 30×22px
  - Export MIDI / Drag MIDI : boutons stylisés PANEL2/LINE2
- **Bottom Panel** (design pack §6) :
  - Toggle unifié Generator | Song (segmented)
  - Partage le même espace
- **Sound Editor** (design pack §6) :
  - Header : "Sound Editor" + nom instrument + Engine selector placeholder
  - Onglets instruments : grille responsive (7 colonnes)
  - Volume en tête sans titre de section
  - Titres de section : UPPERCASE mono 10px INK3
  - Suppression des `ui.group` avec bordures
- **Couleurs des steps** (design pack §6) :
  - off pair/impair : #1b1b22 / #23232c
  - on plock link : PL_LINK (255,140,0)
  - on plock snapshot : PL_SNAP (220,50,50)
  - off plock link : PL_LINK_DIM (180,100,0)
  - off plock snapshot : PL_SNAP_DIM (160,30,30)
  - Playhead : #30303c
- **Réduction globale** :
  - Tous les boutons et contrôles réduits pour tenir dans 800px
  - Gaps entre sections : 4px
  - Élimination du débordement bas
  - Colonne gauche : **912px**, padding 14h/11v, gap vertical 10px, bordure droite 1px `LINE`
  - Colonne droite : **568px**, fond `PANEL` (20,20,25), bordure gauche 1px `LINE`
  - Ordre colonne gauche : séquenceur (page-bar + grille + plock mode) → pattern bank → generator/song
  - Suppression des `ui.separator()` entre sections (remplacés par le gap de 10px)
- **Positionnement** : utilisation de `allocate_new_ui` avec rectangles fixes pour un layout pixel-perfect

---

## 2026-06-11 — Réception du design pack complet (designer)

**Livrable** : `design-pack/Flash_Drum_design_11062026/flash-drum-source/`

### Contenu du design
- **`DESIGN-SYSTEM.md`** — Tokens visuels (palette IBM Plex, typo, widgets, ADSR, états p-lock)
- **`LAYOUT.md`** — Architecture (lanes modulaires, moteurs, layout 2 colonnes, séquenceur, éditeur)
- **`assets/fd-data.js`** — Schémas de paramètres par moteur (synth/sample/midi)
- **`index.html`** — Maquette interactive fonctionnelle

### Architecture proposée (à implémenter)
- **Lanes modulaires** : 4 au départ (BD/SD/HH/TOM), ajoutables jusqu'à 14, réordonnables
- **Registre de moteurs** : Synth (7 types), Sample, Sample FX, MIDI Out
- **Éditeur dynamique** : contenu selon le moteur assigné, aucun paramètre codé en dur
- **Header** : Transport (▶/■/●) + source MIDI (Internal/Ext) + toggles LED
- **Sound Editor** : Sections dynamiques (OSC/ENV/FILTER/SAT/OUTPUT)
- **Generator/Song** : Panneau partagé avec toggle segmented

### Plan d'implémentation
Voir `TODO.md` — section **[100] Redesign UI complet** (phases 1-5)

---

## 2026-06-10 — Redesign UI Phase 1d — Page buttons + glow LED (build 20260610-203051)

**Build:** `20260610-203051`
**Commits:** UI : stylisation des boutons de page (1-4) avec tokens theme + glow sur LED de lecture

### Changements
- **Boutons de page** (1-4) :
  - Actif : fond `BLUE` + bordure `BLUE`
  - Inactif : fond `PANEL2` + bordure `LINE2`
- **LED de lecture** : glow `RED` semi-transparent autour du point central

---

## 2026-06-10 — Redesign UI Phase 1c — Style global sombre (build 20260610-202742)

**Build:** `20260610-202742`
**Commits:** UI : style global sombre via `egui::Visuals`, fond BG, widgets PANEL2/P_HOVER/P_ACTIVE/BLUE

### Changements
- **Style global** : configuration `egui::Visuals::dark()` personnalisée dans le callback d'init :
  - `panel_fill` = `window_fill` = `extreme_bg_color` = `BG` (10,10,15)
  - `widgets.inactive.bg_fill` = `PANEL2` (28,28,36)
  - `widgets.hovered.bg_fill` = `P_HOVER` (36,36,48)
  - `widgets.active.bg_fill` = `P_ACTIVE` (42,42,56)
  - `selection.bg_fill` = `BLUE` (74,158,255)
  - `window_stroke` = `LINE` (42,42,53)

---

## 2026-06-10 — Redesign UI Phase 1b — Header style + widgets (build 20260610-202506)

**Build:** `20260610-202506`
**Commits:** UI : header redesign avec fond PANEL, bordure LINE, séparateurs verticaux, padding 14px

### Changements
- **Header redesign** :
  - Fond `PANEL` (20,20,25) sur toute la largeur
  - Bordure basse `LINE` (42,42,53) 1px
  - Hauteur fixe `HEADER_H` = 44px
  - Padding horizontal 14px
  - Séparateurs verticaux `DIVIDER` entre les groupes (Brand / Sliders / Toggles)
  - Typographie : `INK` pour le brand, `FAINT` pour le build ID

---

## 2026-06-10 — Redesign UI Phase 1a — Fondations (build 20260610-202115)

**Build:** `20260610-202115`
**Commits:** UI : création des widgets custom (ToggleLED, ToggleSwitch, StyledButton, SegmentedControl) + intégration dans header et plock mode

### Changements
- **`src/ui/theme.rs`** — Tokens design (palette IBM Plex, rayons, gaps, strokes, helpers)
- **`src/ui/widgets.rs`** — Widgets custom :
  - `ToggleSwitch` : 34×18 r10, pastille coulissante
  - `ToggleLED` : pilule h26 r7 avec LED Ø7 et glow
  - `StyledButton` : bouton coordonné h26 r6
  - `SegmentedControl` : toggle groupé (Sound/Sequencer) avec retour d'index
- **`src/ui/engine_registry.rs`** — Registre des moteurs (Synth/Sample/MIDI Out) + groupes de paramètres
- **`src/ui.rs`** :
  - Intégration `ToggleLED` dans le header (Seq, Choke, Auto-Edit, Song)
  - Intégration `SegmentedControl` pour le mode Plock (Sound/Sequencer)
  - Fix imports `ParamSlider` direct depuis `nih_plug_egui::widgets`
- Fix `rect_stroke` 4 arguments (StrokeKind) pour egui 0.31.1

---

## 2026-06-10 — Fix boutons Export MIDI + Drag toujours visibles (build 20260610-085721)

**Build:** `20260610-085721`
**Commits:** UI : déplacement des boutons Export MIDI et Drag vers la Pattern Bank Bar

### Changes
- **[28] Drag & Drop MIDI** : les boutons **Export MIDI** et **Drag** ont été déplacés de la barre des presets (`draw_preset_bar`) vers la **Pattern Bank Bar** (`draw_pattern_bank`).
- **[28] Fix** : ces boutons étaient cachés quand le mode **Song** était activé, car le panel generator (qui contient la barre des presets) est remplacé par le Song Editor en mode Song. Maintenant ils sont toujours visibles.

### Validation
- `cargo test` : 90 tests lib + 61 tests standalone OK
- VST3 installé (copie manuelle car fichier verrouillé)

---

## 2026-06-10 — Plock Volume en haut du menu + TODO mise à jour (build 20260610-082223)

**Build:** `20260610-082223`
**Commits:** UI : Volume en premier dans le menu contextuel des plocks

### Changes
- **[75] Plock UI** : le slider `Volume` est maintenant affiché en haut du menu contextuel des plocks, juste après l'indicateur de mode (Link/Snapshot/Mixed).
- **[75] Plock UI** : le slider `Volume` n'est plus rendu dans la liste data-driven standard pour éviter le doublon.
- **TODO.md** : ajout de 5 nouvelles tâches priorisées :
  - **[91]** Sortir automatiquement du mode edit quand on sélectionne en dehors de la cellule (P1)
  - **[92]** Valeurs du menu plock sound par défaut = valeurs globales de l'instrument (P1)
  - **[93]** Investigation : son très écourté intéressant quand slider OSC maintenu (P2)
  - **[94]** Ajouter un paramètre pitch LFO sur les Toms (P2)
  - **[95]** Ajouter un instrument de type MIDI avec MIDI out (P2/P3)

### Validation
- `cargo test` : 90 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installé

---

## 2026-06-09 — Session : Lane Length lock/follow + volumes dB + taille VST fixe

**Session du 2026-06-09** — Builds : `20260609-152742`, `20260609-160617`, `20260609-162726`, `20260609-173417`, `20260609-184928`, `20260609-185930`

### Résumé de la session
- **[64] Lane Length** : implémentation finale du comportement lock/follow pour les longueurs de lane.
  - Par défaut, chaque lane suit `Pattern Length`.
  - Drag sur `Len` = verrouille la lane sur cette valeur (polyrythmie).
  - Si Pattern > valeur verrouillée → lane garde sa valeur.
  - Si Pattern ≤ valeur verrouillée → lane suit le pattern (trop court).
  - Clic droit = "Follow pattern length" pour déverrouiller.
  - Persistance DAW via `LaneLengthLocks` (`lane-locks-v1`).
  - Fix UI : la cellule `Len` affiche la valeur effective, pas la valeur stockée.
- **[75] Volumes en dB** : sliders de volume affichent `-inf dB` à `+6.0 dB`, stockage interne en gain linéaire `0..2`.
- **[89] Taille VST fixe** : fenêtre forcée à `1480×800`, scroll interne dans le Sound Editor.
- **Commits** : 3 commits sur la session (`4c5fccd`, `b500527`, `14bc83d`, `091d979`).

### Validation globale
- `cargo test` : 90 tests lib + 61 tests standalone OK (dernier build `20260609-185930`)
- `build.ps1 -Install` OK sur les builds finaux

---

## 2026-06-09 - Lane Length lock/follow v2 (build 20260609-185930)

**Build:** `20260609-185930`
**Commits:** Sequencer : lane length avec verrouillage — fix affichage effectif

### Changes
- **[64] Fix UI** : la cellule `Len` affiche maintenant la valeur **effective** (pas la valeur stockée). Quand pattern=48 et lane verrouillée à 50, l'UI affiche 48 (car pattern ≤ valeur verrouillée).
- **[64] Lane Length** : comportement inchangé :
  - **Par defaut** : suit `Pattern Length`.
  - **Drag la cellule `Len`** : verrouille sur cette valeur.
  - **Pattern > valeur verrouillee** : garde valeur (polyrythmie).
  - **Pattern <= valeur verrouillee** : suit pattern.
- **[64] Clic droit** : "Follow pattern length" pour déverrouiller.

### Validation
- `cargo test` : 90 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installé

---

## 2026-06-09 - Lane Length lock/follow (build 20260609-184928)

**Build:** `20260609-184928`
**Commits:** Sequencer : lane length avec verrouillage

### Changes
- **[64] Lane Length** : comportement final clarifie :
  - **Par defaut** : chaque lane suit `Pattern Length` (follow).
  - **Drag la cellule `Len`** : la lane se verrouille sur cette valeur (polyrythmie).
  - **Si Pattern > valeur verrouillee** : la lane garde sa valeur (ex: pattern 64, kick 12 → kick sur 12).
  - **Si Pattern <= valeur verrouillee** : la lane suit le pattern (ex: pattern 16, kick 32 → kick sur 16).
- **[64] Clic droit** : "Follow pattern length" pour deverrouiller une lane.
- **[64] Persistance** : `LaneLengthLocks` (masque `AtomicU16` persistant `lane-locks-v1`) conserve l'etat verrouille/deverrouille par session DAW.

### Validation
- `cargo test` : 90 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Lane Length clamp (build 20260609-173417)

**Build:** `20260609-173417`
**Commits:** Sequencer : simplification lane length — clamp au pattern length

### Changes
- **[64] Lane Length** : les longueurs par instrument sont simplement clampees au `Pattern Length` global. Si une lane fait `32` et le pattern passe a `16`, la lane passe a `16`.
- **[64] Suppression** : le systeme d'override persistant (`PersistentTrackLengthOverrides`) a ete retire. Plus de suivi complexe, plus de migration legacy.
- **[64] UI** : la cellule `Len` est un simple `DragValue` borne a `1..master_length`. Pas de menu contextuel, pas d'etat verrouille/deverrouille.

### Validation
- `cargo test` : 87 tests lib + 61 tests standalone OK
- VST3 installe (copie manuelle apres echec permission build.ps1)

---

## 2026-06-09 - Lane Length follow + override (build 20260609-162726)

**Build:** `20260609-162726`
**Commits:** Sequencer : lane length suit pattern length par defaut

### Changes
- **[64] Lane Length** : les longueurs par instrument suivent maintenant automatiquement `Pattern Length` tant qu'elles n'ont pas ete modifiees manuellement.
- **[64] Override manuel** : modifier une cellule `Len` pose un bit d'override persistant pour cette lane, y compris si la valeur choisie est `16` avec un pattern plus long.
- **[64] UI** : clic droit sur une cellule `Len` modifiee permet de revenir a `Follow pattern length`.
- **[64] Migration** : les anciennes sessions sans masque d'override conservent les lanes non-default (`Len != 16`) comme overrides manuels.

### Validation
- `cargo test` : 90 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Volumes instruments en dB (build 20260609-160617)

**Build:** `20260609-160617`
**Commits:** UI : affichage dB pour volumes instruments

### Changes
- **[75] Sound Editor** : le slider `Volume` affiche maintenant une valeur musicale en dB (`-inf dB` a `+6.0 dB`) au lieu du gain lineaire `0..2`.
- **[75] Grille** : les sliders `Vol` des lanes utilisent aussi une courbe dB, tout en stockant toujours le gain lineaire interne.
- **[75] UX** : double-clic sur un slider volume local reset a `0 dB` (unity gain), pas au milieu numerique de la range.

### Validation
- `cargo test` : 86 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Sound Editor Volume unique (build 20260609-152742)

**Build:** `20260609-152742`
**Commits:** UI : volume instrument unique dans Sound Editor

### Changes
- **[75] Sound Editor** : le champ `Volume` data-driven n'est plus rendu dans la section Output, pour eviter un deuxieme slider pour le meme instrument.
- **[75] Ranges** : les definitions internes `StandardField::Volume` passent de `0..1.5` a `0..2.0` pour rester coherentes avec le slider principal et les volumes de lane.
- **[75] UX** : le Sound Editor garde uniquement le slider `Volume` du haut.

### Validation
- `cargo test` : 86 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Taille editeur VST forcee (build 20260609-150545)

**Build:** `20260609-150545`
**Commits:** UI : taille editor forcee a 1480x800

### Changes
- **[89] UI Layout** : ajout d'un mode `fixed_size()` dans le wrapper `ResizableWindow` vendore.
- **[89] Studio One** : l'editeur demande maintenant explicitement `1480x800` meme si le host ou l'etat UI restaure une ancienne hauteur.
- **[89] Regression** : evite que l'ancien auto-resize par hauteur de contenu remonte la fenetre a `850px`.

### Validation
- `cargo test` : 86 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Hauteur editeur VST reduite (build 20260609-145809)

**Build:** `20260609-145809`
**Commits:** UI : hauteur editor fixee a 800 px

### Changes
- **[89] UI Layout** : taille initiale de l'editeur passee de `1480x850` a `1480x800`.
- **[89] Stabilite visuelle** : hauteur minimale de la `ResizableWindow` passee a `800px` avec `resizable(false)`.
- **[89] Sound Editor** : le scroll interne reste actif pour absorber les instruments avec beaucoup de parametres dans la hauteur reduite.

### Validation
- `cargo test` : 86 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Scroll interne Sound Editor (build 20260609-144118)

**Build:** `20260609-144118`
**Commits:** UI : scroll Sound Editor dans hauteur VST fixe

### Changes
- **[89] UI Layout** : le titre `Sound Editor` et les onglets instruments restent fixes dans la colonne droite.
- **[89] Sound Editor** : les controles de synthese sont maintenant enveloppes dans un `ScrollArea::vertical()` limite a la hauteur disponible.
- **[89] Stabilite visuelle** : les instruments avec beaucoup de parametres n'agrandissent plus la fenetre VST fixe `1480x850`.

### Validation
- `cargo test` : 86 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Hauteur editeur VST fixe (build 20260609-141438)

**Build:** `20260609-141438`
**Commits:** UI : hauteur editor fixee a 850 px

### Changes
- **[89] UI Layout** : taille initiale de l'editeur passee de `1480x520` a `1480x850`.
- **[89] Stabilite visuelle** : hauteur minimale de la `ResizableWindow` passee a `850px` avec `resizable(false)` pour eviter les sauts lors des changements d'instruments.

### Validation
- `cargo check`
- `cargo test` : 86 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Step Fusion selection blink (build 20260609-124302)

**Build:** `20260609-124302`
**Commits:** UI Step Fusion : cellule source de selection clignotante

### Changes
- **[87] Fusion Mode** : la premiere cellule selectionnee par Maj+clic apparait maintenant comme une cellule active temporaire (`X` + fond bleu).
- **[87] Visibilite** : la cellule source clignote entre le fond actif bleu et son fond normal, avec une bordure bleue.
- **[87] Interaction** : relacher Maj annule toujours la selection temporaire et restaure l'affichage normal.

### Validation
- `cargo check`
- `cargo test` : 86 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Step Fusion selection highlight (build 20260609-121512)

**Build:** `20260609-121512`
**Commits:** UI Step Fusion : highlight de cellule source pendant selection

### Changes
- **[87] Fusion Mode** : apres le premier Maj+clic d'une creation de fusion, le point central de la cellule source devient bleu.
- **[87] Interaction** : si Maj est relachee avant la deuxieme cellule, la selection temporaire est annulee et le point reprend sa couleur normale.
- **[87] UI** : le highlight reutilise l'etat existant `fusion_selection_start`, sans changer le scheduling audio ni les donnees de fusion.

### Validation
- `cargo check`
- `cargo test` : 86 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Master Volume crash fix (build 20260609-114803)

**Build:** `20260609-114803`
**Commits:** Audio params : smoothing master volume compatible silence

### Changes
- **[88] Crash Studio One** : correction du lissage du slider `Master Volume` en haut a gauche.
- **[88] Audio** : remplacement de `SmoothingStyle::Logarithmic(50.0)` par `SmoothingStyle::Exponential(50.0)`, car le range du gain master inclut `0.0` (`-inf dB`).
- **[88] Regression test** : ajout d'un test verifiant que le smoothing du master volume reste fini depuis le silence.

### Validation
- `cargo check`
- `cargo test` : 86 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Step Fusion double-clic edition (build 20260609-112628)

**Build:** `20260609-112628`
**Commits:** UI Step Fusion : double-clic fusion traite avant le toggle simple

### Changes
- **[87] Interaction** : le double-clic sur une cellule fusionnee ouvre l'edition Fusion avant la logique de clic simple.
- **[87] Regression** : evite que le premier clic du double-clic desactive la cellule source de la fusion.
- **[87] UX** : conserve le clic simple immediat, sans retour au mecanisme de toggle differe/pending toggle rejete.

### Validation
- `cargo check`
- `cargo test` : 85 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Revert toggle differe Step Fusion (build 20260609-105752)

**Build:** `20260609-105752`
**Commits:** UI Step Fusion : retrait du pending toggle du double-clic

### Changes
- **[87] Revert** : suppression du mecanisme de toggle differe ajoute au build `20260609-104936`.
- **[87] Interaction** : retour au comportement immediat precedent pour le clic sur une cellule fusionnee.
- **[87] Suivi** : retrait de l'entree TODO du build rejete `20260609-104936`.

### Validation
- `cargo check`
- `cargo test` : 85 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Step Fusion edit box reservee (build 20260609-102249)

**Build:** `20260609-102249`
**Commits:** UI Step Fusion : panneau d'edition dans une box reservee stable

### Changes
- **[87] Fusion box** : le panneau `Fusion x-y (cells) Steps ... Delete Close` est maintenant dessine dans une boite Fusion fixe sous la grille, a cote du mode plock/fusion.
- **[87] Layout stable** : la boite Fusion est toujours reservee ; son apparition/disparition ne decale plus l'interface.
- **[87] Edition** : cliquer sur le champ `Steps` de la boite ne ferme plus immediatement le mode edition.
- **[87] Clic exterieur** : pendant l'edition, les clics sur la grille sont neutralises ; un clic hors cellule inline et hors boite Fusion ferme l'edition et garde la cellule source active.

### Validation
- `cargo check`
- `cargo test` : 85 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Step Fusion edition inline stable (build 20260609-100205)

**Build:** `20260609-100205`
**Commits:** UI Step Fusion : edition inline contrainte et sortie par clic exterieur

### Changes
- **[87] Edition inline** : le champ `DragValue` du nombre de pulses remplace maintenant le bouton fusionne avec exactement la meme taille, au lieu d'etre dessine en overlay ; la ligne ne se decale plus pendant l'edition.
- **[87] Clic exterieur** : un clic hors de la cellule fusionnee quitte le mode edition et remet la cellule source en mode normal actif.
- **[87] Clavier** : `Enter` et `Escape` quittent aussi le mode edition.

### Validation
- `cargo check`
- `cargo test` : 85 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Focus Studio One menus avec VST ouvert (build 20260609-094555)

**Build:** `20260609-094555`
**Commits:** Vendor nih-plug-egui : focus clavier Windows non-intrusif

### Changes
- **Windows/Studio One** : le workaround clavier ne force plus `SetFocus(plugin)` a chaque frame quand egui ne saisit pas de texte.
- **Focus host** : `set_keyboard_focus()` ne refocalise le VST que si le focus ou le curseur est deja dans l'editeur, ce qui laisse les menus de Studio One s'ouvrir pendant que le VST est visible.
- **Saisie texte plugin** : la redirection vers la message window reste active quand egui veut une saisie clavier.

### Validation
- `cargo check`
- `cargo test` : 85 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-08 - Step Fusion Copy/Paste Page + x2 (build 20260608-195857)

**Build:** `20260608-195857`
**Commits:** UI : copie des fusions dans les operations de page et de duplication

### Changes
- **[87] Copy Page** : le clipboard de page embarque maintenant les groupes Step Fusion par instrument, avec start/end locaux et nombre de pulses.
- **[87] Paste Page** : remappe les fusions vers la page cible, remplace les anciennes fusions de cette page, conserve l'etat ON/OFF de la cellule de depart et nettoie les plocks couverts.
- **[87] x2** : duplique les groupes Step Fusion avec offset `current_len`, en respectant les limites page-locales.
- **[87] Clear Page** : supprime aussi les fusions de la page videe.

### Validation
- `cargo check`
- `cargo test` : 85 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-08 - Step Fusion Shift detection robuste (build 20260608-194139)

**Build:** `20260608-194139`
**Commits:** UI : detection Maj stable via Win32 GetAsyncKeyState

### Changes
- **[87] Fusion Mode** : l'indicateur `Maj for fusion mode` et le Shift+clic utilisent maintenant une detection centralisee.
- **[87] Windows/Studio One** : ajout d'un fallback Win32 `GetAsyncKeyState()` pour lire l'etat reel de Maj gauche/droite, car les modifiers egui peuvent etre aleatoires dans un VST selon le focus clavier du host.
- **[87] Comportement** : la couleur bleue de l'indicateur et la creation de fusion reposent sur la meme detection.

---

## 2026-06-08 - Step Fusion mode indicator (build 20260608-193357)

**Build:** `20260608-193357`
**Commits:** UI : indicateur Maj pour mode fusion

### Changes
- **[87] UI Fusion Mode** : ajout d'une section a droite du mode plock sous la grille.
- **[87] Feedback clavier** : affiche `Maj for fusion mode` en gris au repos, puis en bleu quand Maj/Shift est maintenu.
- **[87] Guidance** : quand Maj est actif, affiche aussi `Select 2 cells` pour clarifier la creation de fusion.

---

## 2026-06-08 - Step Fusion nettoie les plocks couverts (build 20260608-192613)

**Build:** `20260608-192613`
**Commits:** Step Fusion : suppression des plocks sous cellules couvertes

### Changes
- **[87] Creation de fusion** : lors d'une fusion, les plocks sound et sequencer des cellules internes couvertes sont supprimes.
- **[87] Source unique** : les plocks de la cellule de depart sont conserves, car c'est la seule cellule source lue par l'audio pour la fusion.
- **[87] UX** : evite les plocks caches/inactifs sous une fusion qui pourraient reapparaitre de facon confuse apres suppression de la fusion.

---

## 2026-06-08 - Step Fusion UX polish (build 20260608-191352)

**Build:** `20260608-191352`
**Commits:** Step Fusion : style standard, edition inline, activation par defaut

### Changes
- **[87] Style cellule fusionnee** : le bloc fusionne reprend les couleurs des cellules standard (active, plock, seq-plock, current) au lieu d'utiliser un style bleu/cyan dedie.
- **[87] Edition inline** : double-clic sur une fusion ouvre maintenant l'edition du nombre de pulses directement dans la cellule ; le menu contextuel propose aussi "Edit Fusion Steps".
- **[87] Interaction** : le double-clic n'active/desactive plus accidentellement la fusion avant d'entrer en edition.
- **[87] Creation** : une fusion nouvellement creee est activee par defaut sur sa cellule source.

---

## 2026-06-08 - Step Fusion UI: vraie fusion graphique (build 20260608-190515)

**Build:** `20260608-190515`
**Commits:** Rendu UI Step Fusion en bloc continu

### Changes
- **[87] Step Fusion UI** : les cellules fusionnees sont maintenant rendues comme un seul widget large couvrant toute la plage fusionnee.
- **[87] Alignement grille** : la largeur du bloc fusionne inclut les espacements internes des cellules remplacees, ce qui garde les colonnes suivantes alignees avec la grille 16 pas.
- **[87] Interaction** : le clic, double-clic et menu contextuel restent portes par la fusion, en utilisant la cellule de depart comme source des triggers/plocks.

---

## 2026-06-07 — Step Fusion V2 from scratch (build 20260607-131747)

**Build:** `20260607-131747`
**Commits:** Refonte Step Fusion : grille fixe + pulses audio + stutter seq-plock désactivé

### Changes
- **[87] Step Fusion V2 — grille fixe** : suppression du rendu en bouton large qui supprimait des colonnes et décalait la grid. Les 16 cellules de page restent toujours rendues avec une largeur fixe.
- **[87] Step Fusion V2 — vrai scheduling audio** : une fusion active déclenche `N` pulses régulièrement espacés sur la durée des cellules fusionnées, au lieu de remapper les steps vers d'autres cellules.
- **[87] Step Fusion V2 — source unique** : la cellule de départ porte l'état ON/OFF et les plocks sonores ; les cellules internes ne déclenchent plus indépendamment.
- **[87] Step Fusion V2 — page-local only** : les fusions qui traversent une page 16-step sont rejetées.
- **[87] Step Fusion V2 — stutter seq-plock désactivé** : le stutter est ignoré côté audio sur une fusion, et l'UI du plock séquenceur l'affiche comme indisponible.
- **[87] Temps réel** : sync fusion UI→audio via buffer fixe préalloué (`load_fusions_into`) ; plus de `Vec` alloué dans `process()` pour les fusions.
- **Tests** : ajout de tests séquenceur pour filtrage des fusions invalides et suppression des triggers internes avec métadonnées de pulses.

---

## 2026-06-07 — Step Fusion fixes : audio, 1-cell, visual overflow (build 20260607-103006)

**Build:** `20260607-103006`
**Commits:** Correction audio + UI des cellules fusionnées

### Changes
- **[87] Step Fusion — audio fix** : `map_step_to_cell` prend maintenant `track_length` et fait une recherche modulaire. Une fusion sur les cellules globales 16-19 avec `track_length=16` s'applique correctement aux steps 0-3 (car 16≡0 mod 16).
- **[87] Step Fusion — single-cell filter** : `set_fusions` rejette les fusions à 1 cellule (pas d'effet rythmique).
- **[87] Step Fusion — UI clamp to page** : le rendu des fusions est coupé à la fin de la page courante (pas de débordement visuel). Les cellules appartenant à une fusion commencée avant la page sont sautées.
- **[87] Step Fusion — UI no 1-cell creation** : Shift+clic sur une seule cellule ne crée plus de fusion.

---

## 2026-06-05 — Session Pattern Bank : plocks, Clear, Generate, Presets (build 20260605-180924)

**Build:** `20260605-180924`
**Commits:** Stabilisation complète Pattern Bank + UX Clear + Generate 64 steps + Presets étendus

### Changes
- **[85] Crash retour P1** fixé : `MAX_PLOCK_BYTES` calcul dynamique depuis `FIELD_COUNT`/`INSTRUMENT_COUNT`/`STEP_COUNT` (plus de hardcode 18 fields)
- **[86] Plocks résiduels au changement de slot** fixé : `clear_all()` + détection auto format legacy (18 vs 46 fields) dans `restore_from_buffers()`
- **Bouton Clear** : déplacé après les slots P1-P8, confirmation 2 étapes ("Clr" → "Sure?" rouge clignotant), annulation auto sur clic Save/slot
- **Suppression** du bouton "Clear" de la section Generator (doublon)
- **Plocks effacés** automatiquement sur : preset (Rock/Funk/Disco), Random, Generate
- **Presets Rock/Funk/Disco** étendus sur 64 steps (répétition bar-by-bar)
- **Generator tiling** : Probabilistic/Markov/Classic répètent le motif 16-step sur 4 bars (64 steps). Euclidean inchangé (déjà 64 steps)
- **Generator respecte `pattern_length`** : steps au-delà de la longueur sont effacés après génération

---

## 2026-06-05 — Fix Clear + confirmation deux étapes [58]

**Build:** `20260605-124507`
**Commits:** Correction du bouton Clear qui ne vidait pas la grille + ajout confirmation deux étapes

### Changes
- **Fix** : le bouton "Clr" vidait les plocks mais pas la grille (step masks) — il appelle maintenant `load_pattern_for_ui(pattern, &Pattern::empty())` pour vider aussi la grille
- **Confirmation deux étapes** : premier clic sur "Clr" affiche "Sure?" en rouge clignotant, deuxième clic confirme le clear
- **Annulation auto** : le mode confirm est annulé si on clique sur Save, un slot P1-P8, ou ailleurs
- **Bouton "Clr"** déplacé à droite des slots P1-P8 pour un flux de travail cohérent (Save → Slots → Clear)
- **Suppression** du bouton "Clear" de la section Generator qui faisait doublon

---

## 2026-06-05 — Déplacement du bouton Clear dans la Pattern Bank [58]

**Build:** `20260605-123720`
**Commits:** UI — bouton "Clr" déplacé après les slots P1-P8, suppression du "Clear" de la section Generator

### Changes
- **Bouton "Clr"** déplacé à droite des slots P1-P8 pour un flux de travail cohérent (Save → Slots → Clear)
- **Suppression** du bouton "Clear" de la section Generator qui faisait doublon avec le bouton "Clr" de la Pattern Bank
- Le bouton "Clr" vide les plocks sound + sequencer directement depuis l'UI thread

---

## 2026-06-05 — Fix plocks liés au pattern bank (clear + restore + legacy format) [58]

**Build:** `20260605-094135`
**Commits:** Correction plocks qui restaient du pattern précédent au changement de slot

### Changes
- **Problème** : quand on chargeait un pattern depuis la bank, les plocks du pattern précédent restaient visibles
- **Cause** : `restore_from_buffers()` skipait le restore si `plock_bytes.len()` < `expected_plock_size` (calculé avec `FIELD_COUNT=46`). Les slots sauvegardés avant le passage de `FIELD_COUNT` de 18 à 46 avaient des données trop courtes
- **Fix** :
  - `restore_from_buffers()` et `PatternSlot::restore()` détectent automatiquement le format (18 vs 46 fields) depuis la taille des données
  - `PlockState::clear_all()` et `SequencerPlockState::clear_all()` : vident tous les plocks avant restauration pour éviter les résiduels
  - `load_pattern_from_slot()` appelle `clear_all()` sur les plocks sound et sequencer avant `restore_from_buffers()`
- **Rétrocompatibilité** : les anciens slots (FIELD_COUNT=18) sont correctement restaurés

---

## 2026-06-05 — UI Pattern Bank : slot vide plus sombre + save positionne + header cleanup [58]

**Build:** `20260605-092903`
**Commits:** Améliorations UX Pattern Bank + cleanup header

### Changes
- Slot **vide** : fond `rgb(16, 16, 22)` + bordure `rgb(40, 40, 50)` (beaucoup plus sombre)
- Slot **enregistré non lu** : inchangé `rgb(48, 48, 58)`
- **Save positionne le slot** : après sauvegarde, le slot est automatiquement marqué comme "chargé" (vert)
  - `save_pattern_to_slot()` met à jour `audio_last_loaded_slot`
  - Le slot sauvegardé s'affiche en vert dans l'UI
- **Header bar cleanup** : suppression du bouton play (non fonctionnel) et de l'affichage BPM
  - Gardé : Master Volume, Swing, Groove, toggles (Seq, Choke, Auto-Edit, Song)

---

## 2026-06-05 — Fix crash [85] : buffer overflow dans copy_data_for_restore() [58]

**Build:** `20260605-090814`
**Commits:** Correction crash retour P1 — `MAX_PLOCK_BYTES` under-allocatait de 2.4x

### Changes
- **Cause racine identifiée**
  - `MAX_PLOCK_BYTES` utilisait `18` (ancienne valeur de `FIELD_COUNT`) au lieu de `46` (valeur actuelle)
  - Calcul incorrect : 66 664 bytes alloués vs 159 848 bytes écrits par `capture()`
  - `copy_data_for_restore()` copiait 159 848 bytes dans un buffer de 66 664 → panic/crash Studio One
- **Fix**
  - `MAX_PLOCK_BYTES` et `MAX_SEQ_PLOCK_BYTES` calculés dynamiquement depuis `FIELD_COUNT`, `INSTRUMENT_COUNT`, `STEP_COUNT`
  - Plus de hardcode — les constantes suivent automatiquement les évolutions du modèle de données
  - `copy_data_for_restore()` protégé par `.min()` pour éviter tout overflow futur
- **Tests**
  - 82 tests passent, tests pattern bank validés

---

## 2026-06-04 — Fix race condition Pattern Bank (mutex lock + divergence last_loaded_slot) [58]

**Build:** `20260604-200117`
**Commits:** Correction race condition pattern bank — grid bloqué sur P2 après switch rapide

### Changes
- **Réduction temps de verrou audio thread**
  - `PatternSlot::copy_data_for_restore()` : copie les données du slot sous le lock (court)
  - `restore_from_buffers()` : restauration lock-free depuis des buffers temporaires
  - `load_pattern_from_slot()` ne tient plus le mutex `pattern_bank` pendant le restore (qui touche des milliers d'atomics)
- **Synchronisation `last_loaded_slot` audio→UI**
  - Nouvel atomic `audio_last_loaded_slot` mis à jour par l'audio thread après chaque `load_pattern_from_slot`
  - L'UI thread lit cet atomic à chaque frame et synchronise `state.last_loaded_slot`
  - Élimine la divergence entre l'affichage (UI) et l'état réel (audio) quand on clique rapidement
- **Buffers préalloués**
  - `temp_plock_bytes: [u8; MAX_PLOCK_BYTES]` et `temp_seq_plock_bytes` dans `DrumFlashVst`
  - Zéro allocation dans l'audio thread pendant le restore

---

## 2026-06-04 — Refonte UX Pattern Bank v2 (Save mode 2 étapes + indicateurs dirty/actif) [58]

**Build:** `20260604-193124`
**Commits:** Finalisation UX pattern bank — save/load explicites, indicateurs d'état, position sous grille

### Changes
- **Nouvelle position** : Pattern Bank sous la grille, au-dessus du générateur
- **Interaction Save à 2 étapes**
  - Bouton **"Save"** : clic pour activer le mode save (clignote), puis clic sur un slot P1-P8 pour sauvegarder
  - Désactive le mode save après sauvegarde
- **Click direct sur slot = Load**
  - Slot occupé : charge immédiatement le pattern dans la grille
  - Slot vide : rien (pas de chargement)
- **Indicateurs d'état**
  - Cercle **vert** sur le slot actuellement chargé (`last_loaded_slot`)
  - Étoile `*` sur slot si pattern modifié depuis le dernier load/save (dirty detection)
- **Reset indicateurs**
  - Presets (Rock/Funk/Disco/Clear/Random) et Generate resettent `last_loaded_slot = None`
  - Le pattern n'est plus lié au bank après modification via preset/generate
- **Synchro `pattern_length` audio→UI**
  - `pending_pattern_length: Arc<AtomicI32>` notifie l'UI thread qui applique via `setter.set_parameter()`

---

## 2026-06-04 — Refonte UX Pattern Bank (boutons Save/Load explicites) [58]

**Build:** `20260604-175459`
**Commits:** Correction de l'UX pattern bank — interactions confuses remplacées par des boutons explicites

### Changes
- **Nouvelle interaction Pattern Bank**
  - P1-P8 = simples sélecteurs de slot
  - Bouton **"Save"** explicite : sauvegarde le pattern courant dans le slot sélectionné
  - Bouton **"Load"** explicite : charge le pattern du slot sélectionné (grisé si vide)
- **Indicateurs visuels clairs**
  - Slot occupé = petit point vert + bordure verte
  - Slot sélectionné = fond bleu
  - Slot vide = fond gris foncé
- **Tooltips explicites** au survol de chaque élément
- **Bugfix : `pattern_length` se met à jour au load**
  - L'audio thread notifie l'UI via `pending_pattern_length` atomic
  - L'UI thread applique la valeur via `setter.set_parameter()`
  - Auparavant, charger un pattern de 32 steps dans un contexte de 16 steps laissait le slider bloqué

---

## 2026-06-04 — Stabilisation Pattern Bank (pas d'alloc audio thread + pas de panic) [58]

**Build:** `20260604-170429`

### Changes
- **`PatternSlot::default()` préalloue les buffers**
  - `capture()` utilise `clear()` + `extend_from_slice()` — zéro allocation dans l'audio thread
- **`load_pattern_from_slot()` sans `.unwrap()`** sur le mutex
- **Tests unitaires ajoutés** : capture/restore roundtrip, préallocation, persistance song

---

## 2026-06-04 — Song Mode (chaînage patterns P1-P8) [58]

**Build:** `20260604-164354`
**Commits:** Implémentation du song mode — chaînage séquentiel des patterns

### Changes
- **Nouveau paramètre `song_mode` (BoolParam, ID: `song_mode`)**
  - Default: `false` (pattern unique en boucle — comportement existant)
  - Quand activé: le séquenceur avance automatiquement dans la séquence de patterns
- **Structure `SongSequence` dans `PatternBank`**
  - 64 steps max, chaque step référence un slot P1-P8 (ou vide `-1`)
  - `length`: nombre de steps actifs
  - `loop_enabled`: boucle la séquence à la fin
  - Persistance DAW via le champ existant `pattern-bank-v1`
- **Logique de playback dans `process()`**
  - Détection du wrap de pattern via `loop_count` du séquenceur
  - Au wrap: avance `song_position`, charge le pattern du slot suivant
  - Si fin de séquence et `loop_enabled`: retour au step 0
- **UI: Toggle "Song" dans la header bar**
  - Checkbox à côté des autres toggles (Seq, Choke, Auto-Edit)
  - Quand Song mode actif: le panel generator est remplacé par l'éditeur de séquence
- **Song Editor UI**
  - Grille horizontale de steps (16 par ligne)
  - Click sur un step: cycle P1 → P2 → ... → P8 → vide
  - Right-click: efface le step
  - Bouton "Loop": toggle boucle
  - Contrôles "Len +/-": ajuste la longueur de la séquence
  - Highlight rouge sur le step en cours de lecture

---

## 2026-06-04 — Désactivation séquenceur interne / Mode MIDI thru [60]

**Build:** `20260604-141711`
**Commits:** Implémentation du mode MIDI thru pour pilotage DAW

### Changes
- **Nouveau paramètre `use_internal_sequencer` (BoolParam, ID: `int_seq`)**
  - Default: `true` (séquenceur interne actif — comportement existant)
  - Quand désactivé: le plugin ne génère plus de triggers depuis le séquenceur interne
  - Le plugin passe en mode "MIDI thru": les notes MIDI reçues déclenchent les instruments
- **Mapping MIDI note → voix**
  - Fonction `instrument_registry::voice_idx_from_midi_note(note: u8) -> Option<usize>`
  - Mappe les notes MIDI standards (GM Drum Map) aux 13 voix du plugin
  - Kick=36, Snare=38, HiHat=42, OpenHH=46, Tom1=50, Tom2=47, Tom3=43, Clap=39, Ride=51, Cymbal=49, Snare606=40, B8=35, Perc1=37
- **Traitement des événements MIDI entrants dans `process()`**
  - NoteOn reçu → lookup de la voix correspondante → `trigger()` avec velocity MIDI
  - Hi-hat choke open hi-hat respecté aussi en mode MIDI
  - Les événements MIDI sont forwardés à la sortie (channel 9) comme en mode séquenceur
  - Le test panel (bouton T) continue de fonctionner en mode MIDI
- **UI: Toggle "Seq" dans la header bar**
  - Checkbox à côté de "Choke" et "Auto-Edit"
  - Label court pour ne pas surcharger la barre

---

## 2026-06-04 — Fix trigger_hard() remet active=true (stutter machine-gun)

**Build:** `20260604-130503`
**Commits:** Fix trigger_hard() manquait self.active = true sur toutes les voix

### Changes
- **Bugfix critique : `trigger_hard()` ne remettait pas `self.active = true`**
  - Quand l'enveloppe atteignait 0 entre deux stutters, la voix devenait inactive
  - Les coups suivants du stutter étaient muets → un seul long son au lieu de coups distincts
  - Fix appliqué sur les 11 voix : Kick, Snare, HiHat, OpenHiHat, Tom, Clap, Ride, Cymbal, Snare606, BassDrum808, Perc1
  - Chaque `trigger_hard()` commence maintenant par `self.active = true` avant de hard-retrigger l'enveloppe

---

## 2026-06-04 — trigger_hard() machine-gun retrigger chain

**Build:** `20260604-102713`
**Commits:** Ajout trigger_hard() sur toute la chaîne de voix

### Changes
- **Ajout de `trigger_hard()` pour les répétitions stutter en "machine gun"**
  - `ExpDecayEnvelope::trigger_from_zero()` — redémarre l'enveloppe depuis zéro avec une rampe d'attaque complète
  - `DecayReleaseEnvelope::trigger_hard()` — hard-retrigger des deux stages (decay + release)
  - `Voice::trigger_hard()` — méthode par défaut qui fallback sur `trigger()`
  - `DrumVoiceKind::trigger_hard()` — dispatch vers chaque voix concrète
  - `DrumSynthesizer::trigger_hard()` — API publique pour le séquenceur
  - Implémentations par voix : Kick, Snare, HiHat, OpenHiHat, Tom, Clap, Ride, Cymbal, Snare606, BassDrum808, Perc1
  - Seule l'enveloppe d'amplitude est hard-reset ; les autres états (pitch, filtres) restent continus

---

## 2026-06-04 — Stutter max 16 + espacement BPM-sync

**Build:** `20260604-100501`
**Commits:** Ajustements stutter après retour utilisateur

### Changes
- **Stutter max augmenté de 8 à 16**
  - Slider UI : `1..=16` au lieu de `1..=8`
  - Commentaire struct mis à jour (`1-16`)
- **Espacement stutter recalculé proportionnellement au step**
  - `spacing = step_duration / stutter` (revert du hardcodé `step/4`)
  - Le step_duration est dérivé du BPM du DAW : `sample_rate * 60 / (bpm * 4)`
  - x2 = 2 coups sur le step, x4 = 4 coups, x8 = 8 coups, x16 = 16 coups
  - L'espacement s'adapte automatiquement au tempo du projet

---

## 2026-06-04 — Fix UI conditions Plocks Séquenceur

**Build:** `20260604-092857`
**Commits:** Correction interactive après retour test utilisateur

### Changes
- **Fix ComboBox condition qui revenait sur `Always`**
  - Suppression complète du `ComboBox` imbriqué dans le menu contextuel
  - Remplacement par une grille de boutons/radios visible directement dans le menu
  - Chaque option appelle directement `set_condition()` dans le handler `.clicked()`
  - Le bouton `Create Seq Plock` n'apparaît plus comme état inactif après une modification dans le même frame, ce qui évite d'écraser la sélection par `SequencerStepParams::default()`
- **Sécurisation atomique du `SequencerPlockState`**
  - `set_active()` utilise maintenant `fetch_or` / `fetch_and` au lieu d'un cycle load/store
  - Lectures des champs sequencer plock en `Acquire`, écritures en `Release`
- **Tests ajoutés**
  - `sequencer_step_params_default_is_playable`
  - `sequencer_condition_setter_roundtrips`

---

## 2026-06-03 — Fix Plocks Séquenceur: defaults, conditions, stutter spacing

**Build:** `20260603-211721`
**Commits:** Corrections bugs [59] Plocks Séquenceur

### Changes
- **Fix `SequencerStepParams::default()`**
  - `probability` = `1.0` (was `0.0` — new seq-plocks were silent by default)
  - `stutter_count` = `1` (was `0` — caused no-trigger)
  - `condition` = `Always`, `microtiming_ms` = `0.0`
- **Retrait Fill / NotFill de `StepCondition`**
  - Supprimés de l'enum, du label, de `all()`, des match arms `lib.rs` et persistance
  - Seuls `First` / `NotFirst` restent comme conditions de loop
- **Fix Stutter: espacement temporel entre triggers**
  - `pending_stutters` : file fixe 128 slots `(samples_until, voice_idx, velocity, step)`
  - Chaque trigger stutter est espacé de `step_duration / stutter_count` samples
  - Évite l'écrasement de tous les triggers au même `sample_idx`
  - `fire_voice_trigger()` helper extrait pour uniformiser audio + MIDI

---

## 2026-06-03 — Revue code: Plocks 64 steps, Export MIDI, Atomics, NoteOff timing

**Build:** `20260603-095833`
**Commits:** Revue de code et corrections post-revue

### Changes
- **Plocks: support complet des 64 steps**
  - `PlockMasks` passe de `AtomicU16` à `AtomicU64` (masque d'activation par instrument)
  - Persistance `plock-v1` rétrocompatible : détection auto ancien format (masques u16) vs nouveau (u64)
  - Tests ajoutés : `plock_supports_steps_16_to_63`, `plock_persistence_roundtrips_step_63`
- **Export MIDI: respecte la longueur du pattern**
  - `export_pattern_to_midi_data()` accepte `pattern_length` (1-64 steps)
  - Boutons Export MIDI et Drag passent la longueur courante
  - Test ajouté : `midi_export_includes_steps_beyond_first_page`
- **Fix NoteOff timing hors buffer**
  - `NoteOff` envoyé à `sample_idx` au lieu de `sample_idx + 1` pour éviter un offset égal à la taille du buffer
- **Sécurisation atomics UI → audio**
  - `bump_version()` : `Release` au lieu de `Relaxed`
  - Lecture version côté audio : `Acquire` au lieu de `Relaxed`
  - `PlockMasks.set_active()` : `Release`, `is_active()` : `Acquire`
  - `voice_test_triggers` : `Release` en UI, `Acquire` en audio
- **Nettoyage**
  - `Cargo.toml` : description corrigée "64-step drum sequencer"
  - `ui.rs` : suppression du double `algo` et commentaire dupliqué
  - Section "Dev: Preset Dumps" masquée en build release (`cfg!(debug_assertions)`)

---

## 2026-06-03 — Plocks Séquenceur: Phases 1-5 (Probabilité + Stutter + Conditions + UI)

**Build:** `20260603-205246`
**Commits:** Architecture séquenceur plock complète avec probabilité, stutter, conditions et UI couleurs

### Changes
- **Nouveau système: Sequencer Plocks** (`TODO.md` [59] Phases 1-5)
  - `SequencerPlockState` : stockage lock-free par step × instrument (4 paramètres)
  - `StepCondition` enum : Always / 1st loop / Not 1st / 1/2, 2/2 / 1/3, 2/3, 3/3 / 1/4, 2/4, 3/4, 4/4 / Fill / Not Fill
  - `SequencerStepParams` : probability (0-100%), stutter_count (1-8), condition, microtiming_ms (±50ms)
  - Persistance DAW via `PersistentSequencerPlockState` (champ `seq-plock-v1`)
- **Probabilité (Phase 1)**
  - Slider 0-100% dans le menu contextuel mode "Sequencer"
  - Skip aléatoire dans le callback audio (`next_rand()` LCG)
  - Par défaut 100% (pas de changement de comportement)
- **Stutter (Phase 4)**
  - Slider 1-8x dans le menu contextuel séquenceur
  - Déclenche multiple fois le son sur le même step
- **Conditions (Phase 5)**
  - Combobox avec toutes les conditions dans le menu contextuel
  - Filtrage dans le callback audio basé sur `loop_count`
  - Fonctionne sur le nombre de boucles du pattern
- **UI**
  - Label "Plock mode:" avant le switch
  - Switch "Sound / Sequencer" sous la grille avec couleur adaptative (orange = Sound, violet = Sequencer)
  - Menu contextuel adaptatif : Sound → plocks instruments, Seq → plocks séquenceur
  - Bouton "Create Seq Plock" / "Clear Seq Plock"
- **Couleurs par mode**
  - Mode Sound : plocks instruments en rouge/orange (inchangé)
  - Mode Sequencer : plocks séquenceur en violet (#9333EA) visibles uniquement en mode Seq
  - Les steps affichent les couleurs correspondant au mode actif uniquement

---

## 2026-06-03 — Archivage PoC web + Documentation + Cleanup labels UI

**Build:** `20260603-171338`
**Commits:** Archivage PoC web, création docs infrastructure/utilisateur, cleanup labels

### Changes
- **Archivage du PoC web**
  - Déplacement de `index.html` et `index.js` vers `archive/web-poc/`
  - Le plugin VST3 est désormais le seul produit actif
  - README.md mis à jour avec la nouvelle structure du repo
- **Documentation**
  - `docs/infrastructure.md` créé — guide build, architecture technique, tests, déploiement
  - `docs/user-guide.md` créé — guide utilisateur complet (UI, plocks, export, multi-out, analog)
- **Cleanup labels UI**
  - Suppression des préfixes `--` devant "Link to global", "Linked to global", "Mixed"
  - Correction "Snapshot Snapshot" → "Snapshot"
  - Correction "Random Random" → "Random"
  - Correction `Dump failed: { }` → `Dump failed: {}`

---

## 2026-06-03 — Copier/coller de plock + cleanup labels UI

**Build:** `20260603-145433`
**Commits:** Plock copy/paste + audit et correction des labels UI

### Changes
- **Copier/coller de plock individuel** (`TODO.md` [61b])
  - Bouton "Copy Plock" dans le menu contextuel d'une step avec plock existant
  - Bouton "Paste Plock" disponible quand le clipboard contient un plock du même instrument
  - Stockage dans `EditorUIState.plock_clipboard` (`SinglePlockClipboard`)
  - Le collage écrase le plock existant sur la step cible
  - Protection multi-instrument : on ne colle que si l'instrument correspond
  - Disponible à la fois dans le mode "création" (step sans plock) et "édition" (step avec plock)
- **Cleanup labels UI**
  - Suppression des préfixes `--` devant "Link to global", "Linked to global", "Mixed"
  - Correction "Snapshot Snapshot current settings" → "Snapshot current settings"
  - Correction "Random Random" → "Random"
  - Correction "-' Clear plock" → "Clear plock"
  - Correction format string `Dump failed: { }` → `Dump failed: {}`

---

## 2026-06-03 — Slider Analog: défaut 0.3 sur instruments opérationnels

**Build:** `20260603-121232`
**Commits:** Post-revue — Valeur par défaut du slider Analog

### Changes
- **Slider "Analog" passe à 0.3 par défaut sur 7 instruments**
  - Kick, Snare, Tom1, Tom2, Tom3, Cymbal, BassDrum808
  - Correspond au drift opérationnel (pas une alternance binaire)
  - Les instruments avec analog fixé/inactif restent à 1.0 (HiHat, OpenHiHat, Clap, Ride, Snare606, Zap)
- **`instrument_registry.rs`** : `sound_settings_default()` retourne `analog: 0.3` pour les 7 instruments concernés
- **`synthesis/mod.rs`** : `VoiceSettings::default()` et `default_for_instrument()` alignés sur 0.3
- **`ADDING_AN_INSTRUMENT.md`** : convention Analog ajoutée — 0.3 si opérationnel, 1.0 si fixé/inactif
- **135 tests passent**, build VST3 installé

---

## 2026-06-02 — UI: Layout page navigation + x2 + LED lecture

**Build:** `20260602-202855`
**Commits:** À venir

### Changes
- **Slider "Len" déplacé vers la ligne des pages**
  - Retiré du header bar, positionné à côté des boutons 1-4
  - Plus logique : la longueur est liée à la pagination
- **Boutons presets de longueur 16/32/48/64**
  - Accès rapide aux longueurs standard
  - Le bouton actif est surligné en bleu
- **Bouton x2 (doubler le pattern)**
  - Copie les steps 0..len vers len..2×len
  - Copie aussi les parameter locks (plocks)
  - Grisé quand len > 32 (limite 64 steps)
- **LED rouge sous la page en cours de lecture**
  - Petit cercle rouge sous le bouton de page actif dans le séquenceur
  - Indépendant du highlight bleu de la page affichée

---

## 2026-06-02 — Kick: 3 types de click fonctionnels

**Build:** `20260602-174136`
**Commits:** `8188cc6` + fix

### Changes
- **Kick: 3 types de click (Soft/Medium/Hard)**
  - Ajout paramètre `kick_click_type` dans `DrumFlashParams` (special_index: 6)
  - Dropdown UI avec labels "Soft / Medium / Hard" au lieu d'un slider numérique
  - **Fix bug critique** : `set_settings()` ne recréait pas le `ClickGenerator` quand `click_type` changeait
  - Valeurs exagérées pour différenciation audible :
    - Soft: 30ms decay, 80% noise, 0.4 level (feutré)
    - Medium: 10ms decay, 30% noise, 1.0 level (standard)
    - Hard: 2ms decay, 0% noise, 2.5 level (agressif)

---

## 2026-06-02 — Copier/Coller de pages avec parameter locks

**Build:** `20260602-155542`
**Commits:** `93886a3`

### Changes
- **Copy/Paste de pages avec parameter locks (plocks)**
  - Copy Page : copie les triggers + tous les plocks de la page
  - Paste Page : restaure les triggers + les plocks
  - Seuls les steps avec plocks sont stockés (optimisation mémoire)
  - Structures `PlockClipboardEntry` et `PageClipboard` ajoutées

---

## 2026-06-02 — Menu contextuel pages + Clear plocks

**Build:** `20260602-152305`
**Commits:** `44ceed5`

### Changes
- **Menu contextuel sur les boutons de page (1-4)**
  - Copy Page : copie les 16 steps dans le presse-papiers
  - Paste Page : colle le presse-papiers dans la page cible
  - Clear Page : efface les triggers ET les plocks de la page
- **Fix : Clear Page efface aussi les plocks**
  - Appelle `plock.clear()` pour chaque instrument et chaque step de la page

### Tests
- Build et installation OK

---

## 2026-06-02 — Fix : slider Len restauré dans la barre d'en-tête

**Build:** `20260602-145233`
**Commits:** `786e368`

### Changes
- **Slider de longueur (Len) restauré**
  - Le slider était dans `draw_top_bar()` qui n'est plus appelé
  - Déplacé dans `draw_header_bar()` entre Swing et Groove
  - Pages 1-4 et Follow toujours visibles dans la grille

### Tests
- Build et installation OK

---

## 2026-06-02 — UI : largeur augmentée + fix Sound Editor

**Build:** `20260602-143954`
**Commits:** `0db6acc`

### Changes
- **Largeur de fenêtre augmentée** : 1400 → 1480 px
  - Colonne gauche : 860 → 900 px
  - Colonne droite : 520 → 560 px
  - Gap entre colonnes : 12 → 20 px
  - Boutons P1..P8 moins tronqués dans la barre supérieure
- **Fix : Sound Editor de nouveau visible**
  - ScrollArea mal configuré masquait le Sound Editor (hauteur ~0)
  - Retrait du ScrollArea temporairement pour restaurer la visibilité

### Tests
- Build et installation OK
- Sound Editor visible et fonctionnel

---

## 2026-06-02 — Fix : suppression du vide en bas de l'UI

**Build:** `20260602-141849`
**Commits:** `8c17e54`

### Changes
- **Auto-resize de la hauteur de fenêtre** selon le contenu réel
  - `ResizableWindow` mesure la hauteur du contenu après chaque frame
  - Ajuste automatiquement la taille de la fenêtre quand `resizable=false`
  - Élimine le vide noir de ~300px en bas de l'interface

### Tests
- Build et installation OK
- UI s'affiche correctement sans espace vide en bas

---

## 2026-06-02 — Layout UI : 2 colonnes (Option A)

**Build:** `20260602-103224`
**Commits:** `XXXXXXX`

### Changes
- **Nouveau layout 2 colonnes** (conforme au LAYOUT.md)
  - **Barre haute** : Flash Drum v0.2 | ▶ | BPM | Master | Swing | Mode | Choke | Auto-Edit | P1..P8
  - **Colonne gauche** (~850px) : Séquenceur (grille 13×16 avec pagination 64) + Générateur
  - **Colonne droite** (~550px) : Éditeur de son (onglets + sound panel)
  - Toute la logique existante conservée (plock, pattern, sound settings, test, etc.)
- Fondations pour le design system et le schema data-driven
  - `src/ui/design_system.rs` : tokens visuels + widgets de base
  - `src/ui/schema.rs` : ParamSpec, Section, Category, schemas par instrument

### Tests
- Build et installation OK

---

## 2026-06-02 — Renommage : Drum Flash → Flash Drum

**Build:** `20260602-085637`
**Commits:** `XXXXXXX`

### Changes
- **Renommage global de la marque** : Drum Flash → Flash Drum
  - Nom du plugin affiché dans le DAW : `Flash Drum`
  - Titre de fenêtre UI : `Flash Drum`
  - Fenêtre drag-drop MIDI : `Flash Drum MIDI Drag`
  - Dossiers utilisateur : `Documents/Flash Drum/exports` et `Documents/Flash Drum/preset_dumps`
  - Scripts build/verif/install : titres mis à jour
  - Documentation (README, AGENTS, ADDING_AN_INSTRUMENT, STUDIO_ONE_MULTI_OUT)
  - `Cargo.toml` authors, `bundle.toml` name
  - **Non modifié** : `VST3_CLASS_ID = DrumFlashPlugin1` (gelé pour compatibilité DAW)
  - **Non modifié** : nom du dossier racine du repo (`E:\Dev\Projets\Drum Flash`)

### Tests
- Build et installation OK
- Plugin s'affiche comme `Flash Drum` dans le DAW

---

## 2026-06-01 — Session: Pattern 64 steps avec pagination style Digitakt

**Build:** `20260601-175002`
**Commits:** `XXXXXXX`

### Changes
- **Pattern étendu à 64 steps** (4 pages × 16 steps)
  - `STEP_COUNT` : 16 → 64 (`pattern.rs`, `plock.rs`, `lib.rs`)
  - Le séquenceur supporte une `master_length` globale (1-64 steps)
  - Chaque track `length_*` passe de max 16 à max 64
- **Pagination UI** (`ui.rs`)
  - 4 boutons de page (1-2-3-4) au-dessus de la grille
  - Mode **Follow** : la page affichée suit automatiquement la tête de lecture
  - Mode **Free** : navigation manuelle entre les pages
  - La grille affiche toujours 16 steps selon la page courante
- **Persistance** (`lib.rs`, `pattern.rs`)
  - Nouveau format `pattern-v2` avec `PatternMasks` wrapper pour `serde_arrays`
  - Migration automatique `pattern-v1` (16 steps) → `pattern-v2` (64 steps)
  - Migration legacy `st01..st16` → `pattern-v2` (padding avec zéros)
- **Fix stack overflow** (`plock.rs`)
  - `PlockValues` et `PlockFieldMasks` alloués sur le heap (`Vec`) au lieu de la stack

### Tests
- 73 lib tests pass
- Build installé dans le dossier VST3 système

---

## 2026-06-01 — Session: Fix range volume master (coherence 0.0-2.0)

**Build:** `20260601-171606`
**Commits:** `XXXXXXX`

### Changes
- **Uniformisation range volume** (`lib.rs`)
  - `master_volume` : range changé de `0.0..1.5` à `0.0..2.0`
  - Cohérent avec les sliders de lane (`0.0..=2.0`) et le volume instrument (`0.0..=2.0`)

### Tests
- Build installé dans le dossier VST3 système

---

## 2026-06-01 — Session: Fix focus fenêtre plugin bloque Windows

**Build:** `20260601-170350`
**Commits:** `XXXXXXX`

### Changes
- **Correction du vol de focus Windows** (`editor.rs` — `win_keyboard::set_keyboard_focus`)
  - **Problème** : `SetFocus` était appelé à chaque frame même quand l'utilisateur avait switché vers une autre application (navigateur, explorer, etc.)
  - **Cause** : `AttachThreadInput` + `SetFocus` forçaient le focus à revenir vers le plugin indépendamment de la fenêtre active
  - **Fix** : vérification que le plugin (ou sa fenêtre parent DAW) est bien la fenêtre au premier plan (`GetForegroundWindow()`) avant d'appeler `SetFocus`
  - Si l'utilisateur a switché vers une autre app, le plugin ne vole plus le focus

### Tests
- Build installé dans le dossier VST3 système

---

## 2026-06-01 — Session: Fix complet corruption UTF-8 UI (séparateurs et caractères spéciaux)

**Build:** `20260601-165420`
**Commits:** `XXXXXXX`

### Changes
- **Correction complète des caractères corrompus** (`ui.rs` + `envelope_viz.rs`)
  - Suppression de 698 séquences box-drawing corrompues (`━` → `-`)
  - Correction de séquences em-dash/en-dash mal encodées (`—`, `–` → `-`)
  - Correction de caractères accentués double-encodés (`é` → `e`)
  - Remplacement des émojis résiduels par du texte ASCII :
    - `🎲` → `Random`
    - `🗑` → `Clear`
    - `📸` → `Snapshot`
    - `↺` → `Undo`
  - **Cause** : double encodage UTF-8 (UTF-8 → Latin-1 → UTF-8) lors de manipulations PowerShell
  - **Prévention** : utilisation exclusive de caractères ASCII dans les labels de boutons et commentaires

### Tests
- 73 lib tests pass
- Build installé dans le dossier VST3 système

---

## 2026-06-01 — Session: Fix corruption UTF-8 UI (émojis et symboles)

**Build:** `20260601-163923`
**Commits:** `XXXXXXX`

### Changes
- **Correction corruption caractères UI** (`ui.rs`)
  - Remplacement des émojis corrompus (🔗, 📸, 🎲, 🗑) par du texte ASCII :
    - "🔗 Link" → "Link"
    - "📸 Snapshot" → "Snapshot" 
    - "🎲 Random" → "Random"
    - "🗑 Clear" → "Clear"
  - Remplacement des symboles de navigation corrompus :
    - "◀" (précédent) → "<"
    - "▶" (suivant) → ">"
    - "↺" (reset) → "R"
  - Remplacement des séparateurs box-drawing (─) par des tirets simples
  - Remplacement des em-dashes (—) par des tirets
  - **Cause** : encodage UTF-8 → Windows-1252 lors de manipulations PowerShell
  - **Prévention** : utilisation exclusive de caractères ASCII dans les labels de boutons

### Tests
- 73 lib tests pass
- Build installé dans le dossier VST3 système

---

## 2026-06-01 — Session: AnalogDrift sur Snare606 + Perc1

**Build:** `20260601-100457`
**Commits:** `XXXXXXX`

### Changes
- **Snare606** : ajout du drift analogique (`AnalogDrift`) — le slider Analog est maintenant fonctionnel :
  - `pitch` → détune la fréquence du résonateur bridged-T par coup (±3.5 %)
  - `level` → variation de niveau par coup (±10 %)
  - `time` → variation du decay/release par coup (±20 %)
  - Mode digital (`analog < 0.5`) = bit-identical, pas de drift.
- **Perc1** : ajout du drift analogique (`AnalogDrift`) — le slider Analog est maintenant fonctionnel :
  - `pitch` → détune la fréquence du sweep FM par coup
  - `level` → variation de niveau par coup
  - `time` → variation du decay/release par coup
  - Mode digital = bit-identical.
- Audit complet : Kick, Snare, Tom ont déjà le drift ; HiHat/OpenHiHat/Ride/Cymbal/Clap masquent le slider ; Kick808 a son propre comportement cold-start.

### Tests
- 73 lib tests pass
- Build installé dans le dossier VST3 système

---

## 2026-05-31 — Session: Task 71 — sécurisation anti-click des autres voix

**Build:** `20260531-184528`
**Commits:** `XXXXXXX`

### Changes
Application du pattern anti-click validé sur la BD à toutes les voix tonales / exposées :
- **perc1** : supprimé le **reset de phase inconditionnel** (à chaque trigger) → reset au cold-start seulement. + plancher d'attaque + DC-blockers L/R.
- **snare**, **tom** : le reset phase/filtre du mode digital ne se fait plus que sur cold-start ; + **drift analog** (slider exposé : hauteur/niveau/temps d'enveloppe par coup) ; + plancher + DC.
- **snare606** : reset résonateur/filtres → cold-start only ; + plancher + DC.
- **hihat** : pas de reset de phase (déjà ok) ; biquad peaking recalculé **seulement si la fréquence change** ; + plancher + DC.
- **snare / snare606 / hihat** recréaient leur enveloppe d'amplitude à chaque `set_settings` (appelé avant chaque trigger) → l'enveloppe repartait de 0 = click au retrigger. Corrigé via **setters** (préserve l'état de queue).
- Nouveau helper partagé **`AnalogDrift`** dans `dsp.rs` (drift pitch/level/temps ; mode digital = facteurs à 1.0).
- ride / cymbal / clap / open_hihat / kick_808 : déjà click-safe (pas de reset de phase), **non modifiés**.

### Tests
- 73 lib tests pass (nouveau garde-fou `perc1_no_click_on_retrigger_during_tail` : edge au retrigger = 0.004 → phase continue).
- Build installé dans le dossier VST3 système.

---

## 2026-05-31 — Session: Vrai fix du click parasite BD + drift analogique

**Build:** `20260531-155232`
**Commits:** `XXXXXXX`

### Diagnostic (mesuré, pas supposé)
- Le « click parasite » sur changement de hauteur n'était PAS en mode analog (mesuré propre : saut au raccord ~0.001–0.003) mais dans le chemin **digital** (`analog < 0.5`) : reset de phase sur une queue sonore + un **crossfade mathématiquement faux** (snapshot de phase figé, ratio inversé, saut brutal au sample 8). Saut mesuré ~0.20 filtre ouvert.
- Le filtre par défaut très bas (30 Hz) **masquait** le défaut → d'où l'intermittence (« revient plus ou moins fort »).
- Le test de click existant ne mesurait que l'énergie HF 3–20 kHz → **aveugle** à une discontinuité de phase basse fréquence (sortait 0.81× la baseline).

### Changes (`kick.rs`, `dsp.rs`)
- **Suppression du reset de phase en retrigger + suppression totale du crossfade cassé.** La phase n'est jamais resetée sur une queue vivante (les oscillateurs sont des accumulateurs de phase → un changement de fréquence est sans click par nature).
- **Reset au démarrage à froid uniquement** (`!was_active`) : phase + filtre + smoothers + dc_block remis à zéro → attaque propre même à 0 ms.
- **Plancher anti-click sur l'attaque d'amplitude** `MIN_AMP_ATTACK_MS = 0.5` (un attack de 0 ms = une marche = un click par définition).
- **Bug digital corrigé** : `pitch_env.trigger()` plafonnait le sweep à +1 Hz → remplacé par `trigger_reset_to(pitch_peak)`.
- **Mode analog/digital re-rendu utile** : digital = identique au bit près à chaque coup ; analog = drift par coup (hauteur ±3.5 %, niveau ±10 %, temps d'enveloppe decay+release ±20 % — la longueur de queue varie ~624–906 ms, le plus audible).
- `dsp.rs` : ajout `ExpDecayEnvelope::trigger_reset_to`, ré-ajout `SquareOsc::reset_phase`, retrait des getters morts du crossfade.

### Tests
- 72 lib tests pass. Nouveaux garde-fous : `test_kick_no_click_on_plock_retrigger_either_mode`, `test_kick_zero_attack_no_click`, `test_kick_analog_drifts_digital_is_stable` (mesure : digital diff 0.0, analog diff > 0), + rendu WAV digital.
- Build installé dans le dossier VST3 système.

### À suivre
- Appliquer le même pattern anti-click + le sens analog=drift/digital=stable aux autres voix tonales : **perc1, snare, tom, snare606, hihat**.

---

## 2026-05-30 — Session: Fix click BD sur changement de hauteur (plock)

**Build:** `20260530-195702`
**Commits:** `XXXXXXX`

### Changes
- **Anti-click kick sur plock frequency** (`kick.rs`)
  - `FREQ_SMOOTH_MS` : 0.1 ms → **2.0 ms** (lissage de la fréquence d'oscillateur)
  - Crossfade digital mode : 2 → **8 échantillons** (transition de phase plus douce)
  - Ajout d'un `filter_cutoff_smoother` pour éviter les sauts sur `filter_freq`
  - `update_derived_params` ne touche plus le filtre directement (smoothed dans `process_sample`)
  - Test unitaire `test_kick_plock_frequency_change_no_click` qui reproduit le scénario
- **Fix root cause: boucle de version dans `iter_samples`** (`lib.rs`)
  - `sound_settings_state.version` était vérifiée **à chaque échantillon** dans `process()`
  - Si un trigger avec plock se produisait, puis la version changeait (modif UI), les settings globaux écrasaient le plock dans le même buffer → discontinuité d'un échantillon = click
  - Déplacée **avant** `iter_samples`, exécutée une fois par buffer
  - Test de rendu audio `test_kick_plock_click_audio_render` génère un WAV + analyse HF

### Tests
- 67 lib tests pass (2 nouveaux)
- Build installé dans le dossier VST3 système

---

## 2026-05-30 — Session: Réparation ui.rs + Masquage slider Analog

**Build:** `20260530-174031`
**Commits:** `XXXXXXX`

### Changes
- **Réparation fichier ui.rs corrompu** (session précédente plantée)
  - Suppression de ~2500 lignes dupliquées dans la section Preset Dumps
  - Suppression du bloc `if Analog` mal formé dans le match des paramètres sliders
  - Restauration de la structure correcte du match Slider/Checkbox depuis git
- **Masquage du slider Analog** pour 6 instruments (HiHat, OpenHiHat, Ride, Cymbal, Perc1, Zap)
  - Slider masqué dans le Sound Panel (remplacé par placeholder 0.0 dans les dumps)
  - Seuls Kick, Snare, Tom, Snare606, Kick808 et B8 exposent le paramètre Analog

### Tests
- Compilation propre : 0 erreur, 6 warnings (unused_variables/methods)
- Build installé dans le dossier VST3 système

---

## 2026-05-29 — Session: Correction corruption UTF-8 dans l'interface

**Build:** `20260529-174106`
**Commits:** `XXXXXXX`

### Changes
- **Fix caractères ésotériques dans l'UI** (ui.rs, lib.rs)
  - 49 occurrences de caractères corrompus remplacés par les bons caractères Unicode
  - Émojis restaurés : ◀, ▶, 🎲, 🔗, 📸, 🔀, ↺, 🗑
  - Caractères accentués restaurés : é, è, à, ç, ê, ô, ù, î, ï
  - Séparateurs et flèches restaurés : —, →, ─, ■
  - Cause : manipulations PowerShell précédentes en encodage Windows-1252 au lieu d'UTF-8
  - Fix via script Python avec mapping byte UTF-8 explicite

### Tests
- 58 lib tests + 44 standalone tests pass
- Build installé

---

## 2026-05-29 — Session: Tests de stress du séquenceur et documentation

**Build:** `20260530-154620`
**Commits:** `7be01c1` et suivants

### Changes
- **Tests de stress du séquenceur** (sequencer/stress_tests.rs)
  - 6 tests de stress implémentés couvrant :
    * `test_long_session_stability` : stabilité sur 1 minute (extensible à 1h)
    * `test_complex_pattern_changes` : changements dynamiques de patterns
    * `test_daw_sync_scenarios` : synchronisation play/stop/seek
    * `test_high_cpu_load_patterns` : patterns denses à haute charge
    * `test_groove_timing_stability` : stabilité du timing avec différents grooves
    * `test_track_push_pull_stability` : décalages de piste (push/pull)
  - Tous les tests passent : 6/6 nouveaux tests + 59/59 tests existants
  - Couverture étendue : longue durée, charge CPU, synchronisation DAW

- **Analyse complète mode Analog vs Digital** (documentée dans TODO.md)
  - 5 instruments utilisent le mode analog/digital (Kick, Kick808, Snare, Snare606, Tom)
  - 7 instruments toujours en mode "analog" (Clap, HiHat, OpenHiHat, Ride, Cymbal, Perc1, Zap)
  - Documentation des comportements et recommandations d'utilisation

### Tests
- 65 lib tests + 51 standalone tests pass (incluant les 6 nouveaux tests de stress)
- Build prêt pour installation et test dans Studio One
- Validation complète de la stabilité du séquenceur
  - Cela créait un pic de amplitude massif (step de 0.468) superposé à la tail
  - Fix : ne retrigger le click que sur un cold start (!was_active), pas sur un retrigger
  - Les tests confirment : max step passe de 0.468 à 0.0036 avec le fix
  - Tous les tests existants passent (6 kick tests + 52 autres)

### Tests
- 58 lib tests + 44 standalone tests pass
- Build installé et prêt à tester

---

## 2026-05-29 — Session: Correction focus clavier Windows (solution officielle)

**Build:** `20260529-135735`
**Commits:** `XXXXXXX`

### Changes
- **Correction focus clavier** (Windows uniquement) — Solution officielle egui-baseview
  - Problème connu : les événements clavier sont capturés par le DAW parent au lieu de la fenêtre enfant du plugin
  - Référence : [baseview#192](https://github.com/RustAudio/baseview/issues/192), [egui-baseview#20](https://github.com/BillyDM/egui-baseview/issues/20)
  - Solution : activation de la feature windows_keyboard_workaround dans egui-baseview
  - Cette feature appelle window.focus() automatiquement quand des événements de saisie sont détectés
  - Modification : endor/nih-plug/nih_plug_egui/Cargo.toml — ajout de la feature aux features par défaut

### Tests
- Build installé et prêt à tester dans Studio One / Reaper

---

## 2026-05-29 — Session: Correction focus clavier Windows + Preset dumps Phase 1a

**Build:** `20260529-124136`
**Commits:** `XXXXXXX`

### Changes
- **Correction focus clavier** (Windows uniquement)
  - Le DAW capturait les événements clavier et ne les transmettait pas au plugin
  - SetFocus appelé automatiquement sur le HWND de la fenêtre du plugin à chaque frame
  - PLUGIN_HWND stocké dans une variable statique publique (
ih_plug_egui::editor.rs)
  - Fonction publique 
ih_plug_egui::ensure_window_focus() exposée
  - Appel systématique dans ui.rs::update callback via ensure_keyboard_focus()
- **Preset dump dev tools** (Phase 1a)
  - Section "Dev: Preset Dumps" dans le Sound Panel
  - Dump/Load/Delete de presets JSON dans Documents/Flash Drum/preset_dumps/
  - serde_json ajouté aux dépendances

### Tests
- 58 lib tests + 44 standalone tests pass
- Build installé et prêt à tester

---

## 2026-05-29 — Session: Dev tools preset dumps (Phase 1a)

**Build:** `20260529-095403`
**Commits:** `XXXXXXX`

### Changes
- **Preset dump dev tools** (preset_dumps.rs, ui.rs)
  - Collapsible "Dev: Preset Dumps" section in Sound Panel
  - **Dump** : captures current instrument settings (13 standards + algo + specials) to JSON
  - **Load** : restores dumped settings + switches to target instrument tab
  - **Delete** : removes dump file
  - Files stored in Documents/Flash Drum/preset_dumps/
- **New dependency** : serde_json = "1.0" (Cargo.toml)
- **New module** : src/preset_dumps.rs (dump/list/load/delete preset JSONs)

### Tests
- 58 lib tests + 44 standalone tests pass
- Build installed and ready for factory preset authoring

---

## 2026-05-28 â€” Session: UI polish + polyrhythm fix + generator roles

**Build:** `20260528-175015`
**Commits:** `XXXXXXX`

### Changes
- **Volume at top of Sound Editor** (`ui.rs`)
  - Dedicated group with separator before OSC/ENV/FILTER/SAT/OUTPUT families
  - Large slider (0.0â€“2.0) right under "Sound Editor" heading
- **Per-lane volume in pattern grid** (`ui.rs`)
  - Compact 40px slider before Mute/Solo/Test buttons
  - Reads/writes `sound_settings.instruments[inst].volume` directly
- **Plock colour coding** (`ui.rs`)
  - **Orange** (255, 140, 0) â†’ Link mode or mixed plock
  - **Red** (220, 50, 50) â†’ Full snapshot
  - Darker variants for inactive steps
- **True polyrhythm** (`sequencer/mod.rs`)
  - Independent `step_counter` per track, incremented on master-step transition
  - Fixes identical bars bug with `master_step % length`
  - Tracks resync at LCM(master, track_length)
- **Steps beyond lane_length erased** (`ui.rs`)
  - Complete visual removal (no button, no background) for clarity
- **Pattern generator roles enriched** (`generator/styles.rs`)
  - Rock style: Snare 606 backbeat layer, 808 Kick downbeat reinforcement, Perc1 crash/FX accents
  - All 13 instruments now have musically meaningful roles
- **Grid spacing** (`ui.rs`)
  - Steps in horizontal containers with 6px spacing
  - Header labels aligned with exact column widths
- **Bugfix: first step not read on play** (`sequencer/mod.rs`)
  - `play()` and `force_step0_trigger()` initialise `step_counter` to `length - 1`
- **Deployment rule added** (`AGENTS.md`)
  - Systematic build + install after every task completion

### Tests
- 58 lib tests + 44 standalone tests pass
- Multiple builds tested and installed throughout the session

---

## 2026-05-28 â€” Generator roles enriched + polyrhythm fix + dimmed steps

**Build:** `20260528-154125`
**Commits:** `XXXXXXX`

### Changes
- **Pattern generator roles enriched for Rock style** (`styles.rs`)
  - Snare 606: backbeat layer (steps 4, 12, 6, 10) with 35% probability
  - 808 Kick: sub-bass reinforcement on downbeats 0 and 8 only
  - Perc1: crash/FX accents (steps 0, 14, 15, 7, 11) with 20% probability
  - All 13 instruments now have musically meaningful roles (no more user-only)
- **True polyrhythm with independent step counters** (`sequencer/mod.rs`)
  - Each track maintains its own `step_counter` incremented on master-step transition
  - Fixes the bug where `current_step = master_step % length` repeated identically every bar
  - Tracks now cycle independently and resync at LCM(master, track_length)
- **Dimmed steps beyond track length** (`ui.rs`)
  - Steps beyond `lane_length` are completely erased (no button, no background)
  - Active steps beyond length shown in dark blue for clarity

### Tests
- 58 lib tests + 44 standalone tests pass
- Build OK, bundle generated, installed to system VST3 folder

---

## 2026-05-28 â€” UI improvements: per-lane volume & plock colour coding

**Build:** `20260528-142648`
**Commits:** `XXXXXXX`

### Changes
- **Volume moved to top of Sound Editor** (`draw_sound_panel`)
  - Large `LocalParamSlider` (0.0â€“2.0) displayed right under the "Sound Editor" heading
  - No longer buried inside the Output family group
- **Per-lane volume control in pattern grid** (`draw_grid`)
  - Compact 40 px slider next to each instrument label (before Mute/Solo/Test)
  - Reads/writes `sound_settings.instruments[inst].volume` directly
  - Calls `bump_version()` on change so the audio thread picks it up
- **Plock colour coding: link vs snapshot** (`draw_grid`)
  - **Orange** (255, 140, 0) â†’ Link mode or mixed plock (`field_mask == 0` or partial)
  - **Red** (220, 50, 50) â†’ Full snapshot (`field_mask == all_bits`)
  - Darker variants for inactive steps with plock only
  - Makes it immediately obvious which steps are fully frozen vs. following globals

### Tests
- Build OK, bundle generated, installed to system VST3 folder
- 0 new compiler errors (5 pre-existing warnings only)

---

## 2026-05-27 â€” Bugfix B8 + Cymbal shimmer & noise colour

**Build:** `20260527-202249`
**Commits:** `XXXXXXX`

### Changes
- **Bugfix B8 silent after CY param change** (`ExpDecayEnvelope::set_attack_ms`)
  - Division-by-zero when `attack_time` shortened to 0 during active ramp â†’ permanently corrupted envelope with NaN
  - Fix: snap immediately to `attack_peak` and clear `attack_remaining` when zeroed mid-ramp
  - Test button "T" now calls `set_voice_settings` before `trigger` (was using stale params)
- **Cymbal Sound Panel refactor**
  - Removed unused `frequency` parameter (noise-based voice, no oscillator)
  - Added `Shimmer Freq` (1â€“50 Hz, default 15 Hz) â€” modulates FM shimmer LFO rate
  - Added `Noise Type` combobox: White / Pink / Brown / Blue
    - `PinkNoise` (Voss-McCartney), `BrownNoise` (integrator), `BlueNoise` (differentiator) in `dsp.rs`
    - Independent L/R generators for stereo mode, no shared state
  - `CymbalSettings` now stores `shimmer_freq` and `noise_type` via `special[0..1]`
  - Retro-compatibility: old plock snapshots saved `special[0]=0.5` â†’ now interpreted as 0.5 Hz shimmer (slow, nearly static)

### Tests
- 54 lib tests pass, 41 standalone tests pass
- New: `shimmer_produces_varying_filter_cutoff`, `set_settings_updates_shimmer_freq`, `cymbal_shimmer_through_drum_synthesizer`

---

## 2026-05-26 â€” Saturation generalised to all 13 instruments

**Build:** `20260526-101659`
**Commits:** `XXXXXXX`

### Changes
- **Saturation on all 13 voices**: Kick, Snare, HiHat, OpenHiHat, Tom1-3, Clap, Ride, Cymbal, Snare606, BassDrum808, Perc1
- **Dedicated SAT section** in Sound Panel (`ParamFamily::Saturation`) â€” no longer mixed in OSC/OUTPUT
- **Algorithm names displayed** in combobox (SoftClip, Valve, Transistor, HardClip, Tape) instead of numbers
- **Pre-Filter checkbox** now functional â€” routes saturation before or after the filter chain
- **Per-instrument special params** using saturation slots in `special[8]` array
  - Instruments with existing specials (Snap, Echo, Stick, etc.) append saturation after
  - Instruments without specials use indices 0-4
  - BassDrum808 limited to 4 saturation params (no Pre-Filter slot due to 8-element array)
- **65 new FloatParam** declarations in `DrumFlashParams` (5 params Ã— 13 instruments)

---

## 2026-05-23 â€” Saturation / distortion per instrument (Snare 606)

**Build:** `20260523-211642`
**Commits:** `XXXXXXX`

### Changes
- **New saturation module** (`saturation.rs`) with 5 distinct algorithms:
  - **SoftClip** â€” smooth tanh, warm and musical
  - **Valve** â€” strong asymmetry, tube glow, even harmonics
  - **Transistor** â€” germanium grit, crunchy, emphasizes highs (+35% positive side)
  - **HardClip** â€” brutal digital clipping, aggressive and square
  - **Tape** â€” soft compression "glue", smooth transient taming
- **Saturation exposed in Sound Panel** for Snare 606 (S6):
  - Saturation Type (0-5, step 1)
  - Saturation Amount (0-1, drive mapped 1Ã—..20Ã—)
  - Saturation Mix (0-1, dry/wet)
  - Saturation Output Gain (0.5-2.0, makeup)
  - Saturation Pre-Filter â˜‘ (checkbox toggle, post-filter by default)
- **Auto-edit enabled by default** (`BoolParam::new("Auto Edit", true)`)
- **Hold parameter restored** on Snare 606 (was missing from `SNARE606_STD`)
- Special params use slots 3-7 of `special[8]` for saturation (indices 0-2 remain resonance/tone/snap)

---

## 2026-05-23 â€” Snare 606 body enhancement (v4)

**Build:** `20260523-154654`  
**Commits:** `XXXXXXX`

### Changes
- **Hold parameter exposed** in the Sound Panel UI (ENV group). Default 8 ms for a thicker body; user can tweak from 0 to 0.5 s.
- (Retains v3 changes: user-controllable hold, body oscillator, boosted body gain, raw noise excitation, snap envelope, revised mix, tuned defaults.)
- All 43 tests pass; `cargo check --all-targets` clean.

---

## 2026-05-23 â€” Snare 606 body enhancement (v3)

**Build:** `20260523-102533`  
**Commits:** `XXXXXXX`

### Changes
- **Hold now user-controllable** via the Hold parameter (default 8 ms for thicker body). The envelope stays at peak for `hold` seconds before decay starts.
- **Default hold increased** from 0 ms to 8 ms to give a thicker, more rounded body out of the box.
- (Retains v2 changes: body oscillator, boosted body gain, raw noise excitation, snap envelope, revised mix, tuned defaults.)
- All 43 tests pass; `cargo check --all-targets` clean.

---

## 2026-05-23 â€” Snare 606 body enhancement (v2)

**Build:** `20260523-100824`  
**Commits:** `XXXXXXX`

### Changes
- **Body oscillator added**: pure `SineOsc` at resonator frequency mixed with raw noise as excitation (`excitation = noise + sine * 0.6`). Gives the resonator a tonal fundamental to resonate with â€” much closer to the real TR-606 VCO+bridged-T topology.
- **Body gain boosted**: `tone * 1.2` (was `tone * 0.7`). More weight when tone is up.
- (Retains v1 changes: raw noise excitation, snap envelope, revised mix, tuned defaults.)
- All 43 tests pass; `cargo check --all-targets` clean.

---

## 2026-05-23 â€” Snare 606 punch overhaul (v1)

**Build:** `20260523-095847`  
**Commits:** `XXXXXXX`

### Changes
- **Snare 606 signal chain rework** for more punch and closer TR-606 character:
  - **Raw noise excitation**: the bridged-T resonator is now driven by unfiltered white noise (previously the noise was softened by a LP before hitting the resonator, smearing the transient).
  - **Dedicated snap envelope**: ultra-short burst (0.2 ms attack, 3 ms decay) on raw noise for the percussive attack that defines the 606 snare.
  - **Revised mix architecture**: body (resonator) + wires (HP-filtered softened noise) + snap (raw noise burst), each with independent gain.
  - **Body gain now scales 0..0.7** (was 0.4..1.0), so tone=0 gives a pure wires+snap sound.
- **Defaults tuned** for a tighter, more aggressive sound:
  - decay 0.7 s â†’ 0.25 s
  - filter_freq 3000 Hz â†’ 8000 Hz
  - tone 0.55 â†’ 0.4
  - snap 0.3 â†’ 0.6
- All 43 tests pass; `cargo check --all-targets` clean.

---

## 2026-05-23 â€” Session : revert [54], docs update

**Build:** `20260523-092208`  
**Commits:** `520e6d8`, `b604ae8`

### Changes
- Update `ADDING_AN_INSTRUMENT.md` for typed settings ([39] generalization).
- Attempt [54] Alt+mouse precision input on ParamSlider + egui::Slider.
- Revert [54] : egui::Slider is a closed widget, cannot reliably intercept Alt+drag. Custom bar+DragValue replacement broke UX.
- Decision : [54] requires a custom widget built from scratch (separate bar + value text).

---

## 2026-05-21 â€” [39] Typed per-instrument settings (all 13 voices)

**Build:** `20260521-213022`  
**Commits:** `fcde87c`

### Changes
- Generalize typed settings structs to all 13 instruments (Kick prototype â†’ all voices).
- New settings files: `SnareSettings`, `HiHatSettings`, `OpenHiHatSettings`, `TomSettings`, `ClapSettings`, `RideSettings`, `CymbalSettings`, `Snare606Settings`, `Kick808Settings`, `Perc1Settings`.
- Each voice refactored to store its typed struct instead of `VoiceSettings` + opaque `special[N]`.
- `VoiceSettings` remains the persistence boundary; conversions happen in `set_settings()` with zero-allocation stack copies.
- All 43 tests pass; bit-identical guarantee maintained.

---

## 2026-05-21 â€” [39] Prototype Kick : typed per-instrument settings

**Build:** `20260521-201743`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- New typed settings struct for Kick (`KickSettings`) replacing opaque `special[0]` access.
- `KickSettings` contains named fields for all standard and special parameters used by the Kick voice.
- `From<VoiceSettings>` and `Into<VoiceSettings>` implementations for seamless conversion at the persistence boundary.
- `KickVoice` refactored to store `KickSettings` internally; `Voice::set_settings` wrapper handles conversion.
- Round-trip test (`kick_settings_roundtrip_preserves_all_fields`) verifies no data loss.
- All existing kick tests pass unchanged â€” confirms bit-identical behavior.
- No change to `plock-v1` format, `DrumFlashParams`, or automation IDs.

---

## 2026-05-21 - External MIDI drag helper

### Changes
- Add `drum-pattern-midi-drag-helper.exe`, a Windows helper bin that performs OLE `DoDragDrop` outside the DAW process.
- Re-enable `Drag`: the plugin exports the MIDI file, then opens a tiny topmost `Drag MIDI` helper window with the exported `.mid` path.
- Update `build.ps1` to copy the helper next to the VST3 DLL in the bundle/install.
- Keep MIDI file export available through the `MIDI` button and `Copy Path`.
- Polish the helper window into a compact rounded drag handle instead of a raw Windows-looking box.

### Notes
- Direct in-process OLE drag crashed Studio One; the helper isolates that risk from the host.
- The previous invisible helper launch did not provide a reliable Windows drag source. Drag now starts from the helper window itself.

---

## 2026-05-21 â€” Perc1 Hold wiring

### Fixes
- Wire Perc1 `hold` into its amplitude `DecayReleaseEnvelope` on creation and settings updates.
- Add a regression test confirming Perc1 Hold extends the active envelope duration.

---

## 2026-05-21 â€” Targeted stereo controls

### Changes
- Expose Stereo in the Sound Panel for Snare606 without exposing it on B8.
- Keep Kick, B8 and Toms mono-focused in the registry.
- Fix Snare606 resonance retuning so both left and right resonators update when resonance changes.
- Add a Snare606 stereo unit test that verifies stereo mode produces independent L/R channels.

---

## 2026-05-21 â€” Per-instrument Attack parameter

### Changes
- Add `attack` to `VoiceSettings` and expose it in the Sound Panel ENV group for every instrument.
- Wire Attack into each amplitude `DecayReleaseEnvelope`, preserving the existing anti-click ramp defaults per voice.
- Update the amplitude envelope graph and legend to show full A-H-D-R shape.
- Persist sound settings with 13 fields per instrument while migrating old 12-field states.
- Extend plocks with Attack as appended field 18, preserving legacy field 12 for old Clap Echo compatibility.

---

## 2026-05-20 â€” Plock Snapshot vs Link mode

**Build:** `20260520-211700`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- **Plock per-field masks** (`PlockFieldMasks`) : each plock step now tracks which fields are explicitly overridden via an 18-bit `u32` mask.
- **Snapshot mode** (default) : "ðŸ“¸ Snapshot current settings" copies all global values and locks them â€” previous behavior.
- **Link mode** (new) : "ðŸ”— Link to global" activates the plock without copying values; only fields you subsequently modify override the live global settings.
- **`get_settings` merge** : audio thread builds global `VoiceSettings`, then merges with plock â€” overridden fields come from plock storage, unmodified fields fall back to globals.
- **Plock editor UI** :
  - Mode indicator : `ðŸ”— Linked`, `ðŸ“¸ Full snapshot`, or `ðŸ”€ Mixed`.
  - Bold labels for overridden fields, weak labels for linked fields.
  - `â†º` reset button per field to revert to global (clears the bit).
  - Per-field `set_field` writes only the changed field instead of rewriting the entire `VoiceSettings`.
- **Persistence retro-compatibility** : old presets without field masks load as full snapshots (all bits set).
- New unit tests : `link_mode_returns_global`, `merge_takes_modified_fields`, `set_field_only_sets_one_bit`, `clear_field_unlinks_without_clearing_plock`, `clear_removes_field_mask`.

---

## 2026-05-20 â€” Sound Panel redesign (families + interactive envelope viz)

**Build:** `20260520-123040`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- Sound Panel fully data-driven from `instrument_registry.rs`:
  - New `ParamFamily` enum (Osc / Env / Filter / Output) with `StandardParamDef` metadata (range, log scale, suffix, checkbox).
  - Parameters grouped per family with titled frames.
  - Removed legacy `InstrumentCapabilities` â€” parameter visibility is now encoded in `standard_params` slices.
- Interactive envelope visualizations:
  - `draw_amp_envelope` : AHDSR-style curve with colour-coded phases (Hold=cyan, Decay=blue, Release=purple). Attack phase is hidden when no Attack parameter exists.
  - `draw_filter_envelope` : dedicated filter-env curve (orange) inside the FILTER family group.
  - Layout horizontal : params on the left, graph on the right.
  - Real-time update when moving Decay / Release / Curve sliders.
- Fixed decay slider ranges that were clamping long-decay voices (Ride 1.2s, Cymbal 2.0s).

---

## 2026-05-19 â€” Perc1 refactor (Zap â†’ Perc1)

**Build:** `20260519-191344`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- Rename Zap â†’ Perc1 (`perc1.rs`, `DrumVoice::Perc1`, label `"P1"`, all params `perc1_*`).
- Migrate Perc1 `amp_env` from `ExpDecayEnvelope` to `DecayReleaseEnvelope` â€” Release slider is now wired.
- Fix `set_settings` anti-click invariant: use `set_decay()` / `set_release()` / `set_curve()` instead of recreating envelopes.
- Add `filter` + `filter_env` to Perc1 with additive cutoff formula.
- Fix latent bug in `voice_settings_for`: index 12 now correctly reads `algo_perc1`.
- Update plock tests, MIDI export tests, generator comments, and algo registry for Perc1.

### Known issues
- Perc1 Release and other parameters reported as non-responsive in Studio One â€” under investigation ([50]).

---

## 2026-05-19 â€” Revert stable + documentation

**Build:** `20260519-163250`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- Revert code to stable commit `5ae1286` (Zap voice) after critical bugs identified in Perc1 commit `8d56e72` (envelope recreation in `set_settings`, broken release/filter env, hardcoded plock menu).
- Rebuild and reinstall VST3 bundle.
- Create `ADDING_AN_INSTRUMENT.md` â€” complete guide for adding new synthesis voices (architecture, step-by-step checklist, anti-patterns).
- Merge `CLAUDE.md` into `AGENTS.md` for unified agent documentation.
- Synchronize `BACKLOG_VST.md` and `TODO.md`.

### Known issues to fix
- Perc1 needs clean re-implementation: do not recreate envelopes in `set_settings`, migrate to `DecayReleaseEnvelope`, make plock menu data-driven.

---

## 2026-05-16 â€” Mix Bus + plock fix + B8 + conditional params

**Build:** `20260516-205054`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- Per-instrument Mix Bus checkbox (route to Main Mix on/off, independent of Mute).
- Parameter Locks format expanded: `FIELD_COUNT` 12 â†’ 14 (fields 12 = clap_echo, 13 = algo).
- Fix root cause of lost plock echo: `set_special_param()` removed from `process()`, special params now propagated only at trigger time.
- Sound Panel hides inactive parameters per instrument via `InstrumentCapabilities`.
- New instrument B8 (TR-808 Bass Drum) with accent, snap, pitch drop, analog, release, click tone.

---

## 2026-05-15 â€” B8 click tone + plock B8 fix + anti-click

**Build:** `20260515-124610`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- Dedicated LP filter for B8 click tone (100â€“8000 Hz), plockable (field 17).
- Plock B8 fix: special params (accent/snap/pitch_drop/click_tone) stored in fields 14â€“17.
- Attack ramp 1.5 ms on B8 envelope + cold-start-only phase reset + DcBlocker + freq_smoother.
- Cross-DAW validation: plugin loads in Reaper, audio stable.
- Warnings reduced: 17 â†’ 0 (`cargo check --all-targets` clean).

---

## 2026-05-14 â€” DecayReleaseEnvelope + Snare 606 + Clap rework

**Build:** `20260514-220658`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- Bi-stage `DecayReleaseEnvelope` (decay + release) with persistent retrigger (`trigger_at_peak`).
- Hold phase between attack and decay for Snare/HiHat/OpenHH/Snare606.
- Analog-style continuity: no phase/filter/noise reset on retrigger.
- Kick: additive pitch sweep + freq smoother + DcBlocker.
- Clap rework: bandpass, snap transient, 4 bursts with irregular timing, Echo slider (0â€“3).
- New instrument: Snare 606 (TR-606 grey-box) with resonance, tone, snap.
- Fix crash on 11th voice: `IntRange` div-by-zero + index bounds + step mask hardcode.

---

## 2026-05-13 â€” Modular synthesis + groove + generators + UI polish

**Build:** `20260513-202946`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- Modular `Voice` architecture with `set_algo()` and `set_special_param()`.
- Kick: 3 algos (Sine/Square/FM) + click transient.
- Snare: 3 algos (Synth/Noise/Layered) + snap param.
- New voices: Clap, Ride, Cymbal.
- Groove engine: Straight, Swing 16th, Shuffle, MPC Style.
- Push/pull per instrument, humanize per instrument.
- Pattern generators: Euclidean, Markov, Probabilistic.
- MIDI export to `Documents/Flash Drum/exports/`.
- UI: BoolParam â†’ checkbox, EnumParam â†’ combobox, algo â†’ named combobox.
- Sound panel per instrument with frequency, decay, volume, filter, algo, special params.

---

## 2026-05-11 â€” Grid persistence + Studio One save/restore fix

**Build:** `20260511-091259`  
**VST3 Class ID:** `DrumFlashPlugin1`  
**SHA-256:** `62AA5FCC445FEFDBC1E30196E614BCAED53A61C9F9EB2AB9BD5A4E1C5C510CEF`

### Changes
- Grid persisted via `pattern-v1` field (serialized from `SharedPattern`).
- Migration from legacy hidden params `st01`â€“`st16` to `pattern-v1`.
- Vendored `nih-plug` wrapper saves/restores state on both `IComponent` and `IEditController`.
- Studio One multi-out validated: `getRoutingInfo()` maps event input to main audio output.
- DAW sync validated: play, stop, tempo, repositionnement.
- Presets Rock, Funk, Disco.
- Mutes and solos per instrument.



