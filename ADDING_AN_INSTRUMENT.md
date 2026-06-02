# Guide : Ajouter un nouvel instrument dans Flash Drum

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
| `src/instrument_registry.rs` | **Source unique de vérité.** Définit les 13 instruments (nom, label, MIDI note, `standard_params`, `special_params`, defaults). |
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
    // Pour la saturation, ajoute aussi :
    // pub saturation_type: u8,
    // pub saturation_amount: f32,
    // pub saturation_mix: f32,
    // pub saturation_output_gain: f32,
    // pub saturation_pre_filter: f32,
}

impl From<VoiceSettings> for Perc2Settings { ... }
impl From<Perc2Settings> for VoiceSettings { ... }
```

> **Règle :** `VoiceSettings` reste le format de persistance (plock, presets, DAW state). La conversion vers le typed struct se fait dans `set_settings()` — zero-allocation, stack copy uniquement.
>
> **Saturation :** si tu veux le pack saturation complet (5 params), mappe-les sur `special[1..6]` (laisser `special[0]` pour le premier paramètre spécial propre à l'instrument). Si tu n'as pas de paramètre spécial propre, tu peux utiliser `special[0..4]` pour la saturation (4 params : type, amount, mix, output_gain).

### Étape 2 — Voix de synthèse

Créer `src/synthesis/perc2.rs` qui implémente le trait `Voice` :

```rust
use super::{dsp, saturation, settings::perc2::Perc2Settings, Voice, VoiceSettings};

pub struct Perc2Voice {
    settings: Perc2Settings,  // ← typed struct, pas VoiceSettings
    saturation: saturation::SaturationConfig,
    ...
}

impl Perc2Voice {
    pub fn new(sample_rate: f32, settings: Perc2Settings) -> Self {
        ...
        saturation: saturation::SaturationConfig {
            saturation_type: saturation::SaturationType::None,
            amount: 0.0, mix: 1.0, output_gain: 1.0, pre_filter: false,
        },
        ...
    }
}

impl Voice for Perc2Voice {
    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = Perc2Settings::from(settings);
        // Mettre à jour la saturation si présente :
        self.saturation.saturation_type = saturation::SaturationType::from(self.settings.saturation_type);
        self.saturation.amount = self.settings.saturation_amount;
        self.saturation.mix = self.settings.saturation_mix;
        self.saturation.output_gain = self.settings.saturation_output_gain;
        self.saturation.pre_filter = self.settings.saturation_pre_filter > 0.5;
    }
    fn set_special_param(&mut self, index: usize, value: f32) {
        match index {
            0 => self.settings.sweep = value,
            // Saturation (mapper selon les special_index du registry)
            1 => {
                self.settings.saturation_type = value as u8;
                self.saturation.saturation_type = saturation::SaturationType::from(self.settings.saturation_type);
            }
            2 => { self.settings.saturation_amount = value; self.saturation.amount = value; }
            3 => { self.settings.saturation_mix = value; self.saturation.mix = value; }
            4 => { self.settings.saturation_output_gain = value; self.saturation.output_gain = value; }
            5 => { self.settings.saturation_pre_filter = value; self.saturation.pre_filter = value > 0.5; }
            _ => {}
        }
    }
    fn process_sample(&mut self) -> f32 {
        let raw = ...; // ton DSP
        self.saturation.process(raw)  // ← applique la saturation en sortie
    }
    // ... trigger, process_sample_stereo, is_active, reset, set_algo
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
2. Remplir : `index`, `name`, `label` (2-3 caractères max), `full_name`, `midi_note`, `algo_count`, `standard_params`, `special_params`, `sound_settings_default` (13 valeurs f32), `filter_type_label`, **`freq_display_ratio`**.

**`freq_display_ratio`** : ratio appliqué à la fréquence avant affichage en mode Notes (plock uniquement). Pour un Kick dont la fréquence de sustain est `0.3×` la valeur du slider, mettre `0.3`. Pour les autres instruments, mettre `1.0`.

**Standard params :**
Le Sound Panel est **data-driven**. Tu choisis un tableau prédéfini (`FULL_STD`, `KICK_STD`, `NO_HOLD_NO_FILTENV_STD`, `TOM_STD`, `SNARE606_STD`, `MINIMAL_STD`) selon les capacités de l'instrument, ou tu en crées un nouveau. Chaque `StandardParamDef` lie un `StandardField` à une famille (`Osc`, `Env`, `Filter`, `Output`).

