# Mise a jour - 2026-05-11

## Etat de fin de session

Le plugin VST3 Rust est installe avec le build:

```text
20260511-091259
```

Le numero de build est affiche dans l'interface du plugin pour verifier que Studio One charge bien le binaire attendu.

## Corrections importantes

- Sync DAW validee: play, stop, tempo, repositionnement.
- Interface egui utilisable avec grille 16x7 editable.
- Presets Rock, Funk et Disco branches.
- Mutes et solos par instrument branches.
- Multi-out Studio One fonctionnel.
- Diagnostic de sauvegarde/reouverture Studio One effectue.
- Parametres classiques (`master_vol`, `bpm`, mutes/solos) sauvegardes/restaures.
- Probleme de grille isole: les anciens parametres caches `st01` a `st16` ne changeaient pas lors des clics.
- Grille maintenant sauvegardee via le champ persistant `pattern-v1`, directement depuis `SharedPattern`.
- Migration ajoutee depuis les anciens etats `st01` a `st16` vers `pattern-v1`.

## Correctif multi-out Studio One

Le multi-out a ete debloque par le patch `nih-plug` vendore dans:

```powershell
drum-pattern-vst\vendor\nih-plug\src\wrapper\vst3\wrapper.rs
```

Points requis pour Studio One:

- `get_unit_by_bus()` doit retourner le root unit pour les bus audio/event valides.
- `set_bus_arrangements()` doit accepter les activations progressives des sorties.
- `set_bus_arrangements()` doit accepter un pointeur d'entree audio nul quand `num_ins == 0`.
- La validation des buffers doit ignorer les sorties auxiliaires non activees.
- `getRoutingInfo()` doit relier l'entree event/MIDI instrument a la sortie audio principale.
- L'etat VST3 est sauvegarde/restaure cote `IComponent` et `IEditController`.

Le build courant utilise `VST3_CLASS_ID = DrumFlashPlugin1`.

## Correctif sauvegarde/reouverture

Les 16 pas du pattern sont maintenant serialises dans le champ persistant `pattern-v1`.
Ce champ lit directement `SharedPattern`, qui est l'etat reel utilise par l'UI et le sequenceur.
Les anciens parametres caches `st01` a `st16` restent disponibles pour migrer les songs deja
sauvegardees avant ce correctif.

Tests valides:

```text
cargo test: 16 tests OK
```

Binaire installe:

```text
C:\Program Files\Common Files\VST3\drum-pattern-vst.vst3
SHA-256: 62AA5FCC445FEFDBC1E30196E614BCAED53A61C9F9EB2AB9BD5A4E1C5C510CEF
```

## Commande de build recommandee

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
.\build.ps1 -Install
```

Le build utilise la dependance vendoree `drum-pattern-vst/vendor/nih-plug`; il ne modifie plus le
checkout Cargo global.
