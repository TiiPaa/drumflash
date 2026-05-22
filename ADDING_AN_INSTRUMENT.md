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
| `src/synthesis/settings/<voice>.rs` | **Typed settings struct** (`Perc2Settings`) + conversions `From/Into<VoiceSettings>`. |
| `src/synthesis/dsp.rs` | Briques DSP réutilisables : `ExpDecayEnvelope`, `DecayReleaseEnvelope`, `OnePoleFilter`, `PitchEnvelope`, oscillateurs, etc. |
| `src/synthesis/special_params.rs` | Définitions des algorithmes par instrument (`algos_for`) pour le sélecteur d'algo UI. |
| `src/lib.rs` | Plugin principal. Contient `DrumFlashParams` (tous les paramètres nih-plug), `voice_settings_for()`, et la boucle `process()`. |
| `src/ui.rs` | Grille de séquenceur, Sound Panel, plock menu. |
| `src/sound_settings.rs` | `SoundSettingsState` + `InstrumentSettingsState` (atomiques partagées UI/audio). |
| `src/plock.rs` | Stockage per-step des overrides (16 steps × 13 instruments × 18 fields). |

---

## 3. Checklist : ajouter un instrument

Supposons qu'on ajoute un 14e instrument appelé **Perc2** (index 13).

### Étape 1 — Typed settings

Créer `src/synthesis/settings/perc2.rs` avec le struct typé et les conversions :

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Perc2Settings {
    pub frequency: f32,
    pub attack: f32,
    pub decay: f32,
    pub decay_curve: f32,
    pub release: f32,
    pub release_curve: f32,
    pub volume: f32,
    pub filter_freq: f32,
    pub filter_env_amount: f32,
    pub filter_env_decay: f32,
    pub hold: f32,
    pub analog: f32,
    pub stereo: f32,
    pub algo: u8,
    // Ajoute ici les champs spéciaux (ex: sweep, bite...) qui remplacent special[0..]
}

impl From<VoiceSettings> for Perc2Settings { ... }
impl From<Perc2Settings> for VoiceSettings { ... }
```

> **Règle :** `VoiceSettings` reste le format de persistance (plock, presets, DAW state). La conversion vers le typed struct se fait dans `set_settings()` — zero-allocation, stack copy uniquement.

### Étape 2 — Voix de synthèse

Créer `src/synthesis/perc2.rs` qui implémente le trait `Voice` :

```rust
use super::{dsp, settings::perc2::Perc2Settings, Voice, VoiceSettings};

pub struct Perc2Voice {
    settings: Perc2Settings,  // ← typed struct, pas VoiceSettings
    ...
}

impl Perc2Voice {
    pub fn new(sample_rate: f32, settings: Perc2Settings) -> Self { ... }
}

