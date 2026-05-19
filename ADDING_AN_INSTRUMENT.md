# Guide : Ajouter un nouvel instrument dans Drum Flash

> Ce document décrit l'architecture du plugin et la procédure exacte pour ajouter une voix (instrument) de synthèse. Il est destiné à un agent externe qui doit appréhender rapidement le codebase.

---

## 1. Architecture en 30 secondes

**Stack :** Rust + `nih-plug` (framework VST3) + `egui` (UI intégrée).

**Flux audio :**
```
DAW appelle process() → Sequencer déclenche les steps → DrumSynthesizer
  → 13 voix indépendantes (enum DrumVoiceKind) → mix + 13 sorties stéréo aux
```

**Flux données UI → audio :**
```
UI modifie des atomics (SoundSettingsState) → bump_version()
  → audio thread détecte le changement → set_voice_settings() sur chaque voix
```

**Règle d'or :** aucune allocation, aucun lock bloquant, aucun panic dans `process()`.

---

## 2. Fichiers clés

| Fichier | Rôle |
|---------|------|
| `src/instrument_registry.rs` | **Source unique de vérité.** Définit les 13 instruments (nom, label, MIDI note, capabilities, special params, defaults). |
| `src/synthesis/mod.rs` | Enum `DrumVoice`, trait `Voice`, `DrumVoiceKind` (wrapper enum), `DrumSynthesizer`, `VoiceSettings`. |
| `src/synthesis/<voice>.rs` | Implémentation du trait `Voice` pour un instrument (ex: `zap.rs`, `kick.rs`). |
| `src/synthesis/dsp.rs` | Briques DSP réutilisables : `ExpDecayEnvelope`, `DecayReleaseEnvelope`, `OnePoleFilter`, `PitchEnvelope`, oscillateurs, etc. |
| `src/special_params.rs` | Définitions des algorithmes par instrument (`algos_for`) pour le sélecteur d'algo UI. |
| `src/lib.rs` | Plugin principal. Contient `DrumFlashParams` (tous les paramètres nih-plug), `voice_settings_for()`, et la boucle `process()`. |
| `src/ui.rs` | Grille de séquenceur, Sound Panel, plock menu. |
| `src/sound_settings.rs` | `SoundSettingsState` + `InstrumentSettingsState` (atomiques partagées UI/audio). |
| `src/plock.rs` | Stockage per-step des overrides (16 steps × 13 instruments × 18 fields). |

---

## 3. Checklist : ajouter un instrument

Supposons qu'on ajoute un 14e instrument appelé **Perc2** (index 13).

### Étape 1 — Voix de synthèse

Créer `src/synthesis/perc2.rs` qui implémente le trait `Voice` :

```rust
pub struct Perc2Voice { ... }

impl Voice for Perc2Voice {
    fn trigger(&mut self) { ... }
    fn process_sample(&mut self) -> f32 { ... }
    fn process_sample_stereo(&mut self) -> (f32, f32) { ... }
    fn is_active(&self) -> bool { ... }
    fn reset(&mut self) { ... }
    fn set_settings(&mut self, settings: VoiceSettings) { ... }
    fn set_algo(&mut self, algo: u8) { ... }
    fn set_special_param(&mut self, index: usize, value: f32) { ... }
}
```

**Anti-pattern CRITIQUE à éviter dans `set_settings` :**
- ❌ Ne **jamais** recréer les enveloppes (`ExpDecayEnvelope::new(...)`) dans `set_settings`. Cela réinitialise leur état interne à 0 et coupe le son ou le filtre à chaque mouvement de slider.
- ✅ Utiliser les **setters** existants : `.set_decay()`, `.set_curve()`, `.set_release()`, `.set_hold()`.
- ✅ Si une enveloppe n'a pas de setter pour un paramètre dont tu as besoin, ajoute-le dans `dsp.rs` plutôt que de recréer l'enveloppe.

### Étape 2 — Enregistrer la voix dans le système de synthèse

Modifier `src/synthesis/mod.rs` :

