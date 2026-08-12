# Flash Drum

Plugin VST3 de séquenceur de batterie 64 pas (4 pages × 16) avec 13 voix de synthèse dans 14 slots modulaires, écrit en Rust avec le framework `nih-plug`.

## Structure du projet

- **`drum-pattern-vst/`** — Plugin VST3 (produit actif)
- **`docs/`** — Documentation technique et utilisateur
- **`archive/web-poc/`** — PoC web legacy (HTML/React), archivé, non maintenu

## Build & Installation

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
.\build.ps1 -Install
```

## Documentation

- **`CLAUDE.md`** — Référence canonique pour tous les agents IA : architecture, invariants, build/test, contraintes temps réel (`AGENTS.md` y redirige)
- **`TODO.md`** — Tâches en cours et backlog
- **`CHANGELOG.md`** — Historique des builds
- **`ADDING_AN_INSTRUMENT.md`** — Procédure d'ajout d'une voix de synthèse
- **`docs/infrastructure.md`** — Guide infrastructure (build, CI, déploiement)
- **`docs/user-guide.md`** — Guide utilisateur (fonctionnalités, workflow)
- **`docs/analog-mode.md`** — Documentation du mode Analog
