> Ce fichier ne contient que ce qui reste **a faire ou en cours**.
> Tout ce qui est termine vit dans [DONE.md](DONE.md).

## Nouvelles tâches — session 2026-09-24 (plan de remédiation audit)

> Source : `audit_cr/claude-code.json` (audit complet, constats vérifiés dans le code le 2026-09-24). Chaque finding de l'audit est couvert par un ticket ci-dessous. Règle : chaque phase se termine par build + install + CHANGELOG + checklist « À tester dans Studio One ».
> **Prérequis avant tout** : le bundle installé est actuellement amputé du helper MIDI (install refusée 2026-09-23, S1 ouvert) — la phase 0 se termine par une réinstallation propre, ce qui règle aussi la validation [243] en attente.

### Phase 3 — Robustesse des données utilisateur
- [ ] [259] **REPRENDRE ICI** — **Chemins de textures réseau (UNC)** : `user_textures.rs:44` — garde-fou `is_local_path()` (Disk/VerbatimDisk uniquement) en tête de `reload_all()` et `write_slot_sound` ; un chemin réseau restauré (état VST3 ou preset reçu) passe la lane en « missing » sans résolution synchrone (fuite NTLMv2 + gel SMB ~20 s/lane à l'ouverture). Résolution réseau uniquement via sélecteur/drag explicite, hors thread principal.
- [ ] [260] **Dossier utilisateur unifié** : nouveau `src/paths.rs` avec `dirs::document_dir()` (dépendance `dirs = "6"`) remplaçant les 4 copies `USERPROFILE` (config.rs:62, presets.rs:311, preset_dumps.rs:24, ui/midi.rs:20) ; migration au 1er lancement si l'ancien dossier existe (OneDrive Known Folder Move, macOS). Règle de portabilité CLAUDE.md enfin appliquée.
- [ ] [261] **Erreurs de presets/config avalées** : `preset_browser.rs` — afficher `last_error` dans le modal, ne vider le champ nom qu'en cas de succès (lignes 381-396, 494) ; `sanitize_name` doit écarter CON/PRN/AUX/NUL/COM1-9/LPT1-9 ; écritures atomiques (temp + rename) ; config.json illisible renommé en `.bad` au lieu d'être écrasé (config.rs:43-45).
- [ ] [262] **Travail recalculé à chaque image UI** : cache de `list_presets` dans `PresetBrowserState` (invalider à l'ouverture/changement d'onglet/save/rename/delete — preset_browser.rs:245) ; `grid.rs:309-310` remplacer le clone de `TrackLayoutState` par `grid_slot()` sans verrou (test d'équivalence `is_grid_follower`/`grid_slot`).

### Phase 4 — Filets de sécurité (tests & CI)
- [ ] [263] **Tests de persistance réels** : fixture `tests/fixtures/` (JSON d'un get_state réel, jamais supprimée, une par changement de format) restaurée via `filter_state` + `deserialize_fields` avec snapshot figé ; test roundtrip dans l'ordre alphabétique puis inverse ; tests hermétiques (injecter le dossier de config, `FLASH_DRUM_CONFIG_DIR`) ; ne plus compter les tests `test_standalone` dans les bilans.
- [ ] [264] **CI durcie** (`.github/workflows/ci.yml`) : job macOS (`cargo check`), `RUSTFLAGS=-D warnings`, `cargo clippy --all-targets --locked`, `--locked` partout, toolchain 1.94.0 (= binaire livré), tests des patchs vendorés (egui-baseview `file_drop`, remap multi-out), actions épinglées par SHA, `permissions: contents: read`, dependabot + `cargo deny check advisories`, protection de `main`.

### Phase 5 — Légal & dépôt public (à faire sans travail en cours, force-push)
- [ ] [265] **PDF protégé dans le dépôt public** : retirer `resources/Drum.Machine.-.260.Patterns.pdf` et purger l'historique (`git filter-repo`, sauvegarde préalable, force-push assumé) ; garder une référence bibliographique simple.
- [ ] [266] **Licence** : `LICENSE` (GPL-3.0) à la racine + dans le bundle, `license = "GPL-3.0-only"` dans Cargo.toml, `THIRD-PARTY.md` (nih-plug, egui-baseview, ac606 MIT, IBM Plex OFL, provenance des WAV embarqués).

### Phase 6 — Traçabilité & dette (plus tard)
- [ ] [267] **Fork nih-plug traçable** : `vendor/nih-plug/FLASH-DRUM-PATCHES.md` sur le modèle egui-baseview (SHA amont, liste exhaustive des ~9 patchs, `.patch` issu de `git diff`) ; compléter `STUDIO_ONE_MULTI_OUT.md` (remap aux clairsemés, IEditController, fenêtre clavier, journal d'état) ; vendorer les 3 deps git (vst3-sys par branche !, baseview, clap-sys) ou les épingler par rev sur un fork contrôlé ; `windows-sys` sous `[target.'cfg(windows)'.dependencies]`.
- [ ] [268] **Hygiène dépôt** : `#![allow(clippy::excessive_precision)]` sur la table sinc ac606 puis traiter les lints par lot ; `cargo fmt` en commit dédié ; archiver le CHANGELOG par trimestre ; sortir les gros PNG de design du dépôt ; resserrer `.gitignore` (`*backup*`, `*fixed*`, `temp_*`) ; supprimer `bundle.toml` (inutilisé et faux) ; `git gc`.
- [ ] [269] **Dette maintenabilité** : auditer les 61 `#[allow(dead_code)]` ; dispatch `DrumVoiceKind` par macro ; listes par index de voix dans l'UI → champs du registre (`has_analog_drift`, …) — c'est le patron qui a produit [247] et [248] ; découper `process()` et `sound_editor.rs`.

### Idées notées (2026-09-24, validation S1)
- [ ] [270] **Drop WAV sur une lane occupée = remplacer son fichier ?** — le drop crée toujours une NOUVELLE lane One-Shot (jamais de remplacement, choix [243] anti-accident) ; l'utilisateur s'attendait à un remplacement de la lane ciblée (build 20260924-185138). À arbitrer : remplacement direct, ou modificateur (Alt+drop), ou garder tel quel.

## Nouvelles tâches — session 2026-09-22

> Demandes utilisateur du 2026-09-22, analysées et batchées (plan validé). Note : l'auto-assign **audio** existait déjà ([230]).

### Plus tard (gros chantiers, cadrage d'abord)
- [ ] [241] **MIDI learn pour les notes de lane**.

## Nouvelles tâches — session 2026-08-26

> Ticketisation des notes utilisateur (`docs/notes/notes.txt`). Priorité : quick wins d'abord, features moyennes ensuite, gros chantier en fin.

### P1 — Quick wins
- [ ] [209] **Equilibrer les volumes par defaut de tous les instruments** - les charleys sont generalement trop forts. Analyser les defauts du registre (`sound_settings_default[2]` = volume, et le `VoiceSettings::<voix>()` correspondant) sur les **25 voix**, mesurer le niveau reellement produit par chacune a ses reglages d'usine, puis proposer un jeu de volumes coherent. **Mesure d'abord** : un volume ne dit rien du niveau percu, il depend de l'enveloppe, du filtre et de la saturation de la voix - c'est ce qui a rendu [207] concluant. Precedent : [169] avait remonte le clap de 0,7 a 1,0 a l'oreille, sans mesure. **Touche le son de toutes les voix, donc arbitrage utilisateur avant d'appliquer**, et les sessions existantes ne doivent pas etre reecrites (meme regle que [193] [203] [204]).

## Nouvelles tâches — session 2026-08-14

### P2/P3 — Gros chantiers
- [ ] [166] **REPRENDRE ICI** — **Mixer le stutter avec les cellules fusionnées** — étudier attentivement l'interaction stutter × fusion (actuellement exclusifs) : sémantique temporelle, rendu audio, export MIDI [158], UI/plocks. **Bien étudier le point avant de coder.**

### Idées notées (2026-08-16)
- [ ] [173] **Presets d'usine de départ** — composer et embarquer les premiers presets factory (instruments/patterns/grids/songs) via l'outil « Export factory (dev) » + `assets/presets/` + `factory_presets.rs`.

---

## Nouvelles tâches — session 2026-07-29

### Features moyennes (P2)
- [ ] [146] **Enveloppes exponentielles négatives** — pour des attaques plus claquantes (courbe d'attaque exp inversée, par voix ou global ?).

---

## Fonctionnalites P3 (Avancees / Complexes)
- [ ] [69] Creer un instrument percussif a base de wavetables — phase recherche et prototypage (Complexité: Élevée, 2-4 semaines, P3)
- [~] [83] **Instruments sampler TR-606 multisamplé** — **1er instrument livré : BD6smp (build 20260802-160117)**
  - [x] Nouveau type de voix "Sampler" (multisample) — `synthesis/sample_bank.rs` + `synthesis/bd606.rs`
  - [x] Sélection aléatoire du layer à chaque trigger (sans répétition immédiate) pour simuler l'imperfection analogique
  - [x] Chargement de samples WAV embarqués (`include_bytes!` + `OnceLock`, zéro alloc audio thread)
  - [~] Étendre aux autres instruments de la 606 (SD, HH, …) — même infra, il suffit d'ajouter les WAV
    - [x] SD6smp (Snare 606)
    - [x] CH6smp (Closed Hi-hat 606, note MIDI 42) — build 20260804-170126, `wav/CH.wav` → `assets/ch606.wav`
    - [x] OH6smp (Open Hi-hat 606, note MIDI 46) - livre par [208] (build 20260909-144136)
    - [ ] Cymbal, ... restants
- [ ] [84] **Instruments sampler Yamaha RX11**
  - Même architecture sampler que [83] avec le kit RX11
  - 4 layers par son pour l'effet analog random
  - Dépend de [83] (infrastructure sampler)
  - **Complexité : Moyenne, 2-3 semaines, P3**

## Nouveaux éléments (À prioriser)
- [ ] [56] Ajouter une percussion de type Tom Simmons (Complexité: Moyenne, 3-5 jours)
  - Créer un nouveau module de synthèse
  - Ajouter l'instrument dans le registre des instruments
  - Créer les paramètres spécifiques et l'interface utilisateur
  - Intégrer dans le système de mixage et de sortie audio

## Investigation & Features (A prioriser)
- [ ] [94] **Ajouter un parametre pitch LFO sur les Toms** (P2, Synthese)
  - Intensite, Rate, Type de LFO (sine/triangle/square/saw), arrivee progressive
  - Permet des variations de hauteur dynamiques sur les toms
  - Complexite : Moyenne, 3-5 jours
- [ ] [95] **Ajouter un instrument de type MIDI (avec MIDI out)** (P2/P3, Architecture MIDI)
  - Voix virtuelle qui envoie des NoteOn/NoteOff MIDI sur une sortie MIDI externe
  - Pas de synthese interne, juste du routage MIDI
  - Permet de declencher des instruments externes depuis le sequencer
  - Complexite : Moyenne-Elevee, 1-2 semaines
