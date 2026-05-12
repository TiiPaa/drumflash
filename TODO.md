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

- [ ] [10] Tester le plugin dans au moins un autre DAW (Reaper recommande)
- [x] [13] Verifier la precision du timing du sequencer (compteur d'echantillons vs transport hote, correction continue)

## Fonctionnalites P1 (Parite PoC — Impact fort)

- [x] [14] Editer les reglages de synthese par instrument dans l'UI (frequence, decay, volume, filter)
- [x] [15] Connecter `filter_freq` dans `SnareVoice` (actuellement ignore)
- [x] [16] Ajouter un bouton "Test" par instrument pour declencher le son isole
- [x] [17] Ajouter export MIDI fichier depuis le plugin
- [x] [18] Ajouter sortie MIDI temps reel vers hardware externe
- [x] [19] Ajouter la generation de pattern aleatoire (grille + option Random BPM + option Random Sounds)

## Fonctionnalites P2 (Post-V1 — Nice to have)

- [ ] [20] Ajouter swing
- [ ] [21] Ajouter un facteur de groove parametrable
- [ ] [22] Ajouter un parametre analogique pour legeres variations aleatoires
- [ ] [23] Permettre un mode stereo analogique avec variation gauche/droite
- [ ] [24] Ajouter song mode
- [ ] [25] Labels complets des instruments dans l'UI ("Grosse Caisse", "Caisse Claire"...) et couleurs par instrument
- [ ] [26] Barre de progression visuelle du pattern (0-100%)

## Fonctionnalites P3 (Avancees / Complexes)

- [ ] [27] Generation IA de patterns par style (rock, techno, rap, jazz, reggae, metal, funk, latin, disco, trap)
- [ ] [28] Drag & drop MIDI directement vers le DAW
- [ ] [29] Parameter locks (plocks) façon Elektron — changer un paramètre de synthese par step

## Dette technique & Documentation

- [ ] [30] Clarifier si `index.js` doit etre conserve ou archive
- [ ] [31] Revoir l'organisation du repo pour separer clairement PoC web et plugin
- [ ] [32] Synchroniser `BACKLOG_VST.md` avec le code reel (items P2 marques comme "hors V1")
- [ ] [33] Reduire les warnings Rust inutiles
- [ ] [34] Garder les fichiers de sauvegarde hors de `src/`

## Bugs a corriger

- [ ] [35] Diagnostiquer la sauvegarde/reouverture Studio One
- [x] [36] Corriger la persistance de grille via `pattern-v1`
- [x] [37] Migration legacy depuis les parametres caches `st01` a `st16`
- [ ] [38] Ecart entre documentation et code reel a surveiller

## Tests avances (Post-V1)

- [ ] [12] Ajouter un test de stress du sequencer (longue session, stabilite du timing)