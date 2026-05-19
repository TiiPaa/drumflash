# Mission: Conversion Drum Flash -> VST3 Plugin Rust

> Etat 2026-05-11: document historique. La source de verite active est `BACKLOG_VST.md`, avec le build installe `20260511-091259`.

## Objectif

Convertir le PoC web de sequenceur de batterie en plugin VST3 professionnel utilisant Rust et `nih-plug`.

## Stack

- Source: PoC web `index.html` / `index.js`
- Cible: Rust + `nih-plug` + `nih_plug_egui`
- Produit actif: `drum-pattern-vst/`

## Contraintes critiques

- Ne pas paniquer dans le traitement audio.
- Ne pas allouer dans `process()`.
- Ne pas utiliser de lock bloquant dans l'audio thread.
- Garder le comportement deterministe et prealloue.
- Garder la documentation alignee avec le code Rust reel.

## Checkpoints

- [x] POC VST3 fonctionnel dans Studio One.
- [x] Sequenceur 16 pas.
- [x] Grille UI 16x7 editable.
- [x] Presets Rock, Funk et Disco.
- [x] Mutes et solos par instrument.
- [x] Sync DAW: play, stop, tempo, repositionnement.
- [x] Multi-out Studio One fonctionnel.
- [x] Build installe avec numero de build visible dans l'UI.
- [x] Remplacer le patch de checkout Cargo `nih-plug` par vendor reproductible.
- [x] Rappel du pattern apres sauvegarde/reouverture du projet DAW.
- [x] Diagnostic Studio One de l'etat VST3.
- [x] Persistance de grille via `pattern-v1` et migration legacy `st01` a `st16`.
- [ ] Valider dans au moins un autre DAW.
- [ ] Ajouter export MIDI.
- [ ] Ajouter sortie MIDI temps reel.
- [ ] Ajouter reglages de synthese avances par instrument.

## Mapping MIDI de reference

- BD: 36
- SD: 38
- HH: 42
- OH: 46
- T1: 50
- T2: 47
- T3: 43
- Canal cible: 10

## Build valide fin de session

```text
Build UI: 20260511-091259
VST3 class ID: DrumFlashPlugin1
SHA-256: 62AA5FCC445FEFDBC1E30196E614BCAED53A61C9F9EB2AB9BD5A4E1C5C510CEF
```

Commande recommandee:

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
.\build.ps1 -Install
```