1. Ajouter `mod perc2;` en haut.
2. Ajouter `pub use perc2::Perc2Voice;`
3. Ajouter `Perc2 = 13` dans l'enum `DrumVoice`.
4. Mettre à jour `DrumVoice::COUNT` (ex: 14).
5. Ajouter le match arm dans `DrumVoice::from_index()`.
6. Ajouter `DrumVoiceKind::Perc2(Perc2Voice)` dans l'enum.
7. Ajouter le match arm dans **toutes** les méthodes de `impl Voice for DrumVoiceKind` (trigger, process_sample, process_sample_stereo, is_active, reset, set_settings, set_algo, set_special_param).
8. Dans `DrumSynthesizer::initialize()`, pousser la nouvelle voix.

### Étape 3 — Registry

Modifier `src/instrument_registry.rs` :

1. Ajouter une entrée `InstrumentDef` dans le tableau `INSTRUMENTS` (à l'index 13).
2. Remplir : `index`, `name`, `label` (2-3 caractères max), `full_name`, `midi_note`, `algo_count`, `special_params`, `capabilities`, `sound_settings_default` (12 valeurs f32), `filter_type_label`.

**Capabilities :**
```rust
pub struct InstrumentCapabilities {
    pub freq: bool,        // affiche le slider Frequency
    pub hold: bool,        // affiche le slider Hold
    pub filter_env: bool,  // affiche Filter Env + Filter Decay
    pub analog: bool,      // affiche le slider Analog
    pub stereo: bool,      // affiche la checkbox Stereo
}
```

**`sound_settings_default` — ordre des 12 champs :**
```
[ frequency, decay, volume, filter_freq, release, decay_curve,
  release_curve, hold, filter_env_amount, filter_env_decay, analog, stereo ]
```

### Étape 4 — Paramètres nih-plug

Modifier `src/lib.rs` dans `DrumFlashParams` :

1. Ajouter les paramètres `humanize_perc2`, `push_perc2`, `length_perc2`, `mute_perc2`, `mix_perc2`, `solo_perc2` (suivre le pattern existant).
2. Ajouter l'algo param : `algo_perc2: IntParam`.
3. Ajouter les special params (ex: `perc2_sweep: FloatParam`) si besoin.
4. Dans `impl DrumFlashParams`, mettre à jour **toutes** les méthodes d'accès indexé : `mutes()`, `solos()`, `mixes()`, `algos()`, `humanizes()`, etc. (ajouter le 14e élément).
5. Dans `special_param()`, ajouter le match `(13, 0) => Some(&self.perc2_sweep), ...`

### Étape 5 — voice_settings_for

Dans `src/lib.rs`, méthode `voice_settings_for()`, ajouter le match arm :

```rust
13 => self.params.algo_perc2.value() as u8,
```

### Étape 6 — Constants diverses

Dans `src/lib.rs` :

1. `AUX_OUT_COUNT` doit correspondre à `DrumVoice::COUNT`.
2. `OUTPUT_PORT_NAMES` — ajouter le nom de la sortie.
3. `MIDI_NOTE_MAP` — ajouter la note MIDI.

### Étape 7 — Defaults de VoiceSettings

Dans `src/synthesis/mod.rs`, ajouter `pub fn perc2() -> Self` dans `impl VoiceSettings`.

### Étape 8 — Special params / Algorithmes UI

Dans `src/special_params.rs`, ajouter la définition des algorithmes pour le nouvel instrument si `algo_count > 1`.

### Étape 9 — Plock

Le système de plock stocke 18 fields par step/instrument :

| Fields | Contenu |
|--------|---------|
| 0-11   | sound settings standard (freq, decay, vol, filter_freq, release, decay_curve, release_curve, hold, filter_env_amount, filter_env_decay, analog, stereo) |
| 12     | `clap_echo` (legacy, hardcodé) |
| 13     | algo |
| 14-17  | special[0..3] (uniforme pour tous les instruments) |

**À NOTER :** dans le commit `5ae1286`, le plock menu (`draw_plock_menu` dans `ui.rs`) contient encore du code **hardcodé** pour certains instruments (Clap index 7, 808 Kick index 11). Si tu ajoutes un instrument avec des special params, tu dois :
- Soit refactorer `draw_plock_menu` pour qu'il lise les special params depuis `instrument_registry::special_params()` (comme le fait `draw_sound_panel`),
- Soit ajouter manuellement les blocs `if instrument == 13 { ... }` dans le plock menu.

La **bonne pratique** est de rendre `draw_plock_menu` data-driven via le registry (comme `draw_sound_panel`), afin d'éviter tout hardcoding par instrument.

### Étape 10 — UI Sound Panel

Le Sound Panel (`draw_sound_panel` dans `ui.rs`) est **déjà data-driven** via `capabilities()`. Si tu as correctement rempli `capabilities` dans le registry, les sliders apparaîtront ou disparaîtront automatiquement. Aucune modification de `ui.rs` n'est nécessaire pour la Sound Panel.

---

## 4. Pièges courants

| Piège | Explication |
|-------|-------------|
| **Recréer les enveloppes dans `set_settings`** | C'est la cause du bug "le son revient à l'état initial quand je relâche le slider". Utilise toujours les setters. |
| **Oublier de mettre à jour `DrumVoice::COUNT`** | Tous les tableaux de taille fixe (`[T; COUNT]`) planteront à la compilation ou pire, à l'exécution. |
| **Oublier un match arm dans `DrumVoiceKind`** | Rust t'aidera (exhaustiveness check), mais vérifie bien toutes les méthodes du trait `Voice`. |
| **Oublier `special_param()` dans `lib.rs`** | Le paramètre special apparaîtra dans l'UI mais sa valeur sera toujours 0 dans le moteur audio. |
| **Hardcoder des comportements par index** | Évite `if instrument == 7` dans l'UI ou le plock. Préfère `instrument_registry::special_params(instrument)` et des boucles data-driven. |
| **Mauvais ordre dans `sound_settings_default`** | L'ordre est strict : `[freq, decay, vol, filter_freq, release, decay_curve, release_curve, hold, filter_env_amount, filter_env_decay, analog, stereo]`. |
| **VST3 cache** | Studio One (et autres DAWs) mettent en cache le bundle VST3. Après un `build.ps1 -Install`, ferme complètement la DAW avant de rouvrir le plugin, sinon tu testes l'ancienne version. |

---

## 5. Résumé visuel

```
Nouvel instrument "Perc2"
│
├─> src/synthesis/perc2.rs          (trait Voice — NE PAS recréer les env dans set_settings)
├─> src/synthesis/mod.rs            (DrumVoice::Perc2, DrumVoiceKind::Perc2, COUNT, initialize())
├─> src/instrument_registry.rs      (entry INSTRUMENTS[13] avec capabilities & defaults)
├─> src/lib.rs                      (DrumFlashParams : humanize/mute/mix/solo/algo/specials)
├─> src/lib.rs                      (voice_settings_for() algo arm, special_param() match)
├─> src/lib.rs                      (OUTPUT_PORT_NAMES, MIDI_NOTE_MAP, AUX_OUT_COUNT)
├─> src/synthesis/mod.rs            (VoiceSettings::perc2() default)
├─> src/special_params.rs           (algos_for Perc2 si algo_count > 1)
└─> src/ui.rs / src/plock.rs        (vérifier que le plock menu lit bien les specials)
```

---

## 6. Build & Test

```powershell
cd drum-pattern-vst
# Fermer Studio One avant l'install (lock DLL)
.\build.ps1 -Install
```

Puis dans Studio One : insérer le plugin, vérifier que :
1. Le label apparaît dans la grille,
2. Le Sound Panel affiche les bons sliders selon `capabilities`,
3. Les special params apparaissent en bas du Sound Panel,
4. Le son ne coupe pas quand on bouge un slider (pas de recréation d'enveloppes),
5. Le plock menu permet de verrouiller les special params.
