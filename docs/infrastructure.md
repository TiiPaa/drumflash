# Infrastructure — Flash Drum

> Ce document décrit l'infrastructure technique du projet : build, architecture, déploiement et maintenance.

---

## Architecture globale

Flash Drum est un **plugin VST3** écrit en Rust avec le framework [`nih-plug`](https://github.com/robbert-vdh/nih-plug).

```
┌─────────────────────────────────────────┐
│              DAW (Studio One)           │
│  ┌─────────────────────────────────┐    │
│  │     Flash Drum VST3 Plugin      │    │
│  │  ┌─────────┐    ┌────────────┐ │    │
│  │  │   UI    │◄──►│   Audio    │ │    │
│  │  │ (egui)  │    │  Thread    │ │    │
│  │  └─────────┘    └────────────┘ │    │
│  │         │              │        │    │
│  │         ▼              ▼        │    │
│  │    Sequencer (64 steps)         │    │
│  │    14 slots + Plocks             │    │
│  └─────────────────────────────────┘    │
└─────────────────────────────────────────┘
```

### Composants principaux

- **UI Thread** (`src/ui.rs`) — Interface graphique avec `egui`
- **Audio Thread** (`src/lib.rs`) — Callback temps réel `process()`
- **Sequencer** (`src/sequencer/`) — Moteur de séquence 64 pas
- **Synthesis** (`src/synthesis/`) — 27 voix DSP (25 kinds d'instruments) réparties sur 14 slots modulaires
- **Plock System** (`src/plock.rs`) — Parameter locks par step

---

## Build

### Prérequis

- **Rust 1.94.0** — toolchain épinglée par `drum-pattern-vst/rust-toolchain.toml` (le binaire livré et la CI doivent utiliser le même compilateur)
- **Windows** (développement principal)
- **Studio One** (DAW de référence pour les tests)

### Commandes

```powershell
# Build + installation
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
.\build.ps1 -Install

# Tests
cargo test

# Vérification statique
cargo check

# Build standalone (harness de test)
cargo run --bin test_standalone
```

### Sortie du build

- **DLL** : `target/release/drum_pattern_vst.dll`
- **Bundle VST3** : `build/drum-pattern-vst.vst3/`
- **Installation** : `C:\Program Files\Common Files\VST3\drum-pattern-vst.vst3`

Le script `build.ps1` :
1. Compile la DLL en mode `release`
2. Génère un bundle VST3 structuré (`.vst3/Contents/x86_64-win/`)
3. Copie le bundle dans le dossier système VST3 (avec `-Install`, après un test de verrou du DLL installé [250])
4. Injecte un `DRUM_PATTERN_BUILD_ID` (timestamp) affiché dans l'UI

### CI

Une CI GitHub Actions (`.github/workflows/ci.yml`, runner `windows-latest`, push/PR sur `main`) exécute `cargo check`, `cargo test` et `build.ps1`, puis publie le bundle VST3 en artefact (90 jours). Elle ne couvre pas encore macOS, clippy ni fmt — voir [264] dans `TODO.md`.

---

## Architecture technique

### Audio Thread

Le callback `process()` est appelé par le DAW à chaque bloc d'échantillons. Contraintes temps réel strictes :

- **Pas d'allocation** (`no_alloc`)
- **Pas de locks bloquants** — atomiques (`AtomicU32`, `AtomicBool`) et structures lock-free (`SharedPattern`)
- **Pas de panic** — pas de `unwrap()` sur les données du host
- **Buffers préalloués** et réutilisés

### Séquenceur

- **Position maître** : `beat_position` (0.0 .. `master_length` × 0.25 beat), 1 step = 0.25 beat — longueur par défaut 16 steps, maximum 64
- **Grid** : 64 steps (4 pages × 16), 14 slots
- **Plocks** : 46 champs plockables (14 standard + 32 special)
- **Groove** : Swing, shuffle, MPC (appliqué sur la grille maître)

### Voix de synthèse (27 voix / 25 kinds dans 14 slots)

- **13 voix d'origine** : Kick, Snare, HiHat, OpenHiHat, Tom1, Tom2, Tom3, Clap, Ride, Cymbal, Snare606, BassDrum808, Perc1 (les trois Tomn n'existent que comme rôles du registre — le track n'a qu'un seul kind `Tom`)
- **4 samplers TR-606** : BD6smp, SD6smp, CH6smp, OH6smp (multisample 8 layers)
- **Buzz** — percussion tonale + gate rapide
- **Sdrex** — snare à excitation
- **6 voix AC606 modélisées** : Bd6Ac, Sd6Ac, Hh6Ac, Oh6Ac, Cl6Ac, Tm6Ac (portées du moteur analogcode)
- **Rift** — lecture de tranche dans une texture longue (WAV utilisateur possible)
- **One-Shot** — lecture du WAV de la lane, du début à la fin

Le détail par voix (paramètres, défauts) vit dans la source de vérité : `src/instrument_registry.rs`.

### Sorties audio

- **Main Mix** — Mix stéréo de toutes les voix
- **14 sorties Aux** — Une paire stéréo générique `Out 1..14`, routée par slot depuis l'onglet `Track`

### MIDI

- **Entrée** : Canal 10 (index 9) — notes MIDI déclenchent les instruments
- **Sortie** : NoteOn/NoteOff sur canal 10 vers hardware externe
- **Export** : Fichier MIDI + Drag-and-drop via helper Windows

---

## Persistence DAW

L'état du plugin est sauvegardé dans le projet du DAW via `VST3State` :

- **`pattern-v5`** — Grid 64×14 slots (bitmasks + step data)
- **`plock-v1`** — Parameter locks (masques + valeurs + field masks)
- **`seq-plock-v1`** — Parameter locks de séquenceur
- **`sound-settings-v2`** — Réglages de synthèse par slot (46 floats/slot)
- **Paramètres nih-plug** — BPM, swing, longueur pattern, etc. (persistés par le DAW via `DrumFlashParams`)

Migration legacy : les anciens champs `pattern-v1`..`pattern-v4` et paramètres `st01`…`st16` sont convertis automatiquement vers `pattern-v5`.

**Identité VST3 figée** : `VST3_CLASS_ID = *b"DrumFlashPlugin1"` — ne pas modifier pour préserver la compatibilité des projets.

---

## Dépendances

- **nih-plug** (vendored dans `vendor/nih-plug/`) — Framework plugin VST3
- **nih_plug_egui** — Intégration UI egui
- **egui** — UI immediate mode
- **serde** — Sérialisation état
- **hound** — Décodage WAV en production (banques 606 embarquées et textures/WAV utilisateur pour Rift et One-Shot, `src/synthesis/sample_bank.rs`)

⚠️ **Ne pas remplacer le nih-plug vendored par la version crates.io** — des patches locaux sont nécessaires pour :
- Multi-out dans Studio One
- Sauvegarde/restauration d'état côté `IEditController`
- Routing MIDI

---

## Tests

### Tests unitaires (477 tests lib)

```bash
cargo test --lib
```

Couverture :
- Séquenceur (timing, swing, polyrhythmes)
- Plocks (création, lecture, 64 steps)
- Synthèse (rendu audio, anti-click, drift analog)
- Persistance (roundtrip DAW state)

### Tests de stress

```bash
cargo test stress_tests
```

- Sessions longues (stabilité timing)
- Patterns complexes (charge CPU)
- Synchronisation DAW (seek, boucle)

### Validation manuelle

1. Ouvrir Studio One
2. Insérer Flash Drum sur une piste instrument
3. Activer les sorties séparées (`Out 1`, `Out 2`, ...)
4. Sauvegarder le projet
5. Fermer et rouvrir — vérifier que la grille et les réglages sont restaurés

---

## Organisation du repo

```
E:\Dev\Projets\Drum Flash\
├── drum-pattern-vst/          ← Produit actif (plugin Rust)
│   ├── src/
│   │   ├── lib.rs             ← Point d'entrée plugin
│   │   ├── ui.rs              ← Interface graphique
│   │   ├── sequencer/         ← Moteur de séquence
│   │   ├── synthesis/         ← Voix de synthèse (1 fichier par voix)
│   │   ├── generator/         ← Générateurs de patterns
│   │   ├── plock.rs           ← Système de parameter locks
│   │   └── ...
│   ├── build.ps1              ← Script build + install
│   └── Cargo.toml
├── docs/                      ← Documentation
│   ├── analog-mode.md         ← Doc technique Analog
│   ├── infrastructure.md      ← Ce fichier
│   ├── user-guide.md          ← Guide utilisateur
│   └── historique/            ← Archives docs anciennes
├── archive/web-poc/           ← PoC web legacy (archivé)
│   ├── index.html
│   └── index.js
├── AGENTS.md                  ← Guide agent (architecture détaillée)
├── TODO.md                    ← Tâches et backlog
├── CHANGELOG.md               ← Historique builds
├── ADDING_AN_INSTRUMENT.md    ← Guide ajout de voix
└── README.md                  ← Point d'entrée projet
```

---

## Maintenance

### Mise à jour de la version

Modifier `Cargo.toml` :
```toml
version = "0.2.0"
```

### Ajout d'une voix de synthèse

Suivre **`ADDING_AN_INSTRUMENT.md`**.

Points clés :
1. Créer le fichier DSP dans `src/synthesis/<voice>.rs`
2. Implémenter le trait `Voice`
3. Enregistrer dans `instrument_registry.rs`
4. Ajouter les paramètres dans `DrumFlashParams`
5. Mettre à jour `DrumVoice::COUNT` et le registry ; `AUX_OUT_COUNT` reste lié aux 14 slots
6. Ajouter les tests unitaires

### Build ID

Chaque build injecte un timestamp via la variable d'environnement `DRUM_PATTERN_BUILD_ID`. Affiché dans l'UI en bas à gauche pour tracer les versions.