**Special params :**
Déclare chaque paramètre spécial (y compris la saturation) dans `special_params`. Exemple avec saturation :
```rust
special_params: &[
    SpecialParamDef {
        name: "perc2_sweep",
        label: "Sweep",
        default: 0.5,
        min: 0.0,
        max: 1.0,
        special_index: 0,
        family: ParamFamily::Osc,
    },
    // --- Saturation pack (optionnel) ---
    SpecialParamDef {
        name: "perc2_saturation_type",
        label: "Saturation Type",
        default: 0.0, min: 0.0, max: 5.0,
        special_index: 1,
        family: ParamFamily::Saturation,
    },
    SpecialParamDef {
        name: "perc2_saturation_amount",
        label: "Saturation Amount",
        default: 0.0, min: 0.0, max: 1.0,
        special_index: 2,
        family: ParamFamily::Saturation,
    },
    SpecialParamDef {
        name: "perc2_saturation_mix",
        label: "Saturation Mix",
        default: 1.0, min: 0.0, max: 1.0,
        special_index: 3,
        family: ParamFamily::Saturation,
    },
    SpecialParamDef {
        name: "perc2_saturation_output_gain",
        label: "Saturation Output Gain",
        default: 1.0, min: 0.5, max: 2.0,
        special_index: 4,
        family: ParamFamily::Saturation,
    },
    SpecialParamDef {
        name: "perc2_saturation_pre_filter",
        label: "Saturation Pre-Filter",
        default: 0.0, min: 0.0, max: 1.0,
        special_index: 5,
        family: ParamFamily::Saturation,
    },
],
```

**`sound_settings_default` — ordre strict des 13 champs standard :**
```
[ frequency, decay, volume, filter_freq, attack, release, decay_curve,
  release_curve, hold, filter_env_amount, filter_env_decay, analog, stereo ]
```

### Étape 5 — Plock field mapping

Le menu plock est **data-driven** : il lit `instrument.standard_params` et affiche uniquement les champs déclarés. Chaque `StandardField` a un `plock_field_index()` qui mappe vers l'index interne du plock :

| `StandardField` | Index plock | Valeur globale |
|----------------|-------------|----------------|
| `Freq` | 0 | `global.0` |
| `Decay` | 1 | `global.1` |
| `Volume` | 2 | `global.2` |
| `FilterFreq` | 3 | `global.3` |
| `Release` | 4 | `global.5` |
| `DecayCurve` | 5 | `global.6` |
| `ReleaseCurve` | 6 | `global.7` |
| `Hold` | 7 | `global.8` |
| `FilterEnvAmount` | 8 | `global.9` |
| `FilterEnvDecay` | 9 | `global.10` |
| `Analog` | 10 | `global.11` |
| `Stereo` | 11 | `global.12` |
| `Attack` | 18 | `global.4` |

> **Important :** `Attack` est à l'index 18 (pas 4) pour éviter un conflit historique avec les special params. Ne pas modifier ce mapping sans mettre à jour `plock.rs`.

### Étape 6 — Paramètres nih-plug

Modifier `src/lib.rs` dans `DrumFlashParams` :

1. Ajouter les paramètres `humanize_perc2`, `push_perc2`, `length_perc2`, `mute_perc2`, `mix_perc2`, `solo_perc2` (suivre le pattern existant).
2. Ajouter l'algo param : `algo_perc2: IntParam`.
3. **Si l'instrument est une bass drum** (fréquence de sustain différente du slider), ajouter un paramètre `freq_mode` :
   ```rust
   #[id = "freq_mode_perc2"]
   pub freq_mode_perc2: BoolParam,
   ```
   Cela permet le switch Hz/Notes dans le Sound Panel et le plock.
4. Ajouter les special params **avec des `#[id = "..."]` uniques** :
   - `perc2_sweep: FloatParam` (special[0])
   - `perc2_saturation_type: FloatParam` (special[1])
   - `perc2_saturation_amount: FloatParam` (special[2])
   - etc.
5. Dans `impl Default for DrumFlashParams`, instancier chaque nouveau paramètre avec `FloatParam::new(...)`.
6. Dans `impl DrumFlashParams`, mettre à jour **toutes** les méthodes d'accès indexé : `mutes()`, `solos()`, `mixes()`, `algos()`, `humanizes()`, etc. (ajouter le 14e élément).
7. Dans **`special_param()`**, ajouter impérativement les match arms — **sans ça les paramètres apparaissent dans l'UI mais valent toujours 0 dans le moteur audio** :
```rust
(13, 0) => Some(&self.perc2_sweep),
(13, 1) => Some(&self.perc2_saturation_type),
(13, 2) => Some(&self.perc2_saturation_amount),
(13, 3) => Some(&self.perc2_saturation_mix),
(13, 4) => Some(&self.perc2_saturation_output_gain),
(13, 5) => Some(&self.perc2_saturation_pre_filter),
```

