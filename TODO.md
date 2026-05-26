## Court terme (Stabilisation V1 — En cours)

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
- [ ] [10b] Corriger le choke qui ne fonctionne plus
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
- [ ] [26] Barre de progression visuelle du pattern (0-100%) — non prio
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
- [ ] [52] Ajouter un paramètre Sustain level pour un vrai ADSR séquentiel — non prio
- [x] [26k] Refonte UI Phase 1 (grid intégré, sound panel ongleté, auto-edit)
  - Sound Panel regroupé par familles data-driven (OSC/ENV/FILTER/OUTPUT)
  - Visualisations interactives d'enveloppe (Amp AHDSR + Filter Env)
  - Layout horizontal : params à gauche, graph à droite
- [ ] [26l] Corriger le toggle stereo pour certains instruments
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

## Fonctionnalites P3 (Avancees / Complexes)

- [ ] [27] Generation IA de patterns par style (rock, techno, rap, jazz, reggae, metal, funk, latin, disco, trap)
- [x] [28] Drag & drop MIDI directement vers le DAW — helper externe validé dans Studio One
  - [x] remplacer l'ancien `dnd_set_drag_payload(bytes)` interne egui par un drag fichier OS natif Windows (`CF_HDROP` via OLE `DoDragDrop`)
  - [x] garder l'export fichier OK via bouton MIDI dans `Documents/Drum Flash/exports`
  - [x] isoler `DoDragDrop` hors process DAW via `drum-pattern-midi-drag-helper.exe`
  - [x] réactiver le bouton `Drag` : export MIDI puis ouverture d'une petite poignée de drag externe
  - [x] valider dans Studio One : cliquer `Drag`, puis glisser la fenêtre `Drag MIDI` vers une piste/instrument et vérifier qu'un clip MIDI est créé sans crash
- [x] [29] Parameter locks (plocks) façon Elektron — changer un paramètre de synthese par step
  - 14 champs plockables (12 sound settings + clap_echo + algo)
  - special params (accent/snap/pitch_drop) propagés uniquement au trigger (fix echo perdu)
- [x] [29a] Refactor plock UI data-driven depuis `instrument_registry`
  - remplacer les branches hardcodees par instrument dans `draw_plock_menu`
  - exposer automatiquement les `special_params` de Clap, Snare606, B8, Perc1 et futurs instruments
  - aligner les champs plock stockes/lus (`FIELD_COUNT = 18`) avec les special params reels
  - clarifier/corriger l'incoherence Clap Echo : UI lit le champ 12 alors que `PlockState::set_settings()` stocke les specials en 14..17
  - ajouter tests unitaires sur `PlockState::set_settings/get_settings` pour Clap Echo, B8 specials et Perc1 specials
- [ ] [39] Refactor : paramètres dédiés par instrument (au lieu du `VoiceSettings` partagé + `special[8]`). Permet labels, ranges et défauts spécifiques par voix.
  - [x] Prototype Kick : `KickSettings` struct typée, conversion `VoiceSettings ↔ KickSettings`, tests passent
  - [x] Généraliser aux 12 autres instruments (Snare, HiHat, OpenHH, Tom1-3, Clap, Ride, Cymbal, Snare606, B8, Perc1)
- [x] [40] Filter envelope (cutoff modulé par AD/ADSR) — Kick, Snare, Tom, HiHat, Snare606
- [ ] [41] Émulation circuit-exact TR-606 (WDF, modèle non-linéaire VCA, oversampling) — vs grey-box actuelle
- [x] [54] Saisie clavier de valeurs précises + Shift+mouse pour affiner les sliders de paramètres
  - LocalParamSlider créé pour remplacer egui::Slider dans les plocks et paramètres spéciaux
  - Shift+drag implémenté pour le fine-tuning sur tous les sliders
  - Hauteurs de sliders harmonisées pour une expérience visuelle cohérente

## Dette technique & Documentation

- [ ] [30] Clarifier si `index.js` doit etre conserve ou archive
- [ ] [31] Revoir l'organisation du repo pour separer clairement PoC web et plugin
- [x] [31a] Clarifier l'emplacement des docs produit actives
  - `AGENTS.md` cite `PROJECT_BRIEF.md` et `BACKLOG_VST.md`, mais les fichiers presents sont sous `docs/historique/`
  - decider si ces docs doivent revenir a la racine, etre remplacees par `TODO.md`/`README.md`, ou etre explicitement marquees comme archivees
  - mettre a jour `README.md`, `AGENTS.md` et les references croisees en consequence
- [x] [32] Synchroniser `BACKLOG_VST.md` avec `TODO.md`
- [x] [33] Reduire les warnings Rust inutiles (0 warning sur lib + bin + tests, release inclus)
- [ ] [34] Garder les fichiers de sauvegarde hors de `src/`
- [x] [34a] Corriger le click de retrigger kick (2 steps BD proches)
- [x] [34b] Nettoyer le code mort dans `special_params.rs` (struct `SpecialParamDef`, tous les `*_SPECIALS`, helper `specials_for`, methodes trait `supported_algos`/`special_params`)
- [x] [34c] Corriger les libelles obsoletes multi-out dans le code
  - `AUX_OUT_COUNT` vaut 13 mais `lib.rs` parle encore de "10 stereo drum outs"
  - corriger le commentaire "Frozen at 10" et le `PortNames.layout`
  - verifier que la doc Studio One reste alignee avec Main Mix + 13 sorties aux

## Bugs a corriger

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
- [ ] [38] Ecart entre documentation et code reel a surveiller
- [x] [38b] Supprimer les `unwrap()` evitables du chemin audio/UI sensible
  - `lib.rs::process()` utilise `DrumVoice::from_index(...).unwrap()` sur des index bornes par `DrumVoice::COUNT`
  - risque faible aujourd'hui, mais non conforme a la regle stricte "audio thread sans panic"
  - remplacer par API interne sans `Option`, ou par branche defensive sans panic
- [x] [38a] Fusionner `CLAUDE.md` dans `AGENTS.md` (13 instruments, AUX_OUT_COUNT = 13, Zap ajouté)
- [x] [42] Crash a l'instanciation avec 11e voix (cause: `IntRange { min:0, max:0 }` → div par zéro nih-plug)
- [x] [43] Index out of bounds dans UI (`hums`/`pushes`/`lengths` taille 10 vs INSTRUMENT_LABELS taille 11)
- [x] [44] Step mask hardcode `0x3ff` (10 bits) — extensible via `INSTRUMENT_COUNT`

## Tests avances (Post-V1)

- [ ] [12] Ajouter un test de stress du sequencer (longue session, stabilite du timing)
