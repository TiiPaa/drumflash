# Code Review - Flash Drum VST3 - 2026-06-03

Objectif: synthese actionnable des problemes detectes dans `drum-pattern-vst/` pour guider le prochain agent de developpement.

Validation effectuee:
- `cargo test`: OK, 73 tests lib + 59 tests standalone passes.
- `cargo clippy --all-targets`: OK, mais avec beaucoup de warnings.

## Findings Critiques

### 1. Plocks incompatibles avec les patterns 64 steps

Reference: `drum-pattern-vst/src/plock.rs:20-60`, `drum-pattern-vst/src/ui.rs:591-594`, `drum-pattern-vst/src/ui.rs:1452-1454`

Probleme: les plocks utilisent `AtomicU16` pour stocker les steps actifs, alors que `STEP_COUNT = 64`.

Pourquoi: les plocks des pages 2 a 4 ne peuvent pas etre representes correctement. `1u16 << step` avec `step >= 16` est invalide/panique en debug et donne un comportement non fiable en release.

Suggestion:
- Remplacer le masque d'activation par `AtomicU64`.
- Migrer le format persistant de `plock-v1` vers un nouveau format, ou lire l'ancien `u16` en compatibilite.
- Ajouter des tests sur les steps 16, 31 et 63.

Exemple de direction:

```rust
pub struct PlockMasks {
    pub masks: [AtomicU64; INSTRUMENT_COUNT],
}

let bit = 1u64 << step;
```

Confiance: haute.

## Findings Importants

### 2. Export MIDI limite aux 16 premiers steps

Reference: `drum-pattern-vst/src/midi_export.rs:70`, `drum-pattern-vst/src/ui.rs:353-355`, `drum-pattern-vst/src/ui.rs:373-375`

Probleme: l'export MIDI boucle sur `0..16`.

Pourquoi: depuis le passage aux patterns 1-64 steps, un pattern de 32/48/64 steps exporte ou draggue vers le DAW perd les pages 2 a 4.

Suggestion:
- Passer la longueur globale `Pattern Length` a `export_pattern_to_midi_data()`.
- Boucler sur `0..pattern_length.clamp(1, 64)`.
- Ajouter un test avec un trigger au step 32 ou 63.

Confiance: haute.

### 3. `NoteOff` potentiellement hors buffer

Reference: `drum-pattern-vst/src/lib.rs:1714-1716`

Probleme: le `NoteOff` est envoye a `sample_idx + 1`.

Pourquoi: si le trigger arrive sur le dernier sample du buffer, le timing devient egal a la taille du buffer. Certains hosts attendent un offset dans `[0, buffer_len - 1]` et peuvent ignorer ou mal traiter l'evenement.

Suggestion:
- Pour des drums, envoyer le `NoteOff` au meme offset que le `NoteOn`, ou
- Implementer une petite file fixe preallouee de note-offs a emettre au buffer suivant.

Confiance: moyenne.

### 4. Publication atomique UI -> audio trop faible

Reference: `drum-pattern-vst/src/sound_settings.rs:111-154`, `drum-pattern-vst/src/lib.rs:1638`, `drum-pattern-vst/src/plock.rs:277-320`

Probleme: les ecritures UI et lectures audio utilisent majoritairement `Ordering::Relaxed`.

Pourquoi: le thread audio peut observer un bump de version ou un bit plock actif sans garantie formelle que toutes les valeurs precedemment ecrites soient visibles. Sur x86 cela passe souvent, mais le contrat Rust/temps reel n'est pas propre, surtout pour les plocks multi-champs.

Suggestion:
- Garder les stores de valeurs en `Relaxed` si necessaire.
- Publier ensuite avec `Release` sur la version ou le bit actif.
- Lire cote audio avec `Acquire` avant de lire les champs associes.

Confiance: moyenne a haute.

## Suggestions

### 5. Documentation de persistance pattern non alignee

