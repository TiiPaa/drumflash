# Drum Flash

Projet de conversion d'un PoC web de sequenceur de batterie vers un plugin VST3 en Rust.

## Direction du projet

Le produit principal vise est le plugin VST3 dans `drum-pattern-vst/`.

Le PoC web reste dans le depot comme reference fonctionnelle:
- `index.html`
- `index.js`

Il ne doit plus etre traite comme le produit final.

## Documents de reference

- `PROJECT_BRIEF.md`: objectif produit, perimetre V1, contraintes
- `BACKLOG_VST.md`: priorites P0/P1/P2
- `GUIDE_UTILISATION.md`: vue d'ensemble du depot et du build
- `drum-pattern-vst/README.md`: etat et architecture du plugin

## Etat rapide

Le plugin Rust est maintenant la cible active du projet. Il n'est pas encore en parite
complete avec le PoC web, mais il est utilisable dans Studio One pour les workflows V1
de base.

Etat courant au 2026-05-11:
- plugin VST3 `nih-plug` charge dans Studio One
- interface egui visible avec numero de build
- sequenceur 16 pas editable en temps reel
- diagnostic de sauvegarde/reouverture Studio One effectue
- parametres classiques sauvegardes/restaures via l'etat VST3
- grille sauvegardee dans le champ persistant `pattern-v1`
- migration legacy depuis les anciens parametres caches `st01` a `st16`
- presets Rock, Funk et Disco branches
- mutes/solos par instrument
- sync transport DAW: play, stop, tempo, repositionnement
- sortie Main Mix stereo
- multi-out Studio One fonctionnel: Kick, Snare, Hi-Hat, Open HH, Tom 1, Tom 2, Tom 3

Encore a terminer:
- sound design avance et reglages par instrument
- export MIDI
- sortie MIDI temps reel
- tests sur d'autres DAWs

## Build

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
.\build.ps1 -Install
```

Le script compile avec le `nih-plug` vendore dans `drum-pattern-vst/vendor/nih-plug`,
regenere le bundle VST3 et peut l'installer dans `C:\Program Files\Common Files\VST3`.

Build installe valide:

```text
Build UI: 20260511-091259
VST3 class ID: DrumFlashPlugin1
SHA-256: 62AA5FCC445FEFDBC1E30196E614BCAED53A61C9F9EB2AB9BD5A4E1C5C510CEF
```
