# Changelog

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