impl Voice for Perc2Voice {
    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = Perc2Settings::from(settings);
        // utiliser self.settings.frequency, self.settings.decay, etc.
    }
    fn set_special_param(&mut self, index: usize, value: f32) {
        // Mapper index vers champ nommé (ex: 0 => self.settings.sweep = value)
    }
    // ... trigger, process_sample, process_sample_stereo, is_active, reset, set_algo
}
```

**Anti-pattern CRITIQUE à éviter dans `set_settings` :**
- ❌ Ne **jamais** recréer les enveloppes (`ExpDecayEnvelope::new(...)`) dans `set_settings`. Cela réinitialise leur état interne à 0 et coupe le son ou le filtre à chaque mouvement de slider.
- ✅ Utiliser les **setters** existants : `.set_decay()`, `.set_curve()`, `.set_release()`, `.set_hold()`.
- ✅ Si une enveloppe n'a pas de setter pour un paramètre dont tu as besoin, ajoute-le dans `dsp.rs` plutôt que de recréer l'enveloppe.

### Étape 3 — Enregistrer la voix dans le système de synthèse

Modifier `src/synthesis/mod.rs` :

1. Ajouter `mod perc2;` en haut.
2. Ajouter `pub use perc2::Perc2Voice;`
3. Ajouter `pub use settings::perc2::Perc2Settings;`
4. Ajouter `pub mod settings::perc2;` dans `src/synthesis/settings/mod.rs`.
5. Ajouter `Perc2 = 13` dans l'enum `DrumVoice`.
6. Mettre à jour `DrumVoice::COUNT` (ex: 14).
7. Ajouter le match arm dans `DrumVoice::from_index()`.
8. Ajouter `DrumVoiceKind::Perc2(Perc2Voice)` dans l'enum.
9. Ajouter le match arm dans **toutes** les méthodes de `impl Voice for DrumVoiceKind`.
10. Dans `DrumSynthesizer::new()`, pousser la nouvelle voix avec le typed settings :
    ```rust
    self.voices.push(DrumVoiceKind::Perc2(Perc2Voice::new(
        sample_rate,
        Perc2Settings::from(VoiceSettings::perc2()),
    )));
    ```

### Étape 4 — Registry

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

### Étape 5 — Paramètres nih-plug

Modifier `src/lib.rs` dans `DrumFlashParams` :

1. Ajouter les paramètres `humanize_perc2`, `push_perc2`, `length_perc2`, `mute_perc2`, `mix_perc2`, `solo_perc2` (suivre le pattern existant).
2. Ajouter l'algo param : `algo_perc2: IntParam`.
3. Ajouter les special params (ex: `perc2_sweep: FloatParam`) si besoin.
4. Dans `impl DrumFlashParams`, mettre à jour **toutes** les méthodes d'accès indexé : `mutes()`, `solos()`, `mixes()`, `algos()`, `humanizes()`, etc. (ajouter le 14e élément).
5. Dans `special_param()`, ajouter le match `(13, 0) => Some(&self.perc2_sweep), ...`

### Étape 6 — voice_settings_for

Dans `src/lib.rs`, méthode `voice_settings_for()`, ajouter le match arm :

```rust
13 => self.params.algo_perc2.value() as u8,
```

### Étape 7 — Constants diverses

Dans `src/lib.rs` :

1. `AUX_OUT_COUNT` doit correspondre à `DrumVoice::COUNT`.
2. `OUTPUT_PORT_NAMES` — ajouter le nom de la sortie.
3. `MIDI_NOTE_MAP` — ajouter la note MIDI.

### Étape 8 — Defaults de VoiceSettings

Dans `src/synthesis/mod.rs` :
1. Ajouter `pub fn perc2() -> Self` dans `impl VoiceSettings`.
2. Vérifier que les valeurs par défaut correspondent à celles déclarées dans `instrument_registry.rs`.

### Étape 9 — Special params / Algorithmes UI

Dans `src/synthesis/special_params.rs`, ajouter la définition des algorithmes pour le nouvel instrument si `algo_count > 1`.

### Étape 10 — Plock

Le système de plock stocke 18 fields par step/instrument :

| Fields | Contenu |
|--------|---------|
| 0-11   | sound settings standard (freq, decay, vol, filter_freq, release, decay_curve, release_curve, hold, filter_env_amount, filter_env_decay, analog, stereo) |
| 12     | `clap_echo` (legacy, hardcodé) |
| 13     | algo |
| 14-17  | special[0..3] (uniforme pour tous les instruments) |

Le plock menu (`draw_plock_menu` dans `ui.rs`) est **data-driven** : il lit les special params dynamiquement depuis `instrument_registry::special_params(instrument)`. Aucun hardcoding par index n'est nécessaire. Si le registry déclare correctement les `special_params`, ils apparaîtront automatiquement dans le menu plock.

### Étape 11 — UI Sound Panel

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
| **Oublier le typed settings struct** | Le compiler le rappellera (type mismatch), mais vérifie bien que `src/synthesis/settings/<voice>.rs` existe et est réexporté dans `settings/mod.rs`. |
| **Mauvais ordre dans `sound_settings_default`** | L'ordre est strict : `[freq, decay, vol, filter_freq, release, decay_curve, release_curve, hold, filter_env_amount, filter_env_decay, analog, stereo]`. |
| **VST3 cache** | Studio One (et autres DAWs) mettent en cache le bundle VST3. Après un `build.ps1 -Install`, ferme complètement la DAW avant de rouvrir le plugin, sinon tu testes l'ancienne version. |

---

## 5. Résumé visuel

```
Nouvel instrument "Perc2"
│
├─> src/synthesis/settings/perc2.rs (Perc2Settings : typed struct + From/Into<VoiceSettings>)
├─> src/synthesis/perc2.rs          (trait Voice — typed settings, NE PAS recréer les env)
├─> src/synthesis/mod.rs            (DrumVoice::Perc2, DrumVoiceKind::Perc2, COUNT, new())
├─> src/synthesis/settings/mod.rs   (pub mod perc2; + pub use)
├─> src/instrument_registry.rs      (entry INSTRUMENTS[13] avec capabilities & defaults)
├─> src/lib.rs                      (DrumFlashParams : humanize/mute/mix/solo/algo/specials)
├─> src/lib.rs                      (voice_settings_for() algo arm, special_param() match)
├─> src/lib.rs                      (OUTPUT_PORT_NAMES, MIDI_NOTE_MAP, AUX_OUT_COUNT)
├─> src/synthesis/mod.rs            (VoiceSettings::perc2() default)
├─> src/synthesis/special_params.rs (algos_for Perc2 si algo_count > 1)
└─> src/ui.rs / src/plock.rs        (data-driven, aucun hardcoding nécessaire)
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
