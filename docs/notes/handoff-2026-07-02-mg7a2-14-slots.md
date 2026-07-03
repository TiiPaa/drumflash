# Handoff — Session 2026-07-02 — MG-7a.2 : 14 slots actifs

## Objectif de la session
Activer le 14e slot du séquenceur Flash Drum (bouton `+ Add Module`) en passant le moteur audio, le séquenceur et l’UI de 13 voix fixes à 14 slots modulaires, avec onglet Track dans le Sound Editor.

## État à la fin de session
**Terminé et installé.**
- Build ID : `20260702-215053`
- Validation : `cargo check` OK, `cargo test` OK (104 lib + 73 standalone), `build.ps1 -Install` OK.
- Plugin installé dans `C:\Program Files\Common Files\VST3\drum-pattern-vst.vst3`.

## Ce qui fonctionne
- `+ Add Module` active le premier slot inactif (Kind = Kick par défaut).
- Le séquenceur, le synthesizer et les sorties auxiliaires itèrent sur `MAX_TRACKS = 14`.
- `AUX_OUT_COUNT = 14` ; le bus 14 est nommé `Out 14`.
- Onglet `TRK` dans le Sound Editor : change l’instrument du slot, routing Main/Out, note MIDI.
- Migration pattern `pattern-v3` (13 instr) → `pattern-v5` (14 instr) et `pattern-v4` → `pattern-v5`.
- `TODO.md` et `CHANGELOG.md` mis à jour.

## Fichiers modifiés (git status)
```
M CHANGELOG.md
M TODO.md
M drum-pattern-vst/src/generator/euclidean.rs
M drum-pattern-vst/src/generator/mod.rs
M drum-pattern-vst/src/generator/styles.rs
M drum-pattern-vst/src/lib.rs
M drum-pattern-vst/src/sequencer/mod.rs
M drum-pattern-vst/src/sequencer/pattern.rs
M drum-pattern-vst/src/sequencer/stress_tests.rs
M drum-pattern-vst/src/ui.rs
?? docs/superpowers/          # artefacts brainstorming/plan de la session
```

> Note : `drum-pattern-vst/src/sequencer/mod.rs` affiche un gros diff stat (~1.6k lignes) principalement dû à une normalisation de fins de ligne. Le contenu fonctionnel n’a pas été réécrit en profondeur.

## Points techniques critiques pour la suite

### 1. Mapping slot → voix
- `DrumVoice::COUNT = 13` (les modèles de synthèse legacy).
- `crate::track::MAX_TRACKS = 14` (slots de l’UI / séquenceur / audio).
- `TrackInstrumentKind::drum_voice_index()` donne l’index de voix utilisé par un slot.
- `Sequencer::slot_voices()` expose le mapping courant.
- Le `DrumSynthesizer` est indexé par **slot** (pas par voix) : `set_voice_settings(slot_idx, ...)`, `trigger(slot_idx, ...)`.

### 2. Audio engine
- `process()` lit `params.track_layout.state.kind_for_slot(slot)` à chaque bloc.
- `last_slot_kinds` détecte les changements de kind et appelle `synthesizer.reinitialize_slot(slot, kind)`.
- Le choke HiHat → OpenHiHat est résolu dynamiquement en cherchant le slot actuellement mappé à `DrumVoice::OpenHiHat`.

### 3. Persistance
- Champ pattern DAW : `pattern-v5` (`PATTERN_STATE_FIELD`).
- `PatternStateV3` et `PatternStateV4` existent pour la migration depuis 13 instruments.
- `track-layout-v1` reste le champ de persistance de la disposition.

### 4. Rupture de compatibilité DAW
- Nombre de sorties auxiliaires : 13 → 14.
- Les projets Studio One existants vont probablement devoir réaffecter leurs bus aux.
- L’identité VST3 (`DrumFlashPlugin1`) est volontairement inchangée.

### 5. Limitations connues / prochaines étapes suggérées
- Le générateur de patterns (`generator/styles.rs`, `generator/euclidean.rs`) a reçu un rôle neutre pour le 14e slot, mais il n’est pas encore intelligent (pas de génération spécifique par type de piste, pas de gestion des duplicates).
- Le Sound Editor `TRK` permet seulement de changer l’instrument/routing/MIDI note ; il ne gère pas encore le renommage manuel du slot, le mute/solo par slot, ou des paramètres de voix indépendants (les paramètres restent par `DrumVoice`, donc deux slots Kick partagent les mêmes réglages globaux).
- Les plocks restent indexés par `voice_idx`, pas par `slot_idx`.

## Validation à refaire si un agent reprend
1. `cargo test`
2. `cargo check`
3. `build.ps1 -Install` (Studio One doit être fermé)
4. Test dans Studio One : ajouter un module, changer son instrument via l’onglet TRK, vérifier que la sortie Out 14 est audible.

## Commandes utiles
```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
cargo test
cargo check
.\build.ps1 -Install
```

## Contexte de session
- Agent précédent : session OpenCode reprenant le MG-7a.2.
- L’utilisateur a insisté pour pouvoir tester régulièrement dans Studio One.
- Tous les checkpoints de MG-7a.2 et MG-8 (Track tab) sont terminés.