### Étape 7 — voice_settings_for

Dans `src/lib.rs`, méthode `voice_settings_for()`, ajouter le match arm pour l'algo :

```rust
13 => self.params.algo_perc2.value() as u8,
```

> **Pas besoin de toucher les special params ici** : `voice_settings_for()` boucle déjà sur `instrument_registry::INSTRUMENTS[voice_idx].special_params` et lit chaque paramètre via `self.params.special_param()`. Tant que tu as ajouté les match arms dans `special_param()` (Étape 6), les valeurs seront injectées automatiquement dans `VoiceSettings.special[]`.

### Étape 8 — Constants diverses

Dans `src/lib.rs` :

1. `AUX_OUT_COUNT` doit correspondre à `DrumVoice::COUNT`.
2. `OUTPUT_PORT_NAMES` — ajouter le nom de la sortie.
3. `MIDI_NOTE_MAP` — ajouter la note MIDI.

### Étape 9 — Defaults de VoiceSettings

Dans `src/synthesis/mod.rs` :
1. Ajouter `pub fn perc2() -> Self` dans `impl VoiceSettings`.
2. Vérifier que les valeurs par défaut correspondent à celles déclarées dans `instrument_registry.rs`.

### Étape 10 — Special params / Algorithmes UI

Dans `src/synthesis/special_params.rs`, ajouter la définition des algorithmes pour le nouvel instrument si `algo_count > 1`.

### Étape 11 — Plock

Le système de plock stocke 18 fields par step/instrument :

