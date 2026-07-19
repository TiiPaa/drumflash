# Flash Drum VST3

Plugin VST3 de séquenceur de batterie 64 pas (4 pages × 16) en Rust.

## État réel du plugin

- 13 instruments dans 14 slots modulaires : Kick, Snare, HiHat, OpenHH, Tom1-3, Clap, Ride, Cymbal, Snare606, 808 Kick, Perc1
- 14 sorties stéréo aux + Main Mix
- Séquenceur 64 pas éditable en temps réel
- Sync DAW : play, stop, tempo, repositionnement
- Parameter locks (plocks) par step
- Presets Rock, Funk, Disco
- Mutes / solos / Mix Bus par instrument

## Build

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
.\build.ps1 -Install
```

Vérification rapide :

```powershell
cargo check
cargo test
cargo run --bin test_standalone
```

> Le build utilise le `nih-plug` vendore dans `vendor/nih-plug`. Ne pas le remplacer par la version crates.io.

## Documentation

- **`../AGENTS.md`** — architecture complète, contraintes temps réel, anti-click, persistence
- **`../CHANGELOG.md`** — historique des builds
- **`../TODO.md`** — tâches en cours
- **`STUDIO_ONE_MULTI_OUT.md`** — notes techniques du patch multi-out
