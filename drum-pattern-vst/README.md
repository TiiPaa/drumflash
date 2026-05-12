# Drum Flash VST3

Plugin VST3 de sequenceur de batterie 16 pas en Rust.

## Etat reel du code

Le plugin n'est pas encore a la parite complete du PoC web, mais le socle V1 est
utilisable dans Studio One.

Ce qui existe et a ete valide au 2026-05-10:
- plugin VST3 base sur `nih-plug`
- numero de build affiche dans l'interface
- sortie audio Main Mix stereo
- sorties stereo separees: Kick, Snare, Hi-Hat, Open HH, Tom 1, Tom 2, Tom 3
- sequenceur interne 16 pas
- grille 16x7 editable dans l'UI
- presets Rock, Funk et Disco
- mutes et solos par instrument
- sync DAW: play, stop, tempo, repositionnement
- multi-out activable dans Studio One

Correctif ajoute au 2026-05-11:
- le diagnostic Studio One a montre que l'etat VST3 sauvegardait bien `master_vol` et `bpm`
- le diagnostic a isole le probleme sur la grille: les anciens parametres caches `st01` a `st16` ne changeaient pas lors des clics
- la grille est maintenant persistee directement depuis `SharedPattern` dans le champ `pattern-v1`
- les anciens etats bases sur `st01` a `st16` sont migres vers `pattern-v1`
- le wrapper VST3 vendore sauvegarde/restaure le meme etat cote `IEditController` et `IComponent`
- les tests couvrent la serialisation de `SharedPattern` et la migration legacy

Ce qui n'est pas encore finalise:
- vraie sortie MIDI
- export MIDI
- parite complete avec les reglages de synthese du PoC web
- validation multi-DAW hors Studio One

## Architecture actuelle

```text
src/
  lib.rs                point d'entree plugin
  ui.rs                 UI egui: build, grille, presets, mutes, solos
  sequencer/
    mod.rs              moteur de sequence
    pattern.rs          structures de pattern
  synthesis/
    mod.rs              orchestration des voix
    kick.rs             kick
    snare.rs            snare
    hihat.rs            hi-hat
```

## Constat important

Ce fichier decrit l'etat reel valide du plugin Rust. Les anciens documents de conversion
doivent etre lus comme de l'historique ou de la planification, pas comme une preuve d'etat
implemente.

## Build

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
.\build.ps1 -Install
```

Verification:

```powershell
cargo check
cargo test
```

## Notes Studio One

Le multi-out Studio One depend du `nih-plug` vendore dans `vendor/nih-plug`.
Cette copie locale contient le patch de la couche VST3 pour:
- resoudre les bus audio/event vers le root unit avec `get_unit_by_bus()`
- accepter les activations progressives de sorties dans `set_bus_arrangements()`
- accepter `num_ins == 0` avec un pointeur d'entree audio nul
- ignorer les sorties non activees pendant la validation de buffers
- relier l'entree event/MIDI a la sortie audio principale via `getRoutingInfo()`
- sauvegarder/restaurer l'etat plugin cote edit controller en plus du component

Le build installe courant est `20260511-091259`, avec
`VST3_CLASS_ID = DrumFlashPlugin1`. Cet ID est conserve comme identite VST3
permanente pour la ligne V1 afin de ne pas casser les projets Studio One existants.
La validation Studio One de sauvegarde/reouverture doit etre refaite avec ce build.

## Priorites recommandees

1. revalider la sauvegarde/reouverture Studio One avec le build `20260511-091259`
2. valider le multi-out dans au moins un autre DAW
3. ajouter les reglages de synthese par instrument
4. ajouter export MIDI et sortie MIDI temps reel
