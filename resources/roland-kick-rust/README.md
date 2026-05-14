# roland-kick-rust

Petit projet Rust de travail pour modéliser des kicks type Roland TR-606 / TR-808 / TR-909.

## Contenu

- `docs/kick-modeling-synthesis.md`
  - synthèse de la proposition DSP
  - séparation transient / body
  - pseudo-code commun
  - profils 606 / 808 / 909
  - évolution possible vers d'autres oscillateurs
- `docs/retrigger-and-sequencer.md`
  - gestion des retrigs sans clic parasite
  - stratégie sample-accurate côté séquenceur
- `docs/rust-development-guidelines.md`
  - recommandations Rust / temps réel / architecture projet
- `src/models/kick808.rs`
  - exemple concret de voix 808 en Rust
- `src/bin/demo808.rs`
  - petit rendu de démonstration qui écrit un WAV mono dans `target/demo808.wav`

## Intention du code

L'exemple 808 fourni ici est volontairement :

- simple à lire
- modulaire
- orienté temps réel
- conçu pour conserver une attaque franche sans reset brutal de la voix

Ce n'est pas une émulation circuit exacte. C'est un noyau **grey-box DSP** propre pour continuer le développement.

## Utilisation prévue

Quand `cargo` est disponible sur la machine :

```bash
cargo run --bin demo808
```

Le binaire génère un fichier :

```text
target/demo808.wav
```

avec des retrigs volontairement rapprochés pour tester le comportement.

## Note importante

Le projet a été créé dans le workspace OpenClaw, mais **`cargo` n'est pas installé dans l'environnement actuel**, donc je n'ai pas pu faire de compilation/validation locale ici.

La structure et le code sont préparés pour être repris sur ta machine Rust.
