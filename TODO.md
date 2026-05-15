## Court terme (Stabilisation V1 — En cours)

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
- [x] [10b] Ajouter option Hi-Hat Choke (cut Open HH quand Closed HH trigger)
- [x] [10c] Corriger les step skips rares (sync_to_host moins agressif)
- [x] [10d] Remplacer sliders par checkbox/combobox pour paramètres bool/enum/algos
- [x] [13] Verifier la precision du timing du sequencer (compteur d'echantillons vs transport hote, correction continue)

## Fonctionnalites P1 (Parite PoC — Impact fort)

- [x] [14] Editer les reglages de synthese par instrument dans l'UI (frequence, decay, volume, filter)
- [x] [15] Connecter `filter_freq` dans `SnareVoice` (actuellement ignore)
- [x] [16] Ajouter un bouton "Test" par instrument pour declencher le son isole
- [x] [17] Ajouter export MIDI fichier depuis le plugin
- [x] [18] Ajouter sortie MIDI temps reel vers hardware externe
- [x] [19] Ajouter la generation de pattern aleatoire (grille + option Random BPM + option Random Sounds)

## Fonctionnalites P2 (Post-V1 — Nice to have)

- [x] [20] Ajouter swing
- [x] [21] Ajouter un facteur de groove parametrable (Straight/Swing16/Shuffle/MPC)
- [x] [22] Ajouter un parametre analogique pour legeres variations aleatoires (humanize per track)
- [x] [23] Permettre un mode stereo analogique avec variation gauche/droite (push/pull per track)
- [ ] [24] Ajouter song mode
- [x] [24a] Ajouter modularité des instruments (algos + special params)
- [x] [24b] Ajouter Kick algos (Sine/Square/FM) + click transient
- [x] [24c] Ajouter Snare algos (Synth/Noise/Layered) + snap param
- [x] [24d] Ajouter Clap, Ride, Cymbal voices
- [ ] [25] **REPRENDRE ICI** — Labels complets des instruments dans l'UI ("Grosse Caisse", "Caisse Claire"...) et couleurs par instrument
- [ ] [26] Barre de progression visuelle du pattern (0-100%)

## Fonctionnalites P3 (Avancees / Complexes)

- [ ] [27] Generation IA de patterns par style (rock, techno, rap, jazz, reggae, metal, funk, latin, disco, trap)
- [ ] [28] Drag & drop MIDI directement vers le DAW
- [ ] [29] Parameter locks (plocks) façon Elektron — changer un paramètre de synthese par step
- [ ] [39] Refactor : paramètres dédiés par instrument (au lieu du `VoiceSettings` partagé + `special[8]`). Permet labels, ranges et défauts spécifiques par voix.
- [ ] [40] Filter envelope (cutoff modulé par AD/ADSR) — utile sur snare, clap, ride, cymbal
- [ ] [41] Émulation circuit-exact TR-606 (WDF, modèle non-linéaire VCA, oversampling) — vs grey-box actuelle

## Dette technique & Documentation

- [ ] [30] Clarifier si `index.js` doit etre conserve ou archive
- [ ] [31] Revoir l'organisation du repo pour separer clairement PoC web et plugin
- [ ] [32] Synchroniser `BACKLOG_VST.md` avec le code reel (items P2 marques comme "hors V1")
- [x] [33] Reduire les warnings Rust inutiles (0 warning sur lib + bin + tests, release inclus)
- [ ] [34] Garder les fichiers de sauvegarde hors de `src/`
- [x] [34a] Corriger le click de retrigger kick (2 steps BD proches)
- [x] [34b] Nettoyer le code mort dans `special_params.rs` (struct `SpecialParamDef`, tous les `*_SPECIALS`, helper `specials_for`, methodes trait `supported_algos`/`special_params`)

## Bugs a corriger

- [x] [45] Sauts de volume general dans Reaper — diagnostique externe (driver audio, reproduit avec d'autres plugins)
- [ ] [35] Diagnostiquer la sauvegarde/reouverture Studio One
- [x] [36] Corriger la persistance de grille via `pattern-v1`
- [x] [37] Migration legacy depuis les parametres caches `st01` a `st16`
- [ ] [38] Ecart entre documentation et code reel a surveiller
- [x] [42] Crash a l'instanciation avec 11e voix (cause: `IntRange { min:0, max:0 }` → div par zéro nih-plug)
- [x] [43] Index out of bounds dans UI (`hums`/`pushes`/`lengths` taille 10 vs INSTRUMENT_LABELS taille 11)
- [x] [44] Step mask hardcode `0x3ff` (10 bits) — extensible via `INSTRUMENT_COUNT`

## Tests avances (Post-V1)

- [ ] [12] Ajouter un test de stress du sequencer (longue session, stabilite du timing)