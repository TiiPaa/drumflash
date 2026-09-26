# Changelog

## 2026-09-26 - Décalage de grille : avertissement avant de casser une fusion au bord (build 20260926-191728)

**Branche:** `main` - **Build:** `20260926-191728`
**Validation:** `cargo check --all-targets` sans avertissement ; `cargo test` 496 verts lib (1 nouveau). À valider dans Studio One (liste dans le rapport).

Retour utilisateur sur le décalage livré au build 183316 : une fusion à la frontière pouvait disparaître sans prévenir. Désormais :

- Les flèches **‹ ›** détectent d'abord si une fusion enjambe la frontière dans le sens du décalage (`straddling_fusions`). Si non : décalage immédiat, comme avant.
- Si oui : **modal d'avertissement** listant les fusions concernées (« Lane 3: cells 14-15 ») — **Break & Shift** casse la fusion **en gardant sa première cellule** (qui redevient un pas actif normal et tourne avec le reste), **Cancel** annule tout (la grille ne bouge pas).

## 2026-09-26 - Density Randomize à 80 % + flèches ‹ › de décalage de la grille (build 20260926-183316)

**Branche:** `main` - **Build:** `20260926-183316`
**Validation:** `cargo check --all-targets` sans avertissement ; `cargo test` 495 verts lib (2 nouveaux + fix d'une race entre tests de config découverte au passage). À valider dans Studio One (liste dans le rapport).

Trois demandes utilisateur du 2026-09-26 ; la troisième (swing par lane) est **reportée par prudence** → ticket [275] dans TODO.md avec notes de design.

- **Density de Randomize Lane à 80 % par défaut** (était 30 %) — `default_randomize_density` dans `editor_state.rs`.
- **Décalage de la grille d'une cellule** : deux flèches **‹ ›** dans l'en-tête de la grille, au-dessus de la colonne des noms de lanes. Tout tourne ensemble pour que rien ne se désynchronise : pas, **fusions**, **p-locks son** (valeurs + masques + field masks, mode link/snapshot préservé) et **p-locks séquenceur** (probabilité, stutter, conditions, microtiming, solo). Rotation **avec wrap dans la longueur du pattern** (rien n'est perdu). Une fusion qui enjamberait le wrap (ex. 14-15 décalée à droite) ne peut pas être représentée : elle est supprimée (test dédié). Les slots sauvegardés de la banque restent figés, c'est voulu.
- **Fix tests** : la variable `FLASH_DRUM_CONFIG_DIR` [263] mutée par les tests de config pouvait être observée en parallèle par `user_dirs_share_one_root` → mutex `CONFIG_ENV_LOCK` partagé.

## 2026-09-26 - Légende Auto-assign : texte à gauche du bouton, comme la ligne Macros (build 20260926-160139)

**Branche:** `main` - **Build:** `20260926-160139`
**Validation:** `cargo check` sans avertissement ; `cargo test` 493 verts. Retour utilisateur sur le build 151533 : la ligne de légende pleine largeur cassait la mise en page. Désormais « Number lanes from the root note » à gauche du bouton (même gabarit que la ligne Macros en dessous), l'explication détaillée reste au survol. À valider visuellement dans S1 : Settings › MIDI.

## 2026-09-26 - [273] Le drop WAV refusé dit pourquoi + légende Auto-assign + version 0.9.0 (build 20260926-151533)

**Branche:** `main` - **Build:** `20260926-151533` - **Version produit:** **0.9.0** (arbitrage utilisateur, était 0.2.0)
**Validation:** `cargo check --all-targets` sans avertissement ; `cargo test` 493 verts lib ; 4 tests egui-baseview verts (dont le nouveau) ; install atomique OK ; installeur régénéré : **`dist\FlashDrum-Setup-0.9.0.exe`**. À valider dans Studio One (liste dans le rapport).

- **[273] Le drop refusé s'explique** : glisser un WAV quand la grille est pleine ou qu'un modal est ouvert n'affichait qu'un curseur ⊘ — c'est ce qui avait fait croire à un bug (avant de découvrir le mode admin / UIPI). Désormais une étiquette près du curseur dit pourquoi : « Grid full - an occupied lane is never replaced; remove one first. » ou « Close the Presets/Macros/Settings dialog to drop files. » Troisième patch du vendor egui-baseview (`rejected_hover_position`, documenté dans `FLASH-DRUM-PATCHES.md`, test `rejected_drag_reports_its_position_until_it_leaves`).
- **Légende Auto-assign MIDI** (Settings › MIDI) : le bouton n'avait d'explication qu'au survol ; une ligne visible en permanence dit ce qu'il fait (« Active lanes take consecutive MIDI notes from Lane 1's root. »).
- **[274] Version produit 0.9.0** : `Cargo.toml` et l'installeur bumpés ensemble (convention : la version produit bouge sur arbitrage, les build IDs restent la traçabilité quotidienne).

## 2026-09-25 - [272] Installeur Windows (Inno Setup) (pas de nouveau build)

**Pas de build plugin** : outillage de distribution uniquement.

- **`installer/flash-drum.iss`** : installe le bundle complet (DLL + helper MIDI + `LICENSE.txt` + `THIRD-PARTY.md`) dans `C:\Program Files\Common Files\VST3\drum-pattern-vst.vst3`, avec écran de licence GPL et désinstallateur Windows standard (Ajout/Suppression de programmes).
- **`.\build.ps1 -Installer`** : nouveau switch qui compile le bundle puis l'installeur dans `dist\FlashDrum-Setup-0.2.0.exe` (dossier `dist/` ignoré par git). Inno Setup 6 requis (`winget install JRSoftware.InnoSetup`).
- La version de l'installeur est le `#define AppVersion` en tête du `.iss` — à synchroniser avec `Cargo.toml` lors d'un bump.

À tester : double-cliquer `dist\FlashDrum-Setup-0.2.0.exe` (UAC admin), vérifier l'install + un drag & drop MIDI dans S1, puis la désinstallation depuis les Paramètres Windows.

**Note support** : un utilisateur qui lance son DAW **en administrateur** ne pourra pas glisser-déposer de WAV/MIDI depuis l'Explorateur (barrière UIPI de Windows, curseur ⊘) — lancer le DAW normalement.

## 2026-09-25 - Build de resynchronisation (build 20260925-222838)

**Branche:** `main` - **Build:** `20260925-222838`
**Validation:** `cargo test` 493 verts lib. Recompile la source exacte après les commits [264]-[268] (formatage, lints, pins — **aucun changement de comportement** par rapport au build 20260925-145840). Non-régression seulement à vérifier dans S1.

## 2026-09-25 - [267][268] Fork nih-plug traçable + hygiène dépôt (pas de build)

**Branche:** `main` - **CI:** windows + macOS verts. Changements sans impact sur le binaire installé (docs, pins, formatage, poids du dépôt) ; 493 tests verts après `cargo fmt`.

- **[267]** `vendor/nih-plug/FLASH-DRUM-PATCHES.md` : les **9 patchs** du fork enfin inventoriés (5 multi-out documentés + remap aux clairsemés, IEditController, fenêtre clavier, journal d'état) — révision amont inconnue documentée honnêtement, diff reconstituable par contenu. `vst3-sys` épinglé par **rev** au lieu d'une branche mouvante ; `windows-sys` sous `cfg(windows)` ; `STUDIO_ONE_MULTI_OUT.md` à jour (sorties `Out 1..14`).
- **[268]** `excessive_precision` autorisé au niveau module sur la table FIR ac606 (le `-A` global de la CI retiré) ; `cargo fmt` appliqué aux 63 fichiers ; **CHANGELOG archivé par trimestre** (656 → 273 Ko, le H1 et avant dans `docs/historique/changelog/`) ; `atlas-pads.png` (11 Mo) hors du suivi git (reste sur le poste ; l'historique n'a pas été réécrit une 2e fois — option notée) ; `.gitignore` resserré ; `bundle.toml` (faux et inutilisé) supprimé ; `git gc` (pack 28 Mio).

**Plan d'audit : terminé à 5/6 + traçabilité.** Reste [269] (dettes de maintenabilité : dead_code, dispatch par macro, champs registre, découpage) — session dédiée.

## 2026-09-25 - [265] PDF « 260 Drum Machine Patterns » purgé du dépôt public (pas de build)

**Branche:** `main` (réécrite) - **CI:** windows + macOS **verts** après réécriture.

Le scan du recueil publié (16,9 Mo, commit `140fdf8`, ~la moitié du pack git) exposait le dépôt public à un retrait DMCA. Historique **réécrit** avec `git filter-repo` : le PDF n'existe plus dans aucun des 205 commits (pack git 28 Mio), sauvegarde complète préalable (`drumflash-backup-2026-09-25.bundle`), force-push assumé. Le fichier reste sur le poste en référence, **ignoré** via `.gitignore` (avec la référence bibliographique) pour ne jamais être recommité. Premier run de la CI durcie [264] au passage : macOS vert, clippy calibré (`-D correctness/suspicious`, 3 `empty_line_after_doc_comments` corrigés, `unexpected_cfgs` toléré sur macOS pour les macros objc vendorées).

**Attention si tu clones le dépôt ailleurs** : l'historique a changé (nouveaux hashes) — il faut re-cloner ou `git fetch && git reset --hard origin/main`.

## 2026-09-25 - [266] Licence GPL-3.0 posée (build 20260925-145840)

**Branche:** `main` - **Build:** `20260925-145840`
**Validation:** `cargo check --all-targets` sans avertissement ; `cargo test` 493 verts lib ; le bundle installé contient `LICENSE.txt` + `THIRD-PARTY.md`. Rien à tester dans Studio One (aucun changement de code).

- **`LICENSE`** (texte canonique GPL-3.0, téléchargé depuis gnu.org) à la racine et copié dans chaque bundle (`build.ps1`) — le dépôt public n'avait aucune licence alors que l'export VST3 (vst3-sys, GPLv3) l'impose aux binaires diffusés.
- **`Cargo.toml`** : identifiant SPDX correct `GPL-3.0-only` (au lieu de la forme dépréciée `GPL-3.0`).
- **`THIRD-PARTY.md`** : nih-plug (ISC), vst3-sys (GPL-3.0), baseview + egui-baseview (MIT), egui (MIT/Apache-2.0), code AC606 porté (MIT, Matthew Fecher), polices IBM Plex (OFL), textures Rift (travail original via `tools/gen_textures.py`). **Point ouvert** : provenance exacte des 4 WAV TR-606 à documenter (question posée à l'auteur).

Reste dans la phase 5 : [265] purge du PDF « 260 Drum Machine Patterns » du dépôt public + de l'historique (`git filter-repo` + force-push — accord explicite requis).

## 2026-09-25 - [263][264] Remédiation audit, phase 4 : filet de persistance + CI durcie (build 20260925-143151)

**Branche:** `main` - **Build:** `20260925-143151`
**Validation:** `cargo check --all-targets --locked` sans avertissement ; `cargo test` 493 verts lib (4 nouveaux ; désormais seuls les tests lib sont comptés, les 310 de `test_standalone` sont les mêmes recompilés) ; clippy vert ; 3 tests egui-baseview verts. À valider dans Studio One (rien d'audible n'a changé — vérification de non-régression seulement).

- **[263] Filet de persistance** : un agent qui change l'ordre/la taille d'un champ persisté ET met à jour les tests d'aller-retour dans le même commit laissait tout au vert — et les projets des utilisateurs se rouvraient décalés. Désormais : `tests/fixtures/state-2026-09-25.json` (état complet écrit par un build daté, peuplé via les 10 champs persistés) restauré par le vrai chemin de l'hôte (`filter_state` + `deserialize_fields`) et comparé à un résumé **figé** ; le générateur `regenerate_golden_fixture` est un test `--ignored` à n'utiliser qu'après un changement de format voulu (une fixture par format, jamais supprimée). Ajouté : `full_state_roundtrip_is_order_independent` (l'hôte restaure par ordre alphabétique — `track-layout-v1` en dernier ; l'ordre inverse doit donner le même état) et la variable `FLASH_DRUM_CONFIG_DIR` qui rend les tests hermétiques (fini la dépendance au `config.json` réel du poste, utilisée par la CI).
- **[264] CI durcie** : job **macOS** (`cargo check` — la règle de portabilité est enfin surveillée), `-D warnings`, **clippy** (`-A excessive_precision` en attendant [268]), `--locked` partout, toolchain **1.94.0** (= celle du binaire livré, comme `rust-toolchain.toml`), tests du pont `file_drop` d'egui-baseview en CI, `permissions: contents: read`, **actions épinglées par SHA** (checkout v4.3.0, rust-cache v2.8.1, upload-artifact v4.6.2, rust-toolchain) et `dependabot.yml` (actions + cargo, hebdo). Reste manuel : activer la protection de `main` dans GitHub, et cargo-deny (phase 6).

## 2026-09-25 - [271] Modal de confirmation après export MIDI (build 20260925-140504)

**Branche:** `main` - **Build:** `20260925-140504`
**Validation:** `cargo check --all-targets` sans avertissement ; `cargo test` 489 verts ; install atomique OK. À valider dans Studio One (liste dans le rapport).

Demande utilisateur : après un clic sur **Export**, un modal s'ouvre avec le **chemin complet** du fichier `.mid` écrit et un bouton **Open folder** qui ouvre l'Explorateur sur `Documents\Flash Drum\exports` (le dossier d'export était introuvable sans le connaître). Bouton **OK** pour fermer. Le drag MIDI (bouton Drag) est inchangé — pas de modal, le fichier part directement dans le drag. Nouvelle dépendance `open = "5"`.

## 2026-09-25 - [259]-[262] Remédiation audit, phase 3 : données utilisateur robustes (chemins, presets, erreurs) (build 20260925-113806)

**Branche:** `main` - **Builds:** `20260925-101355` (install refusée, S1 ouvert — le garde-fou [250] a protégé le bundle installé) -> `20260925-113806`
**Validation:** `cargo check --all-targets` sans avertissement ; `cargo test` 489 + 1 + 310 verts (6 nouveaux) ; install atomique avec vérification de hash. À valider dans Studio One (liste dans le rapport).

- **[259] Chemins réseau jamais résolus automatiquement** : un projet ou preset reçu d'un tiers pouvait contenir un chemin WAV en `\\hôte\partage` — la restauration le résolvait sans clic, déclenchant une authentification SMB silencieuse (fuite du hash NTLMv2) ou gelant Studio One ~20 s par lane sur un serveur mort. Nouveau garde-fou `is_local_path()` (disques locaux uniquement) : la restauration d'état et le chargement de preset marquent la lane « missing » avec un message, sans toucher au réseau. Un fichier réseau reste chargeable par geste explicite (sélecteur de fichiers, drag & drop).
- **[260] Dossier utilisateur unifié et correct partout** : les 4 copies de `USERPROFILE\Documents` (config, presets, dumps, exports MIDI) laissaient les données dans un dossier fantôme quand OneDrive déplace Documents, et échouaient en silence sur macOS. Nouveau `src/paths.rs` sur `dirs::document_dir()` (le vrai Documents, OneDrive inclus), migration automatique au premier lancement si l'ancien dossier existe, et test `user_dirs_share_one_root`.
- **[261] Les erreurs de presets sont enfin visibles** : « Save » qui échoue (disque plein, dossier verrouillé) vidait le champ de nom comme si tout avait marché — l'erreur s'affiche maintenant dans le modal et le nom est conservé ; pareil pour rename/delete. Les noms réservés Windows (CON, AUX, NUL, COM1-9, LPT1-9) sont suffixés au lieu d'échouer bizarrement. Écritures atomiques (fichier temporaire + rename) pour presets et config — un crash en cours d'écriture ne laisse plus de fichier tronqué, et un `config.json` illisible est renommé en `.bad` au lieu d'être effacé par les défauts.
- **[262] Liste des presets mise en cache** : tant que le modal Presets était ouvert, chaque frame relisait le dossier, relisait et re-parsait CHAQUE JSON (mégaoctets à 60 fps pendant la lecture). La liste est lue à l'ouverture, au changement d'onglet et après chaque mutation, jamais dans le rendu. *Écart par rapport à l'audit :* le clone de layout de la grille est conservé — il porte le nom et le kind de la lane, sans équivalent atomique ; à la place, un test garantit l'équivalence entre le `grid_slot` du thread audio et celui de l'UI.

## 2026-09-24 (soir) - [255]-[258] Remédiation audit, phase 2 : installation atomique, journal de crash, symboles exploitables (build 20260924-194326)

**Branche:** `main` - **Builds:** `20260924-193928` -> `20260924-194326`
**Validation:** `cargo check --all-targets` sans avertissement ; `cargo test` 483 verts ; `test-verification.ps1` vert (helper présent, hash DLL+helper identiques) ; binaire vérifié sans chemins locaux (`baboost`, `E:\Dev`, `E:\tmp` absents). À valider dans Studio One (liste dans le rapport).

- **[255] Install atomique** : fini le bundle à moitié détruit du 2026-09-23. `build.ps1` copie d'abord vers `<dest>.new`, vérifie les **hash** du DLL et du helper stagés contre les artefacts compilés (échec = rien n'est modifié), puis bascule par **rename** (`<dest>` → `<dest>.old`, staging → `<dest>`, rollback sur l'ancien si le rename échoue). Nouveau paramètre `-InstallRoot` pour tester hors dossier système. `test-verification.ps1` durci : helper manquant ou hash différent = **échec** (plus un avertissement).
- **[256] Fallback du helper MIDI réservé au dev** : le chemin `CARGO_MANIFEST_DIR\build\…` dans `ui/midi.rs` n'existe plus qu'en `debug_assertions` — en release, un helper absent de l'install est enfin visible (« MIDI drag helper not found ») au lieu d'être masqué par le dossier de build du poste de dev.
- **[257] Journal de crash** : avec `panic = "abort"`, une panique fermait Studio One sans la moindre trace. Hook chaîné installé à la création du plugin : chaque panique ajoute une ligne `date | build ID | thread | fichier:ligne | message` dans `%TEMP%\flash-drum-crash.log` (borné à 256 Ko, repart de zéro au-delà).
- **[258] Symboles et chemins** : le PDB archivé suit maintenant le profil (`-Debug` archivait un PDB release périmé), l'archivage n'a lieu **qu'après une install réussie**, conservation portée de 10 fichiers à **30 jours**, `debug = "line-tables-only"` enrichit les PDB (piles de crash résolubles), et `--remap-path-prefix` (via `CARGO_ENCODED_RUSTFLAGS`, qui tolère l'espace de « Drum Flash ») purge les chemins locaux du binaire — vérifié : `baboost`, `E:\Dev\Projets` et `E:\tmp` ne figurent plus dans le DLL livré. `cargo test --lib` tourne avant chaque `-Install` (désactivable : `-SkipTests`).

## 2026-09-24 (soir) - [253][254] Remédiation audit, phase 1 : plus aucune libération de WAV sur le thread audio + filet assert_no_alloc (build 20260924-185138)

**Branche:** `main` - **Build:** `20260924-185138`
**Validation:** `cargo check --all-targets` sans avertissement ; `cargo test` 483 + 1 + 309 verts (3 nouveaux tests RT) ; `build.ps1 -Install` OK ; **validé dans S1 le 2026-09-24**.

- **[253] File de retraite des textures** : une voix One-Shot/Rift garde son propre `Arc<TextureBank>` d'un coup à l'autre ; après DEUX remplacements du fichier d'une lane sans coup entre-temps, la voix détenait la dernière référence du premier fichier et le libérait dans `trigger()` — jusqu'à ~23 Mo libérés dans le callback (craquement possible, garantie CLAUDE.md non tenue). Idem au changement de kind en lecture (`reinitialize_slot` écrasait la voix et lâchait son `source`). Désormais : `TexturePool::retire()` pousse l'ancien `Arc` dans une `ArrayQueue` préallouée (64 entrées, push lock-free sans allocation), vidée par le thread UI à chaque frame de l'éditeur et à chaque `publish` (`drain_retired`). Trois sites couverts dans One-Shot (remplacement + deux sorties sans fichier), un dans Rift, plus `release_held_texture()` avant le swap de voix dans `reinitialize_slot`.
- **[254] Filet `assert_no_alloc`** : la règle « zéro allocation/libération sur le thread audio » était appliquée à la relecture, sans outillage — d'où les deux défauts ci-dessus passés inaperçus. Ajout de la dev-dependency `assert_no_alloc` (même fork/révision épinglée que nih-plug), watchdog enregistré dans les builds de test de la lib ET de `test_standalone`, et trois tests qui échouent avant les fixes [244]/[245]/[253] : `save_pattern_to_slot_is_realtime_safe` (capture banque après restore), `trigger_never_frees_a_texture_on_the_audio_thread` (A→B→C puis trigger : la texture est retirée, pas libérée), `every_kind_plays_without_allocations` (les 25 kinds : trigger, 4800 échantillons, retrigger à queue vivante, `set_settings` en cours de lecture, 4800 échantillons — zéro allocation).

## 2026-09-24 - [244]-[252] Remédiation audit, phase 0 (quick wins) : critique temps réel + bugs données + docs (build 20260924-124332)

**Branche:** `main` - **Build:** `20260924-124332`
**Validation:** `cargo check --all-targets` sans avertissement ; `cargo test` 480 + 1 + 307 verts (3 nouveaux) ; 67 tests nih-plug verts ; `build.ps1 -Install` OK (répare au passage le bundle installé amputé du helper MIDI depuis le 2026-09-23) ; **validé dans S1 le 2026-09-24**.

Source : `audit_cr/claude-code.json`, phase 0 du plan dans `TODO.md`.

- **[244] CRITIQUE — sauvegarde de pattern enfin temps réel** : « Save » sur un slot de la banque (P1…P16) exécutait `refresh_snapshot()` dans `process()` — verrou bloquant + clone des 16 slots + sérialisation JSON (état jusqu'à 9,7 Mo) dans le callback, alors que la variante RT-safe existait. Remplacé par `mark_snapshot_dirty()` (`lib.rs`) : le snapshot persisté est reconstruit paresseusement quand l'hôte demande l'état, hors callback.
- **[245] Préallocation de la banque après restauration** : les `Vec<u8>` désérialisés perdaient la capacité de `PatternSlot::default()` → `capture()` réallouait ~165 Ko de p-locks dans le callback à la première sauvegarde après réouverture d'un projet. `reserve()` rétabli dans `PersistentField::set` (thread principal).
- **[246] Renommage de preset = preset invisible** : `rename_preset` reconstruisait le nom avec `path.extension()` (qui ne rend que `json`) → `Nom.fdpat.json` devenait `Nom.json`, absent de tous les onglets du navigateur. Le `PresetKind` est maintenant passé par l'appelant ; test corrigé (il partait d'une extension simple que `save_json` ne produit jamais) + nouveau test « reste listé ».
- **[247] Graphe Oh6smp : mauvaise banque affichée** : la cascade du Sound Editor donnait `ch606()` (slices 0,5 s) à Oh6smp alors que le DSP joue `oh606()` (1 s) — Start/End réglés sur une forme d'onde deux fois trop courte. Nouvelle fonction partagée `sampler_bank(voice_idx)` (`sample_bank.rs`) + test d'équivalence UI/DSP pour les 4 voix sampler.
- **[248] Repli générateur → panique latente** : `_ => kind.drum_voice_index()` indexait une table de 14 cases hors bornes pour tout futur kind oublié (`panic = "abort"` ferme l'hôte). Match rendu **exhaustif** (erreur de compilation sur un kind manquant, philosophie `DrumVoiceKind`) + test `every_kind_survives_generate` (25 kinds × 4 générateurs).
- **[249] Journal d'état `E:\tmp` en release** : le fork nih-plug écrivait un diagnostic à chaque get/set_state vers un chemin de dev codé en dur, et re-sérialisait l'état complet deux fois par sauvegarde. Désormais **opt-in** (`FLASH_DRUM_STATE_LOG`) et écrit dans le dossier temporaire ; inactif = aucun fichier, aucune sérialisation en plus.
- **[250] Install : test de verrou avant suppression** : `build.ps1` ouvre le DLL installé en exclusif AVANT `Remove-Item` — si Studio One le tient, sortie code 2 sans rien toucher (fini le bundle à moitié détruit du 2026-09-23).
- **[251] Clippy + toolchain** : les 2 constantes refusées par `approx_constant` remplacées (`std::f32::consts::TAU`, `std::f64::consts::FRAC_1_SQRT_2`) ; `rust-toolchain.toml` épingle **1.94.0** (CI = binaire livré).
- **[252] Docs resynchronisées** : 27 voix / 25 kinds dans CLAUDE.md, ADDING_AN_INSTRUMENT.md §2 (+ §9 : le rôle générateur oublié est une erreur de compilation, pas un instrument muet), les deux README, user-guide (Auto-Edit déplacé), infrastructure.md (voix, hound en production, 477 tests, toolchain, CI). Garantie « lane textures » de CLAUDE.md corrigée : la lacune One-Shot/Rift est documentée en attendant [253].

## 2026-09-23 (soir) - [243] One-Shot : Offset, enveloppes sampler, waveforms ; fix flash lane ; pitch live One-Shot + Rift (builds 205745 -> 235211)

**Branche:** `main` - **Builds:** `20260923-205745` -> `20260923-235211`
**Validation:** `cargo check` sans avertissement ; `cargo test` 477 + 1 + 306 verts ; tout est validé dans S1 (235211 et 233248 validés le 2026-09-24 avec le build 20260924-124332).

- **Offset** (special 20, p-lockable) : fraction du fichier ou la lecture COMMENCE, dans les deux sens ; en Reverse elle compte depuis la fin (le marqueur reste a la position du knob — premiere version miroir rejetee par l'utilisateur).
- **Enveloppes en fractions de la region jouee** (x pitch), semantique sampler : amp A-H-D, pitch A-H-D, filtre A-H-D tous en 0..1 (1.0 = tout le sample). L'attack en secondes absolues etait inaudible sur sample court — retour utilisateur. Speciaux renommes `_atk`/`_hld` (le garde-fou [182] exige une unite sur `_attack`/`_hold`) ; finders du panneau et test de snapshot d'unites mis a jour.
- **Attack amp reparee en deux temps** : cap 80 ms des voix drum supprime (inaudible sur decay long), puis restart-from-zero a chaque retrigger (`trigger_hard`) — la capture du niveau courant (anti-clic drum) tuait la rampe sur cellules adjacentes et sur les longues attacks. Le declick 3 ms couvre le saut.
- **Waveform en fond** des graphes Amp/Pitch/Filter : la region jouee, inversee en Reverse, avec l'enveloppe sur le meme axe temps (la largeur = la region entiere, clippee a la fin du sample). Graphe Offset section Sample : fichier entier, tête sautee grisee, marqueur.
- **Ordre canonique [240] restaure** : le hoist par suffixe ignorait `_atk`/`_hld` (courbes remontees sous Filter Env, Attack/Hold exiles en queue) ; pitch env declare dans l'ordre canonique ; test de hoist etendu a One-Shot + nouveau test d'ordre du pitch env.
- **« Random Offset on active cells » reserve a Rift** (gate `_wander`).
- **Flash de nom de lane bloque** : egui-baseview jetait le `repaint_delay` demande pendant une frame deja rendue ; la frame d'extinction n'etait jamais schedulee. Deuxieme patch vendor (`FLASH-DRUM-PATCHES.md`).
- **Pitch live (macro/automation [242])** : `base_step` + `region_secs` recalcules dans `set_settings` du One-Shot (valide) et de Rift (build 235211, a installer/valider). nih-plug decoupe le buffer aux points d'automation ; la voix se re-accorde en continu, sans clic (increment = continu en phase). Regle « live vs trigger-latch » ecrite dans `ADDING_AN_INSTRUMENT.md` §5 et la skill nouvel-instrument.

## 2026-09-23 - [243] Depot WAV : raccordement au gestionnaire baseview existant (build 20260923-202713)

**Branche:** `main` - **Build:** `20260923-202713`
**Validation:** `cargo check` sans avertissement ; `cargo test` 469 + 1 + 298 verts ; 3 tests du pont `egui-baseview::file_drop` verts ; `build.ps1 -Install` OK ; **validé dans Studio One le 2026-09-23** (déposer un WAV crée bien la lane One-Shot).

- **Cause confirmee** : le journal du build 200131 renvoie `0x80040101 / DRAGDROP_E_ALREADYREGISTERED` a chaque image. Dans la revision utilisee (`baseview 9a0b42c`), `win/window.rs` enregistre deja son `IDropTarget` et le revoque a la fermeture. Ses evenements `DragEntered / DragMoved / DragDropped` etaient ignores par egui-baseview. L'ajout de `native_drop.rs` etait donc au mauvais niveau.
- **Correction** : copie vendoree d'egui-baseview a la meme revision `ec70c3f`, licence incluse, avec un pont `file_drop`. La grille publie sa zone et l'extension WAV ; l'adaptateur renvoie `AcceptDrop(Copy)` et transmet le fichier a l'image suivante. Suppression du gestionnaire COM redondant, du re-export HWND et de la boucle d'installation/journalisation. Aucun nouvel enregistrement OLE ; etat propre a chaque editeur, recree a chaque ouverture.
- **Position fiable** : utilisation du point de depot fourni par l'OS. Conversion ecran -> client sous Windows puis mise a l'echelle egui (le pointeur egui peut etre perime pendant un drag OS). Les drops hors grille, sans slot libre ou d'un autre format sont refuses.
- **Deuxieme blocage corrige** : `change_slot_kind` ignore les slots inactifs. Le drop appelle maintenant `activate_slot`, comme le bouton Add Module : creation effective, reglages One-Shot et effacement des anciennes donnees musicales du slot. Le WAV est charge ensuite, en stereo si le fichier l'est.
- **Tests** : acceptation WAV / refus autres formats et hors zone, une seule livraison avec sa position, annulation du survol, isolation entre editeurs ; creation reelle d'une lane vide, chargement du fichier, nettoyage des anciennes notes/p-locks et conservation des lanes occupees.
- [243] validé dans S1 et archivé dans DONE.md.

## 2026-09-23 - [243] Diagnostic du depot refuse (build 20260923-200131)

**Build:** `20260923-200131` installe. `cargo check` sans avertissement.

Journal ajoute autour de `OleInitialize` / `RegisterDragDrop` et `DragEnter`. Retour utilisateur : repetition de `DRAGDROP_E_ALREADYREGISTERED`. Contrairement a l'annonce initiale, le journal n'etait pas borne : l'echec relancait l'installation a chaque image et allouait un objet COM sans le liberer. Cette voie est supprimee dans le build 202713 ci-dessus.

## 2026-09-23 - [243] One-Shot, build 2 : drag & drop d'un WAV sur la grille -> lane creee (build 20260923-165905)

**Branche:** `main` - **Build:** `20260923-165905`
**Validation:** `cargo check` warning-clean, `cargo test` 468 + 1 + 298 verts (2 nouveaux), `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

Le chemin « facile » n'existait pas : **egui-baseview ne transmet pas les drops de fichiers de l'OS a egui** (`raw.dropped_files` toujours vide dans le plugin). Il a donc fallu un `IDropTarget` Win32.

- **`src/native_drop.rs`** : COM `IDropTarget` en FFI manuel (miroir entrant de `native_drag.rs`, aucune crate de plus), enregistre sur le HWND du plugin (`PLUGIN_HWND`, re-exporte du vendeur pour l'occasion) au premier frame, une fois. `Drop` lit le `CF_HDROP` (`DragQueryFileW`) et empile les chemins ; la grille les depile au frame suivant. Objet COM leake volontairement : l'enregistrement meurt avec la fenetre, que l'hote detruit toujours avant de decharger la DLL (lecon [186] verifiee). Stubs vides sur macOS (compile des deux cotes).
- **Geste** : un `.wav` lache n'importe ou sur la grille -> la **lane vide sous la souris** (sinon le **premier slot libre**) devient une lane **One-Shot** avec le fichier charge (meme chemin que File > Load... : persistant, suit la lane, saute au kind change). Une lane occupee n'est **jamais remplacee** ; grille pleine = drop ignore ; la plaque de la lane flash pour confirmer.
- Test : `pick_drop_lane` (row vide sous le pointeur > premier slot libre > jamais de remplacement > grille pleine = rien).

## 2026-09-23 - [243] Nouvel instrument One-Shot, build 1 : la voix + fichier par lane (build 20260923-131247)

**Branche:** `main` - **Build:** `20260923-131247`
**Validation:** `cargo check` warning-clean, `cargo test` 466 + 1 + 297 verts (20 nouveaux), `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

One-Shot joue le **fichier sample de la lane**, du debut a la fin - le frere simple de Rift, sans contenu embarque ni offset. Kind **24** / voix **26**, categorie **Perc**, label **OS**, note MIDI **59** (libre), 1 algo, choke 0, role generateur emprunte a Perc1.

- **Fichier par lane via toute l'infra [228]** : meme pool `UserTextures` que Rift (persistant, suit la lane au deplacement/copie/presets, saute si la lane change de kind, decode hors thread audio). Le special `oneshot_texture` est un **marqueur d'infra** qui garde ces regles vivantes ; son « menu » a une entree n'est pas rendu (regle generique : `options.len() <= 1`). Section Osc = **Sample** : ligne **File** + Load.../Clear + switch **Stereo** (grise tant que le fichier n'a pas de canal droit).
- **Enveloppes A-H-D completes avec courbes bipolaires** : amp (standards, decay jusqu'a **10 s** pour que le fichier sonne entier), **pitch** (depth ±24 st + attack/hold/decay + atk/dec curves - nouveau graphe A-H-D dans la section Pitch ; Rift garde sa loi decay-only), **filtre** (meme gabarit que Rift : attack/hold/curves hisses sous Filter Env [240], Filter Decay en standard).
- **Pitch** ±24 st + Pitch Fine ±100 ct (taux de lecture), **filtre LP/HP/BP** + Resonance (meme convention que le plugin : l'enveloppe ouvre la coupure vers 20 kHz), **Reverse** (lecture a l'envers), pack saturation [232], retrigger [179] (etat neuf + declick par canal).
- Lane **sans fichier = voix inerte** (pas de repli embarque : le fichier EST l'instrument).
- Tests : roundtrip settings, 8 tests de voix (finitude, inertie sans fichier, pitch, A-H-D pitch env, reverse, convention filtre, types de filtre, declick au retrigger), snapshot d'unites, case stereo du registre.

Reste le build 2 : **drag & drop d'un WAV sur la grille -> creation de la lane**.

## 2026-09-22 - [242] Modal Macros : colonnes alignees, 16 rangees sans scrollbar (build 20260922-230239)

**Branche:** `main` - **Build:** `20260922-230239`
**Validation:** `cargo check` warning-clean, `cargo test` 456 + 1 + 287 verts, `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

Retour utilisateur (build 200611 valide, « ca fonctionne ») : « aligne mieux les colonnes et affiche toutes les macros sans scrollbar ».

- Colonne « Macro N » a **largeur fixe** (52 px, `allocate_ui_with_layout` - le pitfall « add_sized centre le label ») : knobs, selecteurs lane/parametre et x alignes quelle que soit la longueur du libelle (« Macro 1 » vs « Macro 16 »).
- Modal passe a **600 px de haut**, le `ScrollArea` est supprime : les **16 rangees tiennent d'un coup d'oeil**.
- **Build 20260922-231319** : libelles « Macro N » **alignes a droite** (les unites s'alignent au contact du knob, fini le decalage visuel des dizaines) et modal resserre a **505x570** (plus de bandes vides a droite ni en bas).

## 2026-09-22 - [242] FIX knobs des macros : le pattern eprouve de header_param_slider (build 20260922-200611)

**Branche:** `main` - **Build:** `20260922-200611`
**Validation:** `cargo check` warning-clean, `cargo test` 456 + 1 + 287 verts, `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

Retour utilisateur sur le build 181703 : « je ne peux plus changer le slider de la macro » (le remplissage bougeait d'un pixel et revenait a 0, drag jamais commite). Le commit-on-release + delta accumule en memoire temporaire etait une fausse bonne idee.

**Le fix** : `macro_knob` reecrit en miniature STRICTE de `header_param_slider` (Master/Swing, le code qui marche dans S1 depuis toujours) : `ui.interact` avec id explicite, `begin_set_parameter` au drag-start, `set_parameter_normalized` **a chaque frame** avec la valeur = position du pointeur (pas de delta, pas de memoire, pas de commit au release), `end_set_parameter` au drag-stop, clic = begin/set/end one-shot. Le gel du build 165241 venait du `ParamSlider` nih-plug, pas de la frequence des gestures - l'en-tete les emet a la meme frequence sans probleme.

## 2026-09-22 - [242] FIX gel de Studio One a l'edition des macros : knobs sans flux d'automation (build 20260922-181703)

**Branche:** `main` - **Build:** `20260922-181703`
**Validation:** `cargo check` warning-clean, `cargo test` 456 + 1 + 287 verts, `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

**Le bug (retour utilisateur)** : a l'ouverture/edition du modal Macros, S1 gela ~1 minute - fenetre du plugin editable mais hote inaccessible - puis revenait. Cause : le `ParamSlider` nih-plug envoie des gestures d'automation (begin/set/end-edit) **pendant toute la duree du drag**, et dans une Area popup baseview le mouse-release peut se perdre ; le drag ne se termine alors jamais et l'hote croule sous les notifications (la file se vide au bout d'une minute, d'ou le retour).

**Le fix** : widget maison `macro_knob` - **une** gesture begin/set/end au clic/drag-start (le learn MIDI de S1 n'a besoin que de ca pour voir le knob) et **une** ecriture de la valeur au `drag_stopped` via `set_float_param_if_changed` (le pattern begin/set/end eprouve de `controls.rs`). Aucun flux continu possible. Slider plat 96x14 (WELL_FILL + remplissage BLUE + bordure), drag vertical, course complete sur 120 px.

## 2026-09-22 - [242] Macros MIDI, build 3 : sliders learnables dans le modal (build 20260922-165241)

**Branche:** `main` - **Build:** `20260922-165241`
**Validation:** `cargo check` warning-clean, `cargo test` 456 + 1 + 287 verts, `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

Retour utilisateur : « c'est pas bien, il faut des boutons pour les macros pour que je puisse les learn depuis S1 ». Le learn GUI de Studio One suit le parametre que le plugin signale en cours d'edition (gestures begin/end-edit) : sans widget cote plugin, il n'y avait rien a toucher.

- **Un vrai `ParamSlider` par macro** dans chaque rangee du modal (Macro N + slider + lane + parametre + x) : le drag envoie les gestures a l'hote, donc **S1 peut learner le CC directement depuis le slider** (Control Link le selectionne, ou learn du header du plugin) ; et le slider reflete la valeur ecrite par le DAW (deux sens). Modal elargi a 560 px.

## 2026-09-22 - [242] Macros MIDI, build 2 : modal d'assignation (build 20260922-155823)

**Branche:** `main` - **Build:** `20260922-155823`
**Validation:** `cargo check` warning-clean, `cargo test` 456 + 1 + 287 verts, `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

- **Modal Macros** (`src/ui/macros_panel.rs`), ouvert depuis Settings > MIDI > **Macros › Edit...** : seize rangees « Macro N » + selecteur de **lane** (« - » = non assigne, puis les lanes actives « 1 Kick », « 3 Snare »…) + selecteur de **parametre** (tous les standards declares du kind + tous ses speciaux, libelles du registre) + **x** pour effacer. Choisir une lane repart sur son premier standard ; le selecteur de parametre d'une macro non assignee est grise, pas cache.
- Settings > MIDI gagne la rangee **Macros › Edit...** entre Lane 1 Root Note / Auto-assign et la section Others.

## 2026-09-22 - [242] Macros MIDI, build 1 : moteur + persistance (16 knobs exposes, 32 en magasin) (build 20260922-155001)

**Branche:** `main` - **Build:** `20260922-155001`
**Validation:** `cargo check` warning-clean, `cargo test` 456 + 1 + 287 verts (7 nouveaux), `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

Option B choisie par l'utilisateur (16 macros + mapping interne, extensible a 32 plus tard) plutot que l'exposition des 644 parametres de son. Perimetre V1 : params de son (standards + speciaux) des lanes instanciees ; p-locks sequenceur, structurels et params nih-plug par lane (piege [227]) exclus.

- **`src/macros.rs`** : `MacroMap` (32 emplacements `AtomicU32`, pack slot+kind+index valide au load), `scale_value` (lineaire/log comme le slider), `target_range` (min/max/log lus dans le registre), `apply_macros` (ecrit dans les memes atomiques par slot que l'UI + `bump_version` : les voix polent comme d'habitude, **les p-locks gardent la priorite par pas**), `prune_invalid_targets` (la lane change de kind ou disparait -> l'assignation vers un special/standard non declare saute, regle des textures [228]), `reorder` (les assignations suivent leur lane au deplacement), persistance **`macro-map-v1`** (32 x u32 LE, la longueur EST la version : passer a 32 knobs ne sera pas un v2).
- **`lib.rs`** : 16 `FloatParam` visibles `macro_1..16` (« Macro N », 0..1), application une fois par buffer AVANT le poll des settings (un knob bouge est rejoue dans le buffer meme), `last_macro_values` (-1 force la premiere application, donc la valeur d'automation du DAW s'applique au chargement).
- Hooks : `reconcile_lane_macros` une fois par frame (sound_editor) ; reorder dans `apply_lane_reorder_move`.
- **Pas encore d'UI d'assignation** : c'est le build 2. Pour tester ce build : ecrire une assignation en dur n'est pas possible depuis l'UI ; le test visible est la presence des 16 params « Macro N » dans la liste d'automation du DAW.

## 2026-09-22 - [240] Ordre canonique des enveloppes dans le panneau Sons, tous instruments (build 20260922-124548)

**Branche:** `main` - **Build:** `20260922-124548`
**Validation:** `cargo check` warning-clean, `cargo test` 449 + 1 + 287 verts (2 nouveaux), `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

Demande : « respecter une meme logique dans l'agencement des parametres des enveloppes entre tous les instruments ». Constat : les sections ont un ordre fixe mais les rangees a l'interieur suivaient l'ordre de declaration propre a chaque table du registre (15 tables, 26 voix) - Attack Curve en dernier ici, Hold avant apres la, etc.

- **Section Amp** : tri au RENDU par `env_row_rank` (registre) - **Attack → Attack Curve → Hold → Decay → Decay Curve** (un temps, puis sa courbe), partout. Les 15 tables de standards ne bougent pas (le p-lock et la persistance indexent les CHAMPS, pas l'ordre des rangees : display-only), et tout futur instrument herite de l'ordre.
- **Enveloppe de filtre** (Buzz, SDrex, Rift) : le hissage par suffixe s'etend aux courbes - **Filter Env → Filter Attack → Filter Atk Curve → Filter Hold → Filter Decay → Filter Dec Curve**. Le `_filter_curve` de Buzz EST sa courbe de decay (hissee sous Filter Decay).
- Tests : `amp_section_sorts_to_the_canonical_order_on_every_voice` (26 voix), `filter_envelope_stages_are_hoistable_on_every_voice_that_has_one` (contrat de suffixes + ancres Filter Env/Decay sur Buzz/SDrex/Rift).

## 2026-09-22 - [239] Settings en sections Audio / MIDI / Others, auto-assign MIDI clarifie (build 20260922-120230)

**Branche:** `main` - **Build:** `20260922-120230`
**Validation:** `cargo check` warning-clean, `cargo test` 447 + 1 + 285 verts, `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

Retour utilisateur sur le build 115426 : « l'auto assign midi n'est pas clair dans l'interface, il faut bien specifier la note root pour le lane 1 et le bouton auto assign, faire des sections audio, midi, others dans settings ».

- **Sections dans Settings** : **Audio** (Outputs Auto-assign, Default Analog), **MIDI** (Global MIDI Channel, Lane 1 Root Note, Auto-assign), **Others** (Auto-Edit, Skin) ; About reste en bas. En-tetes au style About (sans_sb bleu).
- **Auto-assign MIDI clarifie** : la note de base s'appelle desormais **« Lane 1 Root Note »** (DragValue a droite du libelle, comme Global MIDI Channel desormais sur une seule rangee), et le bouton **Auto-assign** a sa propre rangee en dessous - avant, le DragValue et le bouton partageaient une rangee sans libelle explicite.
- Aucun changement fonctionnel : meme logique `assign_midi_notes_in_order`, meme persistance.

## 2026-09-22 - [238] Flash MIDI sur la plaque de nom + [239] Auto-assign des notes MIDI (build 20260922-115426)

**Branche:** `main` - **Build:** `20260922-115426`
**Validation:** `cargo check` warning-clean, `cargo test` 447 + 1 + 285 verts (1 nouveau), `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

- **[238] Le MIDI in flashe la plaque de nom, plus de LED orange.** `lane_activity_led` (skeuo.rs) supprimee ; `lane_name` prend un `flash` 0..1 : la plaque entiere vire au **blanc** et le texte bascule en sombre pour rester lisible (premier essai ambre, retour utilisateur : « la plaque entiere flash blanc »). Meme timer qu'avant (`slot_flash_until`, 120 ms au MIDI in, 500 ms aux Paste/Randomize), meme correctif de repaint [216] conserve. Intensite proportionnelle au temps restant (fondu sur les 120 dernieres ms).
- **[239] Auto-assign des notes MIDI** (Settings > « Auto-assign MIDI Notes ») : une note de base (DragValue 0-127, defaut 36, etat transitoire en `egui` memory - rien a persister pour une action ponctuelle) et un bouton **Assign** : les lanes actives, dans l'ordre des slots, prennent des notes consecutives (base, base+1, ...), sature a 127. Action ponctuelle comme [230], pas un mode ; elimine les collisions de notes entre lanes par construction. Persiste via le track layout existant, aucune nouvelle cle. Logique pure `auto_assign_notes_in_order` testee (numerotation, no-op, saturation a 127).

## 2026-09-22 - [234]-[237] Batch polish UI : nom de lane elargi, preset browser (scrollbar, date, rename) (build 20260922-105916)

**Branche:** `main` - **Build:** `20260922-105916`
**Validation:** `cargo check` warning-clean, `cargo test` 446 + 1 + 285 verts (2 nouveaux), `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

Quatre quick wins du batch 1 de la session 2026-09-22 (demandes utilisateur ticketisees [234]-[242]).

- **[234] Box du nom de lane elargie** : le nom de lane passe de 6 a **8 caracteres** (`take(6)` -> `take(8)`) et la plaque de 46 a **62 px** (`name_w` partage par la grille, l'en-tete et les lanes vides, tout suit). La largeur fixe de la grille augmente de 16 px, les cellules gardent leur plancher de 18 px.
- **[235] Scrollbar du preset browser decollee des boutons Del** : la barre est desormais **toujours reservee** (`ScrollBarVisibility::AlwaysVisible`, fini le decalage des rangees quand elle apparait) et le contenu a une marge droite de 8 px (Frame interne), donc Del ne la touche plus.
- **[236] Date de sauvegarde des presets** : chaque preset utilisateur affiche son `mtime` en `YYYY-MM-DD` (gris clair, mono 9, apres le nom) via `PresetFileInfo.modified`. Aucun changement de format JSON - la date vient du fichier. `format_date` : civil-from-days, UTC, sans crate de dates.
- **[237] Bouton Ren (rename)** sur chaque preset utilisateur : la rangee passe en champ texte inline (focus automatique), **Entree** valide, **Echap** annule. `presets::rename_preset` met a jour le champ `name` DANS le JSON **et** le nom de fichier (les deux restent en sync ; si le nom sanitise donne le meme fichier, reecriture en place). Le cache du loader d'instruments de l'onglet Track est invalide pour le kind Instrument.
- Tests : `rename_preset_updates_json_name_and_filename`, `format_date_known_days`.

## 2026-09-21 - [233] Retour en arriere : seuls les sous-parametres de la saturation sont decales (build 20260921-201521)

**Branche:** `main` - **Build:** `20260921-201521`
**Validation:** `cargo check` warning-clean, `cargo test` 444 + 285 verts, `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

Retour utilisateur sur le build 195925 : « c'est n'importe quoi, rien n'est coherent, reviens en arriere et mets juste les sous param de saturation en decale, sans mettre de | ».

- **Retire** : la regle generique `parent_of` / `std_parent_of` / `ParentRef` et son test, le grisage et le decalage de Grain sous Loop, de l'enveloppe de filtre sous Filter Env, de Filter Decay, d'Attack Curve, des temps d'enveloppe et des rates de LFO, d'Advance Step, de Pitch Fine, et la barre verticale a gauche des libelles. Les cas Grain/Loop (grise quand Loop est eteint) et Advance Step (grise quand Advance est eteint) reviennent tels qu'ils etaient avant le build 195925.
- **Garde** : Saturation Amount, Mix et Output Gain ont leur libelle decale de 14 px sous Saturation Type et sont grises quand le type est None (la demande d'origine), sur tous les instruments. Pre-Filter, Crush et Decimate ne sont pas concernes. Le decalage passe toujours par `SUB_INDENT` lu par `editor_label`, sans barre.
- Lecon notee en memoire : une regle « generique » de lisibilite n'est pas une amelioration si l'utilisateur ne l'a pas demandee ; appliquer la demande au perimetre demande.

## 2026-09-21 - [233] Panneau Sons : sous-parametres decales et grises quand leur parent est eteint (build 20260921-195925)

**Branche:** `main` - **Build:** `20260921-195925`
**Validation:** `cargo check` warning-clean, `cargo test` 445 + 286 verts (1 nouveau), `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

Demande utilisateur : « griser les sous-parametres de la saturation quand elle est a None et les decaler pour signifier que ce sont des sous-parametres ; faire ca pour tous les sous-parametres (loop / grain / grain shape...) ».

- **Une regle, pas des cas** : `instrument_registry::parent_of(instrument, special)` decide par CONVENTION DE NOM qui hange de qui, donc chaque instrument qui partage un suffixe en profite sans toucher a son entree : `*_saturation_amount/_mix/_output_gain` sous `*_saturation_type` (None = eteint), `*_grain*` sous `*_loop`, `*_filter_attack/_hold/_atk_curve/_dec_curve/_filter_curve` sous Filter Env, `*_pitch_env_time` sous `*_pitch_env`, `*_pitch_lfo_rate` / `*_filter_lfo_rate` sous leur depth, `*_advance` sous l'interrupteur Advance de la lane. Cote standards, `std_parent_of` : Filter Decay sous Filter Env, Attack Curve sous Attack. Pre-Filter reste au premier niveau : il regle tout le bloc Distortion.
- **Rendu** : le libelle d'un sous-parametre est decale de 14 px avec une barre fine a gauche ; le slider garde son alignement. La rangee est grisee - jamais cachee - tant que le parent est eteint ou a zero (`parent_enabled` : interrupteur a 1, profondeur non nulle, menu au-dela de sa premiere entree). Le decalage passe par un `thread_local` lu par `editor_label`, ce qui evite de changer la signature des dizaines de rangees. Les cas particuliers Grain/Loop et Advance Step, codes a la main avant, sont remplaces par la regle ; les rangees hissees (Filter Attack / Hold sous Filter Env) et Pitch Fine des echantillonneurs suivent aussi.
- Test : les conventions lues sur Rift (`rift_sub_parameters_hang_from_the_expected_parents`).

## 2026-09-21 - [231] P-locks sequenceur dans X2 et le copier-coller de page + [232] Rift : Pre-Filter deplace tout le bloc Distortion (build 20260921-192420)

**Branche:** `main` - **Build:** `20260921-192420`
**Validation:** `cargo check` warning-clean, `cargo test` 444 + 285 verts (2 nouveaux), `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

- **[231]** (« les plock sequencer ne sont pas pris en compte dans le X2 et le copy/paste »). X2 doublait les cellules et les p-locks SON, pas les p-locks sequenceur ; le presse-papier de page les laissait de cote depuis toujours (« for now, sound plocks only »). `SequencerPlockState::snapshot / restore / copy_step` prennent l'image BRUTE d'une cellule - probabilite, stutter, condition avec ses bits Not et And (que `set(&SequencerStepParams)` perdrait), microtiming, solo - ; `PageClipboard::seq_plocks` (`serde(default)`, les anciens presse-papiers se relisent) ; X2 appelle `copy_step` pour chaque cellule doublee ; Paste restaure apres avoir vide la page. Le copier-coller de LANE les emportait deja.
- **[232]** (« le switch pre-filter de la distortion doit prendre en compte le crush et le decimate »). Decimate, Crush puis saturation forment desormais un seul bloc, place avant ou apres le filtre par le switch **Pre-Filter**, qui passe en dernier de la section Distortion. Crush et Decimate agissent sur le signal enveloppe, comme la saturation l'a toujours fait (le placement avant l'enveloppe du build 102711, choisi contre le gating, est abandonne : a deux bits une queue qui decroit tombe dans le pas zero, c'est l'effet). Le pack saturation est aussi applique a la construction de la voix (`sync_saturation`) : le test l'a montre, une voix fraiche ignorait Pre-Filter jusqu'au premier `set_settings`.

## 2026-09-21 - [228] Samples courts : Offset, Wander et Reverse mordent ; la texture suit la lane deplacee ; stereo par defaut (build 20260921-161353)

**Branche:** `main` - **Build:** `20260921-161353`
**Validation:** `cargo check` warning-clean, `cargo test` 442 + 284 verts (2 nouveaux), `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

Trois retours utilisateur sur le build 134421.

- **« l'offset, le reverse et le wander ne fonctionnent pas sur les sons customs »**. Cause : Loop eteint, la fenetre de lecture reservait quatre decays de texture pour ne jamais tronquer la queue du son ; sur 12 s d'usine invisible, sur un sample reel plus court que cette reserve la « place » restante pour Offset, Wander et Advance etait nulle, et Reverse partait du silence de fin de fichier. Desormais Offset parcourt TOUT le fichier, la lecture avant va jusqu'au bout (coupure declickee), et Reverse lit a l'envers la duree exacte de l'enveloppe (`audible_secs` = attaque + hold + decay, l'enveloppe est temporelle) depuis le point d'offset : sur une texture longue l'offset choisit toujours la matiere, sur un sample court un decay au moins aussi long que le fichier le joue a l'envers depuis sa fin, comme un sampler. `TAIL_TIME_CONSTANTS` supprime. Test sur un one-shot de 0,5 s : offset 0 frappe le transitoire, offset 0,6 tombe dans le calme (< 25 %), Wander change le coup, Reverse monte vers le transitoire.
- **« quand je deplace une lane l'instrument custom disparait »**. Le glisser-deposer permute chaque magasin par slot (pattern, sons, p-locks, seq-plocks, params, verrous de longueur) mais pas les textures, qui restaient sur l'ancien numero puis se faisaient effacer par `reconcile_lane_textures`. `UserTextures::reorder(&order)` republie les memes `Arc` sous leurs nouveaux numeros (rien n'est redecode) ; appele dans `apply_lane_reorder_move`. Le signalement « ca a encore saute » venait d'un build pas encore installe (Studio One ouvert).
- **« quand on charge un son stereo il faut qu'il soit en stereo directement »** : au chargement d'un fichier stereo, le standard Stereo de la lane passe a 1 ; l'interrupteur sert a replier en mono.

## 2026-09-21 - [228] Un WAV custom par lane, plus les textures d'usine (build 20260921-134421)

**Branche:** `main` - **Build:** `20260921-134421`
**Validation:** `cargo check` warning-clean, `cargo test` 440 + 283 verts (1 nouveau : partage du decodage), `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

Retour utilisateur : « non ca ne va pas la gestion des wav custom. Il faut juste pouvoir upload un wav par lane en plus des waves par defaut. » Les huit emplacements globaux, les entrees « User 1..8 » et les libelles « 3: kick.wav » disparaissent.

- **Modele** : le pool a un emplacement par LANE (`LANE_TEXTURE_SLOTS = MAX_TRACKS`) ; le menu Texture d'une lane Rift a cinq entrees, les quatre textures d'usine puis **Custom** = le fichier de cette lane (`CUSTOM_TEXTURE_INDEX = 4`), affiche par son nom, « Custom (empty) » sans fichier, « nom (missing) » si le fichier a disparu. `resolve_texture(index, (pool, lane))` ; la voix recoit son numero de lane avec le pool (`Voice::set_texture_pool(pool, lane)`). Une lane sans fichier joue Noise sur Custom, jamais le silence.
- **Panneau** : la ligne **File** (nom, Load..., Clear) est toujours visible sous le menu Texture ; **Load selectionne Custom** dans le menu de la lane. La ligne Stereo reste sous Stereo Spread, active quand le fichier de la lane est stereo et selectionne.
- **Decision utilisateur** : changer le type d'instrument d'une lane (ou la retirer) **efface son fichier** ; `reconcile_lane_textures` le fait a chaque image, quel que soit le chemin par lequel le type a change (menu, preset, kit).
- **Partage** : deux lanes qui chargent le meme fichier partagent une seule texture decodee - cache par chemin + date de modification + taille, references `Weak` (libere quand plus aucune lane ne l'utilise) ; un fichier reexporte sous le meme nom est redecode. Une trentaine de lignes, rien de change cote audio.
- **Le fichier suit la lane** : copie/colle de lane (`LaneClipboardData::texture_path`), presets d'instrument et de pattern (`user_texture_path(kind, lane, ..)`) ; un preset sans fichier efface celui de la lane, pour que la lane sonne comme le preset.
- Persistance : cle `lane-textures-v1` (14 chemins optionnels) ; l'ancienne `user-textures-v1` des builds 091757-102650, jamais validee, est ignoree au chargement.

## 2026-09-15 - Auto-assign en bouton, Stereo en ligne permanente, noms de fichiers dans le menu Texture (build 20260915-102650)

**Branche:** `main` - **Build:** `20260915-102650`
**Validation:** `cargo check` warning-clean, `cargo test` 439 + 283 verts, `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

Trois retours utilisateur sur le build 101709.

- **[230] Auto-assign est un bouton**, pas un interrupteur (« ca n'a pas de sens ») : Settings > Outputs > **Auto-assign** route les lanes actives sur la sortie de leur numero une fois, au clic (`assign_outputs_in_order`). Le `BoolParam` `auto_out`, l'application a chaque image et le grisage du selecteur Aux Out sont retires ; une session sauvee avec ce param le voit ignore au chargement.
- **[228] Stereo est une ligne permanente** de la section Texture, sous Stereo Spread, pas un appendice de la ligne File visible seulement sur un emplacement User (« ca doit etre un parametre utilisable a tout moment si le sample est stereo »). Grisee - jamais cachee - tant que la texture courante n'a pas de canal droit (embarquee, fichier mono, emplacement vide), avec l'explication au survol.
- **[228] Le menu Texture nomme les fichiers** : un emplacement charge s'affiche « 3: kick.wav », un fichier disparu « 3: kick.wav (missing) », un emplacement vide reste « User 3 » (`texture_menu_labels`, libelles construits a chaque image a partir des `options` du registre).

## 2026-09-15 - [228] Interrupteur Stereo pour les fichiers stereo + [230] Settings > Auto-assign outputs (build 20260915-101709)

**Branche:** `main` - **Build:** `20260915-101709`
**Validation:** `cargo check` warning-clean, `cargo test` 439 + 283 verts (1 nouveau), `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

- **[228] Stereo** (demande : « lorsque le sample est stereo il faudrait un switch mono/stereo en plus du stereo spread »). Un fichier stereo garde desormais ses deux canaux au decodage (`TextureBank::right`, `data` = gauche ; plus de deux canaux restent moyennes en mono ; le graphe montre le mix). Rift recoit le standard `Stereo` (`cb(...)`, famille Osc), rendu sous la ligne **File** d'un emplacement User et non dans la boucle des standards (meme mecanique que le Stereo des echantillonneurs) ; grise quand le fichier est mono. Voix : `sample(bank, canal, pos)` lit gauche, droite, ou le mix des deux au moment de la lecture ; `process_sample_stereo` passe en deux canaux des que Stereo Spread OU le fichier stereo l'exige, et le Spread decale la tete droite par-dessus. Test : tonalite a gauche, silence a droite -> eteint, les deux sorties egales au demi-niveau ; allume, la tonalite a gauche et le silence a droite.
- **[230] Auto-assign outputs** (demande : « option assigne audio auto pour assigner tous les lanes a la suite (dans settings) »). `BoolParam` par session `auto_assign_outputs`, interrupteur dans Settings sous Auto-Edit. `enforce_auto_outputs` tourne a chaque image : toute lane active dont la sortie n'est pas `Out N` (N = son numero) y est mise par `assign_slot_output` - donc elle quitte le main mix comme une assignation manuelle, et l'utilisateur peut le rallumer - ; le layout n'est ecrit que s'il y a une lane a deplacer. Les lanes ajoutees ensuite suivent. Le selecteur **Aux Out** de la lane est grise avec l'explication au survol tant que l'option est allumee.

## 2026-09-15 - [228] Le bouton Load ne fait plus tomber Studio One (build 20260915-092641)

**Branche:** `main` - **Build:** `20260915-092641`
**Validation:** `cargo check` warning-clean, `cargo test` 438 + 282 verts, `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

Retour utilisateur sur le build 091757 : « le bouton Load a fait crasher S1 ».

- **Cause** : `rfd::FileDialog::pick_file()` etait appele DANS l'image egui. Un dialogue modal fait tourner sa propre boucle de messages ; celle-ci redistribue les messages de la fenetre du plugin, baseview rappelle egui alors qu'on est deja dans `ui()`, egui panique sur la re-entree, et le plugin est compile avec `panic = "abort"` : l'hote tombe avec lui.
- **Correctif** : le dialogue tourne sur un thread dedie (`flash-drum-file-dialog`, COM initialise en STA par rfd sur ce thread, plus de conflit avec le mode COM de l'hote) et depose sa reponse dans une cellule partagee (`EditorUIState::texture_pick`, non persistee). `poll_texture_pick`, une fois par image avant les rangees, decode le fichier choisi sur le thread UI ; pendant l'attente le bouton affiche « ... » et l'interface demande une image toutes les 100 ms. Un seul dialogue a la fois.
- Note portabilite : sous macOS `NSOpenPanel` veut le thread principal ; ca compile, a revoir au moment du packaging macOS (commentaire dans le code).

## 2026-09-15 - [228] Rift : charger ses propres fichiers audio (build 20260915-091757)

**Branche:** `main` - **Build:** `20260915-091757`
**Validation:** `cargo check` warning-clean, `cargo test` 438 + 282 verts (6 nouveaux : pool et repli, decodage WAV, fichier disparu, restore, voix sur texture utilisateur, preset legacy), `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

- **Menu Texture** : les quatre textures embarquees puis huit emplacements `User 1..8` (`sp_options`, `TEXTURE_OPTION_COUNT = 12`). Sur un emplacement User, une ligne **File** sous le menu : nom du fichier, ou « empty - plays Noise », ou « missing: nom » en rouge (chemin et erreur de decodage au survol), et deux boutons **Load...** / **Clear**. Load ouvre le dialogue natif (`rfd`, IFileDialog sous Windows, NSOpenPanel sous macOS, sans backend GTK), filtre WAV ; le decodage (`hound`, toute profondeur entiere ou flottant, canaux moyennes en mono, 60 s au plus) se fait sur le thread UI, jamais sur le thread audio.
- **Pool par instance** (`sample_bank::TexturePool`, `arc_swap`) : le thread audio lit un emplacement par UN load atomique et un clone d'Arc ; la voix garde le `TextureSource` le temps du coup. Discipline memoire : la generation precedente d'un emplacement est parquee cote UI jusqu'au prochain publish dans ce meme emplacement, pour que la voix ne detienne jamais la derniere reference (liberer sur le thread audio, c'est ce que ca voudrait dire). Le pool arrive aux voix par `Voice::set_texture_pool` (defaut vide ; `DrumSynthesizer::set_texture_pool` avant `initialize_with_layout`, herite par `reinitialize_slot` - un clone d'Arc, sain en RT). Pas de pool global au processus : deux instances dans le meme hote auraient partage et ecrase leurs emplacements.
- **Repli** (`resolve_texture`) : emplacement vide, fichier disparu ou illisible -> la premiere texture embarquee. Un coup ne se tait jamais faute de fichier ; le graphe de texture montre ce qui joue reellement.
- **Persistance** : `user-textures-v1` (`UserTextures`, huit chemins optionnels) ; `PersistentField::set` s'execute sur le thread principal au restore et decode les fichiers la ; un fichier absent garde son chemin et passe « missing ». **Presets** : `InstrumentPreset` et `PatternSlotSound` gagnent `user_texture: Option<String>` (`serde(default)`, les presets anterieurs se lisent), rempli par `user_texture_path` quand la lane pointe un emplacement User ; a l'application, `write_slot_sound` recharge le fichier dans l'emplacement que le menu nomme.
- Dependances : `hound` passe des dev-dependencies aux dependencies ; `arc-swap 1`, `rfd 0.15` (default-features = false) ajoutes.

## 2026-09-14 - [221] Rift : son d'usine neutre - plus de « fausse enveloppe de pitch » apres Default (build 20260914-164149)

**Branche:** `main` - **Build:** `20260914-164149`
**Validation:** `cargo check` warning-clean, `cargo test` 433 + 279 verts, `build.ps1 -Install` OK. A valider dans Studio One.

Retour utilisateur : « quand je fais un Default sur un son du Rift j'ai toujours l'impression qu'il y a une enveloppe de pitch ».

- **Default remet bien tout**, Pitch Env Depth a 0 compris (`reset_slot_to_defaults` -> `reset_specials_for_voice`) : ce n'etait pas un oubli de reinitialisation.
- **Deux causes mesurees dans le son d'usine lui-meme.** (1) Le filtre repris de Buzz - coupure 1200 Hz, Filter Env 0,6 - balayait la coupure de 6,5 kHz a 1,2 kHz en 120 ms sur chaque coup (proxy de hauteur par taux de passages a zero : 950 Hz dans les 30 premieres ms puis 150 Hz ; avec Filter Env a 0 : 200 Hz tout du long). Sur du bruit, ce « pew » descendant est lu comme une enveloppe de pitch. (2) L'offset d'usine 0,25 tombait dans une zone rugueuse de la texture Noise v2 : niveau RMS variant de 34 % d'une fenetre de 10 ms a l'autre (balayage complet de la texture : 5 % a 0,05, 34 % a 0,25, 52 % a 0,70), un tremolo que l'oreille prend aussi pour du mouvement de hauteur sur un coup court.
- **Correctif** : son d'usine **neutre**, la tranche telle qu'elle est - Filter 20 kHz, Filter Env 0 (`sound_settings_default` et `VoiceSettings::rift()`), Offset 0,05 (zone la plus stable et la plus brillante du fichier). Le balayage de filtre reste a un slider de distance ; le graphe du filtre dit alors honnetement « l'enveloppe ne fait rien ».
- Le test anti-clic de fin de coup mesure desormais la coupure **par rapport au signal qui precede** (l'ancien seuil fixe mesurait le bruit brillant de la nouvelle zone d'offset, pas la coupure).
- **A l'oreille de l'utilisateur** : la texture Noise v2 a des zones rugueuses (modulation d'amplitude 18-90 Hz) sur une bonne moitie du fichier ; si elles derangent, la profondeur de cette modulation se regle dans `tools/gen_textures.py` (`0.85 * rough_zone`).

## 2026-09-14 - [212] Boucle de page : une page de 16 pas tourne seule (build 20260914-162358)

**Branche:** `main` - **Build:** `20260914-162358`
**Validation:** `cargo check` warning-clean, `cargo test` 433 + 279 verts (5 nouveaux tests du sequenceur), `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

- **Reglage** : `page_loop` (IntParam 0 = off, 1-4 = la page), sauvegarde et automatisable, pas cache (lecon du build 124217). Dans la barre Page : **double-clic** sur un bouton boucle cette page, double-clic a nouveau libere ; le **menu clic droit** de la page gagne « Loop this page » / « Stop page loop » ; un **anneau ambre** entoure la page bouclee (gris en mode Song, ou la boucle est ignoree). Une page au-dela du pattern (Len 32, page 3) remet le param a 0.
- **Sequenceur** (`Sequencer::set_page_loop`, `loop_bounds`, `fold_into_loop`, `fold_shifted`, `host_to_local`, `realign_tracks_to_position`) : la position maitre est bornee par la page au lieu du pattern, une page partielle (Len 40, page 3 = pas 32 a 39) boucle sur ses huit pas. **Engagement immediat a phase conservee** : le pas 52 devient le pas 20 (meme position dans la page), et les lanes sont recalees au saut - mesure par le test, sans ce recalage la cellule d'arrivee etait rejouee. A chaque retour de page les compteurs de pas des lanes sont re-derives de la position (`shifted_master` different du pas attendu) : une lane de 16 joue k sous le pas 32+k a chaque tour, une lane polymetrique se recale aussi (la boucle rejoue ce que la grille montre). Le wrap du pattern reste un pas normal : la polymetrie y derive comme avant. Tir anticipe (microtiming negatif) : la borne est la fin de page et la cellule suivante le debut de page. `loop_count` avance a chaque tour de page (conditions 1/2, 1/3...).
- **Synchro hote** : `sync_to_host` replie la position hote dans la page (`host_to_local`), les compteurs suivent la position repliee, et le detecteur de seek de `lib.rs` compare sur le cercle de la boucle a travers la meme fonction - sans quoi le retour d'une page partielle (moins d'une mesure) passait pour un saut de transport et resynchronisait a chaque tour.
- **Mode Song** : `set_page_loop(None)` tant que Song joue - un enchainement de patterns et une page tenue en boucle se contredisent. Follow suit la page bouclee. Export MIDI inchange.

## 2026-09-14 - [221] Rift : plus de clic a la fin d'un coup a enveloppe de filtre (build 20260914-160256)

**Branche:** `main` - **Build:** `20260914-160256`
**Validation:** `cargo check` warning-clean, `cargo test` 428 + 274 verts (1 nouveau test de non-regression), `build.ps1 -Install` OK. A valider dans Studio One.

Retour utilisateur : « quand je mets un env de filter j'entends un petit click a la fin de l'env ».

- **Mesure** (rendu de test, derniers echantillons avant le silence) : a la fin de l'enveloppe d'amplitude la sortie n'est pas a zero mais a un residu du filtre et du bloqueur de DC - **-0,00097 avec le filtre ouvert** par l'enveloppe (filter decay plus long que l'amp decay), -0,00008 filtre ferme - et la voix passait a 0 exact en UN echantillon : un pas de -60 dBFS, un tic. Le rapport de douze entre les deux cas explique que le clic n'apparaisse qu'avec l'enveloppe de filtre.
- **Correctif** : `RiftVoice::cut()` - chaque point de coupure (enveloppe finie, tranche epuisee sans Loop, lecture hors texture) arme le `RetrigDeclick` depuis la derniere sortie, comme au declenchement ([179]), et le residu fond en 3 ms. Le declick devient **par canal** (`[RetrigDeclick; 2]`, `last_out: [f32; 2]`) : avec Stereo Spread les deux cotes finissent sur des residus differents, un fondu commun sur le mid aurait fait un pas sur chacun. Sans spread les deux canaux restent identiques octet pour octet (test conserve).
- **Test** `no_click_when_the_amp_envelope_ends_under_an_open_filter` : plus grand saut echantillon a echantillon sur les 140 derniers echantillons audibles < 0,0002 (0,00097 avant). Un premier essai mesurait sur 13 ms et echouait sur le bruit legitime du filtre ouvert, pas sur la coupure - la fenetre ne couvre plus que la rampe et la coupure.

## 2026-09-14 - [227] Reset on page loop, resets exclusifs, options refermees, graphes alignes, bouton Random Offset (build 20260914-154151)

**Branche:** `main` - **Build:** `20260914-154151`
**Validation:** `cargo check` warning-clean, `cargo test` 427 + 273 verts (1 nouveau), `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

Quatre retours utilisateur sur le build 144144.

- **« Reset on page loop »**, plus « on page change » : le compteur repart quand une page REVIENT. `reset_advance_counts_on_wrap` compte desormais un wrap de pattern comme un retour de page - sur un pattern de 16 pas la page ne change jamais, elle revient pourtant a chaque tour, et l'option n'agissait pas. **Les deux resets s'excluent** : allumer l'un eteint l'autre (dans l'UI ; le moteur tolere les deux bits pour les sessions qui les auraient).
- **Advance eteint referme le pli** Advance options (`CollapsingHeader::open(Some(false))` tant que l'interrupteur est off ; rallume, l'utilisateur rouvre).
- **Graphes alignes sur la premiere ligne de leur section** : la ligne params + graphe etait un `horizontal` centre verticalement, donc dans une section haute (Texture) le graphe tombait a mi-hauteur, au niveau d'Advance. `horizontal_top`, pour toutes les sections.
- **Bouton « Random Offset on active cells »** sous le slider Offset : un clic ecrit un p-lock Offset aleatoire sur chaque cellule ACTIVE de la lane, toutes pages, graine tiree de l'horloge a chaque clic (`PlockState::fill_steps`, `plock::random_values` factorise depuis `scatter_values`). Complement du menu clic droit (page entiere, actives ou non) : ici seules les cellules qui jouent sont touchees.

## 2026-09-14 - [221] Texture Noise refaite : niveau constant, 16 s, plus de matiere (build 20260914-150022)

**Branche:** `main` - **Build:** `20260914-150022`
**Validation:** `build.ps1 -Install` OK ; `cargo test` relance apres coup (voir rapport). A ecouter dans Studio One.

Demande utilisateur : « refais le wav Noise pour que son amplitude ne baisse pas, qu'il soit plus evolutif et un peu plus long ».

- **Mesure avant** (RMS par seconde) : l'ancien `noise-field.wav` perdait **29 dB** entre la seconde 0 (-12,7 dB) et la seconde 8 (-41,9 dB) - la « densite » qui modulait le corps creusait des zones presque muettes, et un offset qui y tombait donnait un coup inaudible. **Apres** : 16 s, ecart max **0,5 dB** sur toute la duree.
- **Comment** (`tools/gen_textures.py`, `noise_field` v2) : toutes les derives sont spectrales ou texturales, jamais de volume, et un niveleur RMS a fenetre de 120 ms (`level_flat`) tient le niveau ; le pic final donne la marge. Couleur par un passe-bas a **quatre poles** (deux SVF en cascade, sortie `lp` ajoutee a `svf_varying`) : un pole seul laisse le centroide d'un bruit a plusieurs kilohertz quelle que soit la coupure, donc jamais de zone sourde. Nouvelles couches par zones : un **comb a delai variable** (0,4 a 9 ms, reinjection jusqu'a 0,9) qui fait des zones de tuyau metallique, nourri avec la matiere coloree et non du blanc ; une **modulation d'amplitude** a 18-90 Hz qui donne des zones rugueuses. Le script accepte un filtre : `python tools/gen_textures.py noise` ne regenere que ce fichier.
- **Reste a regler**, mesure : le centroide spectral voyage de 4,3 a 8,2 kHz seulement (x1,9), contre x2,9 avant - les couches aigues prennent le dessus dans les zones sourdes parce que le niveleur egalise la SOMME. La correction (niveler chaque couche avant le melange) etait prete quand l'utilisateur a demande a ecouter d'abord ; elle attend son avis.
- DLL : +0,35 Mo (16 s au lieu de 12).

## 2026-09-14 - [227] Le bouton Advance bascule, et les sections du panneau Sons : Texture / Pitch / Amp / Filter / Modulation / Distortion (build 20260914-144144)

**Branche:** `main` - **Build:** `20260914-144144`
**Validation:** `cargo check` warning-clean, `cargo test` 426 + 273 verts, `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

- **Bug : l'interrupteur Advance ne basculait pas** (build 124217). Les quatorze `IntParam` `advmode_N` etaient declares `.hide()`, et nih-plug presente un parametre cache a l'hote comme **lecture seule** (`kIsReadOnly | kIsHidden`). Or, tant que l'hote traite l'audio, `raw_set_parameter_normalized` ne pose pas la valeur lui-meme : il l'envoie a l'hote par `performEdit`, qui la renvoie dans le callback audio - et Studio One refuse d'ecrire un parametre en lecture seule. Le clic partait donc dans le vide. Les parametres sont desormais ordinaires, comme les algos de slot ; ils apparaissent dans la liste d'automation de S1, c'est le prix.
- **Sections du panneau Sons**, demande utilisateur : « Pitch / Amp / Filter / un nom qui englobe saturation, crush, decimate ». `ParamFamily::PitchEnv` devient **`Pitch`** et recoit, pour Rift, le Pitch, Pitch Fine et l'enveloppe de pitch avec son graphe. Titres : la famille `Osc` s'intitule selon l'instrument (`source_section_title` : **Sample** pour un echantillonneur, **Texture** pour Rift, **Oscillator** sinon, decide par les parametres declares et non par l'index), `Env` -> **Amp**, `Saturation` -> **Distortion** (le terme qui couvre saturation, reduction de bits et decimation). Ordre du panneau : Texture, Pitch, Amp, Filter, Modulation, Distortion.
- **Rangees hissees sous leur rangee standard** : dans la section Filter, `Resonance` vient juste sous la coupure, et `Filter Attack` / `Filter Hold` juste sous `Filter Env`, pour que l'enveloppe se lise A-H-D dans l'ordre. Mecanisme par suffixe de nom et dans la MEME famille (`hoisted`, `draw_plain_special_row`), sur le modele du Pitch Fine des echantillonneurs ; Buzz et SDrex en profitent, et la Resonance des charleys, declaree dans leur section source, ne bouge pas.

## 2026-09-14 - [227] Advance : interrupteur + options, section Pitch Envelope, Reverse et Loop en interrupteurs (build 20260914-124217)

**Branche:** `main` - **Build:** `20260914-124217`
**Validation:** `cargo check` warning-clean, `cargo test` 426 + 273 verts (1 nouveau), `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

Trois demandes utilisateur du 13/09 que le build 3 de Rift avait laissees derriere lui.

- **Advance ([227])**. Le compteur a quitte la voix, qui ne connait ni le pattern ni la page : `lib.rs` compte par slot (`advance_hits`) et decide de l'index de chaque coup dans `fire_voice_trigger`, le seul endroit par ou passent premier coup, stutter et impulsions de fusion ; la voix le recoit par `Voice::set_hit_index` avant `trigger()` (methode par defaut vide, seule Rift l'implemente). Options par lane dans un `IntParam` cache par slot (`advmode_N`, bitfield ON / RESET_PATTERN / RESET_PAGE / EVERY_CELL) : ce sont des reglages de sequenceur, pas de son, donc pas dans `special[]` (dont il ne reste d'ailleurs que l'index 31, inutilisable). Reset sur wrap de pattern ou changement de page detecte une fois par echantillon apres `Sequencer::process_sample`. Le mode « chaque cellule » n'a pas de compteur : l'index se deduit de la position (cellule dans la page, dans le pattern, ou depuis le depart). Panneau Sons : interrupteur `Advance`, slider `Advance Step` grise quand eteint, pli `Advance options` avec trois interrupteurs. Le graphe de texture n'affiche les crans d'Advance que si l'interrupteur est allume.
- **Section Pitch Envelope** : nouvelle `ParamFamily::PitchEnv`, placee sous Oscillator, avec `Pitch Env Depth` (±24 demi-tons) et `Pitch Env Time`, et un graphe (`draw_pitch_envelope`) qui trace la MEME loi que la voix (`exp(-4 t / time)`, constante `PITCH_ENV_CURVE` rendue publique), ligne zero au milieu, fenetre fixe d'une seconde.
- **Interrupteurs** : tout parametre special discret 0/1 sans liste nommee est rendu en interrupteur. Reverse et Loop etaient des sliders parce que la branche du panneau reconnaissait les booleens par une liste de suffixes de noms ; la regle est desormais structurelle.

## 2026-09-14 - [221] Rift build 3 : Crush, Decimate, Stereo Spread et le menu Spread / Scatter (build 20260914-102711)

**Branche:** `main` - **Build:** `20260914-102711`
**Validation:** `cargo check` warning-clean, `cargo test` 425 + 272 verts (dont 6 nouveaux), `build.ps1 -Install` OK. A valider dans Studio One (liste dans le rapport).

- **Lo-fi (`dsp.rs`, partage)** : `crush_levels` / `crush_sample` (16 -> 2 bits, profondeur fractionnaire pour un slider continu) et `Decimator` (sample-and-hold a facteur fractionnaire, 1 -> 64, mapping exponentiel). Les constantes sont recalculees au changement de reglage, jamais par echantillon. `Biquad::copy_coefficients_from` pour qu'une paire stereo partage UN calcul de coupure sans partager sa memoire.
- **Rift** : trois speciaux de plus (28 Crush, 29 Decimate, 30 Stereo Spread ; il reste l'index 31, inutilisable). La quantification est **relative au pic de la tranche** lue, mesure au declenchement sur le resume par colonnes de la texture : mesure avant ce choix, un Crush a fond coupait le son a 5033 echantillons au lieu de 8986 parce que la zone de texture passait sous le premier pas de quantification. Crush et Decimate viennent AVANT l'enveloppe d'amplitude, la queue ne se referme donc jamais en porte. `Stereo Spread` ajoute une seconde tete de lecture pour la voie droite, jusqu'a 0,5 s plus loin (mapping au carre), sous la meme enveloppe, les memes fondus et le meme accord de filtre ; a zero la voie droite EST la gauche, octet pour octet, et rien de l'etat droit n'est touche.
- **Spread / Scatter (`plock.rs` + panneau Sons)** : `PlockState::fill_page` / `clear_field_on_page` et les generateurs purs `spread_values` (lineaire ou en log pour un parametre logarithmique : un balayage de filtre monte par octaves egales) et `scatter_values` (xorshift graine). Le **clic droit sur un slider** ouvre un menu : Spread up / Spread down / Scatter / Clear locks, sur la page que la grille affiche. Generique : standards et speciaux, tous instruments - c'est ce que « offset proportionnel au pas » est devenu, et ca vaut autant sur un filtre qu'un pitch. Effacer retire le verrou du champ et, si la cellule ne verrouille plus rien, ne la laisse pas en p-lock vide qui teinterait la grille.

## 2026-09-14 - [229] fin : l'espace tape un espace dans REAPER (builds 20260914-094219 -> 094858)

**Branche:** `main` - **Build:** `20260914-094858`
**Validation:** `cargo check` warning-clean, `build.ps1 -Install` OK. **Valide par l'utilisateur dans REAPER ET Studio One le 2026-09-14** : saisie, espaces, retour arriere, fleches, Entree ; l'espace relance bien le transport hors saisie.

Le `DLGC_HASSETSEL` du build 093406 n'a rien change : REAPER ne tranche pas « champ de texte ou raccourci » en interrogeant la fenetre focalisee. Deux paris perdus sur ce mecanisme, abandonne au profit d'une mesure.

- **Ce que la trace a etabli, en deux builds.** Build 094219 : la vue VST3 repond « traitee » a `onKeyDown` quand un champ est actif. Journal : `vst3 on_key_down key=100/102/103 wants=true` a chaque lettre - **REAPER passe bien par `IPlugView::onKeyDown`, avant ses raccourcis**. Mais **une seule** lettre a atteint la fenetre de messages dans toute la session : quand le plugin reclame la touche, REAPER **ne poste plus le message Windows**. La saisie est morte parce qu'on prenait sans livrer.
- **Les deux chemins sont donc exclusifs dans REAPER**, et la solution n'est pas d'en choisir un : `on_key_down` **prend la touche** (Play ne part pas) **et la livre lui-meme a egui** par le canal `WM_APP` deja en place, via un `HostKeyHandler` que `nih_plug_egui` enregistre dans `nih_plug::editor::HOST_KEY_HANDLER` au montage de la fenetre. Un caractere devient UN `WM_CHAR` (pas de key-down a cote, baseview emettrait le texte deux fois) ; un code VST3 nomme (retour arriere, fleches, Entree, Suppr, debut/fin, page) devient le `WM_KEYDOWN`/`WM_KEYUP` du VK Windows correspondant. Hors saisie la vue repond « non implementee » et REAPER garde tous ses raccourcis.
- **Trois garde-fous du module clavier revus au passage**, chacun sur mesure : le test « le plugin est-il au premier plan » remontait la chaine des parents (jamais vrai dans une fenetre de FX flottante) et exigeait le survol de la souris (`GetFocus()` rend null depuis ce thread) ; il teste desormais le processus proprietaire puis la parente racine/proprietaire entre la fenetre du plugin et la fenetre active, dans les deux sens. Et la fenetre de messages repond `WM_GETDLGCODE`, sans quoi le dialogue `#32770` de REAPER avalait les `WM_CHAR` comme mnemoniques (deux lettres passaient, puis plus rien).
- Journal de diagnostic (`FLASH_DRUM_KBD_LOG=1`, `%TEMP%lash_drum_kbd.log`) conserve : il trace les deux couches, GUI et VST3, et c'est lui qui a tranche a chaque etape.

## 2026-09-14 - [229] suite : la fenetre de messages se declare champ de texte (build 20260914-093406)

**Branche:** `main` - **Build:** `20260914-093406`
**Validation:** `cargo check` warning-clean, `build.ps1 -Install` OK. **A valider dans REAPER puis Studio One.**

Retour utilisateur apres le build 201813 : « ca marche mais l'espace demarre le morceau ».

- REAPER lit chaque touche dans **sa** boucle de messages avant de la distribuer a la fenetre focalisee, et decide de lancer l'action associee sauf s'il reconnait cette fenetre comme un champ de texte. Rien de ce que fait notre fenetre APRES ne peut annuler ce qu'il a deja fait ; le seul levier est sa decision.
- Un vrai controle `Edit` repond `DLGC_HASSETSEL` a `WM_GETDLGCODE`. La fenetre de messages l'ajoute a sa reponse (avec `WANTALLKEYS | WANTCHARS | WANTARROWS | WANTTAB` deja en place) : un hote qui tranche « champ de texte ou raccourci » en interrogeant la fenetre focalisee nous traite desormais comme un champ de texte.
- **Sans garantie** : le critere exact de REAPER n'est pas lisible d'ici. Si l'espace continue de lancer le transport, la reponse canonique de REAPER existe et se memorise dans le projet : fenetre de FX, menu « + » ou clic droit sur le titre du plugin -> **« Send all keyboard input to plugin »**.

## 2026-09-13 - [229] BUG REAPER : aucune saisie clavier possible (build 20260913-192149)

**Branche:** `main` - **Build:** `20260913-192149`
**Validation:** `cargo check` warning-clean, `build.ps1 -Install` OK. **A valider dans Studio One ET dans REAPER.**

Retour utilisateur : « dans reaper je ne peux pas faire de saisie clavier (pour nommer un preset par exemple) ».

- **Trouve par la trace, pas par deduction.** Le module `win_keyboard` du `nih_plug_egui` vendore journalise chacune de ses etapes, mais seulement en build de debug - or l'interception qu'il combat est un comportement d'HOTE, qui ne se reproduit que dans un vrai DAW. Le journal est desormais **activable en release** par `FLASH_DRUM_KBD_LOG=1` (sortie : `%TEMP%lash_drum_kbd.log`).
- **Ce qu'il disait** : `abort: plugin not in foreground chain`, des centaines de fois, avec `current_focus=0x0`. Le focus n'etait **jamais** pose, donc la saisie n'avait aucun chemin.
- **La cause.** Le garde-fou legitime « ne pas voler le focus quand l'utilisateur passe a une autre application » le verifiait en **remontant la chaine des fenetres parentes** du plugin jusqu'a la fenetre de premier plan. Cela tient dans un editeur **ancre** (Studio One) ; cela ne tient **jamais** dans une fenetre de plugin **flottante** : la fenetre de FX de REAPER est une fenetre de premier niveau a part entiere, hors de la chaine des parents du plugin. La remontee arrivait donc a null et abandonnait a chaque image.
- **Le correctif** teste le **processus proprietaire** de la fenetre de premier plan (`GetWindowThreadProcessId` vs `GetCurrentProcessId`) au lieu de la parente des fenetres. C'est la question que le garde-fou voulait poser depuis le debut, et elle ne depend ni de l'ancrage ni de la hierarchie des fenetres. Studio One reste couvert a l'identique.

## 2026-09-13 - [226] Rift : lecture inversee et fenetre de grain visible (build 20260913-190518)

**Branche:** `main` - **Build:** `20260913-190518`
**Validation:** `cargo test` 416+1+266 OK (3 nouveaux tests), warning-clean, `build.ps1 -Install` OK. **A valider dans Studio One.**

- **`Reverse`** : la tranche se lit a l'envers. Le pas de lecture devient negatif, et c'est desormais le **debut** de la tranche dont la lecture tombe - bornes, fin de voix et **rebouclage** ont ete repris dans les deux sens. Le fondu aux extremites etant deja symetrique, la couture reste propre a l'envers comme a l'endroit. Trois tests : le sens de lecture mesure sur la position, le rendu inverse qui sonne et differe, et la boucle inversee qui ne meurt pas.
- **La fenetre de grain s'affiche** sur le graphe de la texture quand Loop est allume : la portee reellement lue, ses deux bords, et trois chevrons qui donnent le **sens de lecture** et disent qu'elle revient. Loop cesse d'etre une abstraction. La longueur dessinee passe par `rift_grain_seconds`, **la fonction meme qu'utilise la voix**, pour que le dessin ne puisse pas mentir.

## 2026-09-13 - [225] La texture de Rift s'affiche, et tous les sliders ont la meme longueur (build 20260913-185053)

**Branche:** `main` - **Build:** `20260913-185053`
**Validation:** `cargo test` 413+1+263 OK (1 nouveau test), warning-clean, `build.ps1 -Install` OK. **A valider dans Studio One.**

- **La section Oscillator de Rift montre enfin ce qu'elle regle.** Un curseur Offset de 0 a 1 sur douze secondes de matiere ne dit rien de ce qu'il pointe. Le graphe dessine la **forme d'onde de la texture**, la **position de lecture** (trait ambre), la **bande** dans laquelle Wander peut jeter cette position (voile bleu, qui **s'enroule** aux extremites comme le fait la voix), et les **huit prochaines positions** d'Advance (petits traits verts en bas, de plus en plus pales).
- **Le resume de forme d'onde est calcule une fois**, au decodage : 512 colonnes de min/max par texture (`TEXTURE_PEAK_COLUMNS`). Parcourir un demi-million d'echantillons a chaque image d'interface couterait bien plus que l'image ne vaut.
- Le graphe est trouve **par le nom des parametres** (`*_texture`, `*_offset`, `*_wander`, `*_advance`), donc il vaut pour tout instrument qui lirait une texture longue, pas pour un index de voix ecrit en dur.
- **Toutes les sections du Sound Editor gardent desormais la meme largeur de sliders**, celle des sections a graphe. Les sections sans graphe etalaient leurs curseurs sur toute la largeur du panneau : le meme parametre n'avait donc pas la meme longueur d'une section a l'autre, ni d'un instrument a l'autre. **Cela concerne tous les instruments.**

## 2026-09-13 - [224] Le double-clic ne remettait pas la valeur d'usine de l'instrument (build 20260913-162418)

**Branche:** `main` - **Build:** `20260913-162418`
**Validation:** `cargo test` 412+1+262 OK (2 nouveaux tests), warning-clean, `build.ps1 -Install` OK. **A valider dans Studio One.**

Parti d'un retour sur Rift - « le pitch normal doit etre a 0 et quand je fais un reset (double clic) il doit revenir a cette valeur » - le defaut s'est revele bien plus large.

- **`param_default`, la valeur derriere « remets la valeur d'usine », ne lisait la table du registre que pour les samplers, SDrex et Rift.** Les vingt autres voix retombaient sur un `VoiceSettings::default()` **partage par tous**, herite des premieres voix du plugin.
- **Mesure avant correction, sonde temporaire sur les 26 instruments : 20 etaient concernes, et 17 sliders etaient remis a une valeur situee hors de leur propre plage.** Trois exemples : le **Tone du Hi-Hat** revenait a 60 Hz au lieu de 8000 (curseur au minimum), le **Filter Env du Tom 1** a 0 au lieu de 1 (le balayage qui fait le tom disparaissait), le **Decay du Clap** a 0,5 s au lieu de 0,03 s.
- Sur **Rift**, dont le Pitch est en demi-tons (-24..+24), la table partagee ecrivait **60** - ecrete a +24, deux octaves au-dessus. C'est le symptome qui a mis sur la piste.
- **Chaque voix lit desormais sa propre table.** Le geste de double-clic **et** le bouton **Default** (portee Lane, Step et Morph) passent tous deux par la : les trois s'alignent d'un coup.
- **Deux tests de garde** : le contrat sur les 26 voix (la valeur de reset EST celle de la table de l'instrument), et, sur Rift, que cette valeur tombe dans la plage du slider qui l'affiche - c'est ce second controle qui aurait attrape le bug tout seul.

## 2026-09-13 - [221] Rift, build 2 : le mouvement (build 20260913-160405)

**Branche:** `main` - **Build:** `20260913-160405`
**Validation:** `cargo test` 410+1+260 OK (11 nouveaux tests), warning-clean, `build.ps1 -Install` OK. **A valider dans Studio One.**

- **`dsp::Lfo`, la premiere brique LFO du projet.** Chaque voix qui avait besoin d'une modulation periodique se refabriquait un accumulateur de phase a la main (`sdrex.rs` en garde un pour son flanger). Cinq formes : sinus, triangle, carre, dent de scie, et un **sample-and-hold** qui saute au lieu de glisser - c'est lui qui fait begayer une texture. Sortie bipolaire, phase verrouillee au declenchement ou libre.
- **Pitch** : `Pitch Fine` en cents, `Pitch Env` bipolaire (+/-24 demi-tons) sur `Pitch Env Time`, et un **LFO de pitch** (taux, profondeur en demi-tons). Le sweep repart d'un etat deterministe (`trigger_reset_to`) : une enveloppe de pitch qui reprendrait ou la precedente s'est arretee ne se repeterait pas d'un coup a l'autre.
- **LFO de filtre** (taux, profondeur en **octaves**) : il chevauche la coupure deja balayee par l'enveloppe, donc sa profondeur veut dire la meme chose ou que celle-ci l'ait emmenee.
- **Les deux LFO partagent forme et politique de phase** : ce sont deux destinations d'une meme idee, pas deux modulations sans rapport. Seuls taux et profondeur different.
- **`Advance`** : l'offset avance d'un cran a chaque declenchement et boucle en fin de texture, donc un pas repete parcourt la matiere au lieu de rejouer la meme tranche. C'est le mode sequentiel du plan, obtenu sans que la voix ait besoin de connaitre son pas.
- **Cout par echantillon tenu** : l'exponentielle du pitch n'est payee que si enveloppe ou LFO bougent reellement, et le LFO de filtre n'est consomme qu'au re-accordage du biquad (un echantillon sur huit).
- **Deux erreurs de test corrigees en route** : j'avais exige qu'une periode se retrouve a l'identique un cycle plus tard - vrai pour un sinus, faux pour un carre ou une scie, dont la discontinuite tombe pile sur le decalage d'un echantillon qu'introduit le flottant ; et que le sample-and-hold touche ses extremes en dix tirages, ce qu'il n'a aucune obligation de faire. Chaque forme est desormais verifiee sur ce qui la caracterise : periode pour les continues, rapport cyclique pour le carre, rampe monotone pour la scie, dispersion pour le S&H.

## 2026-09-13 - [223] Filter Type en tete de sa section (build 20260913-154758 + reordonnancement)

Le type de filtre decide de ce que veulent dire toutes les rangees sous lui ; il etait declare parmi les parametres speciaux, qui se dessinent **apres** les rangees standard, donc il atterrissait au milieu. Il est desormais rendu en tete de section, et la boucle des speciaux le saute. Repere **par son nom** (`*_filter_type`), donc **Buzz et SDrex sont reordonnes de la meme facon** et le prochain instrument a filtre selectionnable le sera sans rien changer.

## 2026-09-13 - [223] Le graphe d'enveloppe de filtre s'arretait avant le bord (build 20260913-154758)

**Branche:** `main` - **Build:** `20260913-154758`
**Validation:** `cargo test` 399+1+249 OK, warning-clean, `build.ps1 -Install` OK. **A valider dans Studio One.**

Retour utilisateur, capture a l'appui : « pourquoi le graphe ne va pas jusqu'au bout comme les autres graphs ? »

- Le graphe A-H-D du filtre se reservait **15 % de marge** en bout (`span = (attack + hold + decay) * 1.15`) la ou le graphe d'ampli, juste au-dessus dans le meme panneau, divise par `attack + hold + decay` et finit exactement au bord droit. D'ou un plat a droite qui se lit comme un defaut quand les deux graphes se suivent.
- L'intention d'origine etait que le decay « retombe visiblement sur la ligne de base » ; le graphe d'ampli y arrive sans marge. Marge supprimee : la rampe atteint `p = 1` pile au bord droit, donc la courbe atterrit sur la ligne de coupure au repos au bout du trace.
- **Buzz et SDrex en profitent aussi** : ils partagent ce graphe et avaient le meme decalage.

## 2026-09-13 - [222] BUG : en Ext MIDI, aucune lane ne repondait aux notes entrantes (build 20260913-153150)

**Branche:** `main` - **Build:** `20260913-153150`
**Validation:** `cargo test` 399+1+249 OK (1 nouveau test), warning-clean, `build.ps1 -Install` OK. **A valider dans Studio One.**

Retour utilisateur : « des que je passe en Ext Midi, tout est bloque, je peux bouger les sliders mais rien ne joue ».

- **Le plugin emettait et ecoutait sur deux notes differentes.** La sortie utilise la note MIDI **de la lane** (`midi_note_for_slot`, `lib.rs:2209`), reglable par l'utilisateur dans l'onglet Track. L'entree, elle, passait par `voice_idx_from_midi_note`, qui cherchait la note **d'usine du registre**, puis prenait la premiere lane portant cette voix.
- **Deux consequences.** Une lane reaccordee ne repondait jamais. Et comme plusieurs kinds partagent une note d'usine, `position()` renvoyait la premiere voix qui la declare : la note 40 resolvait vers **Snare606**, la 46 vers **OpenHiHat**, toutes deux **retirees des selecteurs depuis [203]/[204]** donc presentes sur aucune lane - la recherche de slot echouait et rien ne se declenchait.
- **Correctif** : nouveau `AtomicTrackLayout::slots_listening_to(note)`, un masque des slots **actifs** dont la note est celle recue. Lock-free, sans allocation, utilisable depuis le thread audio. Entree et sortie parlent desormais de la meme note.
- **Plusieurs lanes peuvent partager une note** volontairement : elles declenchent toutes, au lieu de la premiere seulement. L'echo MIDI sortant reste unique.
- **`voice_idx_from_midi_note` est supprimee** : sans appelant, elle ne serait restee qu'un piege pour le prochain. Test de garde sur les trois cas - lane reaccordee, note partagee, lane inactive.

## 2026-09-13 - [221] Rift, build 1 : le moteur de prelevement (builds 20260913-115048 -> 151632)

**Branche:** `main` - **Build:** `20260913-151632`
**Validation:** `cargo test` 396+1+246 OK (21 nouveaux tests), warning-clean, `build.ps1 -Install` OK. **A valider dans Studio One.**

Les 25 voix existantes fabriquent toutes un son **a partir de rien**. Rift est la premiere a en **prelever** un : il coupe une tranche dans une texture de plusieurs secondes et la sculpte en percussion ou en FX. La meme matiere donne un shaker, une cloche, un clic ou un riser selon l'endroit ou l'on tombe.

- **Ce qui fait l'instrument est deja dans le moteur.** `Offset` est declare comme un parametre special **continu**, donc il est **p-lockable par cellule** et **morphable sur une fusion** sans une ligne de code de notre part : une page de seize pas devient seize prelevements differents dans la meme matiere. C'est la raison pour laquelle cet instrument etait bon marche a construire.
- **Quatre textures embarquees de 6 s**, fabriquees par `tools/gen_textures.py` (graine fixe, donc relancer le script les reproduit a l'octet pres) : champs de bruit colore, resonances metalliques, crepitements, balayages modulaires. Mono PCM 16 bits - **moitie moins lourd** que le float32 des samples 606 a duree egale. **La DLL passe de 14,35 a 16,48 Mo, soit +2,03 Mo**, exactement le poids des fichiers.
- **`TextureBank`** rejoint `SampleBank` dans `sample_bank.rs` : un buffer continu au lieu de huit tranches, mais **le meme parseur RIFF**, extrait en `decode_wav` et partage par les deux. Prechauffe dans `initialize_with_layout`, comme les bancs 606.
- **Le filtre est multi-mode** (passe-bas / passe-haut / passe-bande avec resonance) : c'est le passe-bande resonant qui sculpte une percussion dans du bruit. Il reutilise le `Biquad` existant, **re-accorde un sample sur huit** - ses coefficients coutent deux transcendantes, l'enveloppe qui les pilote bouge lentement, et quatorze slots peuvent jouer en meme temps.
- **Une fenetre a fondu** encadre la tranche : couper une texture en plein cycle est un clic, meme quand l'enveloppe d'ampli est encore ouverte. Le fondu ne descend jamais sous 1 ms.
- **`Wander`** decide si l'aleatoire existe : a 0 la cellule retombe toujours au meme endroit et le pattern se rejoue **a l'identique** (test dedie) ; plus haut, l'offset est re-tire a chaque declenchement.
- **Contrat de retrigger [179] respecte** : position de lecture, filtre et DC blocker repartent d'un etat neuf, `RetrigDeclick` absorbe la discontinuite.
- **Aucun changement de format** : kind 23, voix 25, douze parametres speciaux sur les 32 disponibles. Les sessions existantes sont intactes.
- **Retours du premier build, meme entree** (build 20260913-121200) :
  - **Texture s'affiche en dropdown**, via un **nouveau champ `options` dans le registre** : n'importe quel parametre discret peut desormais declarer ses choix nommes et obtenir un menu, sans ajouter de branche dans le Sound Panel. Les anciennes branches qui reconnaissent un parametre **au libelle** (Saturation Type, Noise Type, Click Type...) restent en place mais ne sont plus le seul chemin. **La valeur devient l'index de la liste, donc 0..3 au lieu de 1..4** : une session enregistree avec le build 115048 relit sa Texture un cran plus bas.
  - **Textures portees de 6 a 12 s et remuees davantage.** Mesure sur des tranches de 150 ms, ecart de centroide entre le 10e et le 90e centile : balayage **x15,7** (quatre ou cinq allers-retours dans le spectre au lieu d'une montee unique), metal **x2,8** (hauteur de base derivant d'une octave et demie, familles de partiels qui entrent et sortent), bruit **x2,7** (couches hautes rendues intermittentes, sinon elles aplatissaient le spectre partout), crepitement **x1,6** (grains rendus tonaux et suivant une zone de hauteur qui derive). Aucune tranche muette sauf 4,4 % sur le bruit, ou c'est la densite qui travaille. **La DLL passe de 13,69 a 17,74 Mo**, exactement le poids des quatre fichiers.
  - **Decay ampli et Decay filtre deviennent des temps ABSOLUS**, 5 ms a **1,5 s**, affiches en secondes, au lieu d'une fraction de la tranche qui les plafonnait a la longueur de celle-ci. Le Hold suit (0 a 1 s). Consequence a connaitre : la **fenetre coupe la lecture**, donc une queue d'1,5 s demande une Window d'au moins autant - d'ou son plafond porte de 2 a **4 s**.

- **Enveloppe de filtre refaite** (build 20260913-122208), retour utilisateur : « quand elle est a fond je n'entends pas le range complet ». **Mesure : il avait raison, et pire que ca.** L'enveloppe AJOUTAIT un nombre fixe de hertz a la coupure (9000 au maximum), ce qui donnait **0 octave de course au reglage d'usine** (filtre a 20 kHz, rien au-dessus), un montant negatif totalement inerte (2000 - 9000 se coince a 20 Hz des le premier echantillon) et 1,8 octave seulement depuis 500 Hz. Un decalage fixe en hertz vaut plusieurs octaves en bas du spectre et une fraction d'octave en haut.
  - Desormais l'enveloppe **interpole en octaves entre la coupure et une extremite** : a +1 elle part du haut du spectre et **atterrit exactement sur le reglage Filter**, a -1 elle part du bas et y monte, les valeurs intermediaires couvrent cette distance au prorata. Course mesuree : 3,3 octaves vers le bas et 6,6 vers le haut depuis 2 kHz, 5,3 / 4,6 depuis 500 Hz.
  - **Le filtre d'usine passe de 20 kHz a 6 kHz** : a 20 kHz la commande paraissait morte faute de place au-dessus. A 6 kHz il reste 1,7 octave vers le haut et 8,2 vers le bas, et les textures (centroide median 6,5 a 9,2 kHz) restent ouvertes. Deux tests de garde remplacent la sonde de mesure.
- **Window devient Grain, plus un interrupteur Loop** (build 20260913-123836), sur une question de l'utilisateur : « pourquoi ne pas plutot gerer le Window avec un hold dans l'env ? ». **Il avait raison** : deux reglages se disputaient la meme grandeur, et le plus court gagnait en silence - un Decay d'1,5 s avec une fenetre de 40 ms donnait 40 ms de son, sans que rien ne le dise.
  - **Loop eteint (defaut) : l'enveloppe decide de tout.** La lecture court jusqu'a son extinction, Grain ne tronque plus rien. Le conflit disparait.
  - **Loop allume : le grain se repete sous l'enveloppe.** Grain cesse d'etre une duree pour devenir une taille de grain (5 ms a 4 s) : un grain de 15 ms tenu par une enveloppe d'une seconde donne des textures soutenues, des roulements et des begaiements - ce qu'aucune des 25 autres voix ne produit. C'est la seule raison qui justifiait de garder deux reglages.
  - `Grain` et `Grain Shape` sont **grises** quand Loop est eteint, jamais caches. La regle est ecrite sur les NOMS des parametres, pas sur l'index de l'instrument : tout instrument declarant un interrupteur `*_loop` a cote de rangees `*_grain*` en heritera.
  - **La couture du bouclage ne claque pas**, mesure a l'appui : le saut entre echantillons voisins AUX POINTS de rebouclage vaut 0,00035 contre 0,0087 d'ecart median ordinaire sur la meme matiere, vingt-cinq fois plus petit, parce que le fondu ramene le signal pres de zero. Le premier test ecrit comparait le plus grand saut du rendu a son niveau crete - sur du bruit, cela mesure le bruit et non un clic ; il declarait donc un defaut inexistant. Le test definitif mesure aux coutures, sur les quatre textures.
  - Un test existant du registre a attrape une erreur de nommage au passage : tout parametre dont le nom contient `_fade` doit declarer son unite, or celui-ci est une proportion. Renomme `grain_shape` plutot que d'affaiblir la regle.

- **Filtre remis sur la convention du plugin** (build 20260913-151632), retour utilisateur : « pourquoi tu fais toujours un filtre tout pourri avec un affichage merdique ? Reprends les systemes de filtre des autres instruments. » **Il avait raison, et la faute est entiere : `buzz.rs` a deja exactement ce filtre - multi-mode, selectionnable - et je ne l'avais pas regarde avant d'ecrire le mien.** Sa ligne est reprise telle quelle :
  `let amt = (env * filter_env_amount).clamp(0.0, 1.0); let cutoff = base * (20000.0 / base).powf(amt);`
  L'enveloppe **ouvre** la coupure vers 20 kHz et elle retombe exactement sur le reglage Filter. Mes deux versions precedentes faisaient autre chose : la premiere ajoutait un nombre fixe de hertz (0 octave de course au reglage d'usine, mesure), la seconde balayait du haut vers le bas avec un montant bipolaire qu'aucune autre voix n'a.
  - **`Filter Env` passe de -1..+1 a 0..1**, comme partout ailleurs.
  - **`filter_type_label` passe de "Multi" a vide**, comme Buzz : le libelle affichait « Filter (Multi) » alors que le type a son propre menu.
  - **Coupure d'usine 1200 Hz et Filter Env 0,6**, les valeurs de Buzz : filtre ferme au repos, ouvert par l'enveloppe, donc le balayage s'entend des qu'on pose l'instrument. Les reglages 20 kHz puis 6 kHz essayes avant laissaient la commande sans effet audible.
  - **Resonance est conservee** - Buzz a un Q fixe, mais sur du bruit c'est elle qui sculpte une percussion - avec pour defaut **0,9**, la valeur exacte de Buzz : tant qu'on n'y touche pas, le filtre se comporte comme les siens.
  - Deux tests de garde verrouillent l'alignement : l'enveloppe doit atteindre 20 kHz a fond et retomber sur la coupure, et la moitie du montant doit couvrir la moitie de la distance **en octaves**.

- **Inclut aussi l'anneau de tete de lecture** reste **blanc sur toutes les cellules**, y compris les vertes (build 20260913-110432, compile mais jamais installe faute de Studio One ferme). Seul le chiffre de pulsations d'une fusion passe en sombre sur fond clair.

## 2026-09-13 - [220] Trois couleurs pour la grille : bleu, violet, vert clair (builds 20260913-093827 -> 20260913-105120)

**Branche:** `main` - **Build:** `20260913-105120`
**Validation:** `cargo check` warning-clean, `build.ps1 -Install` OK. **A valider dans Studio One.**

L'ambre quitte la grille. Elle ne parle plus qu'avec trois teintes : le **bleu** d'un pas actif, le **violet** d'un p-lock **sound**, le **vert clair** d'un p-lock **sequenceur**.

- **Le p-lock sound passe de l'ambre `#ff8c00` au vert clair `#b5ffe1`**, le p-lock sequenceur garde le **violet `#a855f7`** (build 20260913-105120 ; les deux teintes etaient inversees dans les builds precedents). Motif : la pastille violette de [219] tombait sur des cellules ambre, ou elle ne se detachait pas. Aucune paire de teintes moyennes ne passait le seuil de contraste sur le bleu **et** sur l'ambre a la fois ; retirer l'ambre resout le probleme au lieu de le deplacer.
- **La pastille marche desormais dans les deux sens** (`skeuo::plock_pip`, ex-`seq_plock_pip`) : en mode Sound une pastille **violette** signale un p-lock sequenceur, en mode Sequencer une pastille **vert clair** signale un p-lock sound. Avant, seul le premier sens etait marque.
- **L'atlas des pads a ete re-teinte** (`assets/pads/atlas-pads.png`). Les cellules sont des **bitmaps bakes**, pas des rectangles peints : changer les jetons de `theme.rs` ne les atteint pas. Les trois variantes allumees etant le **meme rendu dans trois teintes**, l'echange est mecanique - les 17 emplacements `seq` (pad + 15 longueurs de fusion + variante eteinte) gardent **tels quels** leurs pixels violets, et les 17 emplacements `link` sont un remap **teinte/saturation/valeur** du violet vers le vert clair, calibre sur le mi-ton mesure du pad. Seuls les pixels qui **different** entre les deux bakes sont touches, donc le puits sombre autour de chaque pad reste bit-a-bit identique a celui de ses voisins : pas de couture entre cellules.
- **Les incrustations blanches basculent en sombre sur un pad clair** (`is_light_pad`) : l'anneau de tete de lecture et le nombre de pulsations d'une fusion disparaissaient sur le vert clair.
- **Les accents qui n'etaient pas des p-locks gardent leur teinte chaude** : la puce « Random » du bas d'ecran, « Paste » / « Yes, overwrite » du menu de page et l'export d'usine passent a `AMBER()` - ils empruntaient le jeton du p-lock sans en parler.
- **Les trois skins suivent** (Dark, Midnight, Ember), chacun dans sa nuance. L'atlas, lui, est bake une seule fois : hors Dark, les pads gardent l'art du Dark, comme avant.
- **La pastille est rentree dans le pad** (build 20260913-100237) : collee au coin, elle mordait le bord du sertissage.
- **Une seule pastille par bloc fusionne** (build 20260913-103346) : les cellules d'une fusion lisent toutes les p-locks de la cellule de **depart**, donc la pastille se repetait sur chacune - cinq points en travers d'un bloc de cinq. Elle est desormais posee sur la **derniere** cellule du bloc, c'est-a-dire dans son propre coin haut-droit ; un bloc qui deborde de la page garde sa pastille sur la derniere cellule visible.
- **Aucun changement de format** : ni persistance, ni semantique. Uniquement de la couleur.

## 2026-09-12 - [219] Condition « And » et pastille violette sur les cellules (build 20260912-103510)

**Branche:** `main` - **Build:** `20260912-103510`
**Validation:** `cargo test` 380+1+230 OK (1 nouveau test), warning-clean, `build.ps1 -Install` OK. **A valider dans Studio One.**

- **Bouton « And »**, a cote de « Not » : il ajoute une **seconde condition**, combinee a la premiere. Le pas ne joue que sur les boucles ou **les deux** sont vraies - `1/2` et `1/3` ensemble ne declenchent qu'une boucle sur six. A l'activation, le second terme demarre sur la valeur du premier, donc rien ne change tant qu'on n'en choisit pas un autre ; une seconde grille apparait sous la premiere et n'occupe de place que si elle sert.
- **`Not` inverse l'ensemble**, second terme compris : c'est « pas (A et B) ». Toujours sans effet sur un `Always` seul.
- **Pastille violette en haut a droite des cellules portant un p-lock sequenceur**, visible quand la grille affiche les p-locks **sound** (`skeuo::seq_plock_pip`). Une cellule est coloree par le p-lock du mode affiche, donc un p-lock sequenceur y etait invisible - et depuis [213] on peut en poser un sans jamais quitter le mode Sound. L'inverse ne demande pas de marqueur : en mode Sequencer la cellule est deja violette. **C'est la reponse a [214].**
- **Aucun changement de format.** Le second terme occupe les bits 16-23 du `u32` deja stocke, range en `valeur + 1` pour que zero signifie « pas de second terme » - `Always` etant une condition legitime numerotee 0. Le decodage passe par un `StepCondition::from_raw` partage, et un `_ =>` ramene toute valeur inconnue a `Always` : une donnee corrompue ou future ne peut pas faire taire un pas.

## 2026-09-12 - [218] Menu p-lock allege et "Not" sur les conditions (build 20260912-101156)

**Branche:** `main` - **Build:** `20260912-101156`
**Validation:** `cargo test` 379+1+230 OK (2 nouveaux tests, 1 reecrit), warning-clean, `build.ps1 -Install` OK. **A valider dans Studio One.**

- **Un clic droit ferme le menu contextuel**, ou qu'il tombe, y compris sur le menu lui-meme : le geste qui l'ouvre est celui qui le referme, donc s'en debarrasser ne demande plus de viser le vide.
- **La ligne « Mode » est retiree.** Depuis [211] un p-lock est toujours cree lie, donc elle affichait « Linked to Global » jusqu'au premier parametre touche puis « Mixed » indefiniment - jamais une information sur laquelle agir. Seul un snapshot herite d'une ancienne session meritait d'etre nomme, et ceux-la ne peuvent plus etre crees.
- **Bouton « Not » sur les conditions du p-lock sequenceur.** Il inverse la condition choisie : `3/4` devient « toutes les boucles sauf la 3e sur quatre ». C'est un **modificateur** : il survit au changement de condition. Grise sur `Always`, ou l'inverser voudrait dire « jamais » - ce que desactiver le pas fait deja.
- **« Not 1st loop » quitte la liste** : c'etait ce modificateur cable en dur sur une seule condition. `1st loop only` + `Not` dit exactement la meme chose.
- **Aucun changement de format.** Le drapeau occupe un **bit haut du `u32` deja stocke** (`CONDITION_NEGATE_BIT = 0x100`) : les valeurs basses gardent leur sens, la Pattern Bank qui serialise le mot brut le transporte sans rien changer, et un pas regle sur l'ancien « Not 1st loop » **joue a l'identique** - il est relu comme `1st loop only` inverse, la meme regle ecrite autrement. Test dedie.

## 2026-09-12 - [213] Choisir le type de p-lock dans le menu de la cellule (build 20260912-094023)

**Branche:** `main` - **Build:** `20260912-094023`
**Validation:** `cargo test` 377+1+230 OK, warning-clean, `build.ps1 -Install` OK. **A valider dans Studio One.**

- Poser un p-lock sequenceur demandait de basculer d'abord le **P-Lock Mode** de toute la grille. Le choix se fait desormais **dans le menu de la cellule** : un segmente `Sound | Sequencer` en tete du popup, les memes deux libelles que l'interrupteur global pour que ce soit lu comme le meme choix.
- Le choix est **local au popup** (nouveau champ `PlockPopup::sequencer`, initialise depuis le mode de la grille a l'ouverture) : basculer dans le menu ne change pas le mode de la grille, donc ni l'affichage des cellules ni le prochain clic droit.
- **Le Lane Editor suit** : choisir `Sound` pointe le panneau sur la cellule et bascule sur l'onglet Sound, choisir `Sequencer` relache la selection - le panneau n'a rien a dire d'un p-lock sequenceur.
- **Limite connue, c'est [214]** : la couleur d'une cellule suit toujours le **mode de la grille**. Un p-lock sequenceur pose depuis une grille en mode Sound existe et joue, mais la cellule ne le montre pas tant qu'on n'a pas bascule le mode global.

## 2026-09-12 - [217] Clear Plock qui semblait ne rien faire (build 20260912-093224)

**Branche:** `main` - **Build:** `20260912-093224`
**Validation:** `cargo test` 377+1+230 OK, warning-clean, `build.ps1 -Install` OK. **A valider dans Studio One.**

Retour utilisateur : « un clear plock sur une cellule desactivee semble ne pas fonctionner ». Le `Clear` faisait pourtant bien son travail ; c'est ce qui se passait **juste apres** qui le defaisait.

- **Le mecanisme.** Le clic droit qui ouvre le menu **pointe aussi le Lane Editor sur la cellule**, en portee Step. Apres le Clear, le panneau restait pointe la, sur une cellule sans p-lock - et en portee Step, ecrire n'importe quelle rangee **recree** le p-lock : `PlockSource::set` leve le bit d'activite du pas, ce qui est precisement ce qui permet a une rangee de creer un override. Le premier reglage touche ensuite ramenait donc le p-lock, et le Clear semblait n'avoir rien fait.
- **Le correctif.** Effacer un p-lock **relache aussi la selection du panneau** quand elle designe cette cellule : le Lane Editor repasse sur le son global de la lane. Sans p-lock, il n'y a plus rien a editer en portee Step.
- Non reproduit de mon cote : le mecanisme a ete trouve par lecture du chemin de creation. A confirmer par le test - voir la checklist du build.

## 2026-09-11 - [216] Bouton T retire, lampe MIDI conservee et reparee (build 20260911-200326)

**Branche:** `main` - **Build:** `20260911-200326`
**Validation:** `cargo test` 377+1+230 OK, warning-clean, `build.ps1 -Install` OK. **A valider dans Studio One.**

- **Le bouton « T » disparait des rangees de lane.** Il cumulait deux roles sans rapport : declencher l'audition de la voix au clic, et servir de **lampe temoin** de la lane. Les lanes n'ont plus que deux pastilles, M et S ; l'en-tete suit et la place liberee revient aux cellules.
- **La lampe restait allumee - c'etait un bug, pas un choix.** L'editeur ne se redessine que sur evenement : la frame qui allumait la lampe etait souvent la derniere dessinee, donc les pixels restaient a l'ecran jusqu'a ce qu'autre chose force un rafraichissement. **Rien ne demandait jamais la frame qui l'aurait eteinte.** Le repaint d'extinction est desormais programme (`request_repaint_after`) - meme piege que la pulsation d'edition des fusions, corrigee en son temps pour la meme raison.
- **L'indicateur MIDI est conserve**, retour utilisateur : « l'indicateur de midi in est tres important en fait ». C'est maintenant une petite LED ambre en haut a droite de la **plaque de nom** de la lane (`skeuo::lane_activity_led`), allumee 0,12 s. Elle couvre les memes evenements qu'avant : note MIDI entrante, et les confirmations de Paste Lane, Paste Grid et Randomize Lane.
- **L'audition reste cablee cote audio sans declencheur.** `voice_test_triggers` et sa boucle dans `process()` sont conserves a dessein : la reexposer ailleurs - panneau Sound, raccourci - sera un changement d'interface seulement. Le parametre ne traverse plus la chaine UI (`create_editor`, `draw_grid_v2`, la rangee de lane).

## 2026-09-11 - [216a] Clear Plock ferme le menu contextuel (build 20260911-200326)

- Apres un **Clear Plock**, le menu se ferme : le p-lock n'existe plus, donc les options qui le concernent n'ont plus d'objet.

## 2026-09-11 - [215] Le menu du p-lock se reduit a ce que le panneau ne fait pas (build 20260911-191858)

**Branche:** `main` - **Build:** `20260911-191858`
**Validation:** `cargo test` 377+1+230 OK, warning-clean, `build.ps1 -Install` OK. **A valider dans Studio One.**

- **Creer un p-lock mene directement au panneau.** « Create Plock » pose le p-lock, pointe le Lane Editor sur la cellule, bascule sur l'onglet Sound et **ferme le menu**. Creer un p-lock *est* le debut de son edition : il n'y a plus de second ecran contextuel entre le geste et le panneau qui fait le travail.
- **« Edit In Panel » retire.** Le clic droit qui ouvre ce menu pointait deja le panneau sur la cellule, donc le bouton demandait de confirmer quelque chose qui avait deja eu lieu.
- **Le menu d'une cellule deja p-lockee n'a plus que Copy Plock et Clear Plock**, sous la ligne Mode. « Paste Plock » en est retire sur la meme instruction : ecraser un p-lock depuis le presse-papier demande desormais Clear puis coller. Le bouton Paste reste sur une cellule **vide**.
- La ligne **Mode** (Linked to Global / Full Snapshot / Mixed) est conservee : ce n'est pas un bouton, et c'est le seul endroit qui identifie un p-lock snapshot herite, que [211] a rendu increable.

## 2026-09-11 - [211] P-lock : l'option Snapshot disparait du menu (build 20260911-190921)

**Branche:** `main` - **Build:** `20260911-190921`
**Validation:** `cargo test` 377+1+230 OK, warning-clean, `build.ps1 -Install` OK. **A valider dans Studio One.**

- Le menu de creation d'un p-lock sound proposait **deux modes** : « Link to Global » (seuls les champs touches surchargent) et « Snapshot Current Settings » (les 46 champs geles d'un coup). Le second est retire ; le bouton restant est renomme **« Create Plock »**, le nom « Link to Global » n'opposant plus rien une fois seul.
- **Le format n'est pas touche.** Un p-lock snapshot enregistre avant ce build se charge, se joue et s'edite comme avant, et la ligne **Mode** du menu le nomme toujours « Full Snapshot ». C'est un mode de **creation** qui disparait, pas une capacite du stockage.
- Nettoyage induit : la branche supprimee etait le seul endroit du menu qui lisait les reglages de son, donc `SoundSettingsState` sort de la signature de `draw_plock_menu` **et** de `draw_plock_popup`, avec son import. Un parametre de moins a transporter depuis `ui.rs`.

## 2026-09-09 - [210] Le premier clic d'une fusion se voit enfin (build 20260909-164540)

**Branche:** `main` - **Build:** `20260909-164540`
**Validation:** `cargo test` 377+1+230 OK, warning-clean, `build.ps1 -Install` OK. **Validé dans Studio One (2026-09-09).**

Retour utilisateur : « MAJ + clic sur la premiere cellule, rien n'apparait, on a l'impression que ca ne fonctionne pas. » Deux causes, dont une qui n'est pas graphique.

- **L'etat s'evaporait.** `fusion_mode_active` vaut « MAJ est enfoncee **en ce moment** », et la grille remettait *tous* les debuts de fusion a `None` des que le modificateur remontait. Relacher MAJ une fraction de seconde entre les deux clics jetait donc le geste. Le debut de fusion **survit** maintenant au relachement ; **Echap** l'annule, comme le faisaient deja un clic normal sur une cellule de la lane et les operations de lane.
- **Le marqueur etait invisible.** Remplissage `rgb(20, 34, 58)` contre une cellule vide a `rgb(27, 27, 34)` : meme luminosite, a peine plus bleu, derriere un liseré de 1,5 px. Le remplissage est desormais tire de 30 % vers l'accent (`rgb(36, 71, 117)`) et le liseré passe a 2 px pleins.
- **Un crochet `[` est peint dans la cellule** (`skeuo::fusion_start_bracket`) : une fusion est une **etendue**, donc son debut se lit comme le crochet qui l'ouvre - « le span commence ici, j'attends sa fin » - au lieu d'une simple teinte. Statique a dessein : le lisere pulsant signifie deja « cette fusion est en cours d'edition », et deux etats ne doivent pas partager un signal.
- Le marqueur n'est plus conditionne au modificateur : il reste affiche pendant que la main va vers la seconde cellule.

Ecarte : reutiliser la pulsation existante (elle dit « en cours d'edition »). Propose et non retenu pour l'instant : l'**apercu du span au survol**, qui afficherait en fantome la zone entre le depart et la cellule survolee.

## 2026-09-09 - [193] [194] Routing des sorties : Main deconnecte, sortie partageable (build 20260909-162500)

**Branche:** `main` - **Build:** `20260909-162500` (l'etiquette du selecteur corrigee apres retour utilisateur sur `20260909-161422`)
**Validation:** `cargo test` 377+1+230 OK (2 tests, 1 reecrit), warning-clean, `build.ps1 -Install` OK. **Validé dans Studio One (2026-09-09).**

L'etude a montre que les deux taches etaient bien plus petites qu'annonce : la capacite de [193] existait deja, et l'audio de [194] aussi.

- **[193] Une sortie aux sort la lane du mix principal.** L'interrupteur **Main Mix** par lane et son effet (`main_on` -> `compute_mix_gating`) existaient deja ; il manquait le **couplage**. Choisir `Out N` baisse l'interrupteur, revenir a `No Aux` le remonte - sinon la lane ne sortirait plus nulle part. **Un defaut, pas un verrou** : l'interrupteur reste utilisable, donc une lane peut encore aller dans le mix ET sur son canal, pour du traitement parallele.
- **[194] Plusieurs lanes peuvent partager une sortie.** Rien a faire cote DSP : la boucle de mixage aux itere toutes les lanes et **additionne** deja dans le bus choisi. Le seul obstacle etait `assign_slot_output_exclusive`, qui **volait** la sortie a son detenteur, silencieusement. Devenue `assign_slot_output`, elle se contente d'assigner.
- **La regle d'exclusivite de [117] (« un `Out N` est exclusif a une lane ») est donc levee.** Rien ailleurs n'en dependait : aucune recherche inverse « quelle lane possede Out N », et les noms de bus exposes a l'hote sont statiques.
- **Le selecteur dit ce qui est deja pris** : `Out 3`, `Out 3 - 1 lane`, `Out 3 - 2 lanes`, via `lanes_on_output()`. Le decompte inclut **toutes** les lanes, donc l'etiquette decrit la **sortie** et non celui qui la lit : « 2 lanes » est vrai vu de n'importe ou. Une premiere version comptait les *autres* lanes, ce qui affichait « 1 lane » sur une sortie qui en portait deux - retour utilisateur, corrige avant validation. Partager devient un choix, pas une sommation accidentelle.
- **Les sessions existantes ne sont pas reecrites** : une lane deja en `Out N` *et* dans le Main y reste. Appliquer la nouvelle regle au chargement la ferait disparaitre du mix principal sans que l'utilisateur l'ait demande - meme raisonnement que la migration de [203] [204].
- A savoir : deux lanes sur la meme sortie **somment leurs niveaux**. C'est le comportement attendu d'un bus partage, mais le bus sature plus vite qu'avec une lane seule.

## 2026-09-09 - [191b] Le deplacement de lane ne casse plus les liens (build 20260909-154202)

**Branche:** `main` - **Build:** `20260909-154202`
**Validation:** `cargo test` 376+1+229 OK (4 nouveaux tests), warning-clean, `build.ps1 -Install` OK. **Validé dans Studio One (2026-09-09).**

Un lien est **positionnel** - « je joue la lane juste au-dessus de moi » - donc tout deplacement qui casse l'adjacence re-pointe silencieusement le lien vers un autre instrument. Le glisser-deposer ignorait completement cette contrainte. Trois regles, plus une quatrieme que le probleme implique :

- **Une lane esclave ne se deplace plus.** Sa poignee refuse le glisser, affiche le curseur d'interdiction et explique pourquoi au survol : deplacer la lane suivie, elle emmene ses liees.
- **Une maitresse emporte toute sa chaine.** `chain_len()` mesure la maitresse plus la suite ininterrompue d'esclaves en dessous, et `lane_move_order_block()` deplace le bloc d'un seul tenant. C'est la generalisation de l'ancienne permutation : pour `len == 1` elle donne **exactement** le meme resultat, verifie sur les 196 couples (from, to) - le glisser d'une lane seule n'a donc pas change de comportement.
- **Aucun depot au milieu d'une chaine.** Sans ca, une lane laissee entre une maitresse et son esclave se glissait entre elles et volait le lien. `snap_gap_out_of_chains()` ramene le point de chute a l'extremite la plus proche, et **l'indicateur de depot applique le meme calage** : le trait bleu est dessine ou la lane va reellement atterrir.
- **Deposer une chaine dans elle-meme** ne fait plus rien, au lieu de produire une permutation partielle.

Le layout suivait auparavant `move_slot(from, to)`, qui ne connait qu'une lane ; il applique desormais la meme permutation de bloc que le reste de l'etat (pas, fusions, p-locks, reglages de son, parametres de lane), donc tout reste solidaire.

## 2026-09-09 - [191] Lane liee : une fleche retour sur la seule esclave (build 20260909-152610)

**Branche:** `main` - **Build:** `20260909-152610`
**Validation:** `cargo test` 372+1+229 OK, warning-clean, `build.ps1 -Install` OK. **Validé dans Studio One (2026-09-09).**

- **Ce qui n'allait pas.** Le filet bleu de 2 px colle au bord gauche de la rangee echouait sur trois points : a la limite du visible, muet sur la lane d'origine, et **identique sur deux lanes liees a la meme maitresse** - impossible de lire un groupe ni de reperer la source.
- **Choisi apres maquettes** : une **fleche retour** (glyphe de type `L` inverse), posee sur la **seule lane esclave**. Elle sort de la rangee du dessus, tourne, et pointe dans la lane : « ma grille vient de la-haut ». La lane d'origine ne porte **aucune** marque - la presence de la fleche suffit a dire « liee », et sa direction dit d'ou.
- **Ecartees** : une accolade reliant la maitresse a ses suiveuses (elle disait le groupe mais alourdissait la gouttiere), les cellules ternies (« terni » veut dire *lecture seule* partout ailleurs, alors qu'editer une lane liee **ecrit** dans la grille partagee, et ca se confond avec une lane mutee), et le nom en retrait (il rétrécissait la plaque de nom sur les lanes liees seulement - une zone qui bouge selon un etat, interdit par la regle des zones stables).
- **Zero pixel de mise en page.** Le glyphe (9 x 16 px) tient dans l'espace deja libre entre la matrice de points de la poignee - large de ~6 px dans une gouttiere de 14 - et la plaque de nom, en empietant sur l'ecart de 7 px qui les separait. Aucune rangee ne bouge.
- Rendu dans `skeuo::link_arrow`, comme tout le reste du dessin ([SK]), et juge sur un rendu PNG **a taille reelle** avant build : la premiere version frolait la matrice de points et sa hampe, trop courte, se lisait comme un L trapu.

## 2026-09-09 - [207] Compensation de gain de la saturation : egalisation en energie (build 20260909-150755)

**Branche:** `main` - **Build:** `20260909-150755`
**Validation:** `cargo test` 372+1+229 OK (3 gardes remplacant l'ancien test de contrat), warning-clean, `build.ps1 -Install` OK. **Validé dans Studio One (2026-09-09).**

- **Le defaut.** La compensation normalisait la courbe de saturation en **un seul point** (entree 0,5), ce qui derive des que le signal reel sort de ce point - et un coup de batterie culmine plutot vers 0,8-1,0. Mesure sur un sinus 110 Hz d'amplitude 0,8, ecart maximal au signal sec sur la course de l'Amount :

| type | avant | apres |
|---|---|---|
| SoftClip | -1,68 dB | -1,09 dB |
| Valve | **+3,53 dB** | +0,58 dB |
| Transistor | -1,71 dB | -0,92 dB |
| Tape | -1,51 dB | -1,00 dB |
| HardClip | -2,33 dB | -1,08 dB |

- **Les trois symptomes disparaissent.** Le sens de la derive ne depend plus du type (SoftClip perdait ~4 dB de pic pendant que Valve gagnait), la **dispersion entre types a mi-course passe de 3,16 dB a 0,64 dB** - changer de type auditionne un caractere et non un volume -, et le **pic du Valve a fond descend de 1,357 a 0,899**, donc il ne sort plus au-dela de 1,0. Le saut de -0,69 dB du HardClip et le pas de 1,0 a 1,08 au depart de zero sont absorbes.
- **Comment.** `update_compensation` egalise le **RMS** du signal sature sur celui du sec, mesure sur un cycle de sinus de 32 points a 0,7 de pic (RMS 0,5, le niveau que visait l'ancienne reference ponctuelle). Cout : 32 evaluations de courbe **par changement de parametre**, jamais par echantillon - les appelants sont les constructeurs et `set_settings`, soit au plus une fois par bloc.
- **Le caractere est intact** : les taux de distorsion mesures sont identiques a ceux d'avant (SoftClip 14,9 / 32,2 / 43,4 % au quart, moitie et bout de course). Seul le niveau change.
- **Il reste ~1 dB de derive** sur quatre types, et c'est irreductible : une non-linearite comprime d'autant plus que le signal est fort, donc aucune compensation calculee sur un signal de test ne peut annuler l'ecart pour tous les niveaux. Le garde-fou autorise 2 dB.
- **L'ancien test affirmait le defaut** (« une entree 0,5 ressort a ~0,5 ») : remplace par trois gardes qui pinent le contrat reel - le niveau tient sur toute la course de l'Amount, ne saute pas en quittant zero, et les cinq types atterrissent au meme niveau.

**Ce build change le son des sessions existantes** partout ou une saturation est active : c'etait l'objet de la tache. Le Valve baisse nettement (il etait trop fort de ~3,5 dB), les autres remontent legerement. L'Output Gain manuel de chaque voix reste disponible pour rattraper au gout.

## 2026-09-09 - [208] Nouvel instrument OH6smp (charley ouvert multisample) (build 20260909-144136)

**Branche:** `main` - **Build:** `20260909-144136`
**Validation:** `cargo test` 370+1+227 OK (3 nouveaux tests), warning-clean, `build.ps1 -Install` OK. **Validé dans Studio One (2026-09-09).**

- **Nouvel instrument OH6smp** (kind 22 / voix 24, categorie HH, note MIDI 46, label `o6`), a partir de `wav/OH.wav` copie en `assets/oh606.wav` : mono 44,1 kHz float32, 8 s, soit **8 coups de ~1 s** - deux fois plus longs que ceux du charley ferme, ce qui est le propre d'un charley ouvert.
- **Un seul moteur pour les deux charleys.** Plutot que dupliquer les 655 lignes de `ch606.rs`, la voix porte desormais son banc en champ (`Ch606Voice::with_bank`), resolu une fois a la construction ; sur le thread audio ce n'est qu'une lecture de pointeur. Meme approche que la facade `AcVoice` de [195]. `new()` garde le banc ferme, `DrumVoiceKind::Oh606` est le meme type de voix sur l'autre banc.
- **Defauts identiques a CH6smp sauf le decay**, ouvert a 0,9 s (contre 0,2) - le reste du comportement sampler est partage : Analog Mode (tirage sans repetition immediate), Sample 1..8, One Shot actif ([206]), Start/End, Pitch Fine en cents, et le pack saturation.
- **Banc prechauffe** dans `initialize_with_layout()` comme les trois autres : `create_voice_for_kind()` peut etre appele depuis `process()` via `reinitialize_slot()`, donc le decodage du WAV ne doit jamais arriver la.
- **Onze listes codees en dur remplacees par un predicat nomme.** L'appartenance a la famille sampler s'ecrivait `matches!(voice_idx, 13 | 14 | 15)` dans onze endroits (registre, `sound_settings`, huit dans le panneau Sound) : ajouter OH6smp en aurait fait douze. C'est maintenant `instrument_registry::is_sampler()`, une seule definition.
- **Generateur** : OH6smp emprunte le role 3 (l'OpenHiHat), comme OH6(AC) - GENERATE ecrit donc bien une ligne sur sa lane.
- Tests : le banc ouvert se decode en 8 coups audibles et finis, plus longs que ceux du ferme ; la voix construite sur le banc ouvert **suit bien les samples ouverts** (comparaison par correlation de forme, la seule valable puisque la sortie porte le volume et la rampe d'attaque) et pas les fermes ; elle sonne, reste finie et se tait.

### Corrige au passage

Deux messages d'assertion (`kick.rs`, `sdrex.rs`) contenaient des **paquets d'espaces** laisses par des continuations de ligne avalees lors de sessions precedentes. Le motif de detection que j'utilisais exigeait une lettre avant les espaces et ratait donc les cas suivant une ponctuation ; il est corrige en `'"[^"]*[^ "] {3,}[^ "]'`.

## 2026-09-09 - [203] [204] Snare606 et OpenHiHat retires des menus, lanes migrees (build 20260909-142638)

**Branche:** `main` - **Build:** `20260909-142638`
**Validation:** `cargo test` 367+1+224 OK (3 nouveaux tests), warning-clean, `build.ps1 -Install` OK. **Validé dans Studio One (2026-09-09).**

- **Les deux kinds quittent les pickers, le code les garde.** `index()` vaut `self as usize` et cet index est **persiste** dans les sessions et les presets : supprimer les variantes aurait renumerote tous les kinds suivants et change silencieusement l'instrument de chaque lane sauvegardee. Nouveau `retired_replacement()` (Snare606 -> Sd6Ac, OpenHiHat -> HiHat) et `selectable()`, consulte par `kinds_in()` - donc les **trois** pickers (pastille `+N`, menu de lane, onglet Track) les perdent d'un coup. Les voix DSP restent en place.
- **Migration des lanes sauvegardees** dans `PersistentField::set`, le **seul entonnoir** de toute ecriture de layout : restauration de session, edition UI, presets de layout, kit embarque dans un preset de pattern. Idempotente (test dedie).
- **La lane garde sa note MIDI**, pour qu'un projet qui la pilote depuis le DAW continue de frapper la meme lane. Un nom reste au defaut suit le nouveau kind (une lane affichant « Open Hi-Hat » alors qu'elle est un Hi-Hat mentirait) ; un nom saisi par l'utilisateur est conserve.
- **Les reglages de son ne sont pas retouches.** Ils avaient ete stockes contre la forme de parametres de l'ancienne voix, donc une lane migree ne sonne pas exactement comme la voix retiree - c'est le principe meme du retrait. A noter pour l'OpenHiHat : un decay stocke au-dela de 1,5 s depasse le nouveau plafond du HiHat ([188]) et reste joue tel quel jusqu'a ce que le slider soit touche.
- **Kit 12 lanes** : la lane 4 (OpenHiHat) devient un **second HiHat**, qui choke toujours avec le premier - a allonger pour le son ouvert ; la lane 11 (Snare606) devient **SD6(AC)**.
- **Cas limite traite** : un **preset d'instrument** capture pour une voix retiree bascule la lane sur son remplacant mais **n'ecrit pas ses valeurs** (ses 32 speciaux signifient autre chose sur la nouvelle voix), donc la lane arrive sur le son d'usine du remplacant. C'est la prudence que le chemin des presets de pattern applique deja en sautant une lane dont le kind ne correspond pas.
- **Rien a changer cote generateur** : `Oh6Ac` emprunte le role 3 du pattern de roles, qui existe independamment du kind selectionnable. Et `effective_choke_group` continue de donner le groupe 1 aux sessions legacy : une lane OpenHiHat devenue HiHat y reste.
- Le test de partition des categories garde sa garantie, reformule : il epingle desormais que **seuls** ces deux kinds manquent aux pickers, et verifie a part que `category()` repond toujours pour les 22 kinds.

## 2026-09-09 - [190] Barre de scroll du panneau Sound : sillon skeuo et poignee permanente (build 20260909-115534)

**Branche:** `main` - **Build:** `20260909-115534`
**Validation:** `cargo test` 365+1+222 OK, warning-clean, `build.ps1 -Install` OK. **Validé dans Studio One (2026-09-09).**

- **La cause n'etait pas le style, c'etait l'absence de barre.** Le `ScrollStyle` par defaut d'egui est `floating` avec `dormant_handle_opacity: 0.0` : au repos la poignee ET son fond ne sont **pas dessines du tout**, il n'y avait rien a voir avant que le pointeur entre dans le panneau.
- **Poignee permanente** : les six opacites sont epinglees a 1.0. Elles auraient sinon rendu la poignee *plus terne* au survol qu'au repos (`active_handle_opacity` 0,6 contre 1,0) ; le retour de survol passe donc par la **couleur**, comme partout ailleurs dans le panneau.
- **Look skeuo** : le fond de la barre prend la teinte du sillon d'un slider et la poignee devient une pilule (rayon 5) FAINT au repos, INK3 au survol, INK2 pendant le glisser - assez discrete pour ne pas concurrencer les remplissages bleus des valeurs. Largeur 10 px, longueur minimale 24 px.
- **Une seule definition du sillon** : `skeuo::groove_fill()` / `groove_border()` remplacent les `rgb(16,17,21)` et `rgb(9,9,12)` en dur de `slider_track`, et la barre de scroll les reutilise - les deux ne peuvent plus deriver.
- **Aucun decalage de mise en page** : la barre reste `floating` avec `floating_allocated_width: 0.0`, donc la rendre permanente ne coute pas un pixel aux rangees (l'encart de contenu existant les tenait deja a l'ecart). Regle des zones stables respectee.
- **Le style est confine a la barre** : le style d'origine est restaure en tete de la fermeture de contenu, pour qu'aucune rangee n'herite des remplissages de la poignee.
- Maquettes PNG comparees avant de coder (`ui-preview/`, ignore par git) : FAINT retenu contre INK3 (trop present) et LINE2 (illisible).

### Limite de ce build

**Le relief skeuo n'est pas obtenu.** egui ne peint la barre qu'en deux rectangles plats : regler ses couleurs et sa largeur rend la barre visible mais pas creusee. Retour utilisateur : « juste plus epaisse mais completement unie » - d'autant qu'un contenu depassant a peine la hauteur visible donne une poignee longue de ~85 %, qui se lit comme un bloc uni. Obtenir le creux et le degrade demande de peindre la barre soi-meme dans `skeuo.rs` et de reprendre a egui le clic-glisser (molette, clic-page et contenu de hauteur variable inclus). **Mis en attente sur decision utilisateur.**

## 2026-09-09 - [189] Slider Saturation Amount : loi de reponse progressive (build 20260909-114633)

**Branche:** `main` - **Build:** `20260909-114633`
**Validation:** `cargo test` 365+1+222 OK (3 nouveaux tests), warning-clean, `build.ps1 -Install` OK. **Validé dans Studio One (2026-09-09).**

- **Mesure d'abord.** Le DSP mappe l'amount sur la drive en `1 + amount^2 * 19`, mais ce que l'oreille suit c'est le taux de distorsion, et lui s'aplatit vite. Sonde temporaire sur un sinus 110 Hz d'amplitude 0,8, fenetre alignee sur un nombre entier de cycles : en SoftClip la course lineaire atteignait **15 % de THD au quart**, **32 % a la moitie** et seulement **43 % au bout** - presque tout se jouait dans la premiere moitie, et la seconde ne servait a rien. Meme forme concave sur Valve, Transistor, Tape et HardClip.
- **Loi de reponse `exposant 1,5`** sur les 24 definitions de Saturation Amount, ce qui donne 7 % / 23 % / 38 % aux memes points : l'ecart moyen a une rampe reguliere passe de 7,3 a 3,4 points. Les exposants 2,0 et 2,5, essayes aussi, sur-corrigent (4,9 et 6,9).
- **La loi vit dans le registre**, pas dans une devinette sur le libelle : nouveau champ `SpecialParamDef::curve` (1.0 par defaut), helper `sp_curved`, constante `SAT_AMOUNT_CURVE` documentee avec les mesures. La rangee des speciaux du panneau passe `def.curve`.
- **Cote widget**, la loi vit dans `TrackStyle` a cote du pas de quantification deja present : `with_curve()`, plus `normalize_value_curved` / `denormalize_value_curved` et le drag fin qui reste **uniforme en course** quelle que soit la loi. Aucun site d'appel existant modifie, `curve = 1.0` est bit-identique a l'ancien mapping (test dedie).
- **Aucun son ne change** : la loi ne fait que deplacer la poignee sur la piste, elle ne touche ni la valeur stockee ni le DSP. Le test d'aller-retour le verifie sur trois exposants.

### Constate au passage, pas corrige

La compensation automatique de gain est calibree sur une **entree de reference unique (0,5)** alors qu'un coup de batterie culmine vers 0,8-1,0, ce qui rend le niveau incoherent d'un type a l'autre quand on monte l'amount : sur le meme sinus 0,8, **SoftClip perd** (pic 0,80 -> 0,50, et -0,58 dB des le premier cran au-dessus de zero, car la compensation saute de 1,0 a 1,08) tandis que **Valve gagne** (pic 0,80 -> 1,36, +3,5 dB de RMS sur la course, donc au-dela de 1,0). HardClip a une zone morte jusqu'a ~0,15 puis un saut de -0,69 dB. C'est probablement ce qui contribue le plus a la sensation de saut ; toucher a la compensation change le son de 12 voix, donc c'est une decision a prendre separement.

## 2026-09-09 - [188] HiHat : decay plafonne a 1,5 s (build 20260909-113627)

**Branche:** `main` - **Build:** `20260909-113627`
**Validation:** `cargo test` 362+1+222 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-09-09).**

- Le decay du HiHat passe de **5 s a 1,5 s** dans `HIHAT_STD`, comme le clap en [181] : au-dela la queue est inaudible et la course du slider ne servait a rien. Toute la resolution du slider se retrouve sur la plage utile.
- **L'OpenHiHat garde ses 5 s**, deliberement : une queue longue est precisement ce qui fait un charley OUVERT. `HIHAT_STD` n'etant partage par aucune autre voix (contrairement a `NO_FREQ_STD` que le Cymbal partage avec le clap), aucun dommage collatoral - le test le verifie dans les deux sens.
- Pas de clamp DSP a aligner : l'enveloppe d'ampli du HiHat prend le temps qu'on lui donne, seul `filter_env_decay` avait un plancher.
- Les hats AC606 (HH6/OH6) ne sont pas concernes : ils plafonnent deja a 2 s dans `AC_TONE_STD`.
- **Sessions existantes** : une valeur stockee au-dela de 1,5 s reste jouee telle quelle jusqu'a ce que le slider soit touche - meme comportement que le plafonnement du clap en [181].

## 2026-09-09 - [206] One Shot actif par defaut sur les trois samplers (build 20260909-094534)

**Branche:** `main` - **Build:** `20260909-094534` (porte aussi [205], dont l'install de `20260909-092632` avait ete refaite)
**Validation:** `cargo test` 362+1+222 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-09-09).**

- `One Shot` passe a **1.0 par defaut** sur BD6smp, SD6smp et CH6smp : le sample joue jusqu'a sa fin au lieu d'etre coupe par l'enveloppe d'ampli.
- Change aux **deux** endroits, sinon le defaut du registre et celui du DSP se contredisent : `sp_discrete("<v>_one_shot", ...)` dans `instrument_registry.rs` (ce que lisent une nouvelle lane et le bouton **Default**) et `special[2]` dans `VoiceSettings::{bd606, sd606, ch606}()` (avec quoi la voix est construite).
- **Les sessions et presets existants gardent leur valeur** : les speciaux sont stockes par slot dans `sound-settings-v2`, donc seules les lanes nouvellement creees et le bouton Default prennent le nouveau defaut. Les voix **AC606** ne sont pas concernees (synthese, pas de sample, pas de parametre One Shot).
- Un test de `bd606` prenait les defauts comme reference *gated* ; il demande desormais `special[2] = 0.0` explicitement, sinon ses deux branches devenaient identiques et il ne verifiait plus rien.

## 2026-09-09 - [205] Une lane qui recoit un instrument est vide sur les 16 patterns (build 20260909-092632)

**Branche:** `main` - **Build:** `20260909-092632`
**Validation:** `cargo test` 362+1+222 OK (4 nouveaux tests), `build.ps1 -Install` OK. **Validé dans Studio One (2026-09-09).**

- **Le bug.** Supprimer une lane ne retire que son drapeau `active` (`ui/grid.rs`, `deactivate_slot`) : ses pas, fusions, p-locks sound et p-locks sequenceur restent en place. Ils sont invisibles et muets tant que le slot est vide, puis **ressuscitent des qu'un nouvel instrument est pose la** - `activate_slot` ne remettait a zero que les *reglages de son*, aucune donnee musicale.
- **Le bug, deuxieme moitie.** Chaque `PatternSlot` de la bank stocke les masques de pas des 64 pas et les blobs de p-locks/fusions **pour les 14 lanes**. Aucune operation de lane ne les touchait : ni l'activation, ni la suppression, ni le deplacement. Le fantome apparaissait donc ou non selon que le pattern rappele avait ete enregistre avant ou apres la chirurgie de lane - d'ou le cote intermittent.
- **Adressage par lane dans la bank** (`pattern_bank.rs`) : tout ce que `capture` ecrit est positionnel et emet les 14 lanes a pas fixe, donc une lane s'efface ou se deplace sans deserialiser le reste. `LaneRegions` decrit un blob comme des regions `(offset de la lane 0, octets par lane)`, et les trois sondes de disposition (`plock_lane_regions`, `seq_plock_lane_regions`, `fusion_lane_regions`) reconnaissent **les memes variantes de longueur que le chemin de restauration** - dont le blob p-lock legacy a 18 champs, edite avec SA foulee et non celle de 46. De la : `PatternSlot::clear_lane` / `permute_lanes`, `PatternBank::clear_lane_everywhere` / `permute_lanes_everywhere`, et sur `PersistentPatternBank` les deux entrees qui prennent le verrou et rafraichissent le snapshot.
- **Poser un instrument sur une lane vide nettoie la lane partout** : pattern vivant (pas, fusions, p-locks sound et sequenceur) **et** les 16 patterns sauvegardes. Vaut pour le selecteur `+N` (`activate_slot`) et pour un **Paste Lane sur une lane vide** - meme geste, meme garantie : le presse-papier remplit le pattern affiche, les 15 autres sont vierges.
- **La suppression reste non destructive** (choix explicite) : un `Delete Lane` de trop ne detruit rien, les donnees dorment jusqu'a ce qu'un nouvel instrument prenne la place. Corollaire : les fantomes deja presents sur une lane **deja occupee** ne sont pas nettoyes retroactivement - supprimer la lane puis reposer l'instrument le fait.
- **Le deplacement de lane permute aussi les 16 patterns.** `apply_lane_reorder_move` permutait meticuleusement tout l'etat vivant et laissait les patterns sauvegardes sur l'ancienne affectation : rappeler un pattern apres un glisser remettait les pas de chaque instrument sur la lane qu'il occupait avant. Meme trou, meme correctif.
- **Non touche, volontairement** : `Clear Grid` d'une lane (il vide la lane du pattern **courant**, pas des 16) et l'application d'un kit/preset de layout (re-instrumenter des patterns existants est un acte creatif legitime).
- Tests : effacement d'une lane dans un pattern sauvegarde avec voisines intactes ; permutation qui suit le glisser ; **blob p-lock de longueur legacy non corrompu** ; slot vide et lane hors bornes sans panique.

## 2026-09-02 — [200f] Le chargeur de presets d'instrument rejoint le bandeau d'actions (build 20260902-155102)

**Branche:** `main` · **Build:** `20260902-155102`
**Validation:** `cargo test` 356+1+221 OK. **Validé dans Studio One (2026-09-09).**

- La section **Preset** (dropdown « Load preset… » des presets d'instrument de la lane) quitte l'onglet Track pour la **section droite du bandeau d'actions** de l'onglet Sound, avec Store/Restore/Default : `[Step|Start/End] … [Preset▾] [Store] [Restore] [Default]`. L'onglet Track perd cette section (Instrument / Routing / MIDI restent).

## 2026-09-02 — [200e] Step/Start/End rejoint le bandeau d'actions (build 20260902-145616)

**Branche:** `main` · **Build:** `20260902-145616`
**Validation:** `cargo test` 356+1+221 OK. **Validé dans Studio One (2026-09-09).**

- Le segmenté **Step / Start / End** quitte l'en-tête (où il était dessiné même dans l'onglet Track, qui ne le concerne pas) pour le **bandeau d'actions** de l'onglet Sound, en deux sections propres : à gauche le choix de portée (Step/Start/End), à droite les actions (Store/Restore/Default). Toujours dessiné, grisé quand la cellule n'est pas morphable (règle zones stables). L'en-tête ne garde que le badge de scope.

## 2026-09-02 — [200d] Default/Store/Restore déménagent dans un bandeau d'actions (build 20260902-143620)

**Branche:** `main` · **Build:** `20260902-143620`
**Validation:** `cargo test` 356+1+221 OK. **Validé dans Studio One (2026-09-09).**

- Retour utilisateur (capture) : dans l'en-tête, les trois boutons surchargaient la zone (titre « Lane Editor » tronqué, nom de lane poussé dehors). Nouvelle disposition : l'en-tête redevient comme avant ([184]) ; **Default / Store / Restore** vivent dans un **bandeau d'actions fin aligné à droite, juste sous les onglets Sound/Track** — toujours visible dans l'onglet Sound, jamais scrollé, absent de l'onglet Track.

## 2026-09-02 — [200c] Default devient scope-aware (build 20260902-141717)

**Branche:** `main` · **Build:** `20260902-141717`
**Validation:** `cargo test` 356+1+221 OK, warning-clean. **Validé dans Studio One (2026-09-09).**

- Retour utilisateur : sur un pas avec p-lock sound, Default ne ramenait pas les valeurs verrouillées aux défauts. Le bouton suit désormais **la portée éditée** :
  - **Lane** : reset de la lane aux défauts d'usine (comportement [200], inchangé) ;
  - **P-lock (Step)** : les champs **verrouillés** du pas reviennent aux défauts d'usine de l'instrument — **masque intact** (écriture directe dans le magasin de valeurs, jamais via `PlockSource::set` qui verrouillerait chaque champ écrit et transformerait un p-lock Link en snapshot complet) ;
  - **Morph** : les **cibles existantes** du groupe reviennent aux défauts (aucune nouvelle cible créée).
- L'infobulle du bouton décrit l'action selon la portée.

## 2026-09-02 — [200b] « Sure? » du bouton Default se désarme (build 20260902-124917)

**Branche:** `main` · **Build:** `20260902-124917`
**Validation:** `cargo test` 356+1+221 OK. **Validé dans Studio One (2026-09-09).**

- Retour utilisateur : une fois armé, « Sure? » ne proposait aucune issue. Désormais il se **désarme après 3 s** (avec repaint programmé, le libellé revient seul à « Default ») **ou dès un clic ailleurs** dans l'interface. L'infobulle documente les deux sorties.

## 2026-09-01 — [199] [200] [201] Quick wins Lane Editor (build 20260901-203831)

**Branche:** `main` · **Build:** `20260901-203831`
**Validation:** `cargo test` 356+1+221 OK, warning-clean. **Validé dans Studio One (2026-09-09).**

- **[199] BUG** : créer un **seq plock** laissait le panneau Sound sur l'onglet **Step** (p-lock son). Cause : la sélection de cellule posée par un clic droit en mode Sound persistait quand on passait en mode Sequencer sur la même lane. Fix : un clic droit en mode Sequencer **efface explicitement** la sélection sound.
- **[200] Bouton Default** dans l'en-tête du Lane Editor : remet les paramètres de la lane aux **défauts d'usine de l'instrument** (`reset_slot_to_defaults`). Deux clics (« Sure? », comme Clear All). Les p-locks des pas sont conservés.
- **[201] Boutons Store / Restore** : snapshot A/B rapide des paramètres (settings + algo) **par lane** pendant l'édition — non persisté, aide à l'édition. Restore grisé tant que rien n'est stocké pour la lane.
- Les trois boutons sont **accessibles dans tous les scopes** (Lane / P-Lock / Morph) — retour utilisateur : grisés en scope p-lock à la première itération, corrigé.

## 2026-08-27 — [198b] BD6(AC) : Decay Curve accentuée (build 20260827-163841)

**Branche:** `main` · **Build:** `20260827-163841`
**Validation:** `cargo test` 356+1+221 OK (golden intact). **Validé dans Studio One (2026-09-09).**

- Curve portée à **γ = 2^(3c)** (était 2^(2c)) : +1 = concave ×8 (très punchy), −1 = convexe ×0,125 (queue très étirée). Mesures à decay 0,05 s : +1 → queue 58 ms, 0 → 241 ms, −1 → 694 ms.
- Rappel sémantique (convention identique aux autres voix) : **Decay = la durée** de la chute, **Decay Curve = sa forme** — +1 creuse le début (queue perçue plus courte), −1 garde l'énergie puis plonge (queue perçue plus longue). À decay très court, c'est le −1 qui donne une longue queue, pas le +1.

## 2026-08-27 — [198] BD6(AC) : Decay Curve bipolaire + Decay plafonné à 1 s (build 20260827-162051)

**Branche:** `main` · **Build:** `20260827-162051`
**Validation:** `cargo test` 356+1+221 OK (+1 test), warning-clean. **Validé dans Studio One (2026-09-09).**

- **Decay Curve** sur BD6(AC) (slider bipolaire −1..1, famille Env, comme les autres voix) : exponent γ = 2^(2c) sur l'enveloppe d'ampli du corps — **+1 = concave ×4** (plus punchy), **−1 = convexe ×0,25** (queue qui paraît plus longue). Défaut 0 = exponentielle fittée, golden bit-exact préservé (shortcut ×1).
- **Decay plafonné à 1 s** (était 2 s) dans `AC_BD_STD` — au-delà le moteur saturait déjà sa queue.

## 2026-08-27 — [197] Kits d'usine AC 4 / AC 12 (build 20260827-160721)

**Branche:** `main` · **Build:** `20260827-160721`
**Validation:** `cargo test` 355+1+220 OK (+1 test), warning-clean. **Validé dans Studio One (2026-09-09).**

- **Deux nouveaux kits d'usine dans le preset browser (onglet Grid)** : **AC 4** (BD6(AC)/SD6(AC)/HH6(AC)/TM6(AC)) et **AC 12** (les 6 voix AC + Ride, Cymbal, Snare606, 808 pour ce que le set AC ne couvre pas). Les kits classiques 4/12 Lanes sont inchangés.
- Les hats AC (**HH6(AC)/OH6(AC)**) héritent du **choke group 1** dans les kits qui en contiennent (`from_kinds` élargi), comme HH/OH classiques.

## 2026-08-27 — [196d] BD6(AC) : Tone descend à 80 Hz (build 20260827-144429)

**Branche:** `main` · **Build:** `20260827-144429`
**Validation:** `cargo test` 354+1+219 OK (golden bit-exact préservé). **Validé dans Studio One (2026-09-09).**

- **Pourquoi Tone était inaudible** : il règle le passe-bas du **corps** du kick (la sinus qui balaye ~120→53 Hz) entre 620 et 1200 Hz — or le corps est une sinus quasi pure très en dessous, donc le filtre ne changeait rien. **Tone est un filtre de corps, pas un EQ général.**
- **La plage descend désormais à 80 Hz** sous le point fitté (0,34 → 817 Hz préservé bit-exact) : en dessous de ~400 Hz le filtre mange le haut de l'attaque (kick plus sourd/subbie), et au-delà de 1 il ouvre toujours jusqu'à 6 kHz.

## 2026-08-27 — [196c] BD6(AC) : retrait d'Attack et Punch Decay (build 20260827-142925)

**Branche:** `main` · **Build:** `20260827-142925`
**Validation:** `cargo test` 354+1+219 OK (golden bit-exact préservé). **Validé dans Studio One (2026-09-09).**

Retours utilisateur sur [196b] :

- **Attack retiré** de BD6(AC) — quasi inaudible sur ce moteur ; l'attaque du moteur est figée à la valeur fittée effective (0,2). Le slider standard disparaît du panneau (table `AC_BD_STD` allégée).
- **Punch Decay retiré** — idem ; le decay de l'impulsion est figé au fitté (2,4 ms). L'indice `special[6]` reste inerte (aucune renumérotation, sessions intactes).
- **Punch conservé** (0–2, boost [196b]) : c'est le « thud » initial — l'impulsion dérivée de la pente du corps qui fait percer le kick dans un mix ; au-delà de 1 il gonfle aussi le corps.

Contient aussi [196b] (plages étendues + Sat Mix/Gain au milieu), jamais installé : ce build est la première mise en ligne de ces deux lots.

## 2026-08-27 — [196b] Retours AC606 : plages BD6 étendues + défauts saturation (build 20260827-140855)

**Branche:** `main` · **Build:** `20260827-140855`
**Validation:** `cargo test` 354+1+219 OK (golden bit-exact préservé — les défauts ne changent pas). **Validé dans Studio One (2026-09-09).**

Retours utilisateur sur [196], tous traités côté BD6(AC) — les plages étaient câblées mais trop timides pour être audibles ; tout dépasse désormais le fitté **sans toucher au son par défaut** (coude au-delà de la valeur fittée, golden intact) :

- **Punch** : slider étendu à 0–2 (le niveau d'impulsion double par rapport à l'ancien max, et nourrit aussi le corps via `thud_shape`).
- **Punch Decay** : 0–2, avec extension au-delà de 1.0 → jusqu'à ~39 ms (était 8,8 ms max).
- **Drive** : 0–2, avec coude de boost quadratique au-delà du fitté 0,18 → ~8× de drive à fond (était ~1,36×, quasi inaudible).
- **Tone** : 0–2, le filtre du corps ouvre au-delà de 1200 Hz jusqu'à 6 kHz (620–1200 Hz en dessous, comme avant).
- **Sweep** : 0–2 → étendue ×0 à ×4 (était ×2 max).
- **Attack** : effet étendu au-delà de 10 ms (transient ×2 max — c'est le niveau/la durée du « tick » initial + un peu de corps, désormais clairement audible à forte valeur).
- **Tous les (AC)** : **Saturation Mix** et **Saturation Output Gain** au milieu par défaut (0,5 et 1,25) au lieu de 1,0 / 1,0.

## 2026-08-27 — [196] Exploration des paramètres AC606 + saturation partagée (build 20260827-094426)

**Branche:** `main` · **Build:** `20260827-094426`
**Validation:** `cargo test` 354+1+219 OK (+14 tests), warning-clean. **Validé dans Studio One (2026-09-09).**

Sur décision utilisateur, la fidélité hardware n'est plus une contrainte : les moteurs AC606 deviennent des points de départ explorables. **~30 nouveaux paramètres**, tous en `special[]` (donc **p-lockables et morphables** gratuitement), défauts = valeurs fittées (le son d'origine est au centre des plages) :

- **BD6(AC)** : Sweep (étendue du balayage de pitch), Bend (vitesse de chute), Click + **Click Tone** (400–4000 Hz), Punch + Punch Decay, Tone (LPF du corps), Drive. Attack/Click/Punch sont désormais **découplés** (avant, un seul « transient » pilotait les trois).
- **SD6(AC)** : Wire Color (déplace les bandes 3k/4,6k sans toucher le shell), Shell Bend, Impact (le dip négatif initial), Ring (le ring grave qui colle wires et shell).
- **HH6(AC) / OH6(AC)** : Metal (partiels vs bruit), Click, Bell (accent des 3 lignes), Wobble (×4 : anti-accord → chorus métallique), **Spread** (désaccord déterministe des 47 partiels — nouveau, pas dans l'original), **Brightness** (tilt spectral — nouveau).
- **CL6(AC)** : Noise (corps sec → air/densité), Spread (écartement des 4 bursts ×0,25–3), Tail (queue diffuse), Air (couche HP forcée même à Noise ≤ 0,5).
- **TM6(AC)** : **Model** Auto/Low/High (discret — forcer la spec grave pitchée haut, etc.), Strike, Snap, Glide (au-delà de ×1, le tom aigu gagne la chute du grave), Modes (résonances hautes), Tail Noise.
- **Pack saturation partagé** (Type/Amount/Mix/Output Gain, indices 10–13) sur les six kinds — les voix AC contournait la chaîne commune, elles la traversent désormais comme les autres (sans Pre-Filter : pas d'étage de filtre à contourner).

**Filet de sécurité** : test **golden bit-exact** (hash FNV-1a du rendu à défauts) — les 5 kinds SD/HH/OH/CL/TM reproduisent le son d'origine au bit près ; BD6 a été recapturé après mesure : le découplage Click/Punch déplaçait ces niveaux d'exactement **1 ulp** (inaudible, sémantique voulue). Tests de câblage : chaque param aux deux extrêmes doit rendre différemment (avec sensibilisateurs pour Punch Decay et Modes) + sortie finie ; saturation vérifiée audible.

## 2026-08-26 — [195] Six nouveaux instruments AC606 (voix analogcode portées) (build 20260826-224038)

**Branche:** `main` · **Build:** `20260826-224038` (première itération `20260826-202054` en algos, réorientée en instruments dédiés sur demande utilisateur)
**Validation:** `cargo test` 351+1+216 OK (+11 tests), warning-clean. **Validé dans Studio One (2026-09-09).**

- **Portage Rust fidèle du repo [`analogcode/606-Inspired-Synth-Drums`](https://github.com/analogcode/606-Inspired-Synth-Drums) (MIT, © 2026 Matthew Fecher)** dans `src/synthesis/ac606/` : constantes fittées par mesure sur hardware, flux RNG et topologies de filtres inchangés. Les 768 taps FIR du Clap sont **vérifiés bit-à-bit** contre la source C++ par script (20 typos de transcription initiale attrapées et corrigées avant intégration).
- **Six nouveaux instruments** (kinds 16-21 / voix 18-23), visibles dans le sélecteur Type, le popup Add Module et le menu clic-droit « Instrument », groupés par catégorie :
  - **BD6(AC)** (BD) — « 608 XL » : sweep sine + click + impulsion dérivée de la pente du corps ; Attack → transient
  - **SD6(AC)** (SD) — shell accordé 201 Hz + wires en 2 biquads bandpass fittés sur le sample hardware ; spécial **Snap** → Snappy
  - **HH6(AC)** / **OH6(AC)** (HH) — même source **47 partiels** extraits par FFT (3 lignes « bell » accentuées, wobble corrélé anti-accord), specs closed/open
  - **CL6(AC)** (SD) — RD-6 : 4 bursts timés + 4 FIR de 192 taps sur un flux de bruit unique + upsampler sinc 128 taps quand le pitch bouge
  - **TM6(AC)** (PERC) — spec **Low** sous 166 Hz, **High** au-dessus (glide de 40 Hz sur le grave, ring à 135,6 Hz sur l'aigu)
- **Un seul wrapper `AcVoice`** (`src/synthesis/ac_voice.rs`) + settings partagés `AcVoiceSettings`. Mapping : Frequency/Tone → pitch ratio (1,0 = son fitté au défaut), Decay → longueur normalisée sur la durée fittée, Analog → jitter de pitch par coup, Volume → niveau final. Chaque hit est unique par construction (bruit/phases libres), comme le hardware.
- **Anti-click [179] étendu** : chaque moteur repart d'un état fitté neuf à chaque coup, la queue coupée est fondue par `RetrigDeclick` (3 ms) — test de retrigger auto-calibré sur les six kinds.
- **Crédit** : section **About** dans le popup Settings (remerciement + lien), notice MIT complète dans `src/synthesis/ac606/LICENSE-MIT.txt`, en-têtes des fichiers portés. La table sinc du Clap est pré-calculée dans `initialize_with_layout` (pas d'allocation sur le thread audio).
- **Générateur** : chaque kind AC emprunte le rôle de son équivalent acoustique (Kick/Snare/HiHat/OpenHiHat/Clap/Tom). **Sessions existantes** : inchangées (variants ajoutés à la fin des enums sérialisés, réglages par slot).
- Note : une première itération branchait ces moteurs en **algos** des voix existantes ; réorientée en instruments dédiés sur demande utilisateur.

## 2026-08-22 — [184] phase 4 : troisième onglet Step et suppression des menus redondants (build 20260822-115906)

**Branche:** `main` · **Build:** `20260822-115906`, complété par `20260823-091039`
**Validation:** `cargo test` 340+1+205 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-08-23)**, 10/10 puis 5/5.

- **Le sélecteur d'une cellule fusionnée devient `Step | Start | End`.** `Step` édite le p-lock de la cellule de départ — un override plat, sans rampe. La phase 3 n'exposait que les deux extrémités, ce qui laissait le menu contextuel seul capable de ça ; supprimer ses rangées aurait donc supprimé la possibilité. `Step` réutilise exactement la portée p-lock existante : aucun changement de modèle de données.
- **`Step` est l'onglet par défaut**, parce que créer une rampe doit être un acte délibéré et non ce qui arrive la première fois qu'on touche un slider.
- **~930 lignes supprimées**, pour 161 ajoutées :
  - `draw_fusion_morph_menu` et `draw_morph_target_action_buttons` (372 lignes) : le morphing s'édite dans le panneau, avec ses graphes et ses marqueurs d'override. La rangée « Morphing (…) » du menu de fusion pointe désormais le panneau sur la fusion et se ferme.
  - **Les rangées de valeurs de `draw_plock_menu` (274 lignes)** : deux implémentations du même écran sur le même stockage. Le menu garde ce que lui seul sait faire — créer, copier, coller, vider un p-lock **entier** — plus une rangée « Edit In Panel ». `ui/plock.rs` passe de **1257 à 603 lignes**.
  - `fusion_morph_state` et `current_field_value_for_fusion` (`grid.rs`, 54 lignes), les quatre formateurs de valeur du menu p-lock (`ui/fmt.rs`, 66 lignes), `plock_menu_enum_row` (`ui/menus.rs`), la méthode `logarithmic` de `LocalParamSlider`, et le champ `PlockPopup::morph_menu`.
- Ce qui **reste** dans le menu contextuel, comme prévu : les p-locks séquenceur (probabilité, stutter, nudge, condition, solo), le menu de groupe de fusion (Morphing / Edit Fusion Steps / Delete Fusion) et les actions structurelles du p-lock sound.
- `LocalParamSlider` est conservé : les rangées de p-lock séquenceur l'utilisent toujours.
- **Le menu se ferme après Copy et après Paste** (build `20260823-091039`, les trois emplacements dont celui du bloc de création) : le geste est terminé, le suivant se fait sur une autre cellule. **Clear** garde le menu ouvert sur ses options de création, puisque le pas n'a plus de p-lock et qu'enchaîner sur « Link to Global » ou « Snapshot » est le cas courant.

## 2026-08-21 — [184] phase 3 : le morphing s'édite dans le panneau (build 20260821-171425)

**Branche:** `main` · **Build:** `20260821-171425`, affiné jusqu'à `20260822-112337`
**Validation:** `cargo test` 339+1+205 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-08-22)** après six passes d'ajustement issues des retours.

- **Un segmenté `Start | End`** apparaît dans l'en-tête du panneau quand la cellule sélectionnée appartient à un groupe fusionné, et les rangées éditent l'extrémité choisie. Il est **grisé** hors de ce cas — y compris sur un groupe à **une seule impulsion**, où le moteur désactive de toute façon l'interpolation (`pulse_count == 1`) : proposer le réglage aurait promis quelque chose d'inexistant. Toujours dessiné, donc l'en-tête ne change jamais de largeur.
- **Une seule source gère les deux extrémités.** Le format n'en stocke qu'une (`MorphTarget::end_value`) ; l'autre est résolue au déclenchement depuis la cellule de départ, et `MorphDirection` dit de quel côté se trouve la valeur stockée. L'onglet demandé et le magasin à écrire composent donc en **table 2×2** : `End`+`Target` et `Start`+`Source` écrivent le groupe, `Start`+`Target` et `End`+`Source` écrivent le p-lock de la cellule de départ. C'est **un seul booléen**, et surtout ça fait fonctionner les cibles héritées `Source` **sans aucune migration de données** — raison pour laquelle le drapeau de direction est conservé.
- Un champ pas encore ciblé rapporte `Target` par défaut : un premier glissement dans l'onglet `End` **crée** donc le morph, tandis que le même geste dans `Start` écrit le p-lock. La rangée part de la valeur que le son a déjà, pas de zéro.
- **Le plafond de 4 cibles est désormais visible** : `FusedGroup::set_morph_target` renvoie un booléen au lieu de laisser tomber la cinquième en silence. Une rangée qui n'est pas encore cible est grisée avec son motif quand le groupe est plein, et un avis s'affiche en tête du corps du panneau — **dans** la zone de défilement, donc sans décaler l'en-tête ni les onglets.
- **Un paramètre discret ne peut pas être morphé** et le dit : interpoler linéairement un `Saturation Type` entre Valve et Tape ne produit pas un son mais un arrondi. Nouveau prédicat `param_is_morphable` dans le registre, qui lit le `continuous` déjà déclaré.
- **Écritures groupées** : la copie de travail du groupe est chargée une fois et publiée une fois par `commit()`, et seulement si quelque chose a changé. L'ancien menu de morph republiait **tout le tableau de fusions de la lane à chaque frame de slider**. `commit` refuse aussi de publier un groupe invalide — `store_fusions` supprime silencieusement les groupes qui échouent à `is_valid()`, ce par quoi une fusion pouvait disparaître en pleine édition.
- **Correctif de conception, build `20260821-172758`** : régler une extrémité **crée** désormais le morph, quel que soit l'onglet. Auparavant l'onglet `Start` écrivait le p-lock de la cellule de départ ; sans valeur d'arrivée, le pas jouait donc cette valeur **à plat sur toutes les impulsions** et rien ne morphait — exactement ce qui a été remonté. Désormais `End` stocke la valeur d'arrivée (direction `Target`, la rampe va du vivant vers le stocké) et `Start` stocke la valeur de départ (direction `Source`, du stocké vers le vivant) ; l'autre extrémité reste la valeur fusionnée de la cellule de départ, c'est-à-dire son p-lock s'il en a un, sinon le son de la lane. La table 2×2 continue de s'appliquer aux champs qui portent **déjà** une cible : là, l'extrémité vivante s'édite bien via le p-lock.
- **Deux retours d'usage, build `20260822-092224`** : cliquer le **nom d'une lane** rend désormais la main au son de la lane même quand la cellule sélectionnée appartient déjà à cette lane (le retour n'avait lieu qu'en changeant de lane, alors que cliquer un nom veut clairement dire « montre-moi cette lane ») ; et le marqueur d'override passe de 14 à **18 px**, allongé vers le bas, pour qu'il se lise comme appartenant à toute la rangée.
- **Ajustements issus de la validation** (builds `20260822-092224` → `112337`) :
  - **Grisage uniformisé.** `supports()` n'était consulté que par la chaîne des paramètres spéciaux : la saturation se grisait quand le groupe était plein, mais Filter, Decay ou Volume acceptaient le glissement et l'écriture était refusée en silence. Deux comportements pour une même situation. Les **sept** rangées écrites à la main passent maintenant par un helper unique, `row_scoped`, qui porte à la fois la gouttière et le grisage — « grisé avec son motif » devient une règle et non un cas particulier.
  - **Le bandeau redevient un état** : il s'affiche dès la 4ᵉ cible et non au refus d'une 5ᵉ, puisque désormais les rangées sont bloquées en amont et que l'utilisateur a besoin de savoir **pourquoi** le panneau paraît inerte. Fermable au clic, il se réarme dès que la situation change. Le mécanisme d'événement transitoire, devenu redondant, est retiré.
  - **Texte replié** : `painter.text` ne coupe pas les lignes, donc le message débordait du cadre. Il est mis en page avec une largeur maximale et c'est le bandeau qui s'adapte à sa hauteur, avec la place de l'indice de fermeture réservée.
  - Clic sur le **nom d'une lane** → retour au son de la lane même quand la cellule sélectionnée appartient déjà à cette lane. Marqueur d'override allongé de 14 à 18 px vers le bas.
  - **Sept littéraux de chaîne réparés** : leurs continuations de ligne avaient été avalées à l'écriture, laissant des paquets d'espaces au milieu du texte — dont cinq visibles à l'écran (le bandeau et les infobulles des badges `Lane`, `P-Lock`, `Morph`, plus celle du spécial réservé).
- 8 nouveaux tests, dont la table 2×2 vérifiée dans ses quatre cases, la création par chacun des deux onglets avec la bonne direction, le refus visible de la cinquième cible, et la préservation des **autres** groupes de la lane lors d'une publication.

## 2026-08-21 — [187] Le paramètre que l'Attack masquait redevient verrouillable par pas (build 20260821-160115)

**Branche:** `main` · **Build:** `20260821-160115`
**Validation:** `cargo test` 331+1+205 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-08-21)** : 8/8, sessions existantes et Pattern Bank inclus.

- **Le problème** : dans le format `plock-v1`, l'**Attack** a été ajoutée tardivement au champ **18**, or 18 = `SPECIAL_FIELD_START + 4`. Le spécial d'indice 4 partageait donc sa case et était purement ignoré, **sans que rien ne l'explique** : sa rangée disparaissait du menu. Chaque voix perdait un paramètre — Output Gain sur le Kick, Saturation Amount sur le Cymbal, Saturation Type sur BD808 et Perc1, Noise Type sur Buzz, Wet sur SDrex.
- **Le correctif, sans changer le format** : le spécial 4 est relogé sur le champ **45**, l'emplacement du spécial d'indice 31 — que **aucune voix ne déclare** (le plus haut indice utilisé sur les 18 instruments est 17). La longueur du blob ne change pas, donc **aucune nouvelle version de format**. L'indice 31 devient réservé.
- **Le champ 12 (`LEGACY_CLAP_ECHO_FIELD`) n'est pas touché**, et c'est délibéré : le Clap le lit encore en repli pour les vieux presets qui stockaient l'écho là. Le reloger là aurait été plus simple mais aurait cassé cette compatibilité — le champ 45, inutilisé, évite tout arbitrage. Un test pin ce comportement.
- **Le piège des anciens snapshots, traité** : un p-lock en mode Snapshot écrivait les 46 bits de masque, champ 45 compris, avec la valeur `0.0` (il stockait `special[31]`, que personne n'utilise). Relire ça comme le spécial 4 aurait mis l'Output Gain du Kick à zéro sur chaque pas snapshoté. Un snapshot pris depuis cette version marque désormais les champs **adressables** (tous sauf le champ 12 mort), ce qui donne une valeur de masque **différente** de l'ancienne — cette différence sert de marqueur de version. `sanitize_field_mask`, appliqué dans `set_raw` (le point de passage unique de tout masque venant de l'extérieur : état DAW, Pattern Bank, presets, presse-papiers, réordonnancement, déplacement de pas, collage), ne retire le bit 45 que pour cette **unique** valeur héritée — reproduisant exactement l'ancien comportement : le paramètre suit la lane.
- Les boucles de spéciaux de `get_settings` et `set_settings` passent par `ParamId::Special(i).plock_field()` au lieu de refaire l'arithmétique : le relogement n'existe qu'à un seul endroit.
- **Infobulle réparée** : le motif du grisage était accroché à un `ui.label("")`, or un label vide a une taille nulle — il était donc impossible à survoler. Il est maintenant porté par toute la surface de la rangée, via l'`InnerResponse` de `add_enabled_ui`.
- Tests : aller-retour du spécial relogé par `set_settings`/`get_settings` sans perturber l'Attack, repli legacy du Clap préservé, masque hérité qui ne revendique pas le champ 45, masque courant laissé intact, et la bijection de la disposition dont le seul trou reste le champ 12.

## 2026-08-21 — [184] phase 2c : les overrides se voient et se retirent par rangée (build 20260821-141959)

**Branche:** `main` · **Build:** `20260821-141959`
**Validation:** `cargo test` 327+1+203 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-08-21)** : 8/8 après réparation de l'infobulle du point 7 (voir [187]).

- **Chaque rangée du Lane Editor porte une gouttière de 14 px à gauche**, réservée dans **les deux** portées. Non conditionnelle par choix : si elle n'existait qu'en mode p-lock, sélectionner une cellule décalerait tous les sliders de 14 px — exactement ce qu'interdit la règle des zones stables.
- **En mode p-lock, une rangée surchargée affiche une barre d'accent** dans cette gouttière, **cliquable pour rendre le paramètre à la lane** (`ParamSource::clear`, qui retire le bit de masque du champ sans détruire le p-lock : les autres champs gardent leur override). La barre *est* l'affordance, ce qui garde le libellé en ASCII pur — un glyphe de type flèche de retour est précisément ce qui avait produit la mojibake de la tâche [164]. Infobulle : « Overrides the lane - click to follow the lane again ».
- Les **8 sites de rangée** sont couverts : Volume, Frequency (avec son switch Hz/Note), slider standard générique, case à cocher standard, Pitch Fine, la chaîne complète des paramètres spéciaux, le switch Stereo des samplers, les sliders de gate Buzz. Aucune signature de helper n'a changé : la gouttière est dessinée par l'appelant dans un `horizontal` externe à espacement nul, donc la rangée se décale exactement de 14 px et pas de 14 + l'espacement par défaut d'egui.
- **Un paramètre sans emplacement par pas est grisé AVEC son motif** au lieu d'être masqué — le spécial dont le champ entre en collision avec Attack, notamment. Aujourd'hui il disparaissait sans explication ; désormais il reste visible, inerte, et dit pourquoi au survol.
- **Collision d'identifiants egui évitée** : le panneau et la popup sont maintenant vivants en même temps et saluaient tous deux leurs `styled_select` avec `def.name`, ce qui leur donnait le même identifiant egui — deux menus déroulants partageant un état d'ouverture. Toutes les salaisons du panneau intègrent désormais `ParamSource::salt()`, qui distingue la portée (lane / pas N).
- `Support::is_editable` supprimé au profit de `reason()` : le panneau a besoin du **motif** et non d'un booléen, précisément pour qu'une rangée inerte puisse s'expliquer.
- Pitch Fine et les sliders de gate Buzz affichent maintenant leur unité (`ct`, `Hz`, `s`), ce que la phase [182] avait déclaré dans le registre mais que ces deux rangées, écrites à la main, ignoraient.

## 2026-08-21 — [184] phase 2 : éditer le p-lock d'un pas depuis l'onglet Sound (build 20260821-124406)

**Branche:** `main` · **Build:** `20260821-124406`, corrigé par `20260821-141012`
**Validation:** `cargo test` 327+1+203 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-08-21)** : 9 points sur 10, le 10ᵉ ayant révélé un verrou d'origine (voir la dernière puce).

- **Le clic droit sur une cellule pointe l'onglet Sound sur ce pas.** Le panneau édite alors le **p-lock du pas** au lieu du son global de la lane ; toutes ses rangées, y compris les six graphes, suivent. Le menu contextuel s'ouvre toujours et garde ses rangées : les deux éditeurs écrivent le même stockage, ce qui permet de les comparer sur la même valeur pendant cette phase.
- **Sélection persistante et sûre** : nouvel état `sound_edit_target` (`SelectedCell`), distinct de `PlockPopup` — la popup sert aussi de drapeau modal qui neutralise les clics de la grille, ce que la sélection ne doit **jamais** faire. Le pas est stocké **brut** et normalisé au démarrage d'une fusion à la lecture, donc supprimer une fusion restitue la cellule exacte.
- **Un résolveur pur et testé**, `resolve_edit_scope` : les règles vivent en un seul endroit au lieu d'être redérivées dans l'UI — aucune sélection → global ; sélection sur une autre lane → global mais **conservée** (revenir la restitue) ; Song mode ou grid Follow → global, conservée aussi (la création de p-lock y est déjà interdite) ; lane inactive ou pas au-delà de la longueur de lane → sélection abandonnée ; cellule couverte par une fusion → normalisée sur la cellule de départ, seule porteuse du son du groupe. **6 tests.**
- **Invalidations** aux endroits qui rapiéçaient déjà `plock_popup` : réordonnancement de lane (la sélection suit son slot), suppression de lane, Clear/Randomize d'une lane, changement de lane sélectionnée. Les `clear_all()` en masse n'ont **pas** besoin de crochet : la cellule existe toujours, elle n'a simplement plus d'override, et c'est exactement ce que renvoie le résolveur.
- **Repères visuels** : badge `P-Lock / Step N` aligné à droite dans l'en-tête, avec un bouton `x` qui rend la main à la lane, et un **filet d'accent sous les onglets** en mode p-lock — le badge seul est trop facile à manquer alors que la distinction « j'édite un pas » / « j'édite la lane » est critique. Aucun décalage de mise en page : le filet est peint sur la couture existante.
- **Le clic droit en mode Sequencer ne retargette pas le panneau** : ce geste concerne probabilité/stutter/nudge, il ne doit pas déplacer silencieusement l'édition du son.
- Sous le capot, la source de valeurs devient polymorphe (`Box<dyn ParamSource>`) : `GlobalSource` ou `PlockSource` selon la portée, le reste du panneau ignore la différence.
- **Verrou Follow levé** (build `20260821-141012`) : le clic droit était interdit en mode Follow **et** en mode Song depuis l'origine, au motif que la grille défile sous le curseur. La validation a montré que c'était surtout gênant. Follow est donc rouvert — la cellule sous le curseur **au moment du clic** est bien celle qu'on visait, la popup s'ancre à ce point, et le badge nomme le pas même quand sa page a défilé. **Song mode reste verrouillé** : là, c'est le pattern entier qui change sous vous, donc le pas pourrait appartenir à un autre pattern une seconde plus tard — un vrai danger, pas un inconfort visuel.
- **Pas encore fait, en phase 2c** : le marquage par rangée des paramètres surchargés et le ↺ de retour au global. Ça demande de traverser cinq helpers de rangée et huit sites d'appel, donc c'est livré à part plutôt que bâclé ici.

## 2026-08-21 — [186] CRASH de l'hôte au rechargement du plugin : classe de fenêtre fantôme (build 20260821-120829)

**Branche:** `main` · **Build:** `20260821-120829`
**Validation:** `cargo test` 321+1+203 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-08-21)** : le scénario de crash ne se reproduit plus et le pont clavier fonctionne toujours.

- **Symptôme, reproductible** : ouvrir une session contenant des plugins, la fermer, ouvrir une session vierge, ajouter Flash Drum → Studio One crashe.
- **Preuves** : trois minidumps, tous `0xC0000005` avec le paramètre `8` — une violation **DEP en exécution**, c'est-à-dire un saut vers un pointeur de fonction périmé. Aucun `panicked at` en mémoire, donc aucun panic Rust. L'adresse fautive tombait au même offset dans `arp2600vProcessor.dll`, DLL tierce **déchargée** dans deux des trois cas : elle avait simplement été chargée à l'emplacement libéré par l'ancienne Flash Drum.
- **Cause — notre patch clavier vendoré** (`vendor/nih-plug/nih_plug_egui/src/editor.rs`), trois défauts cumulés :
  1. la classe de fenêtre du pont clavier était enregistrée sous un **nom fixe** (`NihPlugEguiKbdMsg`) et avec le **`HINSTANCE` de l'EXE hôte**, alors que son `WndProc` vit dans notre DLL → l'inscription appartenait au processus et survivait au déchargement, pointeur pendouillant inclus ;
  2. **rien n'était nettoyé** : ni `UnregisterClassW`, ni `DestroyWindow` de la fenêtre de messages, ni restauration du `WndProc` d'origine de la fenêtre hôte (la sous-classe restait posée) ;
  3. au rechargement, `MSG_CLASS_ATOM` étant un `static` de la DLL, il repartait à 0 ; `RegisterClassW` échouait (nom déjà pris par le fantôme) et le code **traitait cet échec comme un succès** avant de créer une fenêtre sur la classe périmée. Le premier message dispatché sautait dans la mémoire libérée.
- **Correctif, trois volets** :
  - **identité par chargement** : la classe s'appelle désormais `NihPlugEguiKbdMsg_<base du module>` et est enregistrée avec le `HINSTANCE` de **notre** DLL (`GetModuleHandleExW` + `FROM_ADDRESS`). Réutiliser une inscription fantôme devient impossible par construction, même si un crash empêche le nettoyage ;
  - **`uninstall()`** appelé depuis `Drop for EguiEditorHandle`, **avant** la destruction de la fenêtre, dans l'ordre exigé par Windows : restaurer le `WndProc` de l'hôte (et retirer la propriété), détruire la fenêtre de messages, puis désenregistrer la classe ;
  - **comptage de références** (`INSTALL_COUNT`) : plusieurs instances partagent un même chargement de DLL, donc seule la fermeture de la **dernière** désenregistre la classe.
- L'échec de `RegisterClassW` n'est plus silencieusement toléré : avec un nom unique par chargement, un 0 signale un vrai problème.

## 2026-08-21 — [185] L'algo p-locké ne durait que quelques millisecondes (build 20260821-111726)

**Branche:** `main` · **Build:** `20260821-111726`
**Validation:** `cargo test` 321+1+203 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-08-21).**

- **Symptôme** (signalé sur le Kick) : changer l'Algo dans le p-lock d'un pas ne s'entendait pas.
- **Cause, et ce n'était pas la refonte [184]** : le stockage, la fusion (`get_settings`, masque bit 13) et l'application au déclenchement (`fire_voice_trigger` → `set_voice_settings`) étaient tous corrects. Mais le prologue de `process()` repoussait l'algo **global** sur chaque voix **à chaque buffer, sans condition** (`lib.rs:2566`) — soit toutes les ~10 ms. Le coup jouait donc son algo p-locké pendant les premières millisecondes, puis toute la queue repassait à l'algo de la lane. L'asymétrie était nette : tous les autres réglages globaux ne sont repoussés que lorsque `bump_version()` change ; l'algo était le seul à l'être inconditionnellement.
- **Correctif** : la propagation devient **conditionnelle au changement** (`last_algos: [u8; MAX_TRACKS]`), comme celle des autres réglages. Re-poussée forcée dans les deux cas où c'est nécessaire : restauration d'état (à côté de l'invalidation de `last_sound_settings_version`) et changement de kind d'un slot (la voix est recréée avec son algo par défaut).
- **Test** `pushing_the_lane_algo_mid_tail_is_audible_so_the_push_must_be_change_only` : il vérifie que la première moitié du rendu est **identique** au bit près et que la queue **diverge** quand on repousse l'algo en pleine queue. Autrement dit, il prouve que le bug était audible — sans quoi la régression serait invisible et pourrait revenir sans que rien ne casse.

## 2026-08-21 — [184] phase 1b : le panneau Sound lit et écrit par la couche de source (build 20260821-104025)

**Branche:** `main` · **Build:** `20260821-104025`
**Validation:** `cargo test` 320+1+203 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-08-21) : aucun changement de comportement**, ce qui est le critère de la phase.

- **Le panneau Sound passe désormais par `GlobalSource`** pour toutes ses lectures et écritures de rangée : les 13 locales viennent de `src.get(ParamId::Std(...))`, les écritures de `store_field`/`set_special`/`set_freq_mode` deviennent `src.set(...)`, et les arguments **spéciaux des six graphes** (sampler, filtre Buzz/SDrex, gate) passent aussi par la source. C'est ce dernier point qui fera suivre les graphes au p-lock en phase 2 : ils étaient structurellement incapables d'afficher autre chose que le global.
- **Approche chirurgicale assumée** : la structure du panneau, ses sections, ses cas particuliers (Pitch Fine, switch Stereo, gate Buzz) et sa chaîne de détection des paramètres discrets par sous-chaîne de libellé sont **inchangés**. La migration des widgets discrets vers le registre est reportée en phase 4, pour que chaque build validé ait un diff qu'on puisse relier à ce qu'on entend et voit.
- **`PanelAlgo`** implémente `AlgoSink` : il fait le pont entre l'`IntParam` du panneau et la couche, qui ne doit pas connaître `ParamSetter` (celui-ci exige un `GuiContext` vivant, ce qui rendrait la couche non testable headless).
- **Un seul `bump_version()` par frame** au lieu d'un par rangée éditée : `GlobalSource` groupe ses écritures et `commit()` les vide une fois. Les rangées de paramètres spéciaux bumpaient chacune la leur.
- **8ᵉ mapping dupliqué supprimé** : le bloc `default_value` à branche par voix (table `sound_settings_default` pour les samplers et SDrex, `VoiceSettings::default()` sinon, `def.default` pour les spéciaux) devient `src.inherited(id)`, via `instrument_registry::param_default`. Les défauts des rangées Volume et Frequency, qui utilisent un chemin différent, sont **volontairement laissés en place** : les aligner changerait la cible du double-clic sur SDrex et les samplers, ce qui violerait le critère « aucun changement » de la phase.
- Ce qui reste délibérément sur `inst` et non sur la source : la migration une-fois du pitch des samplers et l'export/import de presets des outils dev — ce sont des opérations sur le son **global** par nature, pas des rangées.

## 2026-08-21 — [184] phase 0 : une identité de paramètre, six mappings supprimés (build 20260821-101410)

**Branche:** `main` · **Build:** `20260821-101410`
**Validation:** `cargo test` 314+1+203 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-08-21) : aucun changement de comportement**, ce qui est le critère de cette phase de plomberie.

- **Nouveau module `src/param_id.rs`** — `ParamId` : l'identité canonique d'un paramètre de son (`Std(StandardField)` / `Algo` / `Special(i)` / `FreqMode`), indépendante du magasin qui détient sa valeur. Il **possède désormais la disposition des 46 champs** (`FIELD_COUNT`, `ALGO_FIELD`, `SPECIAL_FIELD_START`, `ATTACK_FIELD`…), que `plock.rs` réexporte — la dépendance va de l'identité vers le stockage, jamais l'inverse, ce qui permet au binaire `test_standalone` d'inclure le module sans embarquer la couche de persistance.
- **Le mapping `StandardField` était écrit à la main six fois** et devait rester synchronisé par discipline. Deux numérotations coexistent — le discriminant de l'enum (ordre du tuple `load()` et du blob `sound-settings-v2`, **Attack = 4**) et l'index de champ p-lock (ordre du blob `plock-v1`, **Attack = 18**) — et les confondre décale silencieusement toutes les valeurs. Trois copies sont supprimées dans cette phase : `lib.rs::read_morph_value`/`apply_morph_value` (thread audio), `ui/plock.rs::get_global_value`, et le `match field_index` interne de `ui/grid.rs::current_field_value_for_fusion`.
- **Deux paires d'accesseurs** remplacent ces listes : `VoiceSettings::get/set(ParamId)` (thread audio, sans allocation) et `InstrumentSettingsState::get/set(ParamId)` + `standard/set_standard(StandardField)`. L'algo est explicitement **non détenu** par les atomiques (c'est un `IntParam` nih-plug) : les accesseurs le signalent par `debug_assert` au lieu de mentir, et les appelants génériques le traitent à part.
- **`PlockState::clear_field`** ajouté (retirer UN override en gardant le p-lock actif) — l'affordance ↺ par rangée des phases suivantes.
- **`StandardField::ALL`** permet d'itérer les 13 champs sans les relister.
- **La collision Attack / spécial 4 est documentée à sa source** : Attack ayant atterri sur le champ 18 = `SPECIAL_FIELD_START + 4`, ce spécial partage son emplacement et n'est pas verrouillable (`ParamId::is_lockable`, avec `unlockable_reason()` pour l'infobulle de la rangée grisée à venir). **Correction possible sans changer le format** — l'indice spécial le plus élevé déclaré est 17, donc les emplacements 18..31 sont libres — mais un ancien snapshot a ses 46 bits de masque à 1, donc il faut un marqueur de version avant de faire confiance à cette valeur : tâche séparée.
- 4 nouveaux tests : la table des 46 champs figée avant la suppression des copies (donc équivalence prouvée), l'aller-retour `plock_field` ⇄ `from_plock_field`, la **bijection** de la disposition avec un seul trou documenté (le champ 12, ex-clap echo), et l'unicité du spécial non verrouillable.

## 2026-08-21 — [183] Filter LFO SDrex : modulation vers le haut depuis la base (build 20260821-091344)

**Branche:** `main` · **Build:** `20260821-091344`
**Validation:** `cargo test` 310+1+199 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-09-09).**

- **Symptôme** : en mode Filter LFO, avec Filter au minimum (20 Hz) et Depth à fond, on n'entendait plus rien — l'inverse de ce qu'une modulation à pleine profondeur devrait donner.
- **Cause** : le cutoff était modulé de façon **bipolaire et multiplicative autour** de la base (`filter × 2^(sin × depth × wet)`). À 20 Hz de base et 3 octaves, le balayage allait de 2,5 Hz à 160 Hz : **la moitié basse de chaque cycle était écrasée par le clamp à 20 Hz**, et le sommet de l'autre moitié (160 Hz) restait sous le corps de la voix (185 Hz), loin du metal (620/910 Hz). Mesuré : **−15,8 dB** sous la voix filtre ouvert, contre −35,4 dB filtre fermé sans modulation. La profondeur étant exprimée en octaves *relatives à la base*, aucun réglage de Depth ne pouvait ouvrir assez depuis une base basse.
- **Correctif** : le LFO ouvre le filtre **vers le haut depuis la base** (LFO unipolaire) — « Filter » devient le **plancher** du balayage au lieu d'en être le centre, donc plus aucun demi-cycle perdu contre le clamp. Passer unipolaire ne suffisait pas (−15,1 dB), donc l'échelle est élargie : `FILTER_MOD_OCTAVE_SCALE = 2.0`, soit **6 octaves** à Depth × Wet au maximum (20 Hz → 1280 Hz au bas de la plage Filter). Mesuré après : **−4,8 dB** à base 20 Hz, −2,2 dB à 100 Hz, −0,7 dB à 500 Hz. À 9 ou 12 octaves l'effet s'aplatit (filtre quasi ouvert en permanence), d'où le choix de 6.
- **Conséquence assumée** : un LFO qui n'ouvre que vers le haut rend l'ensemble plus brillant — les réglages SDrex existants en Filter LFO sonneront plus ouverts. Le mode Flanger est inchangé (il lit le même Depth comme des millisecondes de délai).
- Test `filter_lfo_at_the_lowest_base_stays_audible_at_full_depth` : vérifie les trois propriétés — un passe-bas à 20 Hz seul mute bien la voix (< −25 dB), Depth à fond la ramène à moins de 6 dB du filtre ouvert, et le LFO continue de façonner le son (> 1 dB sous l'ouvert, donc pas un bypass).

## 2026-08-20 — [182] Unités affichées sur les paramètres spéciaux et dans le menu plock (build 20260820-184818)

**Branche:** `main` · **Build:** `20260820-184818`
**Validation:** `cargo test` 309+1+198 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-09-09).**

- **Cause du manque d'unités** : les deux catégories de paramètres sont deux structures distinctes. `ParamWidget::Slider` (paramètres standard) porte un `suffix`, mais **`SpecialParamDef` n'avait aucun champ d'unité** — le Sound Panel passait `None` en dur pour tous les spéciaux, donc aucun ne pouvait afficher son unité, même quand il s'agissait de Hz, de secondes ou de ms.
- **`SpecialParamDef` gagne `unit: Option<&'static str>`** + un helper `sp_unit(...)`. `sp()` et `sp_discrete()` restent pour les grandeurs sans dimension (depth, wet, mix, amount…), donc seules les 12 lignes concernées changent : **Hz** — `Gate Rate` (Buzz), `Rate` (LFO SDrex), `Click Tone` (BD808), `Shimmer Freq` (Cymbal) ; **s** — `Filter Attack` et `Filter Hold` (Buzz et SDrex) ; **ms** — `Fade-in` (SDrex) ; **ct** (cents) — `Pitch Fine` des trois samplers 606.
- **Le menu plock affiche désormais les mêmes unités que le Sound Panel** — il formatait tout en `{:.2}` nu, y compris pour les paramètres standard qui ont une unité (« 0.50 » dans le menu contre « 0.50 s » dans le panneau). Les deux formateurs de `ui/fmt.rs` prennent l'unité ; le nombre reste produit par les mêmes règles d'arrondi qu'avant (fonctions internes `format_plock_number` / `format_plock_special_number`).
- Test `physical_special_params_declare_their_unit` : snapshot exact des 12 paramètres portant une unité (pour qu'une grandeur sans dimension n'en gagne pas une par erreur) **et** règle par mot-clé (`_rate`, `_freq`, `_attack`, `_hold`, `_fade`, `_fine_tune`) pour que le prochain paramètre de fréquence ou de durée ne puisse pas l'oublier. `snare606_tone` (mélange 0..1) et les `_atk_curve` ne matchent volontairement pas.

## 2026-08-20 — [181] Fine-tune des sliders, Modulation SDrex explicite, Fade-in, plages d'enveloppes (build 20260820-172925)

**Branche:** `main` · **Build:** `20260820-172925`
**Validation:** `cargo test` 308+1+197 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-09-09).**

- **Fine-tune des sliders réparé** — la modulation fine avait disparu lors de l'unification des sliders : `slider::draw_track` ne faisait que du positionnement absolu (saut à la position du curseur). **Shift ou Alt + glisser** fait maintenant un déplacement *relatif* à la valeur courante, ~4× plus fin (0,0015 unité normalisée par pixel, la même sensibilité que le slider du menu plock). Un simple Shift/Alt+clic ne saute plus. Détection du modificateur via `controls::fine_tune_modifier_pressed`, qui double le test egui d'une lecture clavier plateforme (`GetAsyncKeyState`) — les hôtes qui interceptent le clavier (Studio One, REAPER) empêchaient egui de voir le modificateur, ce qui est très probablement la cause de la panne. Le slider du menu plock utilise désormais la même détection. Maths du drag fin isolée dans `apply_fine_drag` + 4 tests unitaires (relatif, clamp aux bornes, mapping log, respect du pas de quantification).
- **SDrex : le switch « Filter Mod » devient un vrai choix « Flanger / Filter LFO »** — un interrupteur on/off ne disait pas entre quoi il choisissait. Nouveau helper `segmented_row` (label + sélecteur segmenté aligné à droite), le libellé du paramètre passe à « Modulation ».
- **SDrex : le Delay de modulation devient un Fade-in** — le slider « Delay » (délai minimum du flanger, 0–3 ms) est remplacé par **« Fade-in » (0–300 ms, défaut 0)** : le Wet de la modulation monte progressivement après chaque coup, donc le flanger ou le LFO de filtre **s'installe** au lieu d'être présent dès l'attaque. Actif dans **les deux modes**, donc plus grisé en Filter LFO (seul Feedback reste spécifique au flanger). Le délai minimum du flanger devient la constante `FLANGER_MIN_DELAY_MS = 0.7` — la valeur du défaut précédent, le caractère du flanger est inchangé. Les sessions existantes stockaient 0,7 sur ce champ → interprété en ms, soit un fade-in imperceptible : pas de rupture.
- **Plages d'enveloppes resserrées** là où la course du slider ne servait à rien : **Kick** et **BD808** decay 5 s → **2 s**, **Clap** decay 5 s → **1,5 s** (nouveau jeu `CLAP_STD` : le Cymbal, qui partageait `NO_FREQ_STD`, garde ses 5 s), **SDrex** Hold et Filter Hold 2 s → **1 s** (clamps DSP alignés).
- Tests : 4 nouveaux pour le drag fin, 1 pour le fade-in dans les deux modes, 1 pour les plafonds de decay (avec garde sur le Cymbal) ; les 3 tests SDrex existants mis à jour (le Fade-in n'est plus ignoré en Filter LFO, holds à 1 s, composition de la famille Modulation).

## 2026-08-20 — [179] Attaque identique quel que soit l'écart entre deux cellules (Kick + BD808) (build 20260820-162036)

**Branche:** `main` · **Build:** `20260820-162036`
**Validation:** `cargo test` 302+1+195 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-08-20).**

- **Le problème, mesuré** : la queue du coup précédent contaminait l'attaque du suivant. En mode digital (donc sans aucun drift aléatoire) et à réglages strictement identiques, seul l'écart entre deux cellules variant : le **Kick** montrait **3,71 dB** de dispersion de pic, une polarité de première demi-période qui s'inversait et un temps de crête errant de 1,5 à 8,3 ms ; la **BD808** passait d'un temps de crête de 12,7 ms (coup isolé) à 1,5 ms (coup rapproché), et en mode analog — le défaut — même ses coups isolés variaient de **2,6 dB** avec une discontinuité de **0,244** (le drift de niveau appliqué d'un coup sur la queue encore sonnante).
- **Quatre causes** : phase des oscillateurs jamais remise à zéro au retrigger (choix anti-clic historique) ; deux chemins de code distincts (cold start vs retrigger sur queue, bascule au seuil d'extinction de l'enveloppe) ; toutes les enveloppes repartant de leur valeur courante (ampli, pitch, filtre, snap/drop du 808) ; smoothers de fréquence et de cutoff non réinitialisés.
- **Fix — nouveau primitif `dsp::RetrigDeclick`** : chaque coup repart d'un **état neuf identique** (phase 0, filtre, smoothers et DC blocker vidés, enveloppes redémarrées depuis zéro), et un **fondu raised-cosine de 3 ms** du dernier échantillon émis garde la *sortie* continue pendant ce reset. Le reset de phase seul cliquait (step 0,35) : c'est le fondu qui le rend propre.
- **Résultat mesuré** : dispersion de pic **0,00 dB** sur 7 espacements (500 → 15 ms), temps de crête constant, polarité constante, step max au retrigger **0,014** (Kick) et **0,044** (BD808) — soit **4× plus propre** que le retrigger à phase continue qu'il remplace (0,058). Le clic du drift analog de la BD808 disparaît (plus de queue à re-scaler).
- La distinction analog/digital vit désormais **entièrement dans le drift par coup** (pitch/niveau/durée de queue) ; le sweep de pitch est déterministe dans les deux modes. Les stutters (`trigger_hard`) suivent le même contrat.
- **Contrat verrouillé par des tests** : nouveau module `src/synthesis/retrig_tests.rs` (6 tests) — dispersion ≤ 0,3 dB, temps de crête ≤ 0,5 ms d'écart, polarité constante, absence de discontinuité calibrée sur le même coup joué isolément, et coups isolés bit-identiques en digital. Le test de plock existant a été reformulé sur la même base auto-calibrée (un seuil absolu mesurait en réalité la raideur d'attaque légitime).
- **Portée : Kick et BD808 uniquement.** Les autres voix tonales (Tom, Perc1, Snare, Snare606, SDrex, Buzz) et les samplers 606 restent sur l'ancien comportement → tâche [180].

## 2026-08-20 — [178] Graphes d'enveloppe unifiés et factorisés (build 20260820-083705)

**Branche:** `main` · **Build:** `20260820-083705`
**Validation:** `cargo test` 297+1+189 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-08-20).**

- **Tous les graphes du Sound Panel sont construits de la même manière** : socle commun `prep_graph` (cadre LCD encastré + padding unifié 12/10 + hauteur 104, gate Buzz 72) et grille de quarts partagée, utilisés par les 6 graphes (`envelope_viz.rs`).
- **Couleurs de stages partout** : attaque = ambre, hold = vert, decay = bleu (helpers `stage_attack/hold/decay`). Le graphe filtre A-H-D (Buzz/SDrex) et le graphe ampli des samplers 606 colorent désormais leurs segments comme le graphe ampli des synthés ; les courbes filtre mono-stage (Toms, samplers) passent de l'orange au bleu decay. Trait de courbe unifié à 2 px ; ligne de cutoff factorisée (`draw_cutoff_line`).
- Token de thème `envelope_curve` (orange) devenu inutile → supprimé des 3 skins.

## 2026-08-20 — Fix solo/mute invisibles après les clears (build 20260820-082201)

**Branche:** `main` · **Build:** `20260820-082201`
**Validation:** `cargo test` 297+1+189 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-08-20).**

- **Bug** : un solo (ou mute) enclenché sur une lane survivait aux clears ; une fois la lane désactivée, le solo devenait invisible et continuait de muter tout le kit → silence total sans cause apparente.
- Nouveau helper partagé `controls::clear_all_mutes_solos(setter, params)` (14 mutes + 14 solos à off via `ParamSetter`).
- Appelé dans tous les chemins de clear destructifs : **Clear All** du header, **presets de layout** du navigateur de presets (Clear All / 4 Lanes / 12 Lanes + kits pattern/grid), **presets de style** du panneau Generate (House/Dub/DnB/Bossa/Afro/Break), et **Delete Lane** (reset du mute/solo du slot désactivé).
- Non touchés (les lanes restent visibles) : Generate, chips Rock/Funk/Disco/Random, Clear Lane, Clear All du Song.

## 2026-08-19 — SDrex : Flanger / Filter Mod + Decays 1,5 s (build 20260819-202022)

**Branche:** `main` · **Build:** `20260819-202022`
**Validation:** `cargo test` 297+1+189 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-08-20).**

- **Decay maxima ajustés** : Amp Decay et Filter Decay passent de 2 s à **1,5 s** ; les Holds restent à 2 s.
- **Section Flanger → Modulation** : ajout du switch `Filter Mod`. OFF = flanger classique ; ON = le même LFO module le cutoff du filtre LP et le délai flanger est bypassé.
- **Mapping Filter Mod** : `Rate` = fréquence du LFO, `Depth` = excursion bipolaire jusqu’à ±3 octaves, `Wet` = intensité de cette excursion. `Delay` et `Feedback` sont grisés dans l’UI et strictement ignorés par le DSP.
- **Changement de cible sûr** : le buffer de délai est vidé lors du passage Flanger ↔ Filter Mod pour empêcher la réapparition d’un ancien feedback. `Free Phase` contrôle le même LFO dans les deux modes.
- Tests dédiés : routage Depth/Wet vers le cutoff, indépendance bit-identique vis-à-vis de Delay/Feedback en Filter Mod, différence de rendu entre les deux cibles, nettoyage du buffer et roundtrip du switch.

## 2026-08-19 — SDrex : Holds, Decays 2 s, Free Phase flanger (build 20260819-165918)

**Branche:** `main` · **Build:** `20260819-165918`
**Validation:** `cargo test` 295+1+187 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-09-09).**

- **Enveloppe volume A-H-D** : ajout de `Hold` (0-2 s) entre Attack et Decay ; `Decay` monte désormais jusqu’à **2 s**. Le hold retarde les trois décroissances body/noise/metal sans figer le pitch drop.
- **Enveloppe filtre A-H-D** : ajout de `Filter Hold` (0-2 s) ; `Filter Decay` monte désormais jusqu’à **2 s**. DSP et graphe utilisent Attack → Hold → Decay avec les courbes bipolaires existantes.
- **Free Phase corrigé** : le switch quitte Oscillator et rejoint la section **Flanger**. OFF remet la phase du LFO flanger à zéro à chaque trigger ; ON conserve sa phase courante. Il n’agit plus sur les oscillateurs body/metal, qui retrouvent leur reset normal au cold start.
- Tests dédiés : plages UI à 2 s, Holds audibles, roundtrip des nouveaux champs, reset/conservation directe de `flanger_phase`, absence d’effet de Free Phase sur `body_phase`.

## 2026-08-19 — SDrex : section Flanger + enveloppe volume A-D (build 20260819-164808)

**Branche:** `main` · **Build:** `20260819-164808`
**Validation:** `cargo test` 292+1+184 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-09-09).**

- **Filter et Flanger séparés** : nouvelle famille data-driven `ParamFamily::Flanger`; Rate, Delay, Depth, Feedback et Wet apparaissent désormais dans une section **Flanger** autonome. Les paramètres cutoff et enveloppe LP restent seuls dans **Filter**.
- **Enveloppe volume SDrex enrichie** : ajout de `Attack`, `Attack Curve` et `Decay Curve` dans la section **Envelope**, avec graphe A-D. Attack applique une rampe commune aux couches body/noise/metal ; Attack Curve façonne cette rampe et Decay Curve façonne les trois décroissances caractéristiques sans ajouter une seconde enveloppe qui raccourcirait la recette.
- **Défaut neutre préservant le son** : Decay Curve SDrex passe à `0.0` (linéaire/neutre dans `shape_curve`), Attack reste à 0,5 ms et Attack Curve à `0.0`. Le reset par double-clic utilise les défauts SDrex du registry.
- Tests dédiés : séparation exacte des 5 paramètres Flanger, effet audible Attack/Attack Curve/Decay Curve, plus toute la suite de stabilité SDrex.

## 2026-08-19 — Correctif stabilité Clear All / SDrex (build 20260819-163329)

**Branche:** `main` · **Build:** `20260819-163329`
**Validation:** `cargo test` 290+1+182 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-09-09).**

- **Risque de crash natif supprimé dans la persistence Pattern Bank / Clear All** : l'ancien snapshot JSON publié par `AtomicPtr<Vec<u8>>` libérait immédiatement l'ancien buffer pendant qu'un autre thread pouvait encore le lire. Remplacé par un snapshot partagé protégé par `RwLock`, avec test de lecture/rafraîchissement concurrents.
- **Thread audio assaini** : une sauvegarde de pattern ne clone/sérialise plus toute la banque dans `process()` ; elle pose uniquement un drapeau atomique, consommé au prochain accès de persistence hors callback audio.
- **SDrex temps réel sécurisé** : buffer du flanger `Vec` remplacé par un tableau fixe (aucune allocation lors d'un changement de kind à chaud), longueur active bornée jusqu'à 192 kHz avec fallback sûr au-delà, délai interpolé clampé, phases oscillateurs bornées à `2π`. Test extrême fini à 8/44,1/192/384 kHz.
- **Preset Song** : suppression d'un auto-deadlock (`refresh_snapshot()` était appelé alors que le mutex Pattern Bank était encore détenu).
- **P-lock/morph** : `special[4]` ne peut plus écraser le champ réservé `Attack` (18). Pour SDrex, **Flanger Wet reste éditable globalement mais n'est pas proposé en p-lock/morph** tant que le format persistant n'a pas un champ distinct.

## 2026-08-19 — [175] Nouvel instrument SDrex + fix Algo Perc1 + légendes/grilles graphes (build 20260819-145510)

**Branche:** `main` · **Build:** `20260819-145510`
**Validation:** `cargo test` 284+1+178 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-09-09).**

- **[175] SDrex** (kind 15 / voice 17, catégorie SD) : recette « drex_snare » de l'utilisateur portée en voix temps réel — corps sine (pitch drop +95 Hz → base, env rate 32), noise HP par soustraction LP (env 18), metal ring-mod 620×910 Hz (env 25), mix 0.50/0.80/0.18, **flanger** (Rate 0.1-20 Hz / Delay 0-3 ms / Depth 0-3 ms / Fdbk 0-0.9 / Wet 0-1 — les 5 params demandés, en special params famille Filter), drive tanh ×2.2×0.8 fixe dans la chaîne de saturation standard. `Frequency` = base du corps (déplace aussi la paire metal), `Decay` scale les 3 enveloppes, `Analog` = drift par coup. Note MIDI 48, rôle Snare au GENERATE, mono. Enveloppes en formules temporelles → `set_settings` sans recréation (anti-click natif). Tests : son/fini/silence, wet flanger audible, decay étire la queue, roundtrip settings.
- **Fix Perc1 Algorithm** : `set_algo()` ne recréait pas les oscillateurs et faisait échouer la détection de changement dans `set_settings` → Perc1 jouait toujours Sine. `set_algo` reconstruit maintenant les 4 oscs sur changement réel. Test de régression (`perc1_algo_changes_the_output`, chemin moteur bit-identique à une voix Saw fraîche).
- **Graphes** : légende A/H/D supprimée (graphe ampli pleine hauteur) ; barres verticales de quart **sur tous les graphes** via `draw_grid_lines` partagé, atténuées (`white_a(9)`).

## 2026-08-19 — Filtre Tom : câblage corrigé + sweep exponentiel 20k (build 20260819-114620)

**Branche:** `main` · **Build:** `20260819-114620`
**Validation:** `cargo test` 278+1+172 OK. **Validé dans Studio One (2026-09-09).**

Diagnostic utilisateur (« le filtre du Tom ne fonctionne pas ») — 5 corrections en chaîne sur `tom.rs` :
- **Bug pitch** : `pitch_env.next()` appelé 2× par sample (top + branches algo) → sweep de pitch à **double vitesse**. Corrigé (1 appel, réutilisé).
- **Anti-click** : `set_settings()` recréait `pitch_env` à chaque paramètre → sweep redémarré au drag. Nouveau `PitchEnvelope::set_sweep_time` (mutation sans reset).
- **Stick attack contournait le filtre** (ajouté après) → routé à travers le même cutoff via un filtre dédié (`stick_filter`).
- **Plancher de cutoff 100 Hz** (50 en Deep) supprimé → 20 Hz (un Filter à 20 Hz donnait 100 Hz réels — le bas du slider était inopérant).
- **Filtre 1 pôle (6 dB/oct) → biquad 12 dB/oct** (RBJ Butterworth, Q=0.707, sans résonance) pour un filtrage radical en bas de course.
- **Loi du sweep changée** : `cutoff × (1 + env×amount×4)` (Filter 20 Hz + Env max = sweep 100→20 Hz, invisible) → **sweep exponentiel vers 20 kHz** `cutoff × (20000/cutoff)^(env×amount)` (même loi que Buzz) — DSP + graphe alignés.
- **Graphe filtre refait** : affiche le **vrai sweep du cutoff** sur axe log Hz (ligne de repos = Filter, courbe = balayage, fenêtre fixe 1 s) ; Filter Env à 0 = ligne plate honnête.

## 2026-08-18 — [174] Fixes graphes/DSP env filtre + BUG plocks sound perdus au chargement de pattern (build 20260818-182234)

**Branche:** `main` · **Build:** `20260818-182234`
**Validation:** `cargo test` 278+1+172 OK. **Validé dans Studio One (2026-09-09).**

- **BUG (P1) — plocks sound perdus au chargement d'un pattern** (bank slots ET presets) : `restore_from_buffers` écrivait le field mask via `field_masks.set(inst, step, mask as usize)` — or `set()` attend un **index de champ** (`1 << field`), pas un masque → le masque était corrompu à chaque restore (snapshot `(1<<46)-1` → no-op → masque vide → le plock devenait un link sans champ = muet). Remplacé par `set_raw`. La persistence projet (`plock-v1`) utilisait déjà `set_raw` — seul le chargement de patterns était cassé. Test de régression `pattern_preset_roundtrip_preserves_sound_plock`.
- **[174/F1] Toms** : `draw_filter_envelope` normalise la courbe sur toute la largeur (avant : plancher 100 ms sur l'axe X → courbe écrasée à gauche quand Filter Decay < 100 ms).
- **[174/F2] smp (BD6/SD6/CH6) + Perc1** : courbe de l'enveloppe de filtre = **constante dédiée** `FILTER_ENV_CURVE = 6.0` par voix (comme Kick/Snare/Tom). Avant, ces voix lisaient `decay_curve` — devenu **bipolaire (−1..1)** en [159] — comme raideur exp (2..12) → sweep de filtre quasi plat (vieilles sessions clampées à +1). Régression [159] corrigée côté **DSP et graphe** (`filter_env_curve()` étendu). ⚠️ Effet audible : le sweep de filtre de ces 4 voix redevient punchy.
- **[174/F3] Graphe ampli smp** : `draw_sample_amp_graph` réécrit en **A-H-D bipolaire** fidèle au DSP (`shape_curve` attack + decay ; proportion attack/decay approximative, shapes exactes) ; l'attack curve (`release_curve` repurposée) est passée au graphe.

## 2026-08-16 — [167] densité Randomize Lane + [170] curves renforcées + [168] stéréo 2 samples smp (build 20260816-185337)

**Branche:** `main` · **Build:** `20260816-185337` (retour utilisateur intégré : paires + compatible Analog)
**Validation:** `cargo test` 277+1+172 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-09-09).**

- **[167] Densité réglable pour Randomize Lane** : slider « Density » (5-100 %, défaut 30 %) dans le menu clic droit du nom de lane, au-dessus de « Randomize Lane ». Persisté dans l'état éditeur (`randomize_density`, fallback 30 % si 0/legacy).
- **[170] Courbes bipolaires renforcées** (tous les instruments) : exposant `1+3|c|` → `1+5|c|` dans `dsp::shape_curve` (enveloppes d'ampli A-H-D), `buzz::shape_curve` (enveloppe de filtre Buzz) et le graphe `envelope_viz`. Les réglages de courbe existants sonnent plus extrêmes aux bords (voulu).
- **[168] Mode stéréo 2 samples (voix multisamplées)** : switch **Stereo** placé **directement sous le sélecteur Sample** (famille Osc) avec infobulle EN expliquant la relation. Quand Stereo est ON, le sélecteur affiche des **paires** (« 1+2 », « 3+4 », « 5+6 », « 7+8 ») : L = 1er sample de la paire, R = 2e. **Compatible avec l'Analog Mode** : la paire est alors tirée au hasard à chaque coup. DSP : enveloppes partagées (1 avancée/sample), filtre et DC blocker indépendants par canal ; dual mono quand OFF. Tests ×3 voix (paire distincte / dual mono / analog+stereo) ; tests registry stereo/mono mis à jour (13|14|15 → stereo-capable).

## 2026-08-16 — [171] MIDI Pat sans retrig + [172] temps forts éclaircis + [169] Clap plus fort (build 20260816-151800)

**Branche:** `main` · **Build:** `20260816-151800`
**Validation:** `cargo test` 273+1+168 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-09-09).**

- **[171] MIDI Pat : plus de retrig au changement de pattern.** Le switch par note MIDI (60-75) ne lève plus `pending_song_pattern_restart` : le nouveau pattern **reprend à la volée** (position conservée). Si la longueur change, le resync host existant (`sync_to_host`, `rem_euclid` sur la nouvelle longueur) ramène la lecture dans le pattern — une page courante qui n'existe plus retombe dans le pattern (page 1 pour un pattern 1 page). Le mode Song conserve son restart (avancée par bloc).
- **[172] Temps forts 1/5/9/13 éclaircis** : voile `white_a(26)` sur les cellules OFF des temps forts dans `draw_step_cell_v2` (le sprite `pad-off-beat` seul était trop subtil).
- **[169] Clap plus fort** : volume par défaut 0.7 → 1.0 (`sound_settings_default` registry + `VoiceSettings::clap`). N'affecte que les nouvelles lanes / resets (les sessions existantes gardent leur valeur).

## 2026-08-15 — Sélecteur d'instrument : vrai flyout natif identique partout + rendu propre (build 20260815-181345)

**Branche:** `main` · **Build:** `20260815-181345`
**Validation:** `cargo check` warning-clean, `build.ps1 -Install` OK.

- **Refonte de l'unification** (la précédente était bâclée) : les trois points d'entrée ouvrent maintenant **exactement le même menu natif egui** (flyout cascade catégorie ▸ kind), donc rendu et comportement rigoureusement identiques.
  - **Cause des bugs précédents** : `ui.menu_button` brut posé dans un `Area` popup custom (lane vide) n'a pas de racine de menu → les sous-menus (catégories) ne s'ouvraient pas ; et dans l'onglet Track il produisait un petit bouton étroit disgracieux.
  - **Correctif** : passage à `egui::menu::menu_custom_button` (déclencheur custom + **vraie racine de menu native** → sous-menus fonctionnels) pour la lane vide et l'onglet Track. Le clic-droit reste natif. Le flyout est donc le même partout (celui, validé, du clic-droit).
  - **Lane vide** : le « +N » devient un bouton stylé (fill/bordure/coins arrondis) qui ouvre la cascade ; l'ancien popup custom `AddModulePopup` est **supprimé** (struct + champ d'état + `draw_add_module_popup_if_any` + remaps).
  - **Onglet Track « Type »** : champ 146×CTL_HEIGHT (même gabarit que Aux Out / Choke, fill P_ACTIVE + bordure LINE2 + police mono) ouvrant la cascade — fini le carré étroit.
  - Helper `menus::instrument_category_menu` : ferme désormais le menu à la sélection (les lignes custom ne déclenchent pas la fermeture auto d'egui).
- Nettoyage : `draw_empty_lane_name_v2` retiré, imports orphelins supprimés.

## 2026-08-15 — Sélecteur d'instrument unifié partout (cascade par catégorie) (build 20260815-171606)

**Branche:** `main` · **Build:** `20260815-171606`
**Validation:** `cargo check`/build warning-clean, `build.ps1 -Install` OK.

- **Un seul sélecteur d'instrument, identique aux trois endroits** : lane vide (popup « Add Module »), clic-droit sur le nom de lane, et champ « Type » de l'onglet Track. Tous ouvrent désormais la **même cascade par catégorie** (BD/SD/HH/PERC/FX/OTHER ▸ kind), avec le kind courant préfixé « > » et surligné.
- Helper partagé unique `menus::instrument_category_menu(ui, current) -> Option<kind>` (un sous-menu par catégorie, lignes plates) — les trois appelants s'y branchent, plus de duplication.
  - Lane vide : le popup « Add Module » passe de la liste plate groupée à la cascade.
  - Onglet Track : le dropdown plat `styled_select` (15 kinds) est remplacé par un menu-bouton `<kind> ▾` ouvrant la cascade.
  - Clic-droit : inchangé fonctionnellement (déjà en cascade), factorisé sur le helper.
- Nettoyage des imports devenus inutilisés (`InstrumentCategory` dans grid.rs, `TrackInstrumentKind` dans popups.rs/sound_editor.rs).

## 2026-08-15 — Onglet Track : loader de presets d'instrument pour la lane sélectionnée (build 20260815-161113)

**Branche:** `main` · **Build:** `20260815-161113`
**Validation:** `cargo test` 273+1+168 OK, `build.ps1 -Install` (install manuelle — verrou AV transitoire sur le DLL, cf. note).

- **Nouvelle section « Preset » dans l'onglet Track** (sous *Instrument*, avant *Routing*) : un menu **« Load »** liste les presets d'instrument (factory + utilisateur) **du type de la lane sélectionnée** et les applique en un clic (standards + algo + specials) sur le slot courant.
  - Menu « action » : le bouton reste sur « Load preset… » ; après sélection il applique puis revient au placeholder. « No presets » si aucun preset pour ce type (zone stable).
  - Liste mise en cache par (slot, kind) → aucune I/O par frame ; rebâtie au changement de lane/type et après sauvegarde d'un preset d'instrument.
  - `presets::list_instrument_presets(kind)` / `load_instrument_preset(entry)` ; application via `preset_browser::apply_instrument_preset_to_slot` (factorisé avec `apply_instrument`, helper `write_slot_sound` partagé).
- **Note install** : l'écriture dans `C:\Program Files\...\VST3\` a échoué 2× sur un « accès refusé » alors que Studio One était fermé — verrou transitoire de l'antivirus (BitDefender) scannant le DLL fraîchement compilé. Le fichier était libre juste après ; bundle copié manuellement. À surveiller si ça se reproduit.

## 2026-08-15 — Preset de pattern : sauvegarde/restauration du son de chaque instrument (build 20260815-124040)

**Branche:** `main` · **Build:** `20260815-124040`
**Validation:** `cargo test` 273+1+168 OK (2 nouveaux tests presets), `build.ps1 -Install` OK.

- **Le preset de pattern ([150] « Patterns ») embarque désormais le son de chaque lane active** (13 standards + algo + specials), en plus de la grille / fusions / plocks / kit déjà capturés. Avant : charger un preset de pattern restaurait la grille et le kit (types d'instruments) mais **pas** les réglages sonores → les instruments sonnaient avec leurs valeurs courantes/par défaut.
- **Capture** : `capture_pattern` prend `sound_settings` + les algos par slot et sérialise un `PatternSlotSound { slot, kind, standards[13], algo, specials }` par lane active (réutilise l'extraction de `capture_instrument`).
- **Restauration** : `apply_pattern` réapplique chaque son **sur le slot qui porte encore le même kind** (garanti après pose du kit ; les lanes de kind différent sont ignorées quand on charge sans le kit, pour ne jamais écraser un autre instrument). Helper `write_slot_sound` factorisé et partagé avec `apply_instrument`.
- **Rétro-compat** : champ `sounds` en `#[serde(default)]` → les presets enregistrés avant ce changement se chargent avec une liste vide (grille seule, sons inchangés). Tests : capture des sons + chargement d'un preset legacy sans `sounds`.

## 2026-08-15 — Fix : les cellules fusionnées sont bien enregistrées dans un slot de pattern (build 20260815-121906)

**Branche:** `main` · **Build:** `20260815-121906`
**Validation:** `cargo test` 272+1+168 OK (nouveau test `pattern_slot_capture_restore_preserves_fusions`), `build.ps1 -Install` OK.

- **Bug** : sauvegarder le pattern courant dans un slot (P1–P16) **perdait les cellules fusionnées** au rechargement du slot.
- **Cause** : dans `deserialize_fusions` (`pattern_bank.rs`), le gate `expected_new` calculait la taille attendue comme `INSTRUMENT_COUNT × MAX_FUSIONS × FUSION_SLOT_COUNT × 8`, alors que `PatternSlot::capture` sérialise en réalité, par lane, **un compteur u64 nu (8 o) + (MAX_FUSIONS−1) groupes de FUSION_SLOT_COUNT u64** (16 o de moins par lane). Le vrai blob (5152 o) étant plus court que le gate (5376 o), il était **rejeté puis relu en format legacy** → toutes les fusions tombaient.
- **Fix** : gate corrigé à la taille réelle `INSTRUMENT_COUNT × (8 + (MAX_FUSIONS−1) × FUSION_SLOT_COUNT × 8)`. S'applique aux deux chemins de chargement (`PatternSlot::restore` et `restore_from_buffers`). Test de non-régression ajouté.
- N'affectait pas la persistance projet (`pattern-v5`, autre chemin) — d'où des fusions conservées à la sauvegarde du projet mais perdues au save-slot.

## 2026-08-14 — [150] Gestion des presets (instruments / patterns / grid / songs) + outil factory (build 20260814-171844)

**Branche:** `main` · **Build:** `20260814-171844`
**Validation:** `cargo test` 271+1+168 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-09-09).**

- **Retours post-v1** : bouton **« Presets » déplacé dans le header**, entre « MIDI Pat » et « Settings », encadré de barres de séparation (vbar épaissies 1→2 px) ; le bouton de la barre Pattern Bank est retiré.
- **Type de preset « Grid »** (4e onglet du modal) : capture/charge le kit de lanes (kinds par slot, `.fdgrid.json`). Les 3 layouts d'usine de l'ancien dropdown **« Preset » de la page-bar — supprimé** — y vivent désormais : **Clear All** (2 clics, efface la grille) / **4 Lanes** / **12 Lanes**. `LanePresetAction`, le dropdown et le warning popup associés sont supprimés ; `apply_lane_layout_preset` est partagé avec le modal.

- **Modal « Presets »** (nouveau bouton dans la barre Pattern Bank, plaque skeuo centrée) avec 3 onglets :
  - **Instruments** : capture les 13 standards + specials + algo du slot sélectionné. Au load, si le preset vise un autre kind, la lane change de type d'abord (`change_slot_kind`), puis les valeurs s'appliquent.
  - **Patterns** : capture grille 64 steps + fusions + sound plocks + seq plocks (blobs hex, même layout que la Pattern Bank → tolérance legacy) + **kit de lanes**. Toggle **« Load lanes too »** : avec kit = installe les lanes du preset (`apply_lane_layout_preset`), sans kit = steps/plocks sur les lanes actuelles.
  - **Songs** : capture/publie la `SongSequence` (bank + snapshot + `song_controller`).
- **Fichiers JSON versionnés** (`version: 1`) sous `Documents/Flash Drum/presets/{instruments,patterns,songs}/` (extensions `.fdinst.json` / `.fdpat.json` / `.fdsong.json`). Listes Factory (read-only) + User (Load / Del en 2 clics).
- **Presets d'usine embarqués** : `factory_presets.rs` expose `INSTRUMENTS`/`PATTERNS`/`SONGS` en `include_str!` depuis `assets/presets/` (vides pour l'instant).
- **Outil d'authoring factory** : en build debug, bouton « Export factory (dev) » dans le modal → écrit le JSON dans `presets/_factory/<kind>/` ; workflow : copier dans `assets/presets/<kind>/`, ajouter la ligne `include_str!`, commit (documenté dans `factory_presets.rs`).
- Modules : `src/presets.rs` (types, capture, fs, hex) + `src/ui/preset_browser.rs`. L'ancien outil dev `preset_dumps.rs` reste en place (sound editor debug).
- Tests : roundtrips JSON/hex, sanitize, capture pattern (masks + kit + blobs décodables).

## 2026-08-13 — [163] Catégories d'instruments + type via clic droit sur la lane (build 20260814-090820)

**Branche:** `main` · **Build:** `20260814-090820`
**Validation:** `cargo test` 267+1+168 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-08-14).**

- **`InstrumentCategory` (BD, SD, HH, PERC, FX, OTHER)** sur `TrackInstrumentKind` (`track.rs`) : `category()`, `kinds_in(cat)`, `ALL`. Mapping : BD = Kick/808 Kick/BD6smp, SD = Snare/Snare 606/SD6smp/Clap, HH = HiHat/Open Hi-Hat/CH6smp, PERC = Tom/Perc1, FX = Buzz, OTHER = Ride/Cymbal.
- **Le menu clic-droit sur le nom de lane permet de changer le type d'instrument** : sous-menus **cascadés** « Instrument ▸ Catégorie ▸ kind » (2 niveaux), kind courant marqué « > » en bleu et non cliquable. `change_slot_kind()` applique la même sémantique que le dropdown Type de l'onglet Track : nom par défaut + note MIDI du kind + reset des réglages aux défauts (réinit audio via le watch `last_slot_kinds`). **Hover highlight** ajouté sur `context_menu_button` (tous les menus contextuels : fond `P_HOVER` + label blanc au survol, rangées désactivées sans highlight). Lignes d'instruments en version **plate** (`context_menu_row_plain` — pas de keycap 3D dans les sous-menus imbriqués, trop lourd visuellement).
- **Popup Add Module groupé par catégorie** (même regroupement, headers de catégorie).
- Tests : `categories_partition_all_kinds` (chaque kind dans exactement une catégorie, aucune vide) + `category_spot_checks`.

## 2026-08-12 — [164]+[162]+[165]+[160]+[161] Batch de tâches (build 20260813-143901)

**Branche:** `main` · **Build:** `20260813-143901` (inclut les retours : graphe gate replacé à côté des sliders, nudge ±100 ms)
**Validation:** `cargo test` 265+1+166 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-08-13).**

- **[164] Glyphe du bouton reset morphing corrigé** — « × » corrompu (« Ã— », UTF-8 relu en Windows-1252) remplacé par « X » ASCII dans le popup Morph (`ui/plock.rs`), conforme à la convention [73] (ASCII only dans les labels).
- **[162] Enveloppes grisées en One-Shot (voix smp)** — sur BD6smp/SD6smp/CH6smp, activer **One Shot** grise les sliders Attack/Decay/Decay Curve (`add_enabled_ui`, le switch One Shot reste actif car c'est un special param) et la ligne d'enveloppe du graphe passe en gris (`draw_sample_amp_graph`). Layout stable : on grise, on ne cache pas.
- **[165] Drag d'une lane vide** — le grip des lanes inactives est câblé comme celui des lanes actives (`lane_drag_source` + curseurs Grab/Grabbing). `apply_lane_reorder_move` était déjà slot-générique → rien d'autre à toucher ; pas de sélection de track pour un slot inactif (pas d'onglet Sound Editor).
- **[160] Graphe Gate Shape (Buzz)** — `draw_buzz_gate_graph` placé **à droite des sliders Gate Rate/Depth/Shape** (sous-rangée dédiée : sliders à gauche, graphe à droite — pas d'empilement sous le graphe d'ampli qui décalait le bloc). Fenêtre temps **fixe 60 ms** (le Rate est visible : ~3 cycles à 55 Hz, peigne dense à 500 Hz), Smooth = cosinus surélevé `^(1+4·shape)`, Razor = rampe 0,3 ms + spike expo (mêmes constantes que `BuzzVoice`), plancher de Depth en ligne fine, tag « GATE ».
- **[161] Microtiming par cellule (seq plock), ±100 ms complet** — jamais câblé au moteur auparavant (stockage/persistance seuls existaient). Le **séquenceur** décale désormais chaque trigger :
  - `groove::step_start_beat(step, swing, groove)` : inverse exacte de `beat_to_step` (paires swing/shuffle/MPC) pour connaître l'heure des frontières de step.
  - **Nudge positif** : le trigger est différé (`late_trigger`/`late_fire_beat`) → stutter/fusion pulses s'expandent depuis l'heure décalée, tout le train bouge d'un bloc.
  - **Nudge négatif** : peek de la cellule du prochain boundary à chaque sample ; quand le temps restant ≤ −nudge, le trigger part en avance (`classify_cell`/`eval_trigger` partagés avec le chemin normal → masque, humanize, fusions, morphs identiques) ; le boundary réel avance l'état mais reste muet (`suppress_next`). Flag `early_next_loop` sur un early-fire qui croise le wrap → les conditions (First/NotFirst/…) sont évaluées avec `loop_count + 1` côté `lib.rs`.
  - Données : `Sequencer::set_microtimings` copie les atomics seq-plock 1×/buffer (RT-safe) ; état microtiming purgé sur play/stop/reset/seek (`clear_microtiming_state`).
  - **UI** : row « Nudge » (−100..+100 ms, reset double-clic à 0) dans le menu Seq Plock, entre Stutter et Condition.
  - **Export MIDI** : les notes sont décalées du nudge (ms → ticks au tempo d'export, clamp tick 0).
  - Tests : inverse `step_start_beat`↔`beat_to_step` (4 grooves × 5 swings × 16 steps), ±25 ms sample-accurate (±2 samples), early-fire au wrap + flag, zéro nudge = grille inchangée, export MIDI ±50 ms.
  - ⚠️ Limites connues : conditions à la boucle avec push/pull ≠ 0 = approximatif (le wrap de `loop_count` suit la timeline non shiftée, préexistant) ; collision même sample late+transition = report d'1 sample plutôt qu'un hit perdu.

## 2026-08-12 — Passation : docs réorganisées + handoff (pas de build)

- **`CLAUDE.md` devient la référence canonique unique pour tous les agents IA** (Claude/Codex/Kimi/…) : compteurs mis à jour (17 voix, 14 slots, plocks 46×14×64), enveloppe d'ampli A-H-D documentée, et absorption du détail unique d'`AGENTS.md` (patches du fork nih-plug, chaîne de saturation, choke groups, règle checklist « À tester dans Studio One », portabilité, règle « next »).
- **`AGENTS.md` réduit à une redirection** vers `CLAUDE.md` (fini la duplication périmée). Cross-refs `README.md` / `drum-pattern-vst/README.md` mis à jour.
- **`docs/HANDOFF.md` ajouté** : état de session pour un autre agent dev (arbre git, tâches, gotchas, carte des fichiers).
- **[159] validé** dans Studio One. 3 nouvelles idées notées → TODO [162]/[163]/[164]. Point de reprise : **[155]**.

## 2026-08-07 — [159] Enveloppe d'ampli A-H-D bipolaire (retrait du Release) sur toutes les voix (build 20260807-170048)

**Branche:** `skeuo-vector` · **Build:** `20260807-170048`
**Validation:** `cargo test` 259+1+161 OK, `build.ps1 -Install` OK.

- **L'enveloppe d'AMPLITUDE de toutes les voix passe d'un modèle decay+release à un A-H-D (Attack-Hold-Decay) sans release**, avec des **courbes bipolaires concave/convexe indépendantes sur l'attaque et le decay** (comme l'env de filtre du Buzz). Généralise le retour positif de l'utilisateur sur le Buzz.
  - **Réécriture interne de `DecayReleaseEnvelope`** (`dsp.rs`) en A-H-D piloté par le temps, **signatures publiques conservées** → les 14 voix ne changent quasiment pas. `decay_curve` = courbe **decay** bipolaire ; `release_curve` **réutilisé** comme courbe **attack** bipolaire ; `set_release`/`release_time` = **no-op**. `shape_curve(e,c)` partagée : `c≥0 → e^(1+3c)` (convexe), `c<0 → 1-(1-e)^(1-3c)` (concave).
  - **Anti-clic préservé** : `trigger()` rampe depuis la valeur courante (queue vivante) ; `trigger_hard()` repart de zéro (machine-gun/stutter). Tests anti-clic kick/perc1 verts.
  - **Registry** : slider **Release retiré** de toutes les tables ; **« Release Curve » → « Attack Curve »** (−1..1) ; **« Decay Curve »** en −1..1. `morphable_fields` suit (plages dérivées des tables).
  - **Graphe** (`draw_amp_envelope`) : A-H-D bipolaire (attaque + decay façonnées par `shape_curve`), plateau Hold, plus de segment/légende Release.
  - **Buzz** : son ampli passe de `ExpDecayEnvelope` à `DecayReleaseEnvelope` (courbe de decay désormais bipolaire, cohérent avec le reste ; retrigger machine-gun conservé via `trigger_hard`).
  - **Bugs corrigés au passage** : `open_hihat` recréait son env dans `set_settings` (reset d'état → clic au drag de slider) → remplacé par des setters ; `open_hihat`/`cymbal` jetaient le résultat de `with_attack_ms` (`Copy` no-op) → `set_attack_ms` ; drift d'attaque dupliqué du cymbal nettoyé.
- **Persistance (dégradation gracieuse, sans migration)** : blobs positionnels inchangés → aucune casse. Les anciennes valeurs de courbe (0.1–20) relues sur −1..1 sont **clampées à +1** (max convexe ≈ decay exponentiel raide d'avant). Les défauts par voix ne sont pas retouchés → nouvelles instances = courbes max-convexes (sliders au max) ; à affiner par voix si besoin.

## 2026-08-07 — [158] Export MIDI : fusions + stutters inclus (build 20260807-140101)

**Branche:** `skeuo-vector` · **Build:** `20260807-140101`
**Validation:** `cargo test` 259+161 OK, `build.ps1 -Install` OK.

- **L'export/drag MIDI inclut désormais les notes des cellules fusionnées et des stutters** (avant : 1 note par step actif, fusions/stutters ignorés). Réplique la logique du séquenceur audio.
  - **Fusion** : la cellule START d'un groupe émet `step_count` notes réparties uniformément sur toute la durée du span (`cell_span()` pas de 1/16, PPQ 480 → 120 ticks/step) ; les cellules couvertes n'émettent rien.
  - **Stutter** : une cellule non fusionnée avec un p-lock séquenceur `stutter_count = N` émet N notes réparties sur un pas. Fusion et stutter ne se combinent jamais (comme l'audio).
  - Durée de note-off raccourcie (`min(10, spacing-1)`) pour éviter le chevauchement des sous-notes rapprochées.
- `export_pattern_to_midi[_data|_bytes]` + `export_midi_to_documents` prennent un `&SequencerPlockState` ; call sites Export/Drag passent `params.seq_plock_state.state`. Tests `midi_export_expands_stutter_into_multiple_notes` + `midi_export_expands_fusion_into_pulses`.

## 2026-08-07 — [156] Bouton "Save" à gauche des patterns (build 20260807-123130)

**Branche:** `skeuo-vector` · **Build:** `20260807-123130`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK.

- Le keycap **Save** est déplacé **avant** la rangée de slots (`Patterns  [Save]  P1…P16  [Clr]  …Export/Drag`). Comportement inchangé (armer puis cliquer un slot).

## 2026-08-07 — Pas de fausse alerte sur projet rouvert déjà sauvé dans un slot (build 20260807-121446)

**Branche:** `skeuo-vector` · **Build:** `20260807-121446`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK.

- **Nuance** : projet sauvé avec le pattern courant stocké dans P3, quitté puis rouvert → `last_loaded = None` (association runtime perdue) mais la grille **correspond** à P3. Le test précédent (`None + non-vide = dirty`) déclenchait une **fausse alerte** au changement de slot alors que le pattern était bien sauvegardé.
- **Correctif** : sur `None`, dirty **seulement si** la grille a du contenu **ET** ne correspond à **aucun** slot occupé (mêmes `step_masks` + longueur). Un projet rouvert dont le pattern était sauvé dans un Pn matche ce slot → non dirty → pas d'alerte.

## 2026-08-07 — Warning "unsaved" aussi pour un projet chargé sans slot (build 20260807-115649)

**Branche:** `skeuo-vector` · **Build:** `20260807-115649`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK.

- **Suite** : un projet rouvert avec un pattern de travail non associé à un slot a `last_loaded = None` (l'atomique `audio_last_loaded_slot` n'est pas persistée) ; `pattern_is_dirty` renvoyait `false` sur `None` → pas de warning au changement de slot.
- **Correctif** : `None` + grille non-vide = dirty (travail non sauvé sans slot). Le warning « unsaved changes » s'affiche au changement de slot ; aucune étoile (aucun slot n'est marqué chargé). Grille vide → toujours pas dirty.

## 2026-08-07 — Fix régression : warning "unsaved" restauré pour un nouveau pattern (build 20260807-114735)

**Branche:** `skeuo-vector` · **Build:** `20260807-114735`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK.

- **Régression du fix [153]** : avant, se positionner sur un slot vide laissait (par bug) `last_loaded` sur le slot occupé précédent → un pattern construit de zéro était comparé à ce slot → jugé dirty → warning au changement. Le fix [153] a corrigé `last_loaded` (pointe bien le slot vide), mais `pattern_is_dirty` retournait `false` dès que le slot n'était pas occupé → **plus aucun warning** pour un nouveau pattern non sauvé.
- **Correctif** : `pattern_is_dirty` considère désormais un slot **vide avec une grille non-vide** comme dirty (travail non sauvé) → le warning « unsaved changes » réapparaît proprement au changement de slot, et une étoile marque le slot vide en cours de construction. Grille vide → toujours pas dirty (donc [153] préservé : après save + sélection d'un slot vide, la grille est vidée → aucune étoile fantôme).

## 2026-08-07 — [157] Buzz : max Gate Rate 150 → 500 Hz (build 20260807-111951)

**Branche:** `skeuo-vector` · **Build:** `20260807-111951`
**Validation:** `build.ps1 -Install` OK.

- **Gate Rate max relevé de 150 à 500 Hz** (`GATE_RATE_MAX` + max du slider registry) — buzz plus aigus, jusqu'à un caractère AM/tonal. Le clamp audio reste aligné sur la constante.

## 2026-08-07 — [154] Hold visible dans le graphe d'enveloppe (build 20260807-111016)

**Branche:** `skeuo-vector` · **Build:** `20260807-111016`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK.

- **Bug** : `draw_amp_envelope` recevait `hold` mais le repliait dans `decay_time` — aucun palier de maintien n'était dessiné, donc les instruments avec un Hold (Snare, etc.) ne le voyaient pas dans le graphe.
- **Correctif** : tracé A-H-D explicite — rampe d'attaque ↗, **palier plat au sommet pendant le Hold** (couleur teal dédiée), puis decay ↘. Le Hold compte dans l'échelle temporelle (`total = attack + hold + decay [+ release]`). Légende « H » ajoutée quand un Hold est réglé.

## 2026-08-07 — [153] Fix étoile "non sauvegardé" fantôme (build 20260807-105334)

**Branche:** `skeuo-vector` · **Build:** `20260807-105334`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK.

- **Bug** : sauver un pattern puis sélectionner un slot **vide** faisait réapparaître le pattern sauvé avec une étoile (dirty) alors qu'il n'avait pas été modifié.
- **Cause** : l'UI resynchronise `state.last_loaded_slot` **depuis** `audio_last_loaded_slot` à chaque frame (ui.rs). Se positionner sur un slot vide est une action **UI-only** (pas de requête de load) qui vidait la grille + posait `last_loaded_slot` localement, mais **sans** publier le slot vers l'audio → au frame suivant la sync ramenait le slot sauvé, et la grille étant vidée ≠ pattern sauvé → `is_dirty` vrai → étoile fantôme.
- **Correctif** : le positionnement sur slot vide publie maintenant `audio_last_loaded_slot = i` (l'atomique n'est qu'un hint lu par l'UI — l'audio ne l'écrit que sur save/load, jamais ne le lit pour sa logique). La sync est cohérente → plus d'étoile fantôme.

## 2026-08-07 — Buzz : retrigger machine-gun de l'enveloppe d'ampli (build 20260807-093254)

**Branche:** `skeuo-vector` · **Build:** `20260807-093254`
**Validation:** `cargo test` buzz OK, `build.ps1 -Install` OK.

- **Fix « sur cellules consécutives, l'env de volume n'est plus prise en compte »** : le retrigger de l'ampli utilisait `trigger_at_peak` (rampe depuis la valeur courante, anti-clic), donc une queue en cours « absorbait » la nouvelle enveloppe → l'A-H-D de volume ne se ré-articulait pas pleinement par cellule.
- **Correctif** : ampli passé en **retrigger machine-gun** (`trigger_from_zero`) dans `trigger()` et `trigger_hard()` → chaque cellule redémarre **toute** l'enveloppe A-H-D de volume depuis zéro (comportement standard des boîtes à rythme). Déviation volontaire de la convention anti-clic (la rampe d'attaque ≥0.3 ms adoucit le redémarrage).

## 2026-08-06 — Buzz : le gate se resynchronise à chaque hit (build 20260806-203401)

**Branche:** `skeuo-vector` · **Build:** `20260806-203401`
**Validation:** `cargo test` buzz OK, `build.ps1 -Install` OK.

- **Fix « l'attaque de volume n'est pas triggée à chaque cellule »** : l'enveloppe d'ampli se ré-attaquait bien (vérifié en test : queue 0.10 → 1.52 après re-trigger), mais le **gate** ne réinitialisait sa phase qu'au cold-start (anti-clic osc). Sur des hits qui se chevauchent, chaque cellule attrapait donc le gate free-run à une phase aléatoire → pas d'attaque de volume gatée cohérente par cellule.
- **Correctif** : `gate_phase` remis à 0 + `gate_env` ré-articulé à **chaque** trigger (chaque cellule démarre sur un pic de gate = burst d'amplitude identique et net). La phase de l'oscillateur tonal reste réinitialisée **au cold-start uniquement** (anti-clic de la partie tonale).

## 2026-08-06 — Buzz : courbes filtre attack/decay dissociées (build 20260806-201745)

**Branche:** `skeuo-vector` · **Build:** `20260806-201745`
**Validation:** `cargo test` 257+161 OK, `build.ps1 -Install` OK.

- **Filter Atk Curve + Filter Dec Curve** (au lieu d'un unique « Filter Curve ») : la montée et la descente de l'enveloppe de filtre se façonnent indépendamment, chacune bipolaire -1..+1.
- **Enveloppe de filtre passée en AHD manuelle** (compteur de temps depuis le trigger) au lieu d'un `ExpDecayEnvelope` — nécessaire pour connaître la phase (attack vs decay) et appliquer la bonne courbe. Bases **linéaires** façonnées par `shape_curve` bipolaire (0 = linéaire, +1 = convexe, -1 = concave). Défaut Dec Curve = 0.6 (garde le punch), Atk Curve = 0.
- Specials : `buzz_filter_curve` (index 15) renommé « Filter Dec Curve » ; nouveau `buzz_filter_atk_curve` (index 16). Graphe `draw_buzz_filter_envelope` mis à jour pour les deux courbes.

## 2026-08-06 — Buzz : graphe de l'enveloppe de filtre reflète l'AHD réelle (build 20260806-200700)

**Branche:** `skeuo-vector` · **Build:** `20260806-200700`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK (changement UI pur).

- **Fix graphe filtre statique** : `draw_filter_envelope` ne dessinait qu'une décroissance exponentielle simple (curve = decay_curve de l'ampli, car `filter_env_curve()` = None pour Buzz), ignorant Filter Attack / Hold / Filter Curve / amount. Nouveau tracé dédié **`draw_buzz_filter_envelope`** qui reproduit exactement le DSP : rampe d'attaque → hold → decay (raideur interne 4.0) avec la **courbe bipolaire** appliquée, et le cutoff balayé **exponentiellement** (base → base·(20000/base)^(env·amount) → base), axe Y en Hz log + ligne du cutoff au repos. Le graphe réagit désormais à Filter Attack, Hold, Decay, Curve et Filter Env.

## 2026-08-06 — Buzz : Filter Curve bipolaire (concave ↔ convexe) (build 20260806-172100)

**Branche:** `skeuo-vector` · **Build:** `20260806-172100`
**Validation:** `cargo test` 257+161 OK, `build.ps1 -Install` OK.

- **Nouveau contrôle « Filter Curve »** (special index 15, famille Filter, bipolaire **-1 → +1**, défaut 0). Façonne le contour du decay de l'enveloppe de filtre : **-1 = concave** (le filtre tient puis chute vite), **0 = naturel** (exp), **+1 = convexe** (chute rapide puis lente = snappy). Implémenté par une transformation puissance bipolaire de la sortie de l'env (`e^(1+3c)` côté +, `1-(1-e)^(1-3c)` côté -), l'`ExpDecayEnvelope` ne faisant que de la raideur convexe.

## 2026-08-06 — Buzz : env filtre exponentielle + vrai sélecteur LP/HP/BP (build 20260806-171049)

**Branche:** `skeuo-vector` · **Build:** `20260806-171049`
**Validation:** `cargo test` 257+161 OK, `build.ps1 -Install` OK.

Retours utilisateur :
- **Env de filtre repensée (percussive)** : le mapping additif (`base + env·amount·12 kHz`) était peu intuitif et, avec Filter Attack à 2 ms, le filtre s'ouvrait *après* le transitoire. Désormais **balayage exponentiel** `cutoff = base·(20000/base)^(env·amount)` (0 = base, 1 = base→20 kHz→base) + **Filter Attack défaut = 0** (instantané). À amount plein + Filter au minimum → filtre percussif franc (20 Hz→20 kHz→20 Hz). Défauts revus : base 1200 Hz, amount 0.6, decay 0.12 s.
- **Vrai sélecteur Filter Type** : le special `buzz_filter_type` s'affichait en slider (le dropdown n'avait pas été câblé) → branche dropdown **LP / HP / BP** ajoutée dans `sound_editor`.
- **« (LP) » retiré** à côté du slider Filter : `filter_type_label` de Buzz vidé, et le label n'ajoute plus le suffixe quand il est vide (le type est maintenant piloté par le dropdown).

## 2026-08-06 — Buzz : ampli AHD (sans release) + enveloppe AHD de filtre + type LP/HP/BP (build 20260806-170024)

**Branche:** `skeuo-vector` · **Build:** `20260806-170024`
**Validation:** `cargo test` 257+161 OK, `build.ps1 -Install` OK.

- **Ampli en AHD** : l'enveloppe d'ampli passe de decay+release à **Attack-Hold-Decay pure** (`ExpDecayEnvelope`) — le **Release est retiré**. Nouvelle table de params standard `BUZZ_STD` (sans Release / Release Curve).
- **Enveloppe AHD de filtre** : le cutoff est balayé par sa propre enveloppe A-H-D. Contrôles : **Filter Env** (amount) + **Filter Decay** (standard, section Filter) ; **Filter Attack** + **Filter Hold** (specials index 12/13, famille Filter). Modulation additive `cutoff = base + env·amount·12 kHz`.
- **Type de filtre** : nouveau dropdown **Filter Type** (special index 14) → **LP / HP / BP**. Le filtre de base passe d'un one-pole 6 dB à un **Biquad 2 pôles** ; ajout de `Biquad::set_lowpass` / `set_highpass` (RBJ) à `dsp.rs` (Q 0.9 LP/HP, 2.5 BP). Recalcul des coefficients par sample (négligeable pour une voix).
- Défauts : Filter Env amount 0.4, Filter Decay 0.15 s, Filter Attack 2 ms (l'effet est audible d'emblée).
- Tests ajoutés : `filter_type_changes_the_output` ; garde `output_stays_finite_and_stops` OK avec l'ampli sans release.

## 2026-08-06 — Buzz : waveform Sine/Square/Saw + fixes gate/sweep/volume (build 20260806-164108)

**Branche:** `skeuo-vector` · **Build:** `20260806-164108`
**Validation:** `cargo test` 256+160 OK, `build.ps1 -Install` OK.

Retours utilisateur sur la voix Buzz :
- **Waveform Sine/Square/Saw** : oscillateur tonal passé à un accumulateur de phase manuel + sélecteur `Wave` (dropdown, special index 11, famille Osc). Square/Saw = plus riche/buzzant (aliasing naïf assumé pour le grain lo-fi).
- **Smooth vs Razor désormais audibles** : le modèle « fraction de période » rendait les deux identiques (l'enveloppe retombait à ~0 chaque cycle). Redéfinis : **Smooth = trémolo cosinus** (montée/descente douce, Shape resserre le pulse) ; **Razor = spike exponentiel** re-déclenché de zéro chaque cycle (chop franc). Test `smooth_and_razor_differ`.
- **Pitch Sweep audible** : le sweep n'était re-déclenché qu'au *cold-start* → inaudible quand les queues se chevauchent en lecture. Désormais **re-déclenché à chaque trigger** (comme Perc1), profondeur portée à ~2 octaves, absorbé par le smoother de fréquence (anti-clic).
- **Volume** : défaut monté de 0.6 → **1.3** (le gate abaisse le niveau perçu).
- Tests ajoutés : `smooth_and_razor_differ`, `waveform_changes_the_source`.

## 2026-08-06 — Nouvel instrument « Buzz » : percussion tonale + gate rapide (build 20260806-161402)

**Branche:** `skeuo-vector` · **Build:** `20260806-161402`
**Validation:** `cargo test` 254+158 OK, `build.ps1 -Install` OK. Issu de l'investigation [93].

- **Nouvelle voix de synthèse « Buzz »** (15e kind, index voix 16, note MIDI 44) : percussion **tonale** (oscillateur sinus pitché + Pitch Sweep percussif) + **couche de bruit réglable** (montant + couleur White/Pink/Brown/Blue), le tout haché par un **gate/retrigger d'enveloppe rapide** — l'effet observé en [93] rendu délibéré et contrôlable.
- **Module gate** : un phasor à taux réglable (`Gate Rate` 1–150 Hz) ré-articule une `ExpDecayEnvelope` **courte** qui multiplie l'amplitude (reproduit l'« effondrement d'enveloppe »). `Gate Depth` = dry/wet, `Gate Shape` = durée/courbe du decay relatif à la période, `algo` Smooth (ramp) / Razor (from-zero). Chemin : source → gate → LP → amp env → saturation → DC → volume ; stéréo à phase de gate partagée.
- **Anti-clic** respecté : phase osc/gate reset au cold-start uniquement, enveloppes via setters (jamais recréées) dans `set_settings`, Freq/cutoff/depth lissés (`OnePoleSmoother`), `DcBlocker` en sortie, taux plafonné 150 Hz.
- **Contrôles data-driven** (registry) : Gate Rate/Depth/Shape (Env), Noise/Noise Type/Pitch Sweep (Osc), pack Saturation. `FULL_STD` en params standard, `filter_type_label "LP"`.
- Fichiers : `synthesis/buzz.rs` + `synthesis/settings/buzz.rs` (nouveaux) ; `DrumVoice::Buzz`, `TrackInstrumentKind::Buzz`, `TrackLayoutState::from_kinds`-compatible, `BUZZ_ALGOS`, entrée registry index 16, remap generator (emprunte le rôle Perc1), dropdown Type. **Aucune édition de `lib.rs`** (archi par slot).
- **Tests** : `buzz_settings_roundtrip`, `produces_sound_on_trigger`, `output_stays_finite_and_stops`, `gate_depth_modulates_the_output`.

## 2026-08-06 — Generator : anchors intouchables (fix four-on-the-floor) (build 20260806-124254)

**Branche:** `skeuo-vector` · **Build:** `20260806-124254`
**Validation:** `cargo test` 250+154 OK, `build.ps1 -Install` OK.

- **Fix « House/House n'a pas de rythme house »** : le kick était bien en anchors `[0,4,8,12]`, mais la règle de cohérence kick/snare de `generate_from_template` **retirait le kick** partout où le snare tombait aussi (temps 2 & 4) → il ne restait que 0 & 8. Les frappes fondatrices (anchors) étaient détruites.
- **Correctif** : les **anchors sont désormais sacrées**. `generate_from_template` traque un `is_anchor[inst][step]` ; les 3 règles de cohérence (kick/snare stacking, closed/open hat, suppression densité) ne touchent plus que les frappes **candidates** (probabilistes), jamais les anchors. Si kick ET snare sont anchors sur le même step (four-on-the-floor sous le backbeat) → les deux sont conservés.
- Bénéficie à **tous** les styles four-on-the-floor (House, Disco, Techno) et rend chaque template fidèle à sa définition ; strictement plus conservateur (ne peut qu'ajouter des frappes voulues, jamais en retirer).
- **Test** : `four_on_the_floor_kick_survives_backbeat_overlap` (House → kick présent sur 0/4/8/12).

## 2026-08-06 — Presets de style : kit de lanes adéquat + grooves authentiques (build 20260806-102824)

**Branche:** `skeuo-vector` · **Build:** `20260806-102824`
**Validation:** `cargo test` 249+154 OK, `build.ps1 -Install` OK.

- **Les 6 chips de preset (House/Dub/DnB/Bossa/Afro/Break) installent désormais un KIT de lanes adéquat** en plus de charger le groove — avant, les grooves plaquaient sur les 4 lanes par défaut et « ne ressemblaient pas ». Action **destructive** (remplace lanes + sons + grille), via `PersistentField::set(track_layout)` + `reset_slot_to_defaults` par lane (même chemin que le changement de Type d'un slot ; l'audio réinitialise les voix).
- **Kits + grooves authentiques** (par genre) :
  - **House** : Kick · Clap · HiHat · OpenHat · Perc — 4-on-floor, clap 2&4, open-hat contretemps, perc syncopée.
  - **Dub** : Kick · Snare(rim) · HiHat · 808 · Perc — one-drop (temps 3), 808 sub sur 1&3, skank offbeat.
  - **DnB** : Kick · Snare · HiHat · Snare606 · 808 — two-step, ghost snares 606, sub qui suit le kick.
  - **Bossa** : Kick · Snare(cross-stick) · Ride · HiHat · Perc — surdo, clave 3-2, ride comping, pedal hat 2&4, shaker.
  - **Afro** : Kick · Snare(rim) · HiHat · Ride(bell) · Tom(conga) · Perc — kick syncopé, cloche, congas, shaker.
  - **Break** : Kick · Snare · HiHat · Snare606 · Ride — kick cassé, hats 16èmes funky, ghost snares.
- Nouveau `TrackLayoutState::from_kinds(&[kinds])` (active slots 0..n, choke 1 sur HH/OH) ; grooves `pattern.rs` réécrits pour l'ordre de lane de chaque kit ; test `new_style_presets_are_well_formed` mis à jour (hits ≥ 8, lanes dans la taille du kit).
- Rock/Funk/Disco/⟳ Random inchangés (grille seule).

## 2026-08-06 — Presets fixes pour les 6 nouveaux styles (build 20260806-091408)

**Branche:** `skeuo-vector` · **Build:** `20260806-091408`
**Validation:** `cargo test` 249+154 OK, `build.ps1 -Install` OK.

- **6 chips de preset fixe** ajoutés à la rangée **« Presets »** du bottom panel (à côté de Rock / Funk / Disco) : **Bossa, House, DnB, Afro, Dub, Break**. Comme les presets existants, ce sont des grooves **déterministes** (toujours identiques, contrairement à GENERATE qui tire un seed aléatoire).
- Nouveaux constructeurs `Pattern::{bossa,house,dnb,afrobeat,dub,breakbeat}_pattern()` dans `sequencer/pattern.rs` : grooves signature répétés par bar, n'utilisant que les **4 lanes cœur** (0 Kick, 1 Snare, 2 HiHat, 3 Open HH) — comme Rock/Funk/Disco — pour sonner sur le layout par défaut sans dépendre des slots aux.
  - House = four-on-the-floor + open-hat offbeat · DnB = kick two-step + snare 2&4 + ghosts · Dub = one-drop (beat 3) + skank offbeat · Bossa = surdo + cross-stick clave · Afrobeat = kick syncopé + hats busy · Breakbeat = kick cassé + hats funky.
- **Test** : `new_style_presets_are_well_formed` (kick + hats présents, uniquement lanes 0-3).

## 2026-08-05 — [148] Generator : +6 styles (16 au total) (build 20260805-175817)

**Branche:** `skeuo-vector` · **Build:** `20260805-175817`
**Validation:** `cargo test` 248+153 OK, `build.ps1 -Install` OK.

- **6 nouveaux styles** ajoutés à la palette du Generator (dropdowns Style A / B) → **16 styles** au total : **Bossa Nova, House, Drum'n'Bass, Afrobeat, Dub, Breakbeat** (en plus de Rock, Funk, Techno, Hip-Hop, Jazz, Metal, Latin, Disco, Trap, Reggae).
- Chaque style = un `MusicalTemplate` (rôles rythmiques des 14 instruments : anchors / candidates / prob / exclusions + plage BPM), écrit d'après le rythme caractéristique du genre : four-on-the-floor + open-hat offbeat (House), one-drop + gros sub (Dub), amen/ghost-snares (DnB, Breakbeat), clave 3-2 cross-stick (Bossa), tresillo + bell + percussions (Afrobeat).
- **Zéro édition UI** : les dropdowns sont pilotés par `Style::variants()`, ajouter un variant suffit.
- **Persistance** : nouveaux variants ajoutés à la fin de l'enum → indices 0-9 (styles existants) inchangés, les vieilles sessions rechargent leur style correctement.
- **Tests** : `every_style_template_is_well_formed` (garde : steps dans la page 0-15, prob ∈ [0,1], BPM sain, sur les 16 styles) ; les 6 nouveaux ajoutés à `hihat_roles_are_style_specific`.

## 2026-08-04 — Fix cellule fantôme après un tap rapide (step-drag) (build 20260804-224719)

**Branche:** `skeuo-vector` · **Build:** `20260804-224719`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK.

- **Bug** : après un clic bref sur une cellule active, une cellule « fantôme » avec marqueur jaune apparaissait parfois et suivait la souris.
- **Cause** : le déplacement de cellule est un *long-press* (~0,5 s). Le traitement du **relâchement** (`any_released`) était imbriqué dans `if drag.active` — donc un appui **relâché avant le seuil de 0,5 s** ne nettoyait jamais `state.step_drag`. Le compteur de temps continuait aux frames suivants et le drag « s'activait » ~0,5 s **après** le relâchement (bouton déjà relevé) → cellule fantôme qui suit le curseur jusqu'au prochain clic.
- **Fix** : le test de relâchement est sorti de `if drag.active`. Un relâchement termine **toujours** le geste : s'il n'a jamais franchi le seuil (tap rapide) il annule proprement le drag en attente et retombe sur un clic/toggle normal ; s'il était actif il applique le déplacement comme avant.

## 2026-08-04 — Samplers 606 : Attack en millisecondes absolues (build 20260804-174450)

**Branche:** `skeuo-vector` · **Build:** `20260804-174450`
**Validation:** `cargo test` 247+153 OK, `build.ps1 -Install` OK.

- **Fix Attack inutilisable sur les samples courts** (signalé sur CH6smp) : l'Attack des samplers 606 était une **fraction de la durée du sample** (`attack × played_secs`). Sur un charleston fermé (transitoire ~3,5 ms), même une petite fraction montait pendant que le sample était déjà éteint → au lieu d'un fondu d'attaque musical, ça **écrasait/effaçait** le son (fraction 0,1 → 50 ms → pic ÷5 ; 0,5 → 250 ms → quasi silence).
- **Nouveau modèle** : Attack = temps de montée **ABSOLU**, `attack × MAX_AMP_ATTACK_SECS` (**80 ms** plein-échelle), indépendant de la longueur du sample. Défaut 0,001 → ~0,08 ms (plein transitoire, inchangé). Appliqué aux **3 samplers** (BD6smp/SD6smp/CH6smp) pour rester cohérent.
- **Non touché** : le **Decay** et le **Filter Decay** restent des fractions de la durée jouée (ils suivent le sample, ce qui a du sens ; seule l'attaque posait problème).
- ⚠️ **Compat sessions** : les valeurs Attack **non par défaut** sauvegardées sur BD6smp/SD6smp sont réinterprétées (0–1 → 0–80 ms au lieu de 0–durée). Voix ajoutées le 2026-08-02, impact minime.
- **Tests** : `amp_decay_tracks_length_attack_is_absolute` (×3) — prouve que le decay suit le pitch/longueur mais que l'attack reste absolu.

## 2026-08-04 — CH6smp : sampler Closed Hi-hat TR-606 (build 20260804-170126)

**Branche:** `skeuo-vector` · **Build:** `20260804-170126`
**Validation:** `cargo test` 153 OK (dont `ch606::output_stays_finite_and_stops`), `build.ps1 -Install` OK.

- **Nouvelle voix `CH6smp`** (14e slot) : sampler à pitch relatif calqué sur `bd606`/`sd606`, alimenté par `wav/CH.wav` (copié en `assets/ch606.wav`, embarqué via `include_bytes!`).
  - Enum : `DrumVoice::Ch606` (index 15, `COUNT = 16`) + `TrackInstrumentKind::Ch6smp` (index 13, `COUNT = 14`).
  - **Note MIDI 42** (Closed Hi-hat GM), label `c6`, rôle générateur = **HiHat**.
  - Registre : 16e `InstrumentDef` (`SMP606_STD` + specials `ch606_*`, `algo_count = 1`, `filter_type_label "LP"`, root legacy 8000 Hz).
  - Banque d'échantillons : `sample_bank::ch606()` (`OnceLock`, 8 hits), pré-chauffée au `new()` du synthé ; graine RNG distincte (`0x6060_0003`).
  - UI (`sound_editor`) : la voix apparaît dans le dropdown Type, hérite du graphe waveform + End/Start/pitch des voix smp (sites `13|14` → `13|14|15`, sélection de banque 3-way).
  - `reset_specials_for_voice` : marqueur pitch (`special[10] = 1.0`) posé pour la voix 15 comme pour 13/14.
- **Tests** : `synthesis::ch606::tests::output_stays_finite_and_stops`.

## 2026-08-04 — [151] Linker 2 lanes adjacentes (layering) (build 20260804-121207)

**Branche:** `skeuo-vector` · **Build:** `20260804-121207` (fixes) · initial `20260804-105255`
**Validation:** `cargo test` 377 OK, `build.ps1 -Install` OK. **Validé en S1** (fonctionne).

**Fixes 121207 :**
- **Indicateur de lien** refait : la ligne+point qui dépassait de la poignée (effet « bug GUI ») remplacée par une **bande d'accent bleue 2px** propre sur le bord gauche de la lane.
- **Rafraîchissement après suppression de fusion** : l'éditeur est réactif (pas de repaint continu à l'arrêt) → supprimer une fusion (bouton « Del ») laissait la cellule affichée fusionnée. Ajout de `mark_pattern_dirty()` + `ctx().request_repaint()` sur le « Del » et sur les éditions de grille (toggle step/fusion).

- **Layering** : une lane peut « linker » celle **juste au-dessus** → elle partage ses **steps + fusions** (même rythme) tout en gardant **son propre son, plocks, algo, routing, note MIDI, mute/solo, Hum/Push/Len**.
- **Modèle** : nouveau champ `TrackSlot.linked_up` (`#[serde(default)]` → migration transparente) + `AtomicTrackLayout.slot_linked` (lu par l'audio). Helper central **`grid_slot(slot)`** qui remonte la chaîne de liens jusqu'au maître actif (C→B→A) ; le lien se **rompt** si le maître devient inactif. Adjacence garantie (« lien vers le haut »).
- **Audio** (`sequencer`) : nouveau `set_grid_slots` (calculé/bloc depuis le layout atomique) ; les lectures de step + fusion passent par `grid_slots[slot]` (mute/timing restent par lane).
- **UI** (`grid.rs`) : la lane linkée **affiche et édite** les steps/fusions du maître (édition bidirectionnelle) ; menu clic-droit **« Link steps to lane above » / « Unlink steps »** ; indicateur visuel (barre + point bleu dans la poignée).
- **Tests** : `grid_slot_resolves_link_chain_to_active_master`, `linked_up_defaults_false_and_survives_layout_roundtrip`.
- ⚠️ **Limites v1** : le **morph** des fusions reste appliqué au maître (la lane linkée joue le rythme fusionné avec son propre son statique) ; le reorder de lane ne réajuste pas les liens.

## 2026-08-02 — [83] Pitch Fine sous Pitch + fix fusion : actions restaurées en mode Sequencer P-Lock (build 20260802-212008)

**Branche:** `skeuo-vector` · **Build:** `20260802-212008`
**Validation:** `cargo test` 377 OK, `build.ps1 -Install` OK.

- **Pitch Fine déplacé directement sous le slider Pitch** dans la section OSC des voix smp (était après Sample/Start) ; retiré de la boucle des special params pour ces voix.
- **Fix régression fusions (depuis [145])** : en mode **Sequencer P-Lock**, le clic droit sur une cellule fusionnée n'affichait que le menu seq (solo/proba/stutter/…) — **Morphing / Edit Fusion Steps / Delete Fusion étaient devenus inaccessibles** (ils n'existaient plus qu'en mode Sound). Le bloc fusion est extrait en helper `draw_fusion_group_menu` et affiché **au-dessus du menu seq** dans les deux modes ; le sous-menu Morph y est aussi accessible (`popup.morph_menu` honoré en mode Sequencer).
- **Test d'intégration** : `bd606_fine_tune_changes_playback_rate_through_synthesizer` (prouve que Pitch Fine agit sur la vitesse de lecture via DrumSynthesizer — l'effet s'entend au **prochain trigger**, pas sur le hit en cours).

## 2026-08-02 — [83] Fix pitch smp : snapping par pas de 1 DANS le widget slider (build 20260802-204021)

**Branche:** `skeuo-vector` · **Build:** `20260802-204021`
**Validation:** `cargo test` 375 OK, `build.ps1 -Install` OK.

- Le snapping par pas de 1 semitone de la build précédente était appliqué **après** le dessin du slider → pendant le drag, l'affichage restait fractionnaire et le pas de 1 était invisible. Le pas est maintenant géré **dans le widget track** (`TrackStyle.step` + `with_step`, utilisé via `draw_editor_slider_row_full`) : la valeur ET l'affichage snappent à l'entier en temps réel pendant le drag. Comportement continu inchangé pour tous les autres sliders (`step = 0.0`).
- Rappel : **Pitch Fine** (±100 = ±1 semitone) et **End** sont dans la section OSC du Sound Panel (sous Sample/Start), déjà présents depuis la build `20260802-202744`.

## 2026-08-02 — [83] Lanes smp : pitch par pas de 1, param End, graphes waveform croppés (build 20260802-202744)

**Branche:** `skeuo-vector` · **Build:** `20260802-202744`
**Validation:** `cargo test` 375 OK, `build.ps1 -Install` OK.

- **Pitch par pas de 1 semitone** : le slider Pitch des voix smp snappe à l'entier (le special **Pitch Fine** couvre les cents, ±100 = ±1 semitone ; label renommé pour clarifier qu'il s'applique au pitch).
- **Nouveau paramètre End** (`special[11]`, fraction 0..1, défaut 1.0) : la lecture du sample s'arrête à End, symétrique de Start. Les anciennes sessions (blob sans End, `special[11] = 0`) jouent le sample entier — garde-fou legacy côté voix via le marqueur pitch + seeding à 1.0 lors de la migration UI.
- **Graphes waveform rework** (`ui/envelope_viz.rs`) : la waveform est **croppée sur [Start, End]** (les parties offsetées ne sont plus dessinées du tout), barres plus larges et plus lumineuses (64 colonnes, stroke 2). Les courbes amp/filtre sont mappées sur l'axe de la région jouée.
- **Fix courbe env filter tronquée** : après le sweep, la courbe est **tenue au cutoff** jusqu'au bord droit — la fin de la courbe reste visible (elle s'arrêtait net en plein graphe). Idem amp : la courbe atterrit visiblement sur la baseline.
- **Tests** : `end_truncates_playback` (×2 voix), `legacy_settings_without_end_play_the_full_sample`, roundtrips settings couvrant le nouveau champ.

## 2026-08-02 — [83] Lanes smp : pitch relatif ±24 + fine, graphes waveform, env 100 % relatives (build 20260802-200054)

**Branche:** `skeuo-vector` · **Build:** `20260802-200054`
**Validation:** `cargo test` 369 OK, `build.ps1 -Install` OK.

- **Pitch relatif** sur BD6smp/SD6smp : le slider devient **-24/+24 semitones** (0 = pitch natif) + nouveau special **Fine** (-100/+100 cents, `special[9]`). Le mode Hz/Notes est retiré pour ces voix (listes `is_bass_drum` revenues à Kick/B8).
- **Env amp entièrement relative** : **Release et Release Curve retirés** des tables smp ; **Attack** et **Decay** sont désormais des fractions 0..1 de la **durée jouée** (longueur du sample ÷ pitch, recalculées à chaque trigger), comme Filter Decay déjà.
- **Graphes waveform** (`ui/envelope_viz.rs`) : les sections Envelope et Filter des lanes smp affichent la **waveform du sample sélectionné** (normalisée) avec la zone **Start** grisée + ligne amber, et la courbe superposée — env amp bleue (ligne pleine en One Shot), sweep filtre orange + ligne de cutoff. Les autres voix gardent les graphes ADSR/classiques.
- **Migration du pitch Hz → semitones** (sessions des builds d'hier) : marqueur `special[10]` (1 = format relatif, 0 = legacy Hz, caché dans les 32 specials sans changer le blob `sound-settings-v2`). La voix **comprend** l'ancien format (ratio = freq/60 Hz BD, /200 Hz SD) → le son reste correct même sans ouvrir l'UI ; l'ouverture du Sound Panel **commit** la conversion en semitones. Nouvelles lanes et resets seedés à 1.
- **Reset sliders smp** : le double-clic revient aux defaults du registry (`0.0` semitone) et plus au `60 Hz` générique.
- **Tests** : `fine_tune_adds_cents_to_relative_pitch`, `amp_times_track_the_played_sample_length` (×2 voix), `legacy_hz_pitch_keeps_native_rate` (×2), `multisample_defaults_mark_relative_pitch_format`, test `default_frequency_is_nonzero` devenu data-driven (exempte les pitch relatifs à range négative).

## 2026-08-02 — [83] Nouvel instrument SD6smp : SD 606 multisamplée ×8 (build 20260802-165023)

**Branche:** `skeuo-vector` · **Build:** `20260802-165023`
**Validation:** `cargo test` 358 OK, `build.ps1 -Install` OK.

- **2ᵉ instrument multisample** : `TrackInstrumentKind::Sd6smp` (label grille `s6`). 8 coups de la SD d'une TR-606 embarqués (`assets/sd606.wav`, float32 mono 44,1 kHz, 4 s = 8 × 0,5 s). Même moteur de lecture que BD6smp : Analog Mode (random sans répétition / sample fixe 1-8), Pitch (200 Hz = natif, plage 50-1000), env amp, env filtre relative à la durée jouée, One Shot, Start relatif, pack saturation.
- **Sample bank généralisée** (`sample_bank.rs`) : `bd606()` / `sd606()` (deux `OnceLock`), décodeur partagé `load_bank(bytes)` — ajouter un prochain instrument = 1 WAV + 1 accesseur.
- **Enregistrement** : `DrumVoice::Sd606 = 14` (COUNT 15), `DrumVoiceKind::Sd606` (9 matchs + `create_voice_for_kind`), `TrackInstrumentKind::Sd6smp = 12` (COUNT 13, fin d'enum), registry `INSTRUMENTS[14]` (note MIDI 40). Persistance `sound-settings-v2` inchangée.
- **Générateur** : la lane SD6smp emprunte le rôle Snare.
- **UI** : les rendus spéciaux multisample (switch Analog Mode / One Shot, liste Sample grisée quand Analog ON) sont généralisés par suffixe de nom (`_analog_mode`, `_one_shot`, `_sample`) → automatiques pour les prochains instruments 606.
- **Tests** : décodage bank SD (8 hits de 22050 samples), roundtrip settings, voix (son, finie, silence final), sample fixe bit-identique, random sans répétition (×64), Start relatif, Filter Decay relatif (0,25 s natif / 0,125 s à l'octave), registry mono.

## 2026-08-02 — [83] BD6smp : Sample grisé (pas masqué), Start & Filter Decay relatifs au sample (build 20260802-163241)

**Branche:** `skeuo-vector` · **Build:** `20260802-163241`
**Validation:** `cargo test` 342 OK, `build.ps1 -Install` OK.

- **Sample grisé au lieu de masqué** quand Analog Mode est ON (`add_enabled_ui(false)`) → le bas du Sound Panel ne se décale plus au toggle.
- **Start relatif à la longueur du sample** : le paramètre devient une fraction 0..1 du coup sélectionné (était 0-0,5 s absolus) — 0,5 = démarrage au milieu du sample, quel que soit le pitch.
- **Filter Decay relatif à la longueur du sample** : le paramètre devient une fraction 0,01..1 de la **durée jouée** (longueur du sample ÷ pitch, recalculée à chaque trigger) — le balayage filtre suit le hit à tout pitch. Défaut 0,15.
- **Tests** : `start_offset_is_a_fraction_of_the_sample_length` (lecture démarre bien au milieu du hit), `filter_decay_tracks_the_played_sample_length` (0,5 × 1 s à pitch natif, 0,25 s à l'octave).

## 2026-08-02 — [83] BD6smp : Analog Mode en switch + liste Sample conditionnelle (build 20260802-161718)

**Branche:** `skeuo-vector` · **Build:** `20260802-161718`
**Validation:** `cargo test` 338 OK, `build.ps1 -Install` OK.

- **Analog Mode** et **One Shot** rendus comme des **switches** (étaient des sliders 0-1) dans le Sound Panel.
- **Sample** rendu comme une **liste 1-8** (était un slider) et **masqué tant qu'Analog Mode est ON** — il n'a de sens qu'en mode sample fixe. La valeur reste persistée quand la liste est cachée.

## 2026-08-02 — [83] Nouvel instrument BD6smp : BD 606 multisamplée ×8 (build 20260802-160117)

**Branche:** `skeuo-vector` · **Build:** `20260802-160117`
**Validation:** `cargo test` 338 OK, `build.ps1 -Install` OK.

- **Premier instrument à base de multisample** : `TrackInstrumentKind::Bd6smp` (label grille `B6`). 8 coups de la BD d'une TR-606 embarqués (`assets/bd606.wav`, float32 mono 44,1 kHz, 8 s = 8 × 1 s) pour reproduire la variabilité analogique hit-to-hit.
- **Sample bank** (`synthesis/sample_bank.rs`) : WAV embarqué via `include_bytes!`, décodé une fois dans un `OnceLock` global (pré-chauffé dans `initialize_with_layout`, zéro alloc sur le thread audio), split égal 8 × 44 100 samples. **Pas de resampling au chargement** : la lecture à position fractionnaire (interpolation linéaire) absorbe le ratio source/session.
- **Paramètres** (Sound Panel, data-driven) :
  - **Analog Mode** (special, défaut ON) : ON = tirage aléatoire **sans répétition immédiate** parmi les 8 coups (RNG xorshift seedé à la construction, jamais reseedé au trigger) ; OFF = toujours le même coup, choisi par **Sample** (1-8).
  - **Pitch** (slider Freq, 20-500 Hz, 60 Hz = pitch natif) = vitesse de lecture ; mode Hz/Notes actif (bass drum).
  - **Env amp** standard (Attack/Decay/Curve/Release) — contournée en mode **One Shot** (joue le sample entier).
  - **Env filtre** additive sur LP one-pole (Filter/Filter Env/Filter Decay).
  - **Start** (offset 0-0,5 s), **pack saturation** complet (5 params, routage `process_at`, volume post-sat).
- **Enregistrement** : `DrumVoice::Bd606 = 13` (COUNT 14) + `DrumVoiceKind::Bd606` (9 matchs + `create_voice_for_kind`) + `TrackInstrumentKind::Bd6smp = 11` (COUNT 12, ajouté **en fin d'enum** pour la compat `track-layout-v1`) + entrée registry `INSTRUMENTS[13]` (note MIDI 41).
- **Persistance intacte** : les longueurs legacy de `sound-settings-v2` sont gelées sur 13 voix via la nouvelle const `LEGACY_VOICE_COUNT` (ne plus jamais utiliser `DrumVoice::COUNT` là — il grandit désormais).
- **Générateur** : la lane BD6smp emprunte le rôle Kick (sinon silencieuse sur GENERATE).
- **UI** : dropdown Type (Track tab) + popup Add Module ; listes analog-fixed et is_bass_drum (Sound Panel + plock) étendues à l'index 13.
- **Doc** : `ADDING_AN_INSTRUMENT.md` réécrit pour l'architecture **modulaire** réelle (3 enums, `reinitialize_slot`, persistance par slot) — l'ancienne version décrivait encore les voix fixes.
- **Tests** : décodage bank (8 hits non vides, attaques présentes), roundtrip settings, voix (son produit, finie, silence final), sample fixe bit-identique, random sans répétition (×64), pitch raccourcit la durée, one-shot ignore l'env amp, registry mono.

## 2026-08-02 — [145] Solo par-step/fusion finalisé + fix step-drag fantôme (build 20260802-133013)

**Branche:** `skeuo-vector` · **Build:** `20260802-133013`
**Validation:** `cargo test` 320 OK, `build.ps1 -Install` OK. **Validé en Studio One** (solo OK, plus de fantôme, plus de crash projet vide).

- **Sémantique du solo finale = par-step / span de fusion** : le solo mute les autres lanes **uniquement pendant que la tête de lecture est sur la cellule soloée** (1 step, ou toute la durée d'une cellule fusionnée) ; hors de cette fenêtre tout rejoue. Pour soloer sur plusieurs steps → fusionner la cellule. `SequencerPlockState::solo_window(fusion_span_len)` + gating `solo_window.bit(step) && !is_solo(slot,step)`. Toggle « Solo » **par cellule** ; désactiver **efface le seq-plock** s'il ne reste aucun autre param (proba/stutter/condition/micro).
- **Fix bug « plock fantôme à un endroit aléatoire »** (racine réelle, trouvée au trace) : le **clic-droit** qui ouvre le popup démarrait aussi un **step-drag** (`response.is_pointer_button_down_on()` est vrai pour n'importe quel bouton) ; lire le menu >0,5 s activait le drag, et cliquer un contrôle du popup le **relâchait** → **déplacement silencieux** du step (avec son solo/plock) vers la position du popup. Corrigé : le step-drag ne démarre que sur le bouton **primaire** et jamais quand `plock_popup.is_some()`. Le déplacement légitime (glisser gauche) emporte toujours correctement le solo (via `get`/`set`).
- **Crash à l'instanciation sur projet vide** (apparu pendant le dev [145]/[149]) : **disparu** avec le retour au per-step + le fix du drag. **Traçage diagnostic entièrement retiré** (`diag_log`, `install_panic_logger`, compteurs process, marqueurs paint). Si le crash réapparaît → bisect [149] vs [145] (la base `20260801-201613` ne crashait pas).

## 2026-08-01 — [145] Solo de lane retirable depuis n'importe quelle cellule (build 20260801-180017)

**Branche:** `skeuo-vector` · **Build:** `20260801-180017`
**Validation:** `cargo test` OK, `build.ps1 -Install` OK.

- **Fix « impossible de retirer le solo »** : le solo étant par cellule mais l'effet sur toute la lane, retirer le solo depuis une autre cellule (toggle off) ajoutait en fait un 2ᵉ solo. Le toggle **« Solo (lane) »** reflète désormais l'état de la **lane** (`lane_soloed`) et non de la cellule ; le désactiver appelle `clear_lane_solo(slot)` qui efface **tous** les bits solo de la lane → retirable depuis n'importe quelle cellule.
- Test `clear_lane_solo_removes_every_soloed_cell_in_the_lane`.

## 2026-08-01 — [145] Solo de lane (seq-plocks) — solo pattern entier (build 20260801-174923)

**Branche:** `skeuo-vector` · **Build:** `20260801-174923`
**Validation:** `cargo test` 205 OK, `build.ps1 -Install` OK.

- **Nouveau toggle Solo** dans les p-locks séquenceur, par cellule. Sémantique : **dès qu'une cellule est Solo, sa lane joue seule sur TOUT le pattern** — toutes les autres lanes sont muettes. Plusieurs cellules solo dans des lanes différentes → ces lanes jouent ensemble, le reste muet. Indépendant du tag S de lane. *(Première itération 20260801-171811 était un solo par step, jugé trop étroit ; refait en solo de lane pattern-entier.)*
- **Modèle** (`plock.rs`) : `SequencerStepParams.solo` + `SequencerPlockState.solo_masks` (bitmask lock-free 1 bit/step). `set_solo`/`is_solo`, `lane_soloed(slot)` / `any_lane_soloed()`. Activer Solo marque la cellule seq-plock active.
- **Audio** (`lib.rs`) : `any_solo` calculé une fois par bloc (RT-safe) ; gating au trigger : `if any_solo && !lane_soloed(slot) → skip`.
- **Persistance** (`pattern_bank.rs`) : `solo_masks` **appended** en fin de `seq_plock_bytes` → vieux blob `pattern-bank-v1` rechargé tel quel avec solo=false (lecture conditionnée à la longueur). Les 2 chemins de restore mis à jour. Copy/paste lane + reorder de lane emportent le solo.
- **UI** : toggle **Solo** dans le menu seq-plock (violet, après « Mode ») ; marqueur **« S »** en coin haut-gauche des cellules solo (mode séquenceur).
- **Tests** : `lane_solo_marks_whole_lane_and_any_solo`, `set_solo_marks_active_and_roundtrips`, `clear_resets_solo`, `seq_plock_solo_survives_capture_restore`, `seq_plock_legacy_blob_without_solo_defaults_false`.

## 2026-08-01 — [149] Pattern Bank étendue à 16 slots (build 20260801-154337)

**Branche:** `skeuo-vector` · **Build:** `20260801-154337`
**Validation:** `cargo test` 314 OK, `build.ps1 -Install` OK.

- **`SLOT_COUNT: 8 → 16`** — la Pattern Bank passe de P1-P8 à P1-P16.
- **Migration sans perte** : `PatternBank::slots` (`[PatternSlot; 16]`) reçoit un `deserialize_with` **tolérant en longueur**. Une vieille session `pattern-bank-v1` à 8 slots remplit P1-P8 et laisse P9-P16 vides, au lieu de l'échec `from_slice` actuel qui rechargeait un bank **vide** (perte silencieuse de tous les patterns sauvegardés). La clé JSON et le `VST3_CLASS_ID` sont inchangés.
- **UI** (`ui/pattern_bank.rs`) : loop `0..8` → `0..SLOT_COUNT`, slots 30→26 px + espacement resserré (3 px) → 16 slots sur **une seule rangée** (hauteur inchangée, zones stables OK) ; Export/Drag restent collés à droite.
- **MIDI pattern-switch** (`lib.rs`) : notes 60-75 (au lieu de 60-67) → P1-P16, via `SLOT_COUNT`.
- Libellés « P1-P8 » → « P1-P16 » (UI + docstrings). `song.rs` utilisait déjà `SLOT_COUNT` → dropdown des blocks auto-adapté à 16.
- **Tests** : `pattern_bank_migrates_8_slot_blob_to_16` (P1-P8 préservés, P9-P16 vides) + `pattern_bank_roundtrips_16_slots`.

