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
│  │    13 voices + Plocks           │    │
│  └─────────────────────────────────┘    │
└─────────────────────────────────────────┘
```

### Composants principaux

- **UI Thread** (`src/ui.rs`) — Interface graphique avec `egui`
- **Audio Thread** (`src/lib.rs`) — Callback temps réel `process()`
- **Sequencer** (`src/sequencer/`) — Moteur de séquence 64 pas
- **Synthesis** (`src/synthesis/`) — 13 voix de synthèse modulaire
- **Plock System** (`src/plock.rs`) — Parameter locks par step

---

## Build

### Prérequis

- **Rust** (dernière stable) — `cargo`, `rustc`
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
3. Copie le bundle dans le dossier système VST3 (avec `-Install`)
4. Injecte un `DRUM_PATTERN_BUILD_ID` (timestamp) affiché dans l'UI

---

## Architecture technique

### Audio Thread

Le callback `process()` est appelé par le DAW à chaque bloc d'échantillons. Contraintes temps réel strictes :

- **Pas d'allocation** (`no_alloc`)
- **Pas de locks bloquants** — atomiques (`AtomicU32`, `AtomicBool`) et structures lock-free (`SharedPattern`)
- **Pas de panic** — pas de `unwrap()` sur les données du host
- **Buffers préalloués** et réutilisés

### Séquenceur

- **Position maître** : `beat_position` (0.0 → 4.0 = 1 bar = 16 steps)
- **Grid** : 64 steps (4 pages × 16), 13 instruments
- **Plocks** : 46 champs plockables (14 standard + 32 special)
- **Groove** : Swing, shuffle, MPC (appliqué sur la grille maître)

### Voix de synthèse (13)

| # | Instrument | Type | Analog |
|---|-----------|------|--------|
| 0 | Kick | Osc + Click | Drift opérationnel |
| 1 | Snare | Noise + Tone | Drift opérationnel |
| 2 | HiHat | FM Noise | Fixé (1.0) |
| 3 | OpenHiHat | FM Noise | Fixé (1.0) |
| 4 | Tom1 | Osc | Drift opérationnel |
| 5 | Tom2 | Osc | Drift opérationnel |
| 6 | Tom3 | Osc | Drift opérationnel |
| 7 | Clap | Burst + Noise | Fixé (1.0) |
| 8 | Ride | FM Noise | Fixé (1.0) |
| 9 | Cymbal | FM Noise + Shimmer | Drift opérationnel |
| 10 | Snare606 | Resonator + Noise | Fixé (1.0) |
| 11 | BassDrum808 | FM + Click | Drift opérationnel |
| 12 | Zap | FM + Decay | Fixé (1.0) |

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
- **`plock-v1`** — Parameter locks (masques + valeurs)
- **`sound-settings-v1`** — Réglages de synthèse par instrument
- **`global-v1`** — Paramètres globaux (BPM, swing, etc.)

Migration legacy : les anciens champs `pattern-v1`..`pattern-v4` et paramètres `st01`…`st16` sont convertis automatiquement vers `pattern-v5`.

**Identité VST3 figée** : `VST3_CLASS_ID = *b"DrumFlashPlugin1"` — ne pas modifier pour préserver la compatibilité des projets.

---

## Dépendances

- **nih-plug** (vendored dans `vendor/nih-plug/`) — Framework plugin VST3
- **nih_plug_egui** — Intégration UI egui
- **egui** — UI immediate mode
- **serde** — Sérialisation état
- **hound** — Export WAV (tests)

⚠️ **Ne pas remplacer le nih-plug vendored par la version crates.io** — des patches locaux sont nécessaires pour :
- Multi-out dans Studio One
- Sauvegarde/restauration d'état côté `IEditController`
- Routing MIDI

---

## Tests

### Tests unitaires (76)

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
version = "0.1.0"
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