Reference: `drum-pattern-vst/src/lib.rs:53`, `drum-pattern-vst/src/lib.rs:76`, `drum-pattern-vst/src/lib.rs:1776-1813`, `AGENTS.md`

Probleme: le code utilise `pattern-v2`, mais les instructions projet mentionnent encore `pattern-v1` comme contrat stable.

Pourquoi: la migration semble exister, mais la documentation active n'est plus alignee avec l'etat reel. C'est dangereux pour les futures modifications de persistance DAW.

Suggestion:
- Mettre a jour la doc active.
- Clarifier que `pattern-v2` est le nouveau champ et que `pattern-v1` est migre.

Confiance: haute.

### 6. Outils dev visibles dans l'UI utilisateur

Reference: `drum-pattern-vst/src/ui.rs:851-938`

Probleme: `Dev: Preset Dumps` est visible dans l'UI principale.

Pourquoi: cela ajoute de l'I/O fichier, de la complexite et du bruit UX dans un plugin utilisateur.

Suggestion:
- Masquer derriere `#[cfg(debug_assertions)]`, un feature flag, ou un toggle dev non persistant.

Confiance: moyenne.

## Hors Securite

Code mort et warnings:
- `cargo clippy --all-targets` remonte beaucoup de warnings.
- Zones visibles: `ui/design_system.rs`, `ui/schema.rs`, imports inutilises dans `ui.rs`, variables inutilisees dans tests/stress tests.

Redondances:
- `drum-pattern-vst/src/ui.rs:866-869`: variable `algo` definie deux fois et commentaire repete.

Optimisations:
- `drum-pattern-vst/src/synthesis/mod.rs:464-476`: `large_enum_variant` sur `DrumVoiceKind`, car `Perc1Voice` rend toute l'enum tres grosse. Ce n'est pas forcement prioritaire car initialise hors callback audio, mais il faut en faire une decision explicite.

Dette qualite:
- `drum-pattern-vst/Cargo.toml:6` decrit encore un sequencer 16 steps alors que le plugin gere 64 steps.

## Open Questions / Hypotheses

Hypothese: l'export MIDI doit refleter la longueur globale `Pattern Length`.

Impact si faux: si l'export 16 steps est volontaire, l'UI doit l'indiquer explicitement.

Hypothese: les plocks doivent fonctionner sur les 64 steps.

Impact si faux: il faut desactiver ou masquer les plocks sur pages 2 a 4, sinon l'UI promet une feature cassée.

## Points Positifs

- Le chemin audio principal est globalement sain: pas de lock bloquant visible dans `process()`.
- Pas d'allocation evidente dans la boucle sample.
- Les voix utilisent une enum plutot que du `dyn Voice` dans le chemin audio.
- Bonne couverture de tests DSP/sequencer, notamment anti-click et migrations pattern.

## Priorites Recommandees

1. Corriger `PlockMasks` pour supporter 64 steps et ajouter tests de persistance.
2. Corriger l'export MIDI pour respecter `Pattern Length`.
3. Securiser le protocole atomique UI -> audio avec `Acquire/Release` aux points de publication.
4. Nettoyer warnings et code mort UI/design system.
5. Aligner documentation et metadonnees (`pattern-v2`, description 64 steps).

## Tests A Ajouter

- `plock_supports_steps_16_to_63`
- `plock_persistence_roundtrips_step_63`
- `midi_export_includes_steps_beyond_first_page`
- `note_off_timing_never_exceeds_buffer_len`
- Test de publication settings: edition UI simulee puis lecture audio coherente apres `Acquire/Release`.

## Risk Matrix

| Finding | Impact | Probabilite |
|---|---:|---:|
| Plocks casses au-dela du step 16 | Eleve | Elevee |
| Export MIDI limite a 16 steps | Moyen/Eleve | Elevee |
| Ordering atomique trop faible | Moyen | Moyenne |

Score global: `A retravailler` avant de considerer la V1 vraiment stable.
