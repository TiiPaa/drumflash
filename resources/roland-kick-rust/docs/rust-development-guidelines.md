# Préconisations Rust pour continuer le développement

## 1. Architecture conseillée

Séparer le projet en 4 couches :

1. DSP pur
2. moteur musical / séquenceur
3. I/O (audio, MIDI, export)
4. UI

Le DSP ne doit pas dépendre de l'UI ni du backend audio.

## 2. Organisation projet recommandée

Exemple de workspace plus tard :

```text
/crates
  /dsp
  /engine
  /standalone
  /plugin
  /common
```

## 3. Thread audio : règles d'or

Dans le callback audio :

- pas d'allocation
- pas de lock
- pas d'I/O
- pas de logs
- pas de JSON
- pas d'accès disque

## 4. Contrôle vs audio rate

### Audio-rate

- oscillateurs
- enveloppes
- filtres
- résonateurs
- saturation

### Control-rate

- changements UI
- automation lente
- calculs de structure
- scheduling des blocs

## 5. Paramètres

Entre UI et audio :

- atomiques
- snapshot de paramètres par bloc
- ring buffer lock-free pour les événements

Éviter `Mutex` dans le thread audio.

## 6. Lissage

Prévoir un `SmoothedValue` pour :

- gain
- fréquence
- cutoff
- drive
- dry/wet

## 7. Types numériques

- `f32` pour l'audio temps réel
- `f64` seulement si utile pour certains calculs de coeffs ou analyses

## 8. DC offset, denormals, non-linéarités

Prévoir tôt :

- `DcBlocker`
- gestion des denormals
- oversampling ciblé si saturation / carré / pulse

## 9. Stratégie de développement conseillée

### Phase 1

- une voix 808 simple
- rendu offline
- test de retrig

### Phase 2

- profils 909 et 606
- séquenceur sample-accurate

### Phase 3

- app standalone
- presets
- MIDI

### Phase 4

- harmonic layer
- oversampling
- plugin si besoin

## 10. Crates utiles plus tard

- `cpal` pour audio I/O
- `midir` pour MIDI
- `hound` pour WAV
- `criterion` pour benchs
- `serde` pour preset/config
- `egui/eframe` pour une UI simple
- `nih-plug` si plugin plus tard

## 11. Priorité absolue

Avant l'UI et avant les raffinements :

- un moteur propre
- un rendu offline
- un test de retrig serré
- une structure `trigger()` + `process()` propre

C'est la base la plus rentable.
