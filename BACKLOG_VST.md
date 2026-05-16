# Backlog VST

## Etat courant - 2026-05-16

- OK: correctif ajoute pour resynchroniser le pattern joue depuis les parametres restaures par l'hote.
- OK: ouverture de l'editeur et debut de `process()` resynchronisent la grille atomique depuis les 16 parametres de pas.
- OK: diagnostic Studio One confirme que l'etat VST3 restaure les parametres classiques (`master_vol`, `bpm`).
- OK: la grille est maintenant sauvegardee dans le champ persistant `pattern-v1`, directement depuis `SharedPattern`.
- OK: migration ajoutee depuis les anciens parametres caches `st01` a `st16` vers `pattern-v1`.
- OK: wrapper VST3 vendore corrige pour sauvegarder/restaurer le meme etat cote `IEditController` et `IComponent`.
- OK: `cargo test` passe avec 16 tests.
- OK: `build.ps1 -Install` installe le bundle systeme dans `C:\Program Files\Common Files\VST3\drum-pattern-vst.vst3`.
- A valider dans Studio One: sauvegarde/reouverture d'un projet avec pattern modifie.

Build installe courant:

- Build UI: `20260516-205054`
- VST3 class ID: `DrumFlashPlugin1`
- Binaire installe: `C:\Program Files\Common Files\VST3\drum-pattern-vst.vst3\Contents\x86_64-win\drum-pattern-vst.vst3`
- Timestamp: `2026-05-11 09:13:16`
- SHA-256: `62AA5FCC445FEFDBC1E30196E614BCAED53A61C9F9EB2AB9BD5A4E1C5C510CEF`
- 12 instruments dont B8 (TR-808 Bass Drum)
- Mix Bus checkbox par instrument
- Plock fix (special params uniquement au trigger)
- Parameter locks 14 champs (12 sound + clap_echo + algo)

## Etat valide fin de session - 2026-05-10

- OK: plugin charge dans Studio One.
- OK: GUI visible.
- OK: numero de build affiche dans l'interface.
- OK: son valide.
- OK: edition de pattern en temps reel.
- OK: presets Rock, Funk et Disco branches.
- OK: mutes/solos par instrument.
- OK: sync transport DAW validee: play, stop, tempo, repositionnement.
- OK: sequenceur corrige pour eliminer les notes fantomes HH.
- OK: multi-out Studio One fonctionnel et activable.
- OK: chaque voix sort sur son bus stereo dedie en plus du Main Mix.
- OK: `cargo test` passe avec 14 tests.
- OK: `build.ps1 -Install` installe le bundle systeme dans `C:\Program Files\Common Files\VST3\drum-pattern-vst.vst3`.
- OK: seule la copie systeme attendue du VST3 est presente dans `C:\Program Files\Common Files\VST3`.

Build valide:

- Build UI: `20260510-170819`
- VST3 class ID: `DrumPatternVst03`
- Binaire installe: `C:\Program Files\Common Files\VST3\drum-pattern-vst.vst3\Contents\x86_64-win\drum-pattern-vst.vst3`
- Timestamp: `2026-05-10 17:08:36`
- SHA-256: `B318B0FE662C3D39951B8AB9B515AC320AE7D0C4B9F0CF6AB8CC2729CC8BFF6A`

## P0 - Stabilisation obligatoire

- OK: patch du checkout Cargo `nih-plug` remplace par `drum-pattern-vst/vendor/nih-plug`.
- OK: patch VST3 Studio One documente dans `drum-pattern-vst/README.md` et `STUDIO_ONE_MULTI_OUT.md`.
- OK: numero de build visible dans l'UI pour verifier le binaire charge par le DAW.
- OK: les nouveaux builds utilisent un ID horodate sans suffixe diagnostic `s1main`/`s1route`.
- OK: `VST3_CLASS_ID = DrumPatternVst03` conserve comme ID permanent V1.
- OK: `.\build.ps1 -Install` revalide apres vendoring du patch.

## P1 - Validation produit

- Tester le multi-out dans au moins un autre DAW que Studio One.
- Tester un projet Studio One neuf:
  - insertion du plugin
  - activation des sorties Kick/Snare/HH/Open HH/Toms
  - audio sur chaque bus
  - sauvegarde/reouverture du projet.
- Tester un projet Studio One existant avec `VST3_CLASS_ID = DrumPatternVst03`.
- Verifier que le Main Mix reste audible quand les sorties separees sont activees.
- Verifier que mutes/solos affectent correctement Main Mix et sorties separees.
- OK: procedure de regression multi-out ajoutee dans `TEST_PLUGIN.md`.

## P2 - Fonctionnalites V1 restantes

- OK: Mix Bus checkbox par instrument (Main Mix on/off indépendant du Mute).
- OK: Plock echo fonctionnel (clap_echo plockable par step).
- OK: Masquage des paramètres inutiles dans le Sound Panel par instrument.
- OK: Nouvel instrument B8 (TR-808 Bass Drum) avec accent/snap/pitch_drop.
- OK: Special params propagés uniquement au trigger (fix écrasement buffer suivant).

- Finaliser les reglages de synthese par instrument.
- Ajouter export MIDI depuis le plugin.
- Ajouter sortie MIDI temps reel.
- Ajouter swing.
- Ajouter un facteur de groove parametrable.
- OK: B8 — slider Analog actif (pitch smoothing + freq variation).
- OK: B8 — slider Release actif (DecayReleaseEnvelope).
- OK: B8 — Pitch Drop label + accélération du drop.
- Ajouter un parametre analogique pour legeres variations aleatoires.
- Permettre un mode stereo analogique avec variation gauche/droite.
- Ajouter song mode (placeholder UI OK, backend à faire).
- Refonte UI Phase 1 (grid intégré, sound panel ongleté, auto-edit).
- Per-instrument stereo toggles + stereo Snare 606.
- Filter envelope sur Kick, Snare, Tom, HiHat, Snare 606.

## Dette technique

- Eviter la divergence entre fichiers de documentation.
- Clarifier si `index.js` doit etre conserve ou archive.
- Revoir l'organisation du repo pour separer clairement PoC web et plugin.
- Ajouter un chemin de build/test standard documente.
- OK: warnings Rust inutiles reduits sans supprimer les API reservees pour les prochaines fonctions.
- Garder les fichiers de sauvegarde hors de `src/`; `src/lib.rs.backup` a ete supprime le 2026-05-10.

## Notes techniques Studio One

Le multi-out Studio One a ete debloque par le patch `nih-plug` vendore dans:

```text
drum-pattern-vst/vendor/nih-plug/src/wrapper/vst3/wrapper.rs
```

Points requis:

- `get_unit_by_bus()` retourne le root unit pour les bus audio/event valides.
- `set_bus_arrangements()` accepte les activations progressives de sorties.
- `set_bus_arrangements()` accepte `num_ins == 0` avec pointeur d'entree audio nul.
- La validation audio ignore les sorties auxiliaires non activees.
- `getRoutingInfo()` relie l'entree event/MIDI de l'instrument a la sortie audio principale.
- `IEditController::get_state()`, `set_state()` et `set_component_state()` partagent le meme etat que `IComponent`.
- Le build valide utilise des sorties drum annoncees `kMain` + `kDefaultActive`.

Le dernier point teste comme decisif est `getRoutingInfo()`: sans lui, Studio One affichait les sorties mais les gardait grisees.
