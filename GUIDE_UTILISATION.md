# Guide d'utilisation

## Position du projet

`Drum Flash` est maintenant un projet de plugin VST3 en Rust.

La version web (`index.html`, `index.js`) reste dans le depot comme PoC de reference pour:
- le comportement du sequenceur
- le mapping des instruments
- les presets
- l'export MIDI a reproduire plus tard dans le plugin

Le produit cible n'est plus l'application web.

## Etat actuel

Le plugin Rust present dans `drum-pattern-vst/` est le produit principal.

Ce qui est valide au 2026-05-11:
- plugin VST3 Rust base sur `nih-plug`
- chargement dans Studio One
- numero de build visible dans l'interface
- sequenceur 16 pas interne editable en temps reel
- sauvegarde/restauration VST3 diagnostiquee dans Studio One
- parametres classiques restaures au reload de la song
- grille persistee via le champ `pattern-v1`, directement depuis `SharedPattern`
- presets Rock, Funk et Disco
- mutes et solos par instrument
- sync transport DAW: play, stop, tempo, repositionnement
- sortie stereo Main Mix
- multi-out Studio One fonctionnel avec 7 sorties stereo instrument

Ce qui n'est pas encore considere comme finalise:
- parite fonctionnelle complete avec le PoC web
- reglages de synthese detailles par instrument
- vraie sortie MIDI plugin
- export MIDI cote plugin
- validation multi-DAW hors Studio One

## Structure utile

- `drum-pattern-vst/`: implementation plugin VST3
- `index.html`: PoC web de reference fonctionnelle
- `PROJECT_BRIEF.md`: brief produit et perimetre cible
- `BACKLOG_VST.md`: backlog priorise

## Compiler le plugin

Prerequis:
- Rust stable
- toolchain Windows compatible MSVC
- outils de build Visual Studio si necessaire

Commande recommandee:

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
.\build.ps1 -Install
```

Verification rapide:

```powershell
cargo check
cargo test
```

Note:
`cargo test` est valide avec 16 tests au 2026-05-11. Le build utilise le `nih-plug`
vendore dans `drum-pattern-vst/vendor/nih-plug`; il ne patche plus le checkout Cargo global.

## Installer pour test manuel

Le script principal d'installation est:

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
.\build.ps1 -Install
```

Il compile, regenere le bundle et copie le VST3 vers:

```text
C:\Program Files\Common Files\VST3\
```

## Comment lire le depot

Si l'objectif est produit:
- commencer par `PROJECT_BRIEF.md`
- continuer avec `BACKLOG_VST.md`

Si l'objectif est implementation:
- lire `drum-pattern-vst/src/lib.rs`
- puis `drum-pattern-vst/src/sequencer/`
- puis `drum-pattern-vst/src/synthesis/`

## Regle de travail recommandee

Toute nouvelle fonctionnalite plugin doit etre comparee au PoC web, mais la source de verite produit doit progressivement devenir le code Rust et sa documentation associee.
