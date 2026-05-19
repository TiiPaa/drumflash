# Conversion Drum Flash -> Plugin VST

> Etat 2026-05-11: document historique de planification. La TODO active est `BACKLOG_VST.md`.

## Etat actuel valide

Le plugin Rust charge dans Studio One, l'UI de base est fonctionnelle, la sync DAW est validee, le multi-out Studio One fonctionne, et la grille est maintenant persistee via `pattern-v1` avec le build installe `20260511-091259`.

## Inventaire source

Le PoC web reste la reference fonctionnelle pour:

- grille 16 pas
- mapping instruments
- presets de base
- export MIDI attendu a terme
- reglages de synthese avances

## Architecture cible

```text
drum-pattern-vst/
  src/lib.rs          point d'entree VST3
  src/sequencer/     sequenceur, patterns, mutes
  src/synthesis/     voix de batterie
  src/ui.rs          UI egui
  build.ps1          build avec nih-plug vendore, bundle, install
```

## Migration initiale

### Etape 1: POC

- [x] Setup projet Rust + `nih-plug`.
- [x] Premier instrument audible.
- [x] Build VST3 fonctionnel.

### Etape 2: Core engine

- [x] Sequenceur 16 pas.
- [x] 7 instruments/routes de base.
- [x] Synchronisation tempo/transport DAW dans Studio One.

### Etape 3: UI egui

- [x] Grille pattern cliquable.
- [x] Presets Rock, Funk, Disco.
- [x] Mutes et solos.
- [x] Numero de build visible.
- [x] Rappel du pattern apres sauvegarde/reouverture.
- [x] Persistance de grille via `pattern-v1`.
- [x] Migration legacy depuis les parametres caches `st01` a `st16`.
- [ ] Editeur de son avance.

### Etape 4: Features avancees

- [ ] Export MIDI fichier.
- [ ] Sortie MIDI temps reel.
- [ ] Swing/groove.
- [ ] Song mode.

### Etape 5: Polish

- [x] Multi-out Studio One.
- [x] Vendor reproductible pour le patch `nih-plug`.
- [ ] Validation multi-DAW.
- [ ] Tests oracles avec le PoC web.
- [ ] Packaging propre.

## Note Studio One

Le multi-out a ete debloque par le patch VST3 vendore dans `drum-pattern-vst/vendor/nih-plug`, surtout `getRoutingInfo()` qui relie l'entree event/MIDI a la sortie audio principale. Sans ce mapping, Studio One affichait les sorties mais les gardait grisees.
