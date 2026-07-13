## Modular Grid Redesign (active — V1.5)

- [x] [MG-1] Internal track model: `TrackSlot`, `TrackInstrumentKind`, `TrackRouting`, `TrackLayoutState`
- [x] [MG-2] Persist track layout in `track-layout-v1`
- [x] [MG-3] Migrate legacy 13-voice sessions to 14-slot layout
- [x] [MG-4] Adapt sequencer to iterate active tracks
- [x] [MG-5] Adapt audio engine to 14 independent synth instances + routing
- [x] [MG-6] Adapt pattern bank to store only musical data (no layout)
- [ ] [MG-7] Refactor UI grid for modular lanes (active tracks, add/remove/change) — rollback 20260701: reverted after Studio One startup crash
  - [x] [MG-7.1] Checkpoint sûr : ajouter `selected_track_slot` dans l'état UI, synchronisé avec les 13 lanes fixes, sans changement VST3/state audio (build 20260701-172602)
  - [x] [MG-7.2] Checkpoint sûr : sélectionner le slot via les interactions de grille/lane restantes (volume, Hum, Push, Len, fusion double-clic/shift-clic, plock clic-droit) sans changement VST3/state audio (build 20260701-173832)
  - [x] [MG-7.2a] Fix compat audio : tant que l'UI affiche 13 lanes fixes, le layout par défaut et le template 4 slots buggué sont migrés vers les 13 voix legacy pour éviter les lanes 5+ silencieuses (build 20260701-174700)
  - [x] [MG-7.3] Checkpoint sûr : introduire le bridge `slot_idx -> voice_idx` dans la boucle de grille, sans changement visuel ni VST3/state audio (build 20260701-175321)
  - [x] [MG-7.4] Checkpoint sûr : extraire le rendu d'une lane dans `draw_legacy_slot_lane_v2(slot_idx, voice_idx, ...)`, sans changement visuel ni VST3/state audio (build 20260701-183243)
  - [x] [MG-7.4a] Fix Len individuel : une lane lockée utilise sa propre longueur 1..64 même au-delà de la longueur globale, UI et playhead inclus (build 20260701-201011)
- [x] [MG-7a] Move `+ Add module` under lanes + styled empty lanes — rollback 20260701
  - [x] [MG-7a.1] Checkpoint visuel sûr : afficher le slot 14 vide et `+ Add Module` sous les lanes, sans activer l'ajout de piste ni changer audio/VST3/state (build 20260701-205643)
  - [x] [MG-7a.1a] Fix layout : passer la fenêtre fixe de 1480x800 à 1480x900 pour rendre visibles les options/panneaux bas après ajout du slot vide (build 20260701-230011)
  - [x] [MG-7a.2] Activer `+ Add Module` avec sélection d'instrument et mutation contrôlée du `track-layout-v1` (build 20260702-215053)
- [x] [MG-8] Sound editor tabs per track (Sound / Track) + instrument selector + per-slot routing — rollback 20260701 (build 20260702-215053)
- [x] [MG-9] MIDI note/channel behavior per spec — needs revalidation after rollback
- [x] [MG-10] **Adapt generator to track types and duplicate variations** — corrigé (build 20260707-161620)
  - Le générateur prend désormais le `track_layout` courant en entrée et mappe les rôles musicaux par `kind.drum_voice_index()` au lieu de l’index de rangée.
  - Jusqu’à 3 slots `Tom` utilisent les rôles Tom1/Tom2/Tom3 existants ; au-delà (ou pour toute autre duplication de kind) une variation déterministe est appliquée.
  - Les slots vides restent vides après `GENERATE`.
  - Tests déterministes ajoutés : mapping 4 lanes par défaut, rôles Tom distincts, variation des duplicates, slots vides silencieux.
- [x] [MG-11] Build, test, install, update CHANGELOG — done (build 20260702-215053)

## [P0] Stabilisation modular grid 14 slots (session 2026-07-03)

> Constats du test Studio One post-MG-7a.2 (build 20260702-215053) : crash à la 14e piste,
> son défectueux sur piste ajoutée, type non modifiable via TRK, layout 4 lanes au démarrage.
> Diagnostic code fait le 2026-07-03 — références de lignes valables sur le working tree non commité.

- [x] [ST-1] **Crash S1 en ajoutant la 14e piste** : `fusion_selection_start` taillé à 13 mais indexé par slot (0..14) dans la boucle de grille → index out of bounds dès que la lane 14 est dessinée. Corrigé : taillé à `MAX_TRACKS` (build 20260704-165252).
- [x] [ST-2] **Crash clic droit (menu plock) sur la lane 14** : `INSTRUMENTS[slot_idx]` dans les menus Plock/Morph/Seq Plock + `DrumVoice::from_index(slot).expect(...)` dans le dropdown Algo. Corrigé : schéma résolu via `schema_voice_idx(params, slot)` dérivé du kind, stockage plock inchangé (par slot) (build 20260704-165252).
- [x] [ST-3] **Son défectueux sur un slot ajouté** : `reset_slot_to_defaults()` n'était jamais appelé — un slot activé/rekindé gardait les settings d'init de la voix legacy de même index (ex. Tom pour le slot 5). Corrigé : reset aux défauts du kind à l'activation (`+ Add Module`) et au changement d'instrument (onglet TRK) (build 20260704-165252).
- [x] [ST-4] **Sound Panel confond index de slot et index de voix** : `selected_instrument` est désormais un index de slot (0..14), le schéma (registre, special params, filter label, checks Kick/B8, algos) est dérivé du kind du slot ; changer le type dans TRK ne fait plus sauter la sélection ; onglets du Sound Editor = slots actifs ; `effective_lane_length_for_ui` aligné sur l'indexation slot du moteur audio (build 20260704-165252).
- [x] [ST-4b] **Settings/plocks appliqués par voix au trigger** (trouvé par test S1 : "la freq de la lane 1 change celle de la lane 14") : `voice_settings_at_step()` indexait `sound_settings_state.instruments[]` et `plock_state.get_settings()` par voix au lieu du slot → chaque trigger d'un slot dupliqué réappliquait les settings et plocks du premier slot du même kind. Corrigé : signature `(slot_idx, voice_idx, step)`, settings + plocks par slot (build 20260704-173043).
- [x] [ST-4c] **Pastille `+14` de la lane vide non cliquable** (rapporté test S1 : "quand je clique sur +14 rien ne se passe") : la lane vide affichait `+N` avec highlight au survol mais `Sense::hover` seulement — seule la rangée `+ Add Module` était active. Corrigé : pastille cliquable (curseur main + tooltip), activation factorisée dans `activate_next_free_slot()` (build 20260704-174006).
- [x] [ST-7] **Special params par slot (instances vraiment indépendantes)** — **FAIT (build 20260705-122315), à valider dans S1.**
  - [x] ST-7a : `special[32]` + `freq_mode` par slot dans `InstrumentSettingsState` ; persistance `sound-settings-v2` format v3 (644 floats) + flag `needs_param_seed` pour migration
  - [x] ST-7b : moteur — `voice_settings_for(slot, voice, …)` lit specials + algo par slot ; seed one-shot des params legacy dans `process()`
  - [x] ST-7c : Sound Panel — widgets specials + Hz/Notes sur les atomics par slot (plus de ParamSetter)
  - [x] ST-7d : menus plock/morph — défauts specials, Snapshot, morph, toggle Display par slot
  - [x] ST-7e : ranges algo unifiés (`max_algo_index()`), renommés "Slot N Algo", fix ranges 0..0 crashogènes
  - [x] ST-7f : warnings nettoyés, 3 tests unitaires persistance v3/migration/reset, `cargo test` OK (106+72), `build.ps1 -Install` OK, CHANGELOG + AGENTS.md + CLAUDE.md + ADDING_AN_INSTRUMENT.md mis à jour (specials par slot, special_param() = migration only)
