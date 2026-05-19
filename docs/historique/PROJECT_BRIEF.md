# Project Brief

## Produit

Construire un plugin VST3 de sequenceur de batterie 16 pas en Rust a partir du PoC web `Drum Pattern Generator` (legacy).

Le PoC web sert de reference fonctionnelle. Le produit final cible est le plugin VST3, pas l'application navigateur.

## Objectif V1

Livrer un plugin VST3 utilisable dans un DAW, stable, synchronise au transport hote, avec une premiere parite fonctionnelle credible par rapport au PoC web.

## Reference fonctionnelle

Le PoC web definit aujourd'hui:
- la grille 16 pas
- les 7 instruments
- le mapping MIDI
- les presets de base
- les reglages de synthese par instrument
- l'export MIDI attendu a terme

Mapping instruments:
- BD = 36
- SD = 38
- HH = 42
- OH = 46
- T1 = 50
- T2 = 47
- T3 = 43

Canal cible:
- canal 10 pour les usages MIDI batterie

## Portee V1

Fonctionnalites incluses:
- plugin VST3 instrument
- sequenceur 16 pas
- 7 instruments audibles
- sync transport DAW: play, stop, tempo, repositionnement
- pattern editable dans l'UI
- rappel du pattern apres sauvegarde/reouverture du projet DAW
- mute par instrument
- presets Rock, Funk, Disco
- sortie audio stable stereo
- sorties stereo separees par instrument dans Studio One

Fonctionnalites explicitement hors V1:
- export MIDI fichier depuis le plugin
- sortie MIDI temps reel vers hardware externe
- swing
- song mode
- bibliotheque de presets etendue
- sound design avance par instrument

## Exigences non fonctionnelles

### Audio temps reel

- aucune allocation dans `process()`
- aucun lock bloquant dans l'audio thread
- aucun panic dans le chemin audio
- comportement deterministe et robuste en buffer court

### Integrite produit

- documentation synchronisee avec le code reel
- build reproductible
- separation claire entre code de production et code PoC

## Architecture cible

Le plugin doit converger vers les modules suivants:
- `plugin`: point d'entree VST3 et gestion host/transport
- `sequencer`: horloge, position, patterns, presets
- `synthesis`: voix et mixage par instrument
- `ui`: grille, mutes, presets, edition de base
- `midi`: couche de sortie MIDI si ajoutee plus tard
- `export`: export MIDI si ajoute plus tard

## Definition of Done V1

La V1 est consideree prete quand:
- le plugin build en `release`
- il charge dans au moins un DAW cible
- il joue les 7 instruments sans crash
- il suit play/stop/tempo/seek du DAW
- l'UI permet d'editer un pattern simple
- les presets de base sont disponibles
- la documentation de build/test/etat reel est a jour

## Etat courant au 2026-05-11

Correctif ajoute pour le rappel de pattern apres restauration d'etat hote:
- diagnostic Studio One: les parametres classiques sont bien sauvegardes/restaures
- diagnostic Studio One: les anciens parametres caches `st01` a `st16` ne changeaient pas lors de l'edition de grille
- la grille est maintenant serialisee directement depuis `SharedPattern` dans le champ persistant `pattern-v1`
- les anciens etats `st01` a `st16` sont migres vers `pattern-v1` au chargement
- le wrapper VST3 sauvegarde/restaure maintenant le meme etat cote `IEditController` et `IComponent`
- `cargo test` valide la persistance de `SharedPattern` et la migration legacy

Build installe: `20260511-091259`.

Validation Studio One restante: sauvegarder puis rouvrir un projet avec pattern modifie pour confirmer le comportement dans le DAW.

## Etat valide au 2026-05-10

Valide dans Studio One:
- chargement du plugin VST3
- UI egui avec build visible
- grille 16x7 editable
- rappel du pattern apres sauvegarde/reouverture
- presets Rock, Funk et Disco
- mutes et solos par instrument
- sync play/stop/tempo/repositionnement
- Main Mix stereo
- multi-out stereo par instrument

Le build valide est `20260510-170819`.

## Risques actuels

- ecart entre documentation et code reel
- parite incomplete avec le PoC web
- multi-out depend d'un `nih-plug` vendore dans `drum-pattern-vst/vendor/nih-plug`
- multi-out valide dans Studio One mais pas encore dans d'autres DAWs

## Priorite immediate

1. tester le plugin dans au moins un autre DAW
2. revalider dans Studio One la sauvegarde/reouverture avec le build `20260511-091259`
3. poursuivre les fonctionnalites MIDI/export et sound design avance