| Fields | Contenu |
|--------|---------|
| 0-12   | sound settings standard (freq, decay, vol, filter_freq, attack, release, decay_curve, release_curve, hold, filter_env_amount, filter_env_decay, analog, stereo) |
| 13     | algo |
| 14-17  | special[0..3] (legacy, 4 slots — aujourd'hui on utilise `special[0..7]` via le VoiceSettings) |

> **Note :** le plock stocke 18 fields, mais `VoiceSettings.special` contient 8 slots. Les special params de saturation (index 1-5) sont stockés dans `VoiceSettings.special[1..6]` et remontés dans le plock via le `special_param()` data-driven.

Le plock menu (`draw_plock_menu` dans `ui.rs`) est **data-driven** : il lit `instrument.standard_params` et n'affiche que les champs déclarés pour cet instrument. Si le Kick n'a pas `Stereo` dans ses `standard_params`, le plock ne l'affichera pas. Les special params sont aussi data-driven via `instrument_registry::special_params(instrument)`.

**Mode Notes dans le plock :**
Si l'instrument est une bass drum (`freq_display_ratio != 1.0`), le plock affiche un checkbox "Notes". Quand activé :
- La fréquence est affichée comme nom de note (ex: "C2")
- Les boutons `+`/`-` changent par demi-ton
- Le `freq_display_ratio` est appliqué : `note = freq_to_note(freq * ratio)` et `freq = note_to_freq(note) / ratio`

### Étape 12 — UI Sound Panel

Le Sound Panel (`draw_sound_panel` dans `ui.rs`) est **déjà data-driven** via `instrument_registry::INSTRUMENTS[].standard_params` et `special_params`. Si tu as correctement rempli le registry avec les bonnes `ParamFamily`, les sliders apparaîtront groupés par famille (OSC, ENV, FILTER, SAT, OUTPUT). Aucune modification de `ui.rs` n'est nécessaire pour la Sound Panel.

**Mode Notes dans le Sound Panel :**
Pour les bass drums (Kick, B8, etc.), un checkbox "Notes" apparaît à côté du slider Freq. Le comportement est identique au plock : conversion via `freq_display_ratio`, snap à la note juste au toggle, ajustement par demi-tons.

---

## 4. Pièges courants

| Piège | Explication |
|-------|-------------|
| **Recréer les enveloppes dans `set_settings`** | C'est la cause du bug "le son revient à l'état initial quand je relâche le slider". Utilise toujours les setters. |
| **Oublier de mettre à jour `DrumVoice::COUNT`** | Tous les tableaux de taille fixe (`[T; COUNT]`) planteront à la compilation ou pire, à l'exécution. |
| **Oublier un match arm dans `DrumVoiceKind`** | Rust t'aidera (exhaustiveness check), mais vérifie bien toutes les méthodes du trait `Voice`. |
| **Oublier `special_param()` dans `lib.rs`** | Le paramètre special apparaîtra dans l'UI mais sa valeur sera toujours 0 dans le moteur audio. **Impératif** : ajouter chaque `(instrument, special_index)` dans le `match`. |
| **Hardcoder des comportements par index** | Évite `if instrument == 7` dans l'UI ou le plock. Préfère `instrument_registry::special_params(instrument)` et des boucles data-driven. |
| **Oublier le typed settings struct** | Le compiler le rappellera (type mismatch), mais vérifie bien que `src/synthesis/settings/<voice>.rs` existe et est réexporté dans `settings/mod.rs`. |
| **Mauvais ordre dans `sound_settings_default`** | L'ordre est strict : `[freq, decay, vol, filter_freq, attack, release, decay_curve, release_curve, hold, filter_env_amount, filter_env_decay, analog, stereo]`. |
| **Saturation non branchée** | Si tu ajoutes des `SpecialParamDef` de type `Saturation` dans le registry mais que tu n'instancies pas les `FloatParam` correspondants dans `DrumFlashParams`, les sliders apparaîtront dans l'UI mais ne feront rien. Il faut aussi implémenter `set_special_param()` dans la voix pour mapper l'index vers `saturation::SaturationConfig`. |
| **VST3 cache** | Studio One (et autres DAWs) mettent en cache le bundle VST3. Après un `build.ps1 -Install`, ferme complètement la DAW avant de rouvrir le plugin, sinon tu testes l'ancienne version. |
| **`freq_display_ratio` oublié** | Si l'instrument est une bass drum et que tu mets `freq_display_ratio: 1.0`, le mode Notes affichera une note fausse (la fréquence du slider au lieu de la fréquence réelle de sustain). |
| **Plock data-driven** | Le plock menu ne montre que les champs listés dans `instrument.standard_params`. Si tu retires un champ du tableau (ex: `Stereo` du Kick), il disparaît aussi du plock — c'est le comportement attendu. |

---

## 5. Résumé visuel

```
Nouvel instrument "Perc2"
│
├─> src/synthesis/settings/perc2.rs (Perc2Settings : typed struct + From/Into<VoiceSettings>)
│                                    ↳ Inclure saturation_type, saturation_amount, etc.
├─> src/synthesis/perc2.rs          (trait Voice — typed settings, NE PAS recréer les env)
│                                    ↳ saturation::SaturationConfig + process() en sortie
│                                    ↳ set_special_param() : mapper index vers saturation
├─> src/synthesis/mod.rs            (DrumVoice::Perc2, DrumVoiceKind::Perc2, COUNT, new())
├─> src/synthesis/settings/mod.rs   (pub mod perc2; + pub use)
├─> src/instrument_registry.rs      (entry INSTRUMENTS[13] avec standard_params + special_params)
│                                    ↳ special_params incluant saturation (family=Saturation)
│                                    ↳ freq_display_ratio: 1.0 (ou 0.3 pour bass drum)
├─> src/instrument_registry.rs      (StandardField::plock_field_index() si nouveau champ)
├─> src/lib.rs                      (DrumFlashParams : humanize/mute/mix/solo/algo/specials)
│                                    ↳ FloatParam pour chaque special (saturation comprise)
│                                    ↳ special_param() : match (13, 0..5)
│                                    ↳ freq_mode_perc2: BoolParam (si bass drum)
├─> src/lib.rs                      (voice_settings_for() algo arm — special auto-injectés)
├─> src/lib.rs                      (OUTPUT_PORT_NAMES, MIDI_NOTE_MAP, AUX_OUT_COUNT)
├─> src/synthesis/mod.rs            (VoiceSettings::perc2() default)
├─> src/synthesis/special_params.rs (algos_for Perc2 si algo_count > 1)
└─> src/ui.rs / src/plock.rs        (data-driven, mode Notes si freq_display_ratio != 1.0)
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
2. Le Sound Panel affiche les bons sliders selon `standard_params`,
3. Les special params apparaissent groupés par famille (OSC, ENV, FILTER, **SAT**, OUTPUT),
4. Le son ne coupe pas quand on bouge un slider (pas de recréation d'enveloppes),
5. Le plock menu permet de verrouiller les special params (y compris la saturation),
6. Si la saturation est activée (type > 0, amount > 0), le son change réellement,
7. Si `freq_display_ratio != 1.0`, le mode Notes dans Sound Panel et plock affiche la bonne note et les boutons `+`/`-` changent par demi-ton juste.