- [x] [ST-9] **Retours UI 2026-07-05** (build 20260705-122315, à valider dans S1) :
  - [x] Onglets fixes `Sound Editor` | `Track` — suppression des boutons par instrument (sélection de la lane via la grille, en-tête "Slot N - nom")
  - [x] Onglet Track complet : instrument, note MIDI, routing, Humanize, Push/Pull, Length
  - [x] Pastille `+N` → menu de choix parmi les 11 instruments à la création
  - [x] Fix lock de longueur de lane indexé par voix côté UI (aligné slot, comme l'audio)
- [x] [ST-5] **Layout 4 lanes au démarrage** : résolu par décision produit 2026-07-04 — le défaut EST maintenant 4 lanes (BD/SD/HH/Tom, `modular_default_layout()`), migration anti-template supprimée. ⚠️ Songs pré-`track-layout-v1` s'ouvrent en 4 lanes (build 20260704-195335).
- [x] [ST-8] **Règle UI zones stables** : la grille rend toujours 14 rangées (lanes actives + vides cliquables), rangée `+ Add Module` supprimée — plus aucune ligne conditionnelle qui décale les panneaux du bas (build 20260704-195335). Règle générale à respecter dans toute l'UI.
- [x] [ST-6] **Revalidation S1 après fixes** : instances BD indépendantes confirmées par l'utilisateur (2026-07-04) ; reste à re-vérifier après le passage au défaut 4 lanes : activation de chaque lane vide, 14 pistes, Out 14 audible.

## Plan d'action — Audit code review 2026-07-05

### [AUDIT-1] P0 — Eliminer les verrous Mutex bloquants sur le thread audio
- [x] Remplacer `PatternBank::lock()` par `try_lock()` dans `process()` (lib.rs:2750, 1954, 1981)
- [x] Si contention, reporter save/load/song au bloc suivant
- [ ] Option : double-buffer atomique + file SPSC UI→audio pour le song mode

### [AUDIT-2] P0 — Supprimer les allocations sur le thread audio
- [x] `Box::new` dans `reinitialize_slot()` (synthesis/mod.rs:846) → réutilise la `Box` existante du slot au lieu d'allouer/désallouer sur changement de kind
- [x] `Vec::with_capacity`/`push` dans `save_pattern_to_slot` (pattern_bank.rs:223) et `restore_from_buffers` (pattern_bank.rs:585) → tableaux fixes `[FusedGroup; MAX_FUSIONS]`
- [x] Conditionner `nih_log!` dans `process()` à `#[cfg(debug_assertions)]` (lib.rs:2012, 2036, 2066, 2083, 2100, 2116, 2290) + `println!` de `fire_voice_trigger`

### [AUDIT-3] Important — Corriger l'export MIDI 14 slots + note par slot
- [x] Itérer `0..MAX_TRACKS` au lieu de `INSTRUMENTS` (midi_export.rs:81)
- [x] Lire `track_layout[slot].midi_note` plutôt que `def.midi_note`
- [x] Ajouter un test couvrant le 14e slot et une note personnalisée

### [AUDIT-4] Important — Adapter le générateur aux kinds réels des slots (MG-10)
- [x] Mapper rôles musicaux par `kind.drum_voice_index()` / `track_layout`, pas par index de rangée (déjà en place via `remap_roles_to_slots`)
- [x] Ajouter `seed: u64` à `GeneratorParams` et rendre `generate()` déterministe
- [x] Ajouter des tests déterministes seedés : même graine = même pattern, graine différente = pattern différent, Kick mappé par kind (slot 1 et 13), aucun Kick quand le layout n’en a pas

### [AUDIT-5] Important — Tests mute/solo/mix routing
- [x] Extraire la logique de gating (effective_mutes/mix_gains) en fonction pure (lib.rs:2271)
- [x] Ajouter des cas : 1 mute, 1 solo, mute+solo, plusieurs solos, aucun


### [AUDIT-6] Suggestion — Tests round-trip synth settings
- [x] Généraliser le test de round-trip à toutes les voix (macro `settings_roundtrip_test!` dans `synthesis/settings/mod.rs`, appliquée à chaque fichier settings)
- [x] Corriger les défauts `special[]` des saturations non-entiers (Kick, Snare, HiHat, OpenHiHat, Ride) pour que le round-trip soit stable

### [AUDIT-7] Infrastructure
- [x] Committer `Cargo.lock` et le retirer de `.gitignore`
- [x] Supprimer `fix_roles.pdb` et les `.zip` redondants du suivi git
- [x] Retirer `.claude/settings.local.json` du suivi
- [x] Corriger docs : `13 voix/aux` → `14 slots`, `pattern-v1` → `pattern-v5`

### [AUDIT-8] Dette UI / qualité
- [x] Nettoyer échafaudage UI mort (`design_system.rs`, `StyledButton`, `allocate_ui_at_rect`) → tâche [100aa]
- [x] Renommage ports auxiliaires génériques `Out 1..14` — build 20260706-173427
- [x] Documenter invariants `// SAFETY:` dans `native_drag.rs` + test `build_hdrop_medium`

## Feedback utilisateur — 2026-07-05 post build 20260705-150850

### Bugs / régressions P1
- [x] [117] **P0 — Gros bug de son distordu lors de l'activation/désactivation d'une output dans le DAW** — corrigé côté écriture aux défensive (build 20260706-141836) + routing Track par slot (build 20260706-172704) + sorties auxiliaires exclusives par lane (build 20260706-175157) + mapping sparse VST3 Studio One (build 20260706-185857) + init synth sur layout courant (build 20260706-190624, à valider dans Studio One)
  - Reproduire dans Studio One : activer/désactiver une sortie auxiliaire du plugin pendant que le séquenceur joue.
  - Vérifier routing main/aux, buffers non activés, état de bus VST3 côté vendor nih-plug et écriture dans `aux.outputs`.
  - Attendu : aucune distorsion, aucun burst, aucun signal corrompu lors du changement d'activation de sortie.
  - Régression associée corrigée : changer `Track > Out` ne doit plus changer le son entendu ; le slot sélectionné est réellement routé vers la sortie choisie.
  - Régression associée corrigée : assigner un Tom à `Out 2` ne doit plus laisser un HH caché sur le même bus ; un `Out N` est maintenant exclusif à une lane.
  - Cause profonde corrigée : Studio One peut fournir des buffers auxiliaires compactés pour des sorties sparse ; le wrapper VST3 remappe maintenant ces buffers vers le vrai `Out N`.
  - Cause profonde corrigée : à l'activation, le synthé ne doit plus recréer le slot Tom avec la voix legacy OpenHH.
  - UX routing corrigée : la liste `Out` affiche `No Aux` au lieu de `Main`, car le Main Mix est déjà contrôlé par le switch `Main` (build 20260706-192033).
- [x] [103] **Régression : le drag & drop MIDI a disparu** — corrigé (build 20260707-091113)
  - Le bouton `Drag MIDI` est à nouveau sensible au glisser (`Sense::click_and_drag`).
  - Le helper OLE démarre automatiquement si le bouton gauche est déjà enfoncé.
  - L’export temporaire MIDI utilise le `track-layout` courant (14 slots + note par slot).

### Retours Studio One — 2026-07-07 post build 20260707-094444
- [x] **Ext MIDI : tête de lecture interne masquée** — corrigé (build 20260707-103907)
  - En mode `Ext MIDI` la grille ne surligne plus de step ; le playhead interne est gelé.
- [x] **Ext MIDI : flash visuel du `T` par lane** — corrigé (build 20260707-103907)
  - Chaque lane dont la note MIDI est reçue fait clignoter sa pastille `T`.
  - Couleur ajustée en AMBER/texte noir dans la build 20260707-111442.
- [x] **Export MIDI : swing/groove appliqué** — corrigé (build 20260707-103907)
  - Les fichiers MIDI exportés (Export + Drag) respectent le Swing et le Groove sélectionnés.

- [x] [118] **Morphing : Saturation Amount / Mix reveniennent à la valeur de base + cohérence tous instruments** — corrigé (build 20260707-155108)
  - Popup Morph élargi (284 → 350 px) et sliders réduits (104 → 96 px) pour éviter que les longs labels ne poussent le slider hors du cadre sur tous les instruments.
  - Clamp systématique de la valeur morph affichée/stockée à min..max (Volume, standard params, specials continus).
  - `morphable_fields()` inclut désormais les champs standard de type checkbox (ex. `Stereo`) pour correspondre au menu Morph ; test de régression ajouté.
  - Correction similaire s'applique à tous les instruments : Kick, Snare, HiHat, OpenHiHat, Tom1/2/3, Clap, Ride, Cymbal, Snare606, 808 Kick, Perc1, Zap.

- [x] [104] **Ligne avec le bloc Fusion décalée / perte de place** — corrigé (build 20260707-125743)
  - Revoir le placement du panneau Fusion sous la grille : il ne doit pas décaler inutilement les zones ni consommer de hauteur excessive.
  - La Fusion box (380 px) est maintenant affichée sur la même ligne que le sélecteur P-Lock Mode (Sound/Sequencer), alignée à droite. Elle ne pousse plus la Pattern Bank et le Bottom Panel vers le bas.
  - Fix : allocation de taille exacte (`allocate_exact_size`) pour que la hauteur de la box soit identique en idle et en édition, évitant tout saut de l’interface.
- [x] [105] **Plock sound : Frequency à 0 par défaut sur certains instruments** — vérifié + tests de régression (build 20260707-113932)
  - Probablement lié à [92] ; vérifier tous les instruments, notamment B8/HH/Tom et les slots dupliqués.
  - Attendu : le menu plock doit initialiser `Frequency` avec la valeur globale courante du slot/instrument, jamais `0` sauf si c'est réellement la valeur globale.
  - Résultat : les défauts du registre et le reset aux défauts du kind (`ST-3`/`ST-7`) garantissent une fréquence > 0 ; tests `default_frequency_is_nonzero_for_every_instrument_kind` et `duplicate_slots_keep_nonzero_default_frequency` ajoutés.

### UX grille / lanes P1
- [x] [106] **Retirer Hum et Push de l'onglet Track** — corrigé (build 20260707-164821)
  - Décision utilisateur 2026-07-07 : conserver `Humanize` et `Push/Pull` dans la grille pour l'instant, et les retirer seulement de l’onglet `Track`.
  - L’onglet `Track` garde `Instrument`, `Routing`, `MIDI Note` et `Length`.
- [x] [107] **Cellules hors longueur en pointillé** — corrigé (build 20260707-174844)
  - Quand `Len` global ou individuel est inférieur au maximum affiché, rendre les cellules non jouées en pointillé.
  - Attendu : distinguer clairement les steps visibles mais inactifs à cause de la longueur.
  - Résultat : les cellules hors longueur et les lanes non activées utilisent le même design inactif : fond très sombre + bordure segmentée épaisse/sombre.
  - Fix layout : le bloc `Len` global conserve une largeur fixe quand la valeur passe sous 10 ; l’indicateur `N steps` est dessiné dans un rectangle fixe pour que les boutons `16/32/48/64` ne se décalent plus.
- [x] [108] **Réarranger les lanes avec la poignée** — corrigé (build 20260708-162542)
  - Activer le drag de lanes via la poignée prévue dans le design.
  - Préserver instrument, paramètres, séquence, plocks, longueur, mute/solo/routing lors du déplacement.
  - Résultat : drag depuis la poignée vers une autre rangée active ou vide ; détection immédiate au clic via `is_pointer_button_down_on` + geste drag classique ; curseur Grab/Grabbing.
  - Feedback visuel : trait bleu horizontal (2 px) affiché à la limite exacte de la lane cible (haut de la ligne cible, ou bas de la dernière ligne pour le drop final), calculé par rapport au centre des lanes.
  - Déplacement complet de `track-layout`, pattern, fusions, plocks sound/seq, sound settings, algo, mute/solo/mix, Hum/Push/Len, locks et sélection UI.

### Presets / gestion lanes P1
- [x] [109] **Boutons de presets de lanes** — corrigé (build 20260708-145331)
  - Ajouter `Clear All Lanes`.
  - Ajouter `Preset 12 Lanes`.
  - Ajouter `Preset 4 Lanes`.
  - Vérifier que les zones UI restent stables : aucune ligne conditionnelle qui décale les panneaux.
  - Résultat : dropdown global `Preset` centré entre `Follow` et `Len` dans la page-bar avec marge dédiée avant `Len`, options `Clear All`, `Preset 4`, `Preset 12`; warning de confirmation avant application ; les presets ne sont plus dans l’onglet `Track`; tests `empty_layout_has_no_active_lanes` et `preset_12_layout_uses_core_legacy_kit_without_perc1` ajoutés.
- [x] [111] **Revoir le preset du Tom** — ajusté (build 20260707-120216)
  - Ajuster les valeurs par défaut Tom pour un rendu plus musical/utilisable dès création de lane.
  - Changements : Tom1 (lane Tom par défaut) **196 Hz** / 0.35 s / vol 0.7 / filter 600 Hz / release 0.25 ; Tom2 150 Hz / 0.30 s ; Tom3 100 Hz / 0.45 s. Stick Attack ramené à 0.3. `VoiceSettings::tom1/2/3()` alignés sur les défauts du registre.

### Song / Generator P1
- [x] [112] **Revoir le Song Editor, actuellement peu pratique** — corrigé (build 20260708-164626)
  - **Répétitions par step :** champ `repeats` ajouté à `SongSequence` (compatibilité `pattern-bank-v1` via `serde(default)`), moteur audio `lib.rs` lit `repeat_at()` et reste sur le step courant le nombre de boucles demandé avant d’avancer.
  - **Hauteur du panneau Song/Generator augmentée** de 144 px à 180 px pour accueillir une rangée d’inspection.
  - **Rangée d’inspection de la step sélectionnée :** dropdown `P1-P8` / vide, compteur de répétitions `1..64`, boutons `Copy` / `Paste` / `Dup` / `Clear`.
  - **Grille 4×16 conservée :** clic gauche sélectionne la step, clic droit contextuel `Copy / Paste / Duplicate / Clear`.
  - **Raccourcis globaux :** bouton `Reset` pour remettre la position song à 0, `Clear All` pour vider la song, `Len` via `DragValue` (0-64), `Loop` conservé.
  - **Suppression du toggle `Song Enabled` redondant ;** le playhead song est maintenant suivi dès que l’onglet `Song` est actif (`params.song_mode`).
  - **Reset automatique de la position song** quand on quitte le mode Song ou quand le transport s’arrête.
  - Tests ajoutés : `song_sequence_repeat_clamps_and_defaults`, `pattern_bank_legacy_load_without_repeats_defaults_to_one`, `pattern_bank_persistence_roundtrips_song` mis à jour avec les répétitions.
  - [x] Corrections UI post-build : dropdown d’inspection inscriptif, répétition affichée `P1xN`, step courant bleu lisible, hauteur de grille fixée pour les 4 rangées (build 20260708-171322).
  - [x] Refonte UX : 16 blocks fixes, onglet Song = vue uniquement, checkbox `Song Mode` dans le panneau, suppression de `Loop`/`Len`, cellule en deux parties (pattern en haut, repeat en bas), retour au début sur block vide (build 20260708-182802).
  - [x] Ajustement : panneau Song/Generator agrandi de 30 px (210 px total), suppression de la rangée d'inspection, édition directe du pattern et du repeat dans chaque block (build 20260708-183824).
  - [x] Polish : marge interne aux blocks pour éviter les débordements, blocks vides assombris, retrait de `Reset`, confirmation sur `Clear All` (build 20260708-185335).

- [x] [113] **Revoir le Generator : HiHats trop similaires entre styles** — corrigé (build 20260707-163927)
  - Les rôles HiHat sont maintenant différenciés par style : Rock 8ths, Funk offbeats, Techno/Metal/Disco 16ths, Hip-Hop sparse/swung, Jazz skip-beats, Latin clave-like, Trap dense rolls, Reggae one-drop.
  - Test `hihat_roles_are_style_specific` ajouté pour éviter un retour au motif quasi identique sur tous les styles.
  - Attendu : varier densité, accents, ouvertures, syncopes et probabilités selon style.

### Sound Editor / synthèse P2
- [x] [114] **Clarifier et enrichir HiHat / OpenHiHat** (build 20260709-121611)
  - Renommer `Frequency` → `Tone` (range 100–20000 Hz) et `Filter` → `Cutoff`.
  - Ajouter `Resonance`, `Noise Type` (White/Pink/Brown/Blue), `Shimmer`.
  - Supprimer l’algorithme `Bright` inutilisé.
- [x] [115] **Mettre Analog au milieu pour tous les instruments** (build 20260709-141013)
  - Création d’une famille `ParamFamily::Analog` dédiée.
  - Déplacement du champ `Analog` depuis `Output` vers la nouvelle section `Analog` pour tous les instruments.
  - Ordre UI : `Osc` → `Env` → `Analog` → `Filter` → `Sat` → `Output`.
  - Pour les instruments tonaux (Kick, Snare, Tom, Kick808, Perc1, Snare606), le slider `Analog` pilote `AnalogDrift` (pitch/level/time par hit).
  - Pour les instruments non tonaux (HiHat, OpenHiHat, Clap, Ride, Cymbal), le slider `Analog` module désormais le **tone** :
    - HiHat / OpenHiHat : `Tone` (centre du peaking filter), dérive **±25 %**.
    - Ride : `Frequency` (base des oscillateurs inharmoniques), dérive **±7.5 %**.
    - Clap / Cymbal : `Cutoff` / `Filter` (highpass cutoff), dérive **±25 %**.
  - `Zap` n’existe pas dans le code actuel ; la 13e voix est `Perc1`.

### Copier / coller lanes P2
- [x] [116] **Copier/coller une lane vers une autre** — corrigé (build 20260709-182258)
  - Menu contextuel sur le nom d'une lane active : `Copy Lane`, `Paste Lane`, `Paste Grid`.
  - Menu contextuel sur le nom d'une lane active : `Clear Grid` avec confirmation en deux clics.
  - Menu contextuel sur une lane vide : `Paste Lane` si un clipboard existe.
  - `Paste Lane` copie instrument, réglages sonores (standard + specials + Hz/Notes), algo, steps, fusions, sound plocks, seq plocks, Humanize, Push/Pull, Len et lock Len.
  - `Paste Grid` copie uniquement les steps on/off de la grille sur une lane active cible ; instrument, réglages sonores, algo, fusions, plocks, Hum/Push/Len, lock Len, routing, mute/solo/mix et note MIDI restent inchangés sur la cible.
  - `Clear Grid` efface uniquement les steps/fusions/sound plocks/seq plocks de la lane active ; instrument, réglages sonores, algo, Hum/Push/Len, lock Len, routing, mute/solo/mix et note MIDI restent inchangés.
  - Routing, Main/Out, note MIDI source personnalisée, mute/solo/mix ne sont pas copiés ; un changement d'instrument remet la note MIDI du slot cible sur le défaut du kind.

## Court terme (Stabilisation V1 — En cours)

- [x] **[DEBUG]** Routing `Out 1` silent in Studio One while Main Mix works
  - Check host output enable / aux routing
  - Review audio-thread routing code for off-by-one or output-activation issue

- [x] **[FIX]** New tracks silent + solo shared by instrument family + all UI interactions now track-based — rollback 20260701: redo with Studio One compatibility preserved

- [x] [69] Vrai fix du click parasite BD (changement de hauteur/plock) : chemin digital = reset de phase + crossfade cass� supprim�s ; phase reset�e au cold-start uniquement ; plancher d'attaque anti-click (MIN_AMP_ATTACK_MS) ; bug sweep digital +1 Hz corrig� (build 20260531-155232)
- [x] [70] Mode analog/digital BD re-rendu audible : digital = identique au bit pr�s, analog = drift par coup (hauteur �3.5 %, niveau �10 %, temps d'enveloppe �20 %)
- [x] [71] S�curis� les autres voix : perc1 (reset phase inconditionnel ? cold-start only), snare/tom/snare606 (reset digital ? cold-start only + enveloppes recr��es ? setters), hihat (enveloppe recr��e ? setters + biquad peaking recalcul� seulement si freq change). Plancher d'attaque + DC-blockers partout ; drift analog sur snare & tom (sliders expos�s) ; helper partag� `AnalogDrift`. ride/cymbal/clap/open_hihat/kick_808 d�j� click-safe, non modifi�s. (build 20260531-184528)
- [x] [71a] Ajout du drift analogique sur Snare606 + Perc1 (sliders Analog inactifs ? fonctionnels). Audit complet de tous les instruments avec slider Analog.
- [x] [72] Nettoyer les fichiers de cruft h�rit�s de la r�paration ui.rs + .gitignore
- [x] [81] **Bug P1 � Plock li� � la page et pas � la position grid** (FAUX POSITIF � code d�j� correct)
  - Le code utilise `global_step = page_offset + local_step` partout (affichage, clic, x2, copier/coller)
  - Les plocks sont bien index�s par step absolu (0-63) et suivent correctement la pagination
  - Tests confirment le bon fonctionnement des steps 16-63

## Nouveaux bugs & Feedback (Session 2026-06-01)

### Bugs P1 (Critiques � � traiter en priorit�)

- [x] [73] **Corruption caract�res UTF-8 r�currente** dans les boutons/texte UI (CORRIG� - build 20260601-163923)
  - Remplacement des �mojis corrompus (??, ??, ??, ??) par du texte ASCII (Link, Snapshot, Random, Clear)
  - Remplacement des symboles de navigation (?, ?, ?) par des caract�res ASCII (<, >, R)
  - Remplacement des s�parateurs box-drawing (-) et em-dashes (�) par des tirets simples
  - **Cause** : encodage UTF-8 ? Windows-1252 lors de manipulations PowerShell
  - **Pr�vention** : utilisation exclusive de caract�res ASCII dans les labels de boutons pour �viter les probl�mes d'encodage
- [x] [74] **Focus fen�tre plugin bloque Windows** � impossible de switcher vers une autre fen�tre (CORRIG� - build 20260601-170350)
  - Quand la fen�tre Flash Drum est ouverte, le focus revient automatiquement vers Studio One/Flash Drum
  - Bloque l'utilisation d'autres applications (navigateur, explorateur, etc.)
  - Potentiellement li� au workaround focus clavier (SetFocus sur HWND)
  - **Action** : identifier et corriger le hook/m�canisme qui force le focus
  - [x] Regression Studio One menus corrigee (build 20260609-094555) : le workaround clavier ne refocalise plus le VST a chaque frame hors saisie texte
- [x] [88] **Crash Studio One en manipulant le slider Master Volume dB** (CORRIGE - build 20260609-114803)
  - Cause probable : `master_volume` autorise `0.0` (`-inf dB`) mais utilisait `SmoothingStyle::Logarithmic`, incompatible avec un range passant par zero.
  - Correction : passage a `SmoothingStyle::Exponential(50.0)` pour conserver le lissage sans produire de valeurs non finies.
  - Test : `master_volume_smoothing_stays_finite_from_silence`.

### Bugs P1 (UI/UX)

- [x] [75] **Incoh�rence des ranges de volume** dans l'interface (CORRIG� - builds 20260601-171606, 20260609-152742, 20260609-160617)
  - Slider en haut du Sound Editor : affiche en dB (`-inf dB` a `+6.0 dB`), stockage interne gain lineaire `0.0..2.0`.
  - Slider dans la lane de la grille : courbe dB coherente, stockage interne gain lineaire `0.0..2.0`.
  - Ancien slider Volume data-driven en bas du Sound Editor supprime.
  - **Action** : uniformiser � 0.0�2.0 partout (coh�rent avec le gain de sortie)
  - Regression corrigee : Sound Editor garde uniquement le Volume du haut ; `StandardField::Volume` aligne a `0.0..2.0`.
  - UX corrigee : double-clic sur un volume local reset a `0 dB`.
- [x] [89] **Hauteur VST fixe pour eviter les sauts d'interface** (CORRIGE - builds 20260609-141438, 20260609-144118, 20260609-145809, 20260609-150545)
  - `EguiState::from_size` passe a `1480x800`.
  - `ResizableWindow::min_size` passe a `1480x800` avec `resizable(false)`.
  - Fix Studio One : `ResizableWindow::fixed_size(1480x800)` force la taille effective et bloque l'ancien auto-resize par contenu a `850px`.
  - Sound Editor : ajout d'un scroll interne pour les controles de synthese, avec le titre et les onglets instruments hors scroll.
  - Objectif : conserver une hauteur stable lors des changements d'instruments.

### Features P1 (Parit� PoC / Impact fort)

- [x] [76] **Longueur globale du pattern ajustable 1 ? 64 steps** (CORRIG� - build 20260601-175002)
  - 4 pages de 16 steps maximum
  - Pr�voir un switch "Follow lecture" (la grille suit le playhead ou reste fixe)
  - Complexit� : Moyenne-�lev�e
  - **Note** : cela implique de revoir la logique `SharedPattern` (actuellement 16 steps) et l'UI de pagination

### Features P2 (Am�lioration)

- [x] [77] **3 types de clicks pour la Bass Drum** (CORRIG� - build 20260602-174136)
  - Soft : click subtil, rond
  - Medium : click standard (actuel)
  - Hard : click agressif, transitoire pointu
  - Complexit� : Moyenne, 3-5 jours
  - **Fix bug** : `set_settings()` ne recr�ait pas le `ClickGenerator` quand `click_type` changeait
  - Valeurs exag�r�es pour diff�renciation audible (Soft/Medium/Hard)
- [x] [79] **D�placer le slider de longueur � c�t� de la pagination**
  - Slider "Len" retir� du header bar, positionn� avec les boutons de page
  - Ajout de boutons rapides 16/32/48/64
  - Ajout du bouton x2 pour doubler le pattern (avec copie des plocks)
  - Grisage du bouton x2 quand len > 32
- [x] [80] **LED rouge sous la page en cours de lecture**
  - Petit cercle rouge sous le bouton de page active dans le s�quenceur
  - Ind�pendant du highlight bleu de la page affich�e
- [x] [78] **Clarifier/documenter le mode Analog** � Document cr�� dans `docs/analog-mode.md`
  - Diff�renciation claire entre instruments avec drift op�rationnel (Kick, Snare, Tom, Cymbal, B8) et analog fix� (HiHat, OpenHH, Clap, Ride, Snare606, Zap)
  - Amplitude du drift document�e : Kick �3.5% pitch / �10% niveau, autres ~7.5% pitch max
  - Valeurs par d�faut 0.3 vs 1.0 expliqu�es
  - Recommandations par style musical et conseils de d�pannage inclus
  - La section "Analyse Technique (Reference)" de ce fichier reste disponible pour les d�tails d'impl�mentation
- [x] [67] Positionner le volume en haut du sound editor + ajouter un controle de volume sur chaque lane de la grille (Complexité: Faible, P1)
- [x] [68] Couleurs differentes pour plock link global vs full snapshot (orange / rouge) pour distinguer visuellement les modes (Complexité: Faible, P1)
- [x] [55] Ameliorer le rendu Snare 606 (plus percutant, plus proche TR-606)
  - raw noise excite le resonator directement
  - snap envelope ultra-court (0.2ms attack, 3ms decay)
  - defaults ajustes : decay 0.25s, filter_freq 8000Hz, tone 0.4, snap 0.6
- [x] [1] Corriger la double instanciation du `Sequencer` dans `lib.rs` (lignes 250 + 256)
- [x] [2] Revalider dans Studio One la sauvegarde/reouverture d'une song avec grille modifiee (build `20260511-091259`)
- [x] [3] Tester un projet Studio One neuf : insertion, sorties Kick/Snare/HH/Open HH/Toms, audio sur chaque bus, sauvegarde/reouverture
- [x] [4] Verifier que le Main Mix reste audible quand les sorties separees sont activees
- [x] [5] Verifier que mutes/solos affectent correctement Main Mix et sorties separees

## Refactoring & Qualite de code

- [x] [6] Remplacer `Vec<Box<dyn Voice>>` par une `enum` pour eliminer le dynamic dispatch dans le moteur audio
- [x] [7] Nettoyer les parametres legacy `st01` a `st16` dans `DrumFlashParams` (garder uniquement la logique de migration dans `filter_state`)
- [x] [8] Corriger le warning clippy dans `test_standalone.rs` (needless_range_loop)
- [x] [9] Passer `cargo clippy --all-targets` proprement sans warnings

## Tests & Validation

- [x] [10] Tester le plugin dans au moins un autre DAW (Reaper recommande)
  - chargement OK dans Reaper
  - sauts de volume initialement suspectes : RMS du plugin mesure stable a ~0.4 dB pres
  - confirme reproduit avec un autre plugin dans Reaper → pb driver audio, plugin innocent
- [x] [10a] Corriger les defaults de decay (snare 0.47, hihat 0.36, open_hh 0.66)
- [x] [10b] Corriger le choke qui ne fonctionne plus
- [x] [10c] Corriger les step skips rares (sync_to_host moins agressif)
- [x] [10d] Remplacer sliders par checkbox/combobox pour paramètres bool/enum/algos
- [x] [13] Verifier la precision du timing du sequencer (compteur d'echantillons vs transport hote, correction continue)

## Fonctionnalites P1 (Parite PoC — Impact fort)

- [x] [14] Editer les reglages de synthese par instrument dans l'UI (frequence, decay, volume, filter)
- [x] [15] Connecter `filter_freq` dans `SnareVoice` (actuellement ignore)
- [x] [16] Ajouter un bouton "Test" par instrument pour declencher le son isole
- [x] [17] Ajouter export MIDI fichier depuis le plugin
- [x] [17a] Corriger l'export MIDI fichier/drag-drop pour inclure les 13 instruments
  - `midi_export.rs` utilise encore deux tableaux `midi_notes` hardcodes a 12 notes
  - deriver les notes depuis `instrument_registry::INSTRUMENTS` ou une constante unique partagee avec `MIDI_NOTE_MAP`
  - verifier que Perc1 (note 37) est exporte en fichier MIDI et dans `export_pattern_to_midi_bytes`
  - ajouter un test unitaire couvrant au moins le 13e instrument
- [x] [18] Ajouter sortie MIDI temps reel vers hardware externe
- [x] [19] Ajouter la generation de pattern aleatoire (grille + option Random BPM + option Random Sounds)

## Fonctionnalites P2 (Post-V1 — Nice to have)
- [x] [66b] Correction focus clavier Windows (SetFocus sur HWND plugin) � build 20260529-124136\n
- [x] [20] Ajouter swing
- [x] [21] Ajouter un facteur de groove parametrable (Straight/Swing16/Shuffle/MPC)
- [x] [22] Ajouter un parametre analogique pour legeres variations aleatoires (humanize per track)
- [x] [23] Permettre un mode stereo analogique avec variation gauche/droite (push/pull per track)
- [x] [24] Ajouter song mode (placeholder UI P1-P8, backend à câbler)
- [x] [24a] Ajouter modularité des instruments (algos + special params)
- [x] [24b] Ajouter Kick algos (Sine/Square/FM) + click transient
- [x] [24c] Ajouter Snare algos (Synth/Noise/Layered) + snap param
- [x] [24d] Ajouter Clap, Ride, Cymbal voices
- [x] [25] Labels complets des instruments dans l'UI et couleurs par instrument (labels courts BD/SD/HH, couleurs blocs de 4 steps, grisage len)
- [x] [26a] Per-instrument Mix Bus checkbox (exclure du Main Mix)
- [x] [26b] Clap Echo plockable par step
- [x] [26c] Masquer les paramètres inutiles par instrument dans le Sound Panel
- [x] [26d] Nouvel instrument B8 (TR-808 Bass Drum)
- [x] [26e] Slider Analog actif pour B8
- [x] [26f] Release fonctionnel pour B8 (DecayReleaseEnvelope)
- [x] [26g] Attack ramp 1.5 ms sur B8 (élimine click de démarrage)
- [x] [26h] Filtre LP dédié Click Tone sur B8 (100-8000 Hz)
- [x] [26i] Plock B8 fix : special params (accent/snap/pitch_drop/click_tone) stockés
- [x] [26j] Finaliser les réglages de synthèse par instrument
  - `standard_params` data-driven avec ranges UI (min/max, log, suffix, checkbox)
  - Ranges corrigés pour éviter le clamp involontaire (ex: Ride decay 1.2s > slider 0.5s)
- [x] [51] Ajouter un paramètre Attack réglable par instrument (graphique AHDSR complet A-H-D-R)
- [x] **[EN COURS � Phase 1a OK]** [66] Presets d'instruments — sauvegarder/charger des réglages de synthese par voix (Complexité: Moyenne, P2)
- [x] [26k] Refonte UI Phase 1 (grid intégré, sound panel ongleté, auto-edit)
  - Sound Panel regroupé par familles data-driven (OSC/ENV/FILTER/OUTPUT)
  - Visualisations interactives d'enveloppe (Amp AHDSR + Filter Env)
  - Layout horizontal : params à gauche, graph à droite
- [x] [26l] Corriger le toggle stereo pour certains instruments
  - exposer/finir les toggles stéréo pour les voix où la largeur apporte une vraie valeur : Snare, HiHat, OpenHH, Clap, Ride, Cymbal, Snare606, Perc1
  - garder Kick, B8 et Toms mono par défaut pour préserver le centre et la compatibilité mono
  - priorité technique : finir stereo Snare606 et vérifier que les toggles UI ne sont visibles que sur les voix concernées
- [x] [55] Saturation / distortion par instrument (tous les 13 voix)
  - Module `saturation.rs` avec 5 algorithmes distincts (SoftClip, Valve, Transistor, HardClip, Tape)
  - Paramètres exposés dans le Sound Panel : Type, Amount, Mix, Output Gain, Pre-Filter
  - Drive d'entrée mappé 1×..20× pour effet audible
  - Pre-Filter comme checkbox toggle (post-filter par défaut)
  - Section SAT dédiée dans le Sound Panel (ParamFamily::Saturation)
  - Combobox affichant les noms d'algorithmes (SoftClip, Valve, etc.)
  - Saturation appliquée sur 8/13 instruments : Kick, Snare, Snare606, B8, Tom1-3, Perc1
  - ~~Saturation sur les 5 restants (HiHat, OpenHH, Clap, Ride, Cymbal)~~ — pas prioritaire
  - Special params augmentés de 8 à 32 slots (`special: [f32; 32]`)
  - Plock field masks passés de u32 à u64 (46 fields total, 32 special params plockables)
  - Auto-edit activé par défaut
- [x] [62] Cymbal : retirer frequency inutilisé, ajouter Shimmer Freq + Noise Type
  - `frequency` retiré du Sound Panel (paramètre inutilisé sur un bruit)
  - `Shimmer Freq` (1-50 Hz) : module la fréquence du FM shimmer (était hardcodé à 15 Hz)
  - `Noise Type` : White / Pink / Brown / Blue — générateurs Voss-McCartney dans dsp.rs
  - Combobox UI pour sélectionner le type de bruit
- [x] [63] Bug B8 se coupe quand on modifie CY : corrigé division par zéro dans `ExpDecayEnvelope::set_attack_ms`
  - Quand attack_time passe à 0 pendant un ramp actif → snap à peak immédiat pour éviter NaN
  - Bouton "T" (Test) : appelle maintenant `set_voice_settings` avant `trigger`
- [x] [82] **Intégrer les éléments graphiques définis avec Claude Design**
  - Phase 1a : Widgets custom (ToggleLED, ToggleSwitch, StyledButton, SegmentedControl)
  - Phase 1b : Header redesign (fond PANEL, bordure LINE, séparateurs DIVIDER, padding 14px)
  - Phase 1c : Style global sombre (BG, PANEL2, P_HOVER, P_ACTIVE, BLUE via egui::Visuals)
  - Phase 1d : Boutons page 1-4 stylisés + glow LED lecture
  - Phase 2 reprise : layout fixe 2 colonnes, grille custom, Sound Editor et panneaux bas rapprochés du design pack (builds 20260611-184532, 20260611-194657, 20260611-201611)
  - Assets UI (icônes, couleurs, fonts, layout) produits par Claude Design
  - Remplacer les widgets egui basiques par des widgets custom avec le design system
  - **Complexité : Moyenne-Élevée, 1-2 semaines, P2**

## Fonctionnalites P3 (Avancees / Complexes)

- [ ] [69] Creer un instrument percussif a base de wavetables — phase recherche et prototypage (Complexité: Élevée, 2-4 semaines, P3)
- [ ] [27] Generation IA de patterns par style (rock, techno, rap, jazz, reggae, metal, funk, latin, disco, trap)
- [ ] [83] **Instruments sampler TR-606 multisamplé (x4 layers)**
  - Nouveau type de voix "Sampler" avec 4 variations par instrument (multisample)
  - Sélection aléatoire du layer à chaque trigger pour simuler l'imperfection analogique
  - Nécessite un système de chargement de samples WAV en mémoire préallouée
  - Architecture : voix sampler hybride (peut coexister avec les voix de synthèse actuelles)
  - **Complexité : Élevée, 3-4 semaines, P3**
- [ ] [84] **Instruments sampler Yamaha RX11**
  - Même architecture sampler que [83] avec le kit RX11
  - 4 layers par son pour l'effet analog random
  - Dépend de [83] (infrastructure sampler)
  - **Complexité : Moyenne, 2-3 semaines, P3**
- [x] [28] Drag & drop MIDI directement vers le DAW — helper externe validé dans Studio One
  - [x] remplacer l'ancien `dnd_set_drag_payload(bytes)` interne egui par un drag fichier OS natif Windows (`CF_HDROP` via OLE `DoDragDrop`)
  - [x] garder l'export fichier OK via bouton MIDI dans `Documents/Flash Drum/exports`
  - [x] isoler `DoDragDrop` hors process DAW via `drum-pattern-midi-drag-helper.exe`
  - [x] réactiver le bouton `Drag` : export MIDI puis ouverture d'une petite poignée de drag externe
  - [x] valider dans Studio One : cliquer `Drag`, puis glisser la fenêtre `Drag MIDI` vers une piste/instrument et vérifier qu'un clip MIDI est créé sans crash
- [x] [29] Parameter locks (plocks) façon Elektron — changer un paramètre de synthese par step
  - 14 champs plockables (12 sound settings + clap_echo + algo)
  - special params (accent/snap/pitch_drop) propagés uniquement au trigger (fix echo perdu)

## Nouveaux éléments (À prioriser)

- [ ] [56] Ajouter une percussion de type Tom Simmons (Complexité: Moyenne, 3-5 jours)
  - Créer un nouveau module de synthèse
  - Ajouter l'instrument dans le registre des instruments
  - Créer les paramètres spécifiques et l'interface utilisateur
  - Intégrer dans le système de mixage et de sortie audio

- [ ] [57] Créer un séquencer modulaire avec instruments dynamiques (Complexité: Élevée, 4-6 semaines) — **EN COURS V1.5 (voir tâches MG-*)**
  - Refonte majeure de l'architecture du séquencer
  - Système de tracks fixes à 14 slots, actives visibles dans l'UI
  - Gestion de l'ajout/suppression d'instruments à chaud
  - Interface utilisateur pour la configuration modulaire
  - Système de sauvegarde/restoration des configurations

- [x] [58] Gestion des patterns et song (Complexité: Moyenne-Élevée, 3-5 semaines) � **FAIT (build 20260604-175459)**
  - [x] Pattern bank P1-P8 : sauvegarde/chargement de patterns complets (grid + plocks + seq plocks)
  - [x] **Stabilisation pattern bank** (build 20260604-170055) : preallocation buffers, pas d'alloc audio thread, pas de panic mutex
  - [x] Song mode : chaînage séquentiel des patterns via `SongSequence` (64 steps max)
  - [x] **Refonte UX Pattern Bank** (build 20260604-175459) : boutons Save/Load explicites, indicateurs d'occupation, tooltips, sync pattern_length au load
  - [x] UI Song Editor : grille de steps avec sélection P1-P8, loop toggle, longueur ajustable
  - [x] Persistance DAW : `SongSequence` intégrée à `PatternBank` (champ `pattern-bank-v1`)
  - [x] Playback : détection de wrap via `loop_count`, avance auto song_position, chargement pattern

- [x] [59] **Gestion des plocks de type séquenceur � COMPLET** (FAIT build 20260603-205246)
  - [x] Architecture `SequencerPlockState` lock-free (probabilité, stutter, condition, microtiming)
  - [x] Switch UI "Plock mode: Sound / Sequencer" sous la grille
  - [x] Menu contextuel adaptatif (mode Seq = paramètres séquenceur)
  - [x] Probabilité 0-100% par step � instrument
  - [x] Skip aléatoire dans le callback audio (LCG)
  - [x] Persistance DAW (`seq-plock-v1`)
  - [x] **Phase 2:** Couleurs violet pour les plocks séquenceur (visibles uniquement en mode Seq)
  - [x] **Phase 3:** Switch avec label "Plock mode" + couleur orange (Sound) / violet (Sequencer)
  - [x] **Phase 4:** Stutter (1-8x) � déclenche multiple fois le son
  - [x] **Phase 5:** Conditions (Always, 1st, Not 1st, 1/2, 2/2, 1/3, 2/3, 3/3, 1/4, 2/4, 3/4, 4/4)
  - [x] **Fix build 20260603-211721:** `SequencerStepParams::default()` probability=1.0, stutter_count=1 ; retrait Fill/NotFill ; stutter avec espacement temporel

- [x] [60] Désactivation du séquenceur interne et pilotage MIDI depuis le DAW (Complexité: Moyenne) — **FAIT (build 20260604-141711)**
  - [x] Ajout d'un paramètre `use_internal_sequencer` (ID: `int_seq`) pour activer/désactiver le séquenceur interne
  - [x] Mode "MIDI thru" : les NoteOn reçus déclenchent les voix via `instrument_registry::voice_idx_from_midi_note()`
  - [x] Mapping complet GM Drum Map pour les 13 voix
  - [x] UI: checkbox "Seq" dans la header bar

- [x] [61] Pour les BD, ajouter un switch de tuning entre Hz et Notes (Complexité: Faible, 2-3 jours)
  - Ajouter un paramètre booléen pour basculer entre les modes de tuning
  - Implémenter la conversion Hz ↔ Notes (standard MIDI)
  - Mettre à jour l'interface utilisateur pour afficher le bon format
  - S'assurer que la valeur est correctement sauvegardée/restaurée
  - Appliquer aux instruments Kick et B8 (et potentiellement autres bass drums)
- [x] [61b] Ajouter copier/coller un plock dans le menu bouton droit � **FAIT (build 20260603-142316)**
  - Stocker le plock copié dans l'état de l'éditeur (`EditorUIState.plock_clipboard` via `SinglePlockClipboard`)
  - Boutons "Copy Plock" / "Paste Plock" dans le menu contextuel de la grid
  - Coller écrase le plock existant sur la step cible
  - Support multi-instrument : on ne colle que si l'instrument correspond
  - Disponible à la fois en mode création (step vide) et édition (step avec plock)
- [x] [29a] Refactor plock UI data-driven depuis `instrument_registry`
  - remplacer les branches hardcodees par instrument dans `draw_plock_menu`
  - exposer automatiquement les `special_params` de Clap, Snare606, B8, Perc1 et futurs instruments
  - aligner les champs plock stockes/lus (`FIELD_COUNT = 18`) avec les special params reels
  - clarifier/corriger l'incoherence Clap Echo : UI lit le champ 12 alors que `PlockState::set_settings()` stocke les specials en 14..17
  - ajouter tests unitaires sur `PlockState::set_settings/get_settings` pour Clap Echo, B8 specials et Perc1 specials
- [x] [39] Refactor : paramètres dédiés par instrument (au lieu du `VoiceSettings` partagé + `special[8]`). Permet labels, ranges et défauts spécifiques par voix.
  - [x] Prototype Kick : `KickSettings` struct typée, conversion `VoiceSettings ↔ KickSettings`, tests passents
  - [x] Généraliser aux 12 autres instruments (Snare, HiHat, OpenHH, Tom1-3, Clap, Ride, Cymbal, Snare606, B8, Perc1)
- [x] [40] Filter envelope (cutoff modulé par AD/ADSR) — Kick, Snare, Tom, HiHat, Snare606
- [ ] [41] Émulation circuit-exact TR-606 (WDF, modèle non-linéaire VCA, oversampling) — vs grey-box actuelle
- [x] [54] Saisie clavier de valeurs précises + Shift+mouse pour affiner les sliders de paramètres
  - LocalParamSlider créé pour remplacer egui::Slider dans les plocks et paramètres spéciaux
  - Shift+drag implémenté pour le fine-tuning sur tous les sliders
  - Hauteurs de sliders harmonisées pour une expérience visuelle cohérente

## Dette technique & Documentation

- [x] [30] ~~Clarifier si `index.js` doit etre conserve ou archive~~ � **ARCHIV�**
  - Les fichiers `index.html` et `index.js` (PoC web React) ont �t� d�plac�s dans `archive/web-poc/`
  - Le plugin VST3 est d�sormais le seul produit actif
- [x] [31] ~~Revoir l'organisation du repo pour separer clairement PoC web et plugin~~ � **FAIT**
  - Le PoC web est archiv� dans `archive/web-poc/`
  - La racine du repo contient uniquement le plugin (`drum-pattern-vst/`), la doc et les fichiers de suivi
- [x] [31a] Clarifier l'emplacement des docs produit actives
  - `AGENTS.md` cite `PROJECT_BRIEF.md` et `BACKLOG_VST.md`, mais les fichiers presents sont sous `docs/historique/`
  - decider si ces docs doivent revenir a la racine, etre remplacees par `TODO.md`/`README.md`, ou etre explicitement marquees comme archivees
  - mettre a jour `README.md`, `AGENTS.md` et les references croisees en consequence
- [x] [32] Synchroniser `BACKLOG_VST.md` avec `TODO.md`
- [x] [33] Reduire les warnings Rust inutiles (0 warning sur lib + bin + tests, release inclus)
- [x] [34] Garder les fichiers de sauvegarde hors de `src/` — Dossier `drum-pattern-vst/backups/` créé, `.gitignore` déjà configuré
- [x] **[87] Step Fusion V2** — Fusion de cellules pour tuplets/micro-rhythmes (build 20260607-131747)
  - **Spécifications:**
    - Shift+clic début → Shift+clic fin = sélection plage à fusionner
    - Double-clic sur groupe fusionné = édition inline du nombre de steps
    - Limites: 1-64 steps, par instrument, indépendant par ligne
    - Générateur/Clear: suppriment les fusions (reset)
    - Plocks: appliqués par cellule de départ (tous les pulses partagent le même plock sonore)
    - Stutter seq-plock: désactivé/ignoré sur une fusion
  - **Implémentation:**
    - [x] Data model `FusedGroup { start_cell, end_cell, step_count }` par instrument
    - [x] UI: grille fixe 16 colonnes/page, Shift+clic sélection page-local, double-clic édition pulses
    - [x] Séquenceur: cellule de départ uniquement, cellules internes supprimées, métadonnées fusion vers audio
    - [x] Audio: pulses régulièrement espacés sur la durée de la fusion via queue préallouée
    - [x] Rendu visuel: cellules fixes avec bordure/couleur fusion + texte "pulses/cells" sur la cellule de départ
    - [x] Fix UI build 20260608-190515: rendu en vrai bloc graphique unique, sans subdivisions internes visibles
    - [x] Fix UX build 20260608-191352: style aligne cellules standard, edition inline du nombre de pulses, creation active par defaut
    - [x] Fix build 20260608-192613: creation de fusion supprime les plocks sound/seq des cellules internes couvertes
    - [x] Fix UI build 20260608-193357: indicateur "Maj for fusion mode" gris/bleu + "Select 2 cells" sous la grille
    - [x] Fix build 20260608-194139: detection Maj robuste via Win32 `GetAsyncKeyState()` pour l'indicateur et Shift+clic fusion
    - [x] Fix build 20260608-195857: `Copy/Paste Page` et `x2` copient aussi les groupes Step Fusion ; `Clear Page` supprime les fusions de la page
    - [x] Fix UI build 20260609-100205: edition inline du nombre de pulses sans decalage de ligne ; clic exterieur ferme l'edition et garde la fusion active
    - [x] Fix UI build 20260609-102249: panneau `Fusion x-y (cells) Steps` deplace dans une box Fusion reservee stable ; clic sur son champ `Steps` ne ferme plus l'edition
    - [x] Fix UI build 20260609-112628: double-clic sur cellule fusionnee traite avant le clic simple, ouvre l'edition sans desactiver la fusion et sans toggle differe
    - [x] Fix UI build 20260609-121512: premier Maj+clic de fusion colore le point central de la cellule source en bleu ; relacher Maj annule la selection et restaure la couleur normale
    - [x] Fix UI build 20260609-124302: cellule source de selection Fusion rendue comme active temporaire (`X` + fond bleu clignotant + bordure bleue) pour etre plus visible
    - [x] Fix UI build 20260712-103414: valider le champ `Steps` avec `Enter` applique la valeur 1..64 puis ferme l'edition inline de la fusion
    - [x] Fix UI build 20260712-104124: regression freeze Studio One apres `Enter` corrigee en relachant le focus clavier et en fermant l'edition a la frame suivante
    - [x] Fix UI build 20260712-110414: remplacement du `TextEdit` par un `DragValue` natif egui pour le champ `Steps`; validation `Enter` geree en interne sans freeze
    - [ ] Persistance DAW (champ `fusion-v1`)
    - [x] Tests: filtrage invalides + suppression triggers internes + métadonnées pulses
- [x] [34a] Corriger le click de retrigger kick (2 steps BD proches)
- [x] [34b] Nettoyer le code mort dans `special_params.rs` (struct `SpecialParamDef`, tous les `*_SPECIALS`, helper `specials_for`, methodes trait `supported_algos`/`special_params`)
- [x] [34c] Corriger les libelles obsoletes multi-out dans le code
  - `AUX_OUT_COUNT` vaut 13 mais `lib.rs` parle encore de "10 stereo drum outs"
  - corriger le commentaire "Frozen at 10" et le `PortNames.layout`
  - verifier que la doc Studio One reste alignee avec Main Mix + 13 sorties aux

## Bugs a corriger
- [x] [70] Kick : click de retrigger quand la queue percute l'attaque du suivant � corrig� (ne pas retrigger le click pendant la tail) � build 20260529-172133\n
- [x] [64] Revoir l'algo de polyrythmie (lane length) — comportement bizarre, longueurs mal synchronisées (Complexité: Moyenne, P1)
  - Build 20260609-185930 : fix affichage valeur effective dans l'UI.
  - Par defaut suit Pattern Length. Drag = verrouille sur cette valeur.
  - Si Pattern > valeur verrouillee → garde valeur (polyrythmie).
  - Si Pattern <= valeur verrouillee → suit pattern (trop court).
  - Clic droit = deverrouille. Persistance DAW via `lane-locks-v1`.
- [x] [65] Revoir les algos de generation pattern avec les nouveaux instruments (13 voix) — tous les générateurs gèrent 13 instruments; rôles musicaux enrichis pour Snare 606, B8, Perc1 dans le style Rock (démonstration)
- [x] [45] Sauts de volume general dans Reaper — diagnostique externe (driver audio, reproduit avec d'autres plugins)
- [x] [46] Revert du code Perc1 au commit 5ae1286 (Zap) — build stable réinstallé
- [x] [47] Refaire Perc1 proprement : ne pas recréer les enveloppes dans `set_settings`
- [x] [48] Refaire Perc1 proprement : utiliser `DecayReleaseEnvelope` pour le slider Release
- [x] [50] Diagnostiquer pourquoi la moitié des paramètres Perc1 ne sont pas actionnables (faux positif — tests unitaires confirment que decay/release fonctionnent)
- [x] [49] Refaire Perc1 proprement : rendre le plock menu data-driven (plus de hardcode par index)
- [x] [53] Plock Snapshot vs Link : choix à la création du plock (snapshot fige tout, link ne stocke que les champs modifiés)
- [x] [35] Diagnostiquer la sauvegarde/reouverture Studio One
- [x] [35a] Plock B8 : accent/snap/pitch_drop/click_tone plockables
- [x] [36] Corriger la persistance de grille via `pattern-v1`
- [x] [37] Migration legacy depuis les parametres caches `st01` a `st16`
- [x] [38] ~~Ecart entre documentation et code reel a surveiller~~ � **DOCUMENTATION � JOUR**
  - `README.md` : mis � jour avec la structure du repo (archive/web-poc/)
  - `docs/infrastructure.md` : cr�� � guide build, architecture, tests, d�ploiement
  - `docs/user-guide.md` : cr�� � guide utilisateur complet (UI, plocks, export, multi-out)
  - `docs/analog-mode.md` : cr�� pr�c�demment � documentation technique du mode Analog
- [x] [38b] Supprimer les `unwrap()` evitables du chemin audio/UI sensible
  - `lib.rs::process()` utilise `DrumVoice::from_index(...).unwrap()` sur des index bornes par `DrumVoice::COUNT`
  - risque faible aujourd'hui, mais non conforme a la regle stricte "audio thread sans panic"
  - remplacer par API interne sans `Option`, ou par branche defensive sans panic
- [x] [38a] Fusionner `CLAUDE.md` dans `AGENTS.md` (13 instruments, AUX_OUT_COUNT = 13, Zap ajouté)
- [x] [42] Crash a l'instanciation avec 11e voix (cause: `IntRange { min:0, max:0 }` → div par zéro nih-plug)
- [x] [43] Index out of bounds dans UI (`hums`/`pushes`/`lengths` taille 10 vs INSTRUMENT_LABELS taille 11)
- [x] [44] Step mask hardcode `0x3ff` (10 bits) — extensible via `INSTRUMENT_COUNT`

## Bugs a corriger (Nouveaux)

- [x] **[86] Plocks restent du pattern precedent au changement de slot** (CORRIGE — build 20260605-094135)
  - **Symptome :** charger un pattern depuis P1→P2 laissait les plocks de P1 visibles
  - **Cause double :**
    1. `restore_from_buffers()` skipait le restore si `plock_bytes.len()` < taille attendue (FIELD_COUNT=46). Les anciens slots (FIELD_COUNT=18) n'etaient pas restaures
    2. Aucun `clear_all()` avant restauration → plocks residuels du pattern precedent persistaient
  - **Fix :**
    - Detection automatique du format (18 vs 46 fields) dans `restore_from_buffers()` et `PatternSlot::restore()`
    - `PlockState::clear_all()` + `SequencerPlockState::clear_all()` : vident tous les plocks avant restauration
    - `load_pattern_from_slot()` appelle `clear_all()` sur les deux types de plocks avant `restore_from_buffers()`

- [x] **[85] CRASH — Retour � P1 apr�s avoir sauvegard� 2 patterns fait crasher Studio One** (CORRIG� — build 20260605-090814)
  - **Cause :** `MAX_PLOCK_BYTES` utilisait `18` (ancien `FIELD_COUNT`) au lieu de `46` (actuel)
  - Buffer sous-allou� de 66 664 → 159 848 bytes ; `copy_data_for_restore()` overflow → crash
  - **Fix :** calcul dynamique depuis `FIELD_COUNT`/`INSTRUMENT_COUNT`/`STEP_COUNT`, plus de hardcode
  - **S�curit� :** `copy_data_for_restore()` prot�g� par `.min()` pour �viter tout overflow futur

- [x] [71] Longueur globale du pattern ajustable 1 => 64 avec 4 pages de 16 steps max. Prevoir un switch de follow de la lecture ou pas (Complexite: Moyenne-Elevee, 1-2 semaines, P1) � **DOUBLON de [76], D�J� CORRIG�**
- [x] [72] Probleme d'affichage du volume : slider en haut de l'editor (1.5 max) et en bas (1) et dans la lane (1.5) � incoherence de range a uniformiser (Complexite: Faible, 1-2 jours, P1) � **DOUBLON de [75], D�J� CORRIG�**
- [x] [73] caracteres esoteriques ont remplace aleatoirement les caracteres normaux dans les boutons/texte UI � CORRIGE (restauration UTF-8 via script Python) � build 20260529-174106 (Complexite: Faible, 1 jour, P1)
- [x] [74] Proposer 3 types de clicks pour la BD (Kick) : soft/medium/hard ou impulse/noise/transient (Complexite: Moyenne, 3-5 jours, P2)

## Bugs a corriger (Actifs)

- [x] [102] **Song mode : changement de pattern de longueur différente ne remet pas la tête à zéro** (P1, Sequencer/Song)
  - Constat utilisateur 2026-07-05 : en song-mode, après un pattern dont la longueur diffère des autres, la tête de lecture continue au lieu de repartir à step 0.
  - Cause : le load de slot envoyait la longueur à l'UI via `pending_pattern_length`, mais l'audio utilisait encore temporairement l'ancien paramètre ; la resynchro hôte pouvait ensuite recaler la position sur la timeline DAW absolue.
  - Correctif : longueur audio-local mise à jour immédiatement au load, redémarrage song programmé à step 0 après transition, resync hôte continue désactivée pendant le song-mode.

- [x] [101] **Regression Push/Pull apres correction playhead** (P1, Sequencer/UI)
  - Constat utilisateur fin de session 2026-06-12 : avec du Push, le decalage devient enorme et impossible a annuler correctement.
  - Dernier changement suspect : build `20260612-210534`, UI playhead decouplee de `current_steps` et basee sur `current_step` global.
  - Correctif (build `20260613-105028`) :
    - `sync_to_host` recalcule `step_counter` depuis la timeline shifted (position hote - push/pull) au lieu de la timeline master.
    - UI grille : playhead alignee sur `current_step` global ; Push/Pull ne deplace plus l'anneau de lecture, seul le timing audio est decale.
  - Objectif atteint : timing audio Push/Pull correct et annulable, playhead visuelle stable quand on module Push.

- [x] [91] **Sortir automatiquement du mode edit quand on selectionne en dehors de la cellule** (P1, UI/UX)
  - Actuellement, le mode edit reste actif meme si on clique ailleurs
  - Comportement attendu : deselection de la cellule = sortie du mode edit
  - Complexite : Faible
  - Correctif (build `20260623-120806`) : lors d'un clic normal, si le clic ne porte pas sur le groupe fusionné en cours d'édition, `finish_fusion_editing_for_ui` est appelé avant de traiter le toggle.

- [x] [92] **Valeurs du menu plock sound par defaut = valeurs globales de l'instrument** (P1, Donnees) — résolu par ST-7 + tests de régression (build 20260707-113932)
  - Constate : la frequence de BD8 (BassDrum808) est a 0 dans le plock au lieu de la valeur globale
  - Verifier que tous les instruments initialisent correctement les valeurs par defaut des plocks
  - Complexite : Faible

## [100] Redesign UI complet (design pack 2026-06-11) — EN COURS

> **Livrable designer** : `design-pack/Flash_Drum_design_11062026/flash-drum-source/`
> Fichiers clés : `DESIGN-SYSTEM.md` (tokens), `LAYOUT.md` (architecture), `assets/fd-data.js` (schémas moteurs)

### Architecture (invariants du design)
- **Système de lanes modulaires** : 4 lanes au départ (BD/SD/HH/TOM), ajoutables jusqu'à 14, réordonnables par drag
- **Registre de moteurs** : Synth (kick/snare/tom/hat/cymbal/clap/perc), Sample, Sample FX, MIDI Out
- **Éditeur dynamique** : contenu reconstruit selon le moteur assigné, aucun paramètre codé en dur
- **Séparation données ↔ rendu** : ajouter instrument/paramètre = éditer une donnée

### Phases d'implémentation

#### Phase 1 — Fondations (structure + tokens)
- [x] [100a] **Mettre à jour `design_system.rs`** avec nouveaux tokens (palette IBM Plex, rayons, gaps, strokes)
- [x] [100b] **Intégrer polices IBM Plex** (Sans + Mono) via `FontDefinitions` egui (build 20260612-090421)
- [x] [100c] **Créer `theme.rs`** — constants `Color32` et helpers (`blue_glow`, `white_a`)
- [x] [100d] **Créer `widgets.rs`** — widgets custom coordonnés (Slider, Freq, Select, Switch, ToggleLED, Knob)
- [x] [100e] **Créer `engine_registry.rs`** — struct `Engine`, `EngineGroup`, `schema_for_engine()`, registre `ENGINES`

#### Phase 2 — Layout général (header + colonnes)
- [x] [100f] **Header redesign** — Brand + Transport (▶/■/●) + Master/Swing/Groove + Seq source (Internal/Ext MIDI segmented) + toggles LED
- [x] [100g] **Layout 2 colonnes** — Gauche (~910px) : séquenceur + page-bar + p-lock-bar + patterns + generator/song | Droite (~568px) : Sound Editor
- [x] [100h] **Sound Editor** — En-tête dynamique (nom + Engine selector) + onglets instruments (14) + zone scroll avec sections

#### Phase 3 — Séquenceur (grille + lanes)
- [x] [100i] **Lane modulaire** — Poignée drag, nom cliquable, menu clic-droit (rename, assign engine, remove), tag M/S/T — rename fait dans l'onglet Track (build 20260713-143422) ; assign engine / remove font partie de la phase modulaire B (reporté, cf. [100v] et [57])
- [x] [100j] **Grille de steps** — 16 colonnes visibles, états p-lock (Sound/Sequencer exclusifs), playhead, fusion
- [x] [100k] **Page/Length bar** — Pages 1-4, Follow ON/OFF, Len slider 1-64, presets 16/32/48/64, ×2
- [x] [100l] **P-lock modes** — Toggle segmented Sound/Sequencer, menus contextuels (Volume en premier, undo ↺)

#### Phase 4 — Panneaux bas (patterns + generator/song)
- [x] [100m] **Pattern Bank** — Save/Load, slots P1-P8, Clear, Export MIDI, Drag MIDI
- [x] [100n] **Generator/Song panel** — Segmented toggle Generator|Song, Generator = type + A/B + Mix/Dens/Var + Random + GENERATE
- [ ] [100o] **Song arranger** — Chaîne de blocs pattern × répétitions, toggle Song Enabled

#### Phase 5 — Polish & validation
- [x] [100p] **ADSR visualization** — graphe inline réécrit (modèle 3 segments colorés A/D/R, cadre #0c0c11, espacé)
- [ ] [100q] **Animations** — Hover transitions, step playback glow, toggle LED
- [ ] [100r] **Tests** — Vérifier que tous les moteurs rendent correctement, pas de régression audio
- [x] [100s] **Build + install** — VST3 fonctionnel avec nouveau design

### Tâches découvertes pendant la reprise UI 2026-06-11
- [x] [100t] Nettoyer le code UI legacy (~1300 lignes : `draw_grid` & helpers morts + modules `schema.rs` et `engine_registry.rs` supprimés). Restent des warnings de scaffolding (`design_system.rs`, `StyledButton`) — cf. `docs/design/UI-REDESIGN-HANDOFF.md` §4.
- [x] [100u] Polish pixel : Sound Editor (sliders/labels/sections/ADSR), combos → Select stylé, page-bar, bloc Generator réorganisé en 2 rangées + knob non tronqué (jusqu'au build 20260614-205742).
- [ ] [100v] **(Phase B / modulaire)** Engine selector fonctionnel + registre de moteurs — **reporté** ; le selector inerte a été retiré du Sound Editor.
- [x] [100w] Bouton GENERATE invisible après refonte en 2 lignes — corrigé en revenant à une seule ligne horizontale avec le bouton poussé à droite (build 20260614-092628).

#### Reste à faire (worklist détaillée : `docs/design/UI-REDESIGN-HANDOFF.md` §4)
- [x] [100x] Menus clic-droit p-lock → style `.plk` (284px, fond P_ACTIVE, r9, bordure LINE2), Volume en tête, mode Sound=orange / Sequencer=violet (build 20260616-203617).
- [x] [100y] Recâbler le menu page Copy/Paste/Clear sur la page-bar (helpers conservés sous `#[allow(dead_code)]` : `clear_page_fusions_for_ui`, `replace_page_fusions_for_ui`).
  - Bouton droit sur les numéros de page pour ouvrir le menu.
  - Actions : Copy Page, Paste Page, Clear Page.
  - Warnings de confirmation avant Paste (écrase la page cible) et Clear (supprime grille + plocks + fusions de la page).
  - Build `20260623-124600`.
- [ ] [100z] Animations .14s (hover/toggle) — basse priorité.
- [ ] [100aa] Nettoyage final : adopter `StyledButton` (hover chrome), retirer `design_system.rs`/`SegmentedControl` non câblés, remplacer `allocate_ui_at_rect` (déprécié) par `allocate_new_ui`.
- [x] [100ab] Dropdown Algo dynamique dans le menu p-lock (plage selon algo_count, nom affiché, masquage si 1 algo) - build 20260624-171823.
- [x] [100ac] Morphing par pulse sur les cellules fusionnees (select Morph + slider End, interpolation lineaire, params continus + special params continus, persistence DAW pattern-v3 + pattern bank) - build 20260629-160624.

### Notes
- **Volume** : range -60 dB à +6 dB (actuellement 0..2 linéaire, à convertir)
- **Norme de casse** : Title Case partout
- **Pas de gradients** : aplats + ombres/glow subtils
- **Contrainte egui** : tout en primitives (rect, cercle, texte), pas d'images

---

## Investigation & Features (A prioriser)

- [ ] [93] **Son tres ecourte interessant quand on maintient un slider OSC appuye** (P2, Audio/Design)
  - Quand on laisse un slider d'OSC appuye (sur n'importe quel instrument), un son tres court et interessant sort des Toms et HiHats
  - Probablement du aux re-triggers continus lors du changement de parametre
  - **Action :** expliquer le mecanisme et reproduire de facon controlee (effet design intentionnel ?)
  - Complexite : Moyenne

- [ ] [94] **Ajouter un parametre pitch LFO sur les Toms** (P2, Synthese)
  - Intensite, Rate, Type de LFO (sine/triangle/square/saw), arrivee progressive
  - Permet des variations de hauteur dynamiques sur les toms
  - Complexite : Moyenne, 3-5 jours

- [ ] [95] **Ajouter un instrument de type MIDI (avec MIDI out)** (P2/P3, Architecture MIDI)
  - Voix virtuelle qui envoie des NoteOn/NoteOff MIDI sur une sortie MIDI externe
  - Pas de synthese interne, juste du routage MIDI
  - Permet de declencher des instruments externes depuis le sequencer
  - Complexite : Moyenne-Elevee, 1-2 semaines

## Tests avances (Post-V1)

- [x] [12] Ajouter un test de stress du sequencer (longue session, stabilite du timing) - 6 tests impl�ment�s

## Analyse Technique (Reference)

### Mode Analog vs Digital - Comportement par Instrument

**Fonctionnement du mode Analog (`analog >= 0.5`)** :
- Oscillateurs conservent leur phase actuelle (kick.rs:142-148)
- Enveloppes relanc�es depuis leur valeur actuelle via `trigger_at_peak()`
- Son organique et continu, comme un vrai circuit analogique
- Retriggers pendant une queue ajoutent de l'�nergie plut�t que de r�initialiser
- Comportement similaire aux drum machines analogiques (Roland TR-808/909)

**Mode Digital (`analog < 0.5`)** :
- Oscillateurs r�initialis�s � phase = 0.0 avec crossfade sur 2 samples (kick.rs:150-165)
- Enveloppes repartent de z�ro via `trigger()`
- Son propre et r�p�table, id�al pour l'EDM et le techno
- Chaque hit sonne identique, m�me sur des retriggers rapides
- Comportement similaire aux drum machines num�riques (Roland TR-626, LinnDrum)

**Impl�mentation technique par instrument** :

**Kick (kick.rs)** :
- Analog: `self.osc.phase` pr�serv�, `self.noise_osc.phase` pr�serv�
- Digital: Crossfade entre ancienne et nouvelle phase sur 2 samples
- Impact sonore: Analog = plus de "punch" sur les retriggers, Digital = plus pr�cis

**Kick 808 (kick_808.rs)** :
- Analog: Phase pr�serv�e, simulate le comportement du circuit original
- Digital: R�initialisation compl�te ("cold start" comme l'original 808)
- Impact sonore: Analog = plus chaud, Digital = plus cliquety

**Snare (snare.rs)** :
- Analog: Phase pr�serv�e + noise generator NON reseed�
- Digital: Phase r�initialis�e + noise generator reseed�
- Impact sonore: Analog = plus de variation naturelle, Digital = plus constant

**Snare 606 (snare606.rs)** :
- Analog: Comportement similaire au snare mais avec envelope diff�rente
- Digital: R�initialisation compl�te comme le 606 original
- Impact sonore: Analog = plus organique, Digital = plus m�canique

**Tom (tom.rs)** :
- Analog: Phase pr�serv�e pour un son plus naturel
- Digital: R�initialisation pour un son plus synth�tique
- Impact sonore: Analog = comme des toms acoustiques, Digital = comme des toms �lectroniques

**Instruments SANS mode Analog/Digital** (toujours "analog") :
- Clap: Toujours analog (0.3) - n�cessite la continuit� pour le son r�aliste
- HiHat: Toujours analog (1.0) - les retriggers doivent �tre fluides
- OpenHiHat: Toujours analog (1.0) - m�me raison que HiHat
- Ride: Toujours analog (1.0) - n�cessite un decay naturel
- Cymbal: Toujours analog (1.0) - le shimmer n�cessite la continuit�
- Perc1: Valeur interm�diaire (0.3) - comportement hybride
- Zap: Valeur basse (0.0) - mais toujours trait� comme analog

**Valeurs par d�faut et plage typique** :
- Analog pur: 1.0 (Kick, Snare, Tom, HiHat, etc.)
- Digital pur: 0.0 (utilis� pour les sons �lectroniques pr�cis)
- Hybride: 0.3-0.7 (pour un m�lange des deux caract�res)

**Impact CPU par mode** :
- Analog: L�g�rement plus �lev� (calculs de phase pr�serv�e)
- Digital: L�g�rement plus bas (r�initialisations simples)
- Diff�rence: <2% sur un Core i7 (mesur� avec `test_high_cpu_load_patterns`)

**Quand utiliser chaque mode** :
- Analog: Sons organiques, patterns denses (>120 BPM), caract�re vintage
  Ex: House, Disco, Funk, Drum & Bass
- Digital: Sons propres, patterns clairsem�s (<110 BPM), caract�re moderne
  Ex: Techno, Minimal, Electro, Trance
- Hybride (0.3-0.7): Pour un m�lange des deux caract�res
  Ex: Progressive House, Melodic Techno

**Guide pratique par instrument** :

**Kick** :
- Analog (1.0): Id�al pour House/Disco - retriggers ajoutent du punch
- Digital (0.0): Parfait pour Techno - chaque hit identique
- Test: Essayez un pattern 16e notes � 125 BPM avec release=300ms

**Snare** :
- Analog (1.0): Son r�aliste comme une vraie caisse claire
- Digital (0.0): Son �lectronique pr�cis pour l'EDM
- Astuce: En mode analog, activez le noise pour plus de r�alisme

**Tom** :
- Analog (1.0): Sons comme des toms acoustiques
- Digital (0.0): Sons synth�tiques style 808
- Conseil: Utilisez analog pour les fills, digital pour les riffs

**HiHat/OpenHiHat** :
- Toujours analog (1.0) - ne peut pas �tre chang�
- Pourquoi: Les retriggers rapides n�cessitent une continuit� parfaite
- Astuce: Utilisez le param�tre "Tight" pour ajuster le caract�re

**Clap** :
- Toujours analog (0.3) - valeur fixe
- Pourquoi: Le son r�aliste n�cessite la continuit� des oscillateurs
- Alternative: Utilisez le Snare en mode digital pour un clap �lectronique

**Ride/Cymbal** :
- Toujours analog (1.0) - pour le shimmer naturel
- Astuce: Ajustez le param�tre "Shimmer" pour plus/moins de brillance

**Perc1** :
- Valeur interm�diaire (0.3) - comportement hybride
- Utilisation: Pour des sons de percussion interm�diaires
- Exp�rimentation: Essayez entre 0.1 et 0.5 pour diff�rents caract�res

**Zap** :
- Valeur basse (0.0) mais trait� comme analog
- Comportement: Son �lectronique avec une touche organique
- Utilisation: Pour des effets sp�ciaux et transitions

**Recettes par style musical** :

**1. Classic House (� la Kerri Chandler)** :
- Kick: 0.9 (l�g�rement digital pour la pr�cision)
- Snare: 1.0 (full analog pour le groove)
- HiHat: 1.0 (toujours analog)
- Tom: 0.8 (presque analog)
- Clap: 0.3 (d�faut)
- Groove: Swing16 � 55%

**2. Detroit Techno (� la Jeff Mills)** :
- Kick: 0.2 (tr�s digital pour la pr�cision)
- Snare: 0.3 (l�g�rement analog pour le corps)
- HiHat: 1.0 (toujours analog)
- Tom: 0.4 (mi-chemin)
- Clap: 0.3 (d�faut)
- Groove: Straight (pas de swing)

**3. Drum & Bass (� la LTJ Bukem)** :
- Kick: 0.7 (analog pour les retriggers rapides)
- Snare: 0.8 (presque analog pour le groove)
- HiHat: 1.0 (toujours analog)
- Tom: 0.9 (presque analog)
- Clap: 0.3 (d�faut)
- Groove: Shuffle � 40%

**4. Minimal Techno (� la Richie Hawtin)** :
- Kick: 0.1 (tr�s digital)
- Snare: 0.2 (tr�s digital)
- HiHat: 1.0 (toujours analog)
- Tom: 0.3 (digital)
- Clap: 0.3 (d�faut)
- Groove: Straight (pas de swing)

**Conseils avanc�s** :

1. **Automatisation du param�tre analog** :
   - Automatisez le param�tre analog pendant un breakdown
   - Passez de digital (pr�cis) � analog (organique) pour un effet dramatique

2. **Per-instrument settings** :
   - Chaque instrument peut avoir sa propre valeur analog
   - Ex: Kick digital (0.2) + Snare analog (1.0) = combo puissant

3. **Pattern density** :
   - Patterns denses (>120 BPM, 16e notes) ? privil�giez analog
   - Patterns clairsem�s (<110 BPM, 8e notes) ? digital fonctionne bien

4. **Velocity interaction** :
   - En mode analog: la velocity affecte plus le timbre
   - En mode digital: la velocity affecte plus le volume

**D�pannage** :

Probl�me: "Mon kick sonne diff�rent � chaque hit"
- Solution: Passez en mode digital (0.0) pour une consistance parfaite

Probl�me: "Mon pattern dense sonne m�canique"
- Solution: Passez en mode analog (1.0) pour plus de groove

Probl�me: "Je veux un m�lange des deux"
- Solution: Essayez des valeurs entre 0.3 et 0.7

**Exemples de r�glages par style** :
- TR-808 style: Kick=1.0, Snare=1.0, Tom=1.0 (full analog)
- TR-909 style: Kick=0.8, Snare=0.7, Tom=0.9 (l�g�rement digital)
- Modern Techno: Kick=0.2, Snare=0.3, Tom=0.4 (plus digital)
- Acoustic simulation: Tous � 1.0 avec long decay




