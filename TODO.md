## Court terme (Stabilisation V1 â€” En cours)

- [x] [69] Vrai fix du click parasite BD (changement de hauteur/plock) : chemin digital = reset de phase + crossfade cassé supprimés ; phase resetée au cold-start uniquement ; plancher d'attaque anti-click (MIN_AMP_ATTACK_MS) ; bug sweep digital +1 Hz corrigé (build 20260531-155232)
- [x] [70] Mode analog/digital BD re-rendu audible : digital = identique au bit près, analog = drift par coup (hauteur ±3.5 %, niveau ±10 %, temps d'enveloppe ±20 %)
- [x] [71] Sécurisé les autres voix : perc1 (reset phase inconditionnel → cold-start only), snare/tom/snare606 (reset digital → cold-start only + enveloppes recréées → setters), hihat (enveloppe recréée → setters + biquad peaking recalculé seulement si freq change). Plancher d'attaque + DC-blockers partout ; drift analog sur snare & tom (sliders exposés) ; helper partagé `AnalogDrift`. ride/cymbal/clap/open_hihat/kick_808 déjà click-safe, non modifiés. (build 20260531-184528)
- [ ] [72] **REPRENDRE ICI** — Nettoyer les fichiers de cruft hérités de la réparation ui.rs (src/ui_backup.rs, src/ui_fixed.rs, remaining_content.txt, tail_content.txt, temp_*.txt) + les ajouter au .gitignore si besoin
- [x] [67] Positionner le volume en haut du sound editor + ajouter un controle de volume sur chaque lane de la grille (ComplexitÃ©: Faible, P1)
- [x] [68] Couleurs differentes pour plock link global vs full snapshot (orange / rouge) pour distinguer visuellement les modes (ComplexitÃ©: Faible, P1)
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
  - confirme reproduit avec un autre plugin dans Reaper â†’ pb driver audio, plugin innocent
- [x] [10a] Corriger les defaults de decay (snare 0.47, hihat 0.36, open_hh 0.66)
- [x] [10b] Corriger le choke qui ne fonctionne plus
- [x] [10c] Corriger les step skips rares (sync_to_host moins agressif)
- [x] [10d] Remplacer sliders par checkbox/combobox pour paramÃ¨tres bool/enum/algos
- [x] [13] Verifier la precision du timing du sequencer (compteur d'echantillons vs transport hote, correction continue)

## Fonctionnalites P1 (Parite PoC â€” Impact fort)

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

## Fonctionnalites P2 (Post-V1 â€” Nice to have)
- [x] [66b] Correction focus clavier Windows (SetFocus sur HWND plugin) — build 20260529-124136\n
- [x] [20] Ajouter swing
- [x] [21] Ajouter un facteur de groove parametrable (Straight/Swing16/Shuffle/MPC)
- [x] [22] Ajouter un parametre analogique pour legeres variations aleatoires (humanize per track)
- [x] [23] Permettre un mode stereo analogique avec variation gauche/droite (push/pull per track)
- [x] [24] Ajouter song mode (placeholder UI P1-P8, backend Ã  cÃ¢bler)
- [x] [24a] Ajouter modularitÃ© des instruments (algos + special params)
- [x] [24b] Ajouter Kick algos (Sine/Square/FM) + click transient
- [x] [24c] Ajouter Snare algos (Synth/Noise/Layered) + snap param
- [x] [24d] Ajouter Clap, Ride, Cymbal voices
- [x] [25] Labels complets des instruments dans l'UI et couleurs par instrument (labels courts BD/SD/HH, couleurs blocs de 4 steps, grisage len)
- [ ] [26] Barre de progression visuelle du pattern (0-100%) â€” non prio
- [x] [26a] Per-instrument Mix Bus checkbox (exclure du Main Mix)
- [x] [26b] Clap Echo plockable par step
- [x] [26c] Masquer les paramÃ¨tres inutiles par instrument dans le Sound Panel
- [x] [26d] Nouvel instrument B8 (TR-808 Bass Drum)
- [x] [26e] Slider Analog actif pour B8
- [x] [26f] Release fonctionnel pour B8 (DecayReleaseEnvelope)
- [x] [26g] Attack ramp 1.5 ms sur B8 (Ã©limine click de dÃ©marrage)
- [x] [26h] Filtre LP dÃ©diÃ© Click Tone sur B8 (100-8000 Hz)
- [x] [26i] Plock B8 fix : special params (accent/snap/pitch_drop/click_tone) stockÃ©s
- [x] [26j] Finaliser les rÃ©glages de synthÃ¨se par instrument
  - `standard_params` data-driven avec ranges UI (min/max, log, suffix, checkbox)
  - Ranges corrigÃ©s pour Ã©viter le clamp involontaire (ex: Ride decay 1.2s > slider 0.5s)
- [x] [51] Ajouter un paramÃ¨tre Attack rÃ©glable par instrument (graphique AHDSR complet A-H-D-R)
- [x] **[EN COURS — Phase 1a OK]** [66] Presets d'instruments â€” sauvegarder/charger des rÃ©glages de synthese par voix (ComplexitÃ©: Moyenne, P2)
- [ ] [52] Ajouter un paramÃ¨tre Sustain level pour un vrai ADSR sÃ©quentiel â€” non prio
- [x] [26k] Refonte UI Phase 1 (grid intÃ©grÃ©, sound panel ongletÃ©, auto-edit)
  - Sound Panel regroupÃ© par familles data-driven (OSC/ENV/FILTER/OUTPUT)
  - Visualisations interactives d'enveloppe (Amp AHDSR + Filter Env)
  - Layout horizontal : params Ã  gauche, graph Ã  droite
- [x] [26l] Corriger le toggle stereo pour certains instruments
  - exposer/finir les toggles stÃ©rÃ©o pour les voix oÃ¹ la largeur apporte une vraie valeur : Snare, HiHat, OpenHH, Clap, Ride, Cymbal, Snare606, Perc1
  - garder Kick, B8 et Toms mono par dÃ©faut pour prÃ©server le centre et la compatibilitÃ© mono
  - prioritÃ© technique : finir stereo Snare606 et vÃ©rifier que les toggles UI ne sont visibles que sur les voix concernÃ©es
- [x] [55] Saturation / distortion par instrument (tous les 13 voix)
  - Module `saturation.rs` avec 5 algorithmes distincts (SoftClip, Valve, Transistor, HardClip, Tape)
  - ParamÃ¨tres exposÃ©s dans le Sound Panel : Type, Amount, Mix, Output Gain, Pre-Filter
  - Drive d'entrÃ©e mappÃ© 1Ã—..20Ã— pour effet audible
  - Pre-Filter comme checkbox toggle (post-filter par dÃ©faut)
  - Section SAT dÃ©diÃ©e dans le Sound Panel (ParamFamily::Saturation)
  - Combobox affichant les noms d'algorithmes (SoftClip, Valve, etc.)
  - Saturation appliquÃ©e sur 8/13 instruments : Kick, Snare, Snare606, B8, Tom1-3, Perc1
  - ~~Saturation sur les 5 restants (HiHat, OpenHH, Clap, Ride, Cymbal)~~ â€” pas prioritaire
  - Special params augmentÃ©s de 8 Ã  32 slots (`special: [f32; 32]`)
  - Plock field masks passÃ©s de u32 Ã  u64 (46 fields total, 32 special params plockables)
  - Auto-edit activÃ© par dÃ©faut
- [x] [62] Cymbal : retirer frequency inutilisÃ©, ajouter Shimmer Freq + Noise Type
  - `frequency` retirÃ© du Sound Panel (paramÃ¨tre inutilisÃ© sur un bruit)
  - `Shimmer Freq` (1-50 Hz) : module la frÃ©quence du FM shimmer (Ã©tait hardcodÃ© Ã  15 Hz)
  - `Noise Type` : White / Pink / Brown / Blue â€” gÃ©nÃ©rateurs Voss-McCartney dans dsp.rs
  - Combobox UI pour sÃ©lectionner le type de bruit
- [x] [63] Bug B8 se coupe quand on modifie CY : corrigÃ© division par zÃ©ro dans `ExpDecayEnvelope::set_attack_ms`
  - Quand attack_time passe Ã  0 pendant un ramp actif â†’ snap Ã  peak immÃ©diat pour Ã©viter NaN
  - Bouton "T" (Test) : appelle maintenant `set_voice_settings` avant `trigger`

## Fonctionnalites P3 (Avancees / Complexes)

- [ ] [69] Creer un instrument percussif a base de wavetables â€” phase recherche et prototypage (ComplexitÃ©: Ã‰levÃ©e, 2-4 semaines, P3)
- [ ] [27] Generation IA de patterns par style (rock, techno, rap, jazz, reggae, metal, funk, latin, disco, trap)
- [x] [28] Drag & drop MIDI directement vers le DAW â€” helper externe validÃ© dans Studio One
  - [x] remplacer l'ancien `dnd_set_drag_payload(bytes)` interne egui par un drag fichier OS natif Windows (`CF_HDROP` via OLE `DoDragDrop`)
  - [x] garder l'export fichier OK via bouton MIDI dans `Documents/Drum Flash/exports`
  - [x] isoler `DoDragDrop` hors process DAW via `drum-pattern-midi-drag-helper.exe`
  - [x] rÃ©activer le bouton `Drag` : export MIDI puis ouverture d'une petite poignÃ©e de drag externe
  - [x] valider dans Studio One : cliquer `Drag`, puis glisser la fenÃªtre `Drag MIDI` vers une piste/instrument et vÃ©rifier qu'un clip MIDI est crÃ©Ã© sans crash
- [x] [29] Parameter locks (plocks) faÃ§on Elektron â€” changer un paramÃ¨tre de synthese par step
  - 14 champs plockables (12 sound settings + clap_echo + algo)
  - special params (accent/snap/pitch_drop) propagÃ©s uniquement au trigger (fix echo perdu)

## Nouveaux Ã©lÃ©ments (Ã€ prioriser)

- [ ] [56] Ajouter une percussion de type Tom Simmons (ComplexitÃ©: Moyenne, 3-5 jours)
  - CrÃ©er un nouveau module de synthÃ¨se
  - Ajouter l'instrument dans le registre des instruments
  - CrÃ©er les paramÃ¨tres spÃ©cifiques et l'interface utilisateur
  - IntÃ©grer dans le systÃ¨me de mixage et de sortie audio

- [ ] [57] CrÃ©er un sÃ©quencer modulaire avec instruments dynamiques (ComplexitÃ©: Ã‰levÃ©e, 4-6 semaines)
  - Refonte majeure de l'architecture du sÃ©quencer
  - SystÃ¨me de plugins/instruments dynamiques
  - Gestion de l'ajout/suppression d'instruments Ã  chaud
  - Interface utilisateur pour la configuration modulaire
  - SystÃ¨me de sauvegarde/restoration des configurations

- [ ] [58] Gestion des patterns et song (ComplexitÃ©: Moyenne-Ã‰levÃ©e, 3-5 semaines)
  - SystÃ¨me de gestion de plusieurs patterns
  - Organisation en songs (chaÃ®nes de patterns)
  - Interface de navigation et d'Ã©dition
  - SystÃ¨me de sauvegarde/restoration

- [ ] [59] Gestion des plocks de type sÃ©quenceur (ComplexitÃ©: Moyenne, 2-3 semaines)
  - ImplÃ©mentation d'un systÃ¨me de modes de plock (ex: mode "step", mode "sequenceur")
  - Logique de basculement entre les modes
  - SystÃ¨me de couleurs pour diffÃ©rencier visuellement les types de plock
  - IntÃ©gration avec l'interface utilisateur existante
  - Sauvegarde/restoration de l'Ã©tat du mode

- [ ] [60] DÃ©sactivation du sÃ©quenceur interne et pilotage MIDI depuis le DAW (ComplexitÃ©: Moyenne, 1-2 semaines)
  - Ajout d'un paramÃ¨tre pour activer/dÃ©sactiver le sÃ©quenceur interne
  - ImplÃ©mentation d'un mode "MIDI thru" oÃ¹ le plugin transmet simplement les notes MIDI aux instruments
  - Gestion des canaux MIDI et mapping des instruments
  - Interface utilisateur pour la configuration MIDI
  - SystÃ¨me de routage MIDI flexible

- [x] [61] Pour les BD, ajouter un switch de tuning entre Hz et Notes (ComplexitÃ©: Faible, 2-3 jours)
  - Ajouter un paramÃ¨tre boolÃ©en pour basculer entre les modes de tuning
  - ImplÃ©menter la conversion Hz â†” Notes (standard MIDI)
  - Mettre Ã  jour l'interface utilisateur pour afficher le bon format
  - S'assurer que la valeur est correctement sauvegardÃ©e/restaurÃ©e
  - Appliquer aux instruments Kick et B8 (et potentiellement autres bass drums)
- [ ] [61b] Ajouter copier/coller un plock dans le menu bouton droit
  - Stocker le plock copiÃ© dans l'Ã©tat de l'Ã©diteur (EditorUIState)
  - Afficher "Copier plock" / "Coller plock" dans le menu contextuel
  - Coller doit Ã©craser le plock existant sur la step cible
  - Support multi-instrument (on ne colle que si le type d'instrument correspond)
- [x] [29a] Refactor plock UI data-driven depuis `instrument_registry`
  - remplacer les branches hardcodees par instrument dans `draw_plock_menu`
  - exposer automatiquement les `special_params` de Clap, Snare606, B8, Perc1 et futurs instruments
  - aligner les champs plock stockes/lus (`FIELD_COUNT = 18`) avec les special params reels
  - clarifier/corriger l'incoherence Clap Echo : UI lit le champ 12 alors que `PlockState::set_settings()` stocke les specials en 14..17
  - ajouter tests unitaires sur `PlockState::set_settings/get_settings` pour Clap Echo, B8 specials et Perc1 specials
- [ ] [39] Refactor : paramÃ¨tres dÃ©diÃ©s par instrument (au lieu du `VoiceSettings` partagÃ© + `special[8]`). Permet labels, ranges et dÃ©fauts spÃ©cifiques par voix.
  - [x] Prototype Kick : `KickSettings` struct typÃ©e, conversion `VoiceSettings â†” KickSettings`, tests passent
  - [x] GÃ©nÃ©raliser aux 12 autres instruments (Snare, HiHat, OpenHH, Tom1-3, Clap, Ride, Cymbal, Snare606, B8, Perc1)
- [x] [40] Filter envelope (cutoff modulÃ© par AD/ADSR) â€” Kick, Snare, Tom, HiHat, Snare606
- [ ] [41] Ã‰mulation circuit-exact TR-606 (WDF, modÃ¨le non-linÃ©aire VCA, oversampling) â€” vs grey-box actuelle
- [x] [54] Saisie clavier de valeurs prÃ©cises + Shift+mouse pour affiner les sliders de paramÃ¨tres
  - LocalParamSlider crÃ©Ã© pour remplacer egui::Slider dans les plocks et paramÃ¨tres spÃ©ciaux
  - Shift+drag implÃ©mentÃ© pour le fine-tuning sur tous les sliders
  - Hauteurs de sliders harmonisÃ©es pour une expÃ©rience visuelle cohÃ©rente

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
- [x] [70] Kick : click de retrigger quand la queue percute l'attaque du suivant — corrigé (ne pas retrigger le click pendant la tail) — build 20260529-172133\n
- [x] [64] Revoir l'algo de polyrythmie (lane length) â€” comportement bizarre, longueurs mal synchronisÃ©es (ComplexitÃ©: Moyenne, P1)
- [x] [65] Revoir les algos de generation pattern avec les nouveaux instruments (13 voix) â€” tous les gÃ©nÃ©rateurs gÃ¨rent 13 instruments; rÃ´les musicaux enrichis pour Snare 606, B8, Perc1 dans le style Rock (dÃ©monstration)
- [x] [45] Sauts de volume general dans Reaper â€” diagnostique externe (driver audio, reproduit avec d'autres plugins)
- [x] [46] Revert du code Perc1 au commit 5ae1286 (Zap) â€” build stable rÃ©installÃ©
- [x] [47] Refaire Perc1 proprement : ne pas recrÃ©er les enveloppes dans `set_settings`
- [x] [48] Refaire Perc1 proprement : utiliser `DecayReleaseEnvelope` pour le slider Release
- [x] [50] Diagnostiquer pourquoi la moitiÃ© des paramÃ¨tres Perc1 ne sont pas actionnables (faux positif â€” tests unitaires confirment que decay/release fonctionnent)
- [x] [49] Refaire Perc1 proprement : rendre le plock menu data-driven (plus de hardcode par index)
- [x] [53] Plock Snapshot vs Link : choix Ã  la crÃ©ation du plock (snapshot fige tout, link ne stocke que les champs modifiÃ©s)
- [x] [35] Diagnostiquer la sauvegarde/reouverture Studio One
- [x] [35a] Plock B8 : accent/snap/pitch_drop/click_tone plockables
- [x] [36] Corriger la persistance de grille via `pattern-v1`
- [x] [37] Migration legacy depuis les parametres caches `st01` a `st16`
- [ ] [38] Ecart entre documentation et code reel a surveiller
- [x] [38b] Supprimer les `unwrap()` evitables du chemin audio/UI sensible
  - `lib.rs::process()` utilise `DrumVoice::from_index(...).unwrap()` sur des index bornes par `DrumVoice::COUNT`
  - risque faible aujourd'hui, mais non conforme a la regle stricte "audio thread sans panic"
  - remplacer par API interne sans `Option`, ou par branche defensive sans panic
- [x] [38a] Fusionner `CLAUDE.md` dans `AGENTS.md` (13 instruments, AUX_OUT_COUNT = 13, Zap ajoutÃ©)
- [x] [42] Crash a l'instanciation avec 11e voix (cause: `IntRange { min:0, max:0 }` â†’ div par zÃ©ro nih-plug)
- [x] [43] Index out of bounds dans UI (`hums`/`pushes`/`lengths` taille 10 vs INSTRUMENT_LABELS taille 11)
- [x] [44] Step mask hardcode `0x3ff` (10 bits) â€” extensible via `INSTRUMENT_COUNT`

## Bugs a corriger (Nouveaux)

- [ ] [71] Longueur globale du pattern ajustable 1 => 64 avec 4 pages de 16 steps max. Prevoir un switch de follow de la lecture ou pas (Complexite: Moyenne-Elevee, 1-2 semaines, P1)
- [ ] [72] Probleme d'affichage du volume : slider en haut de l'editor (1.5 max) et en bas (1) et dans la lane (1.5) — incoherence de range a uniformiser (Complexite: Faible, 1-2 jours, P1)
- [x] [73] caracteres esoteriques ont remplace aleatoirement les caracteres normaux dans les boutons/texte UI — CORRIGE (restauration UTF-8 via script Python) — build 20260529-174106 (Complexite: Faible, 1 jour, P1)
- [x] [74] Proposer 3 types de clicks pour la BD (Kick) : soft/medium/hard ou impulse/noise/transient (Complexite: Moyenne, 3-5 jours, P2)

## Tests avances (Post-V1)

- [x] [12] Ajouter un test de stress du sequencer (longue session, stabilite du timing) - 6 tests implémentés

## Analyse Technique (Reference)

### Mode Analog vs Digital - Comportement par Instrument

**Fonctionnement du mode Analog (`analog >= 0.5`)** :
- Oscillateurs conservent leur phase actuelle (kick.rs:142-148)
- Enveloppes relancées depuis leur valeur actuelle via `trigger_at_peak()`
- Son organique et continu, comme un vrai circuit analogique
- Retriggers pendant une queue ajoutent de l'énergie plutôt que de réinitialiser
- Comportement similaire aux drum machines analogiques (Roland TR-808/909)

**Mode Digital (`analog < 0.5`)** :
- Oscillateurs réinitialisés à phase = 0.0 avec crossfade sur 2 samples (kick.rs:150-165)
- Enveloppes repartent de zéro via `trigger()`
- Son propre et répétable, idéal pour l'EDM et le techno
- Chaque hit sonne identique, même sur des retriggers rapides
- Comportement similaire aux drum machines numériques (Roland TR-626, LinnDrum)

**Implémentation technique par instrument** :

**Kick (kick.rs)** :
- Analog: `self.osc.phase` préservé, `self.noise_osc.phase` préservé
- Digital: Crossfade entre ancienne et nouvelle phase sur 2 samples
- Impact sonore: Analog = plus de "punch" sur les retriggers, Digital = plus précis

**Kick 808 (kick_808.rs)** :
- Analog: Phase préservée, simulate le comportement du circuit original
- Digital: Réinitialisation complète ("cold start" comme l'original 808)
- Impact sonore: Analog = plus chaud, Digital = plus cliquety

**Snare (snare.rs)** :
- Analog: Phase préservée + noise generator NON reseedé
- Digital: Phase réinitialisée + noise generator reseedé
- Impact sonore: Analog = plus de variation naturelle, Digital = plus constant

**Snare 606 (snare606.rs)** :
- Analog: Comportement similaire au snare mais avec envelope différente
- Digital: Réinitialisation complète comme le 606 original
- Impact sonore: Analog = plus organique, Digital = plus mécanique

**Tom (tom.rs)** :
- Analog: Phase préservée pour un son plus naturel
- Digital: Réinitialisation pour un son plus synthétique
- Impact sonore: Analog = comme des toms acoustiques, Digital = comme des toms électroniques

**Instruments SANS mode Analog/Digital** (toujours "analog") :
- Clap: Toujours analog (0.3) - nécessite la continuité pour le son réaliste
- HiHat: Toujours analog (1.0) - les retriggers doivent être fluides
- OpenHiHat: Toujours analog (1.0) - même raison que HiHat
- Ride: Toujours analog (1.0) - nécessite un decay naturel
- Cymbal: Toujours analog (1.0) - le shimmer nécessite la continuité
- Perc1: Valeur intermédiaire (0.3) - comportement hybride
- Zap: Valeur basse (0.0) - mais toujours traité comme analog

**Valeurs par défaut et plage typique** :
- Analog pur: 1.0 (Kick, Snare, Tom, HiHat, etc.)
- Digital pur: 0.0 (utilisé pour les sons électroniques précis)
- Hybride: 0.3-0.7 (pour un mélange des deux caractères)

**Impact CPU par mode** :
- Analog: Légèrement plus élevé (calculs de phase préservée)
- Digital: Légèrement plus bas (réinitialisations simples)
- Différence: <2% sur un Core i7 (mesuré avec `test_high_cpu_load_patterns`)

**Quand utiliser chaque mode** :
- Analog: Sons organiques, patterns denses (>120 BPM), caractère vintage
  Ex: House, Disco, Funk, Drum & Bass
- Digital: Sons propres, patterns clairsemés (<110 BPM), caractère moderne
  Ex: Techno, Minimal, Electro, Trance
- Hybride (0.3-0.7): Pour un mélange des deux caractères
  Ex: Progressive House, Melodic Techno

**Guide pratique par instrument** :

**Kick** :
- Analog (1.0): Idéal pour House/Disco - retriggers ajoutent du punch
- Digital (0.0): Parfait pour Techno - chaque hit identique
- Test: Essayez un pattern 16e notes à 125 BPM avec release=300ms

**Snare** :
- Analog (1.0): Son réaliste comme une vraie caisse claire
- Digital (0.0): Son électronique précis pour l'EDM
- Astuce: En mode analog, activez le noise pour plus de réalisme

**Tom** :
- Analog (1.0): Sons comme des toms acoustiques
- Digital (0.0): Sons synthétiques style 808
- Conseil: Utilisez analog pour les fills, digital pour les riffs

**HiHat/OpenHiHat** :
- Toujours analog (1.0) - ne peut pas être changé
- Pourquoi: Les retriggers rapides nécessitent une continuité parfaite
- Astuce: Utilisez le paramètre "Tight" pour ajuster le caractère

**Clap** :
- Toujours analog (0.3) - valeur fixe
- Pourquoi: Le son réaliste nécessite la continuité des oscillateurs
- Alternative: Utilisez le Snare en mode digital pour un clap électronique

**Ride/Cymbal** :
- Toujours analog (1.0) - pour le shimmer naturel
- Astuce: Ajustez le paramètre "Shimmer" pour plus/moins de brillance

**Perc1** :
- Valeur intermédiaire (0.3) - comportement hybride
- Utilisation: Pour des sons de percussion intermédiaires
- Expérimentation: Essayez entre 0.1 et 0.5 pour différents caractères

**Zap** :
- Valeur basse (0.0) mais traité comme analog
- Comportement: Son électronique avec une touche organique
- Utilisation: Pour des effets spéciaux et transitions

**Recettes par style musical** :

**1. Classic House (à la Kerri Chandler)** :
- Kick: 0.9 (légèrement digital pour la précision)
- Snare: 1.0 (full analog pour le groove)
- HiHat: 1.0 (toujours analog)
- Tom: 0.8 (presque analog)
- Clap: 0.3 (défaut)
- Groove: Swing16 à 55%

**2. Detroit Techno (à la Jeff Mills)** :
- Kick: 0.2 (très digital pour la précision)
- Snare: 0.3 (légèrement analog pour le corps)
- HiHat: 1.0 (toujours analog)
- Tom: 0.4 (mi-chemin)
- Clap: 0.3 (défaut)
- Groove: Straight (pas de swing)

**3. Drum & Bass (à la LTJ Bukem)** :
- Kick: 0.7 (analog pour les retriggers rapides)
- Snare: 0.8 (presque analog pour le groove)
- HiHat: 1.0 (toujours analog)
- Tom: 0.9 (presque analog)
- Clap: 0.3 (défaut)
- Groove: Shuffle à 40%

**4. Minimal Techno (à la Richie Hawtin)** :
- Kick: 0.1 (très digital)
- Snare: 0.2 (très digital)
- HiHat: 1.0 (toujours analog)
- Tom: 0.3 (digital)
- Clap: 0.3 (défaut)
- Groove: Straight (pas de swing)

**Conseils avancés** :

1. **Automatisation du paramètre analog** :
   - Automatisez le paramètre analog pendant un breakdown
   - Passez de digital (précis) à analog (organique) pour un effet dramatique

2. **Per-instrument settings** :
   - Chaque instrument peut avoir sa propre valeur analog
   - Ex: Kick digital (0.2) + Snare analog (1.0) = combo puissant

3. **Pattern density** :
   - Patterns denses (>120 BPM, 16e notes) → privilégiez analog
   - Patterns clairsemés (<110 BPM, 8e notes) → digital fonctionne bien

4. **Velocity interaction** :
   - En mode analog: la velocity affecte plus le timbre
   - En mode digital: la velocity affecte plus le volume

**Dépannage** :

Problème: "Mon kick sonne différent à chaque hit"
- Solution: Passez en mode digital (0.0) pour une consistance parfaite

Problème: "Mon pattern dense sonne mécanique"
- Solution: Passez en mode analog (1.0) pour plus de groove

Problème: "Je veux un mélange des deux"
- Solution: Essayez des valeurs entre 0.3 et 0.7

**Exemples de réglages par style** :
- TR-808 style: Kick=1.0, Snare=1.0, Tom=1.0 (full analog)
- TR-909 style: Kick=0.8, Snare=0.7, Tom=0.9 (légèrement digital)
- Modern Techno: Kick=0.2, Snare=0.3, Tom=0.4 (plus digital)
- Acoustic simulation: Tous à 1.0 avec long decay
