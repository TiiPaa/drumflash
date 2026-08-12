# Guide : Ajouter un nouvel instrument dans Flash Drum

> Ce document décrit l'architecture **modulaire** actuelle du plugin et la
> procédure exacte pour ajouter un instrument (kind). Référence vivante :
> l'ajout du **BD606 multisample** (build 2026-08-02) a suivi exactement cette
> checklist — sers-t'en comme exemple dans le code.

---

## 1. Architecture en 30 secondes

**Stack :** Rust + `nih-plug` (VST3) + `egui` (UI intégrée).

**Il n'y a plus de voix fixes.** Le plugin expose un pool de **14 slots**
(`MAX_TRACKS`, `src/track.rs`). Chaque slot a un **kind** d'instrument
choisi par l'utilisateur (`TrackInstrumentKind`), ses propres réglages, son
routing, sa note MIDI et sa ligne de pattern.

```
DAW appelle process()
  → AtomicTrackLayout diff → reinitialize_slot() si kind changé (sans alloc)
  → Sequencer déclenche les steps (par slot)
  → DrumSynthesizer : 1 voix pré-allouée par slot (DrumVoiceKind)
  → mix + 14 sorties stéréo aux
```

**Règle d'or :** aucune allocation, aucun lock bloquant, aucun panic dans
`process()`.

---

## 2. Les TROIS enums à synchroniser

Ajouter un instrument = toucher **trois espaces d'index** distincts :

| Enum | Fichier | Rôle |
|------|---------|------|
| `TrackInstrumentKind` | `src/track.rs` | Kind exposé à l'UI/track. **Sérialisé dans `track-layout-v1`** → ajouter les nouvelles variantes **à la fin** (indices stables). |
| `DrumVoiceKind` | `src/synthesis/mod.rs` | Wrapper concret des voix DSP (enum, pas de dyn). |
| `DrumVoice` (legacy) | `src/synthesis/mod.rs` | Espace d'index du registry `INSTRUMENTS` (garde Tom1/2/3 séparés). |

Le pont entre eux : `TrackInstrumentKind::drum_voice_index()` → index
`DrumVoice`/registry.

---

## 3. Fichiers clés

| Fichier | Rôle |
|---------|------|
| `src/track.rs` | `TrackInstrumentKind`, `TrackSlot`, `TrackLayoutState`, `AtomicTrackLayout` (vue lock-free audio). |
| `src/synthesis/mod.rs` | `DrumVoice`, `DrumVoiceKind`, trait `Voice`, `DrumSynthesizer`, `VoiceSettings`, `create_voice_for_kind()`. |
| `src/synthesis/<voice>.rs` | Implémentation du trait `Voice` (ex: `bd606.rs`, `perc1.rs`). |
| `src/synthesis/settings/<voice>.rs` | Typed settings struct + conversions `From/Into<VoiceSettings>`. |
| `src/synthesis/dsp.rs` | Briques DSP : enveloppes, filtres, oscillateurs, smoothers, `AnalogDrift`, `DcBlocker`. |
| `src/synthesis/sample_bank.rs` | Banque de samples embarqués (BD606) — pattern réutilisable pour d'autres multisamples. |
| `src/synthesis/special_params.rs` | `algos_for` — définitions d'algorithmes par voix. |
| `src/instrument_registry.rs` | **Source de vérité UI** : `INSTRUMENTS` (standard_params, special_params, defaults, `freq_display_ratio`). |
| `src/sound_settings.rs` | `SoundSettingsState` — atomiques par slot (13 standards + `special[32]` + freq_mode), persistance `sound-settings-v2`. |
| `src/lib.rs` | Plugin : `DrumFlashParams`, boucle `process()`, hot kind-change, seed migration. |
| `src/ui/sound_editor.rs` | Onglet Track (dropdown **Type**) + Sound Panel (data-driven). |
| `src/ui/popups.rs` | Popup "Add Module" (boucle sur `TrackInstrumentKind::COUNT` — automatique). |
| `src/generator/mod.rs` | Remap des rôles du générateur vers les kinds de slots. |

---

## 4. Checklist : ajouter un instrument (ex. « BD6smp », kind 11 / voice 13)

### Étape 1 — Typed settings

Créer `src/synthesis/settings/bd606.rs` (modèle : `settings/perc1.rs`) :

- Struct typé avec les 13 champs standard + les champs spéciaux nommés.
- `From<VoiceSettings>` et `From<Bd606Settings> for VoiceSettings` : les
  spéciaux vivent dans `special[0..N]` (32 slots disponibles).
- Test : `crate::settings_roundtrip_test!(bd606_settings_roundtrip, bd606, Bd606Settings);`
- Déclarer `pub mod bd606;` dans `settings/mod.rs`.

> **Convention saturation pack** : 5 params mappés sur des `special[i]`
> consécutifs (type = `sp_discrete`, amount, mix, output_gain, pre_filter =
> `sp_discrete`). Voir BD606 : `special[4..8]`.

### Étape 2 — Voix de synthèse

Créer `src/synthesis/bd606.rs` implémentant le trait `Voice` (modèle :
`perc1.rs`). Points critiques :

- **`set_settings`** : JAMAIS recréer les enveloppes — utiliser les setters
  (`.set_decay()`, `.set_curve()`, `.set_attack_ms()`, `.set_hold()`…).
- **`trigger()`** : ne pas reset de phase osc/filtre/RNG (continuité
  analogique anti-click). L'enveloppe d'amp gère la rampe anti-click.
- **Saturation** : uniquement via `saturation.process_at(pre_stage, x)` —
  appelée deux fois (pré/post filtre), le flag `pre_filter` route.
- **Volume post-saturation** : `settings.volume` multiplie APRÈS
  `process_at(false, …)`.

### Étape 3 — `synthesis/mod.rs`

1. `mod bd606;` + `pub use bd606::Bd606Voice;` + `pub use settings::bd606::Bd606Settings;`
2. `DrumVoice::Bd606 = 13` + `COUNT` (13→14) + arm `from_index()`.
3. `VoiceSettings::bd606()` — defaults identiques au `sound_settings_default`
   du registry + defaults des spéciaux (ordre `special[i]`).
4. `DrumVoiceKind::Bd606(Bd606Voice)` + l'arm dans les **9 matchs** du trait
   (`trigger`, `trigger_hard`, `process_sample`, `process_sample_stereo`,
   `is_active`, `reset`, `set_settings`, `set_algo`, `set_special_param`).
5. Arm dans `create_voice_for_kind()`.
6. **Si la voix utilise des données lourdes partagées** (samples) : pré-chauffer
   dans `initialize_with_layout()` (voir `let _ = sample_bank::bank();`) car
   `create_voice_for_kind()` peut être appelé depuis `process()` via
   `reinitialize_slot()` — interdiction d'allouer à ce moment-là.

### Étape 4 — `track.rs` (`TrackInstrumentKind`)

Ajouter la variante **à la fin** de l'enum (indice sérialisé !) + mettre à
jour : `COUNT`, `from_index`, `default_label` (2 chars), `default_name`,
`default_midi_note` (GM, éviter les collisions), `drum_voice_index`,
`from_drum_voice_index`.

### Étape 5 — Registry (`instrument_registry.rs`)

1. Table `BD606_STD` (ou réutiliser `FULL_STD`/`TOM_STD`/…) avec les
   `StandardParamDef` voulus — le Sound Panel et le plock sont data-driven.
2. Entrée `InstrumentDef` à l'index = `drum_voice_index()` : `name`, `label`,
   `full_name`, `midi_note`, `algo_count`, `standard_params`,
   `special_params` (`sp` = continu/morphable, `sp_discrete` = pas de morph),
   `sound_settings_default` (**ordre strict** des 13 champs :
   `[freq, decay, vol, filter_freq, attack, release, decay_curve,
   release_curve, hold, filter_env_amount, filter_env_decay, analog, stereo]`),
   `filter_type_label`, `freq_display_ratio`.
3. Tests en bas du fichier : ajouter l'index à la liste mono OU stéréo.

### Étape 6 — `sound_settings.rs` : ne rien casser

La persistance `sound-settings-v2` est **versionnée par longueur de blob**.
Les longueurs legacy sont gelées sur **13 voix** via `LEGACY_VOICE_COUNT`
(ne JAMAIS utiliser `DrumVoice::COUNT` ici — il grandit avec les nouvelles
voix et casserait la détection des anciens formats). Aucun changement de
format nécessaire si les params tiennent dans `special[32]`.

### Étape 7 — Listes hardcodées UI

- `ui/sound_editor.rs` : tableau `kinds` du dropdown **Type** (~ligne 334).
- `ui/sound_editor.rs` : listes « analog fixed » `2|3|7|8|10|12` (~lignes 691
  et 809) — ajouter l'index si le drift analogique standard ne s'applique pas.
- `ui/sound_editor.rs` : `is_bass_drum` (`voice_idx == 0 || 11`, ~ligne 940) —
  ajouter si le mode Hz/Notes a du sens.
- `ui/plock.rs` : `matches!(voice_idx, 0 | 11)` (~lignes 179 et 604) — idem.
- Popups « Add Module », plock menu, morph : **data-driven**, rien à faire.

### Étape 8 — Générateur (`generator/mod.rs`)

`remap_roles_to_slots()` mappe les slots vers les 13 rôles legacy via
`drum_voice_index()`. Un nouvel index sans rôle resterait **silencieux** sur
GENERATE → mapper explicitement vers un rôle existant (BD606 emprunte le
rôle Kick) ou ajouter un rôle dans `generator/styles.rs`.

### Étape 9 — Algorithmes (`synthesis/special_params.rs`)

Si `algo_count > 1` : ajouter la const `*_ALGOS` + l'arm dans `algos_for`
(match exhaustif sur `DrumVoice`). Sinon une entrée « Standard ».

### Étape 10 — Tests

- Roundtrip settings (macro, étape 1).
- Tests voix : son produit, sortie finie, silence après decay, comportement
  des spéciaux (modèle : tests de `bd606.rs` / `perc1.rs`).
- `cargo test` complet vert (les tests `sound_settings.rs` itèrent sur
  `TrackInstrumentKind::COUNT` — un default de freq ≤ 0 fera échouer).

---

## 5. Cas particulier : instrument à samples (pattern BD606)

- WAV embarqué via `include_bytes!` dans `src/synthesis/sample_bank.rs`,
  décodé **une fois** dans un `OnceLock` global → `&'static SampleBank`.
- Pré-chauffé dans `DrumSynthesizer::initialize_with_layout()` (non-RT) ;
  sur le thread audio, `bank()` n'est qu'un load atomique.
- **Pas de resampling au chargement** : la lecture à position fractionnaire
  (interpolation linéaire) absorbe le ratio `source_rate / session_rate`.
- Le fallback « fichier non parsable » produit des hits vides (voix inerte),
  jamais de panic.
- RNG de sélection (xorshift) seedé à la construction, **jamais reseedé au
  trigger** (convention anti-click).

---

## 6. Pièges courants

| Piège | Explication |
|-------|-------------|
| **Recréer les enveloppes dans `set_settings`** | Coupe le son à chaque mouvement de slider. Setters uniquement. |
| **Variante `TrackInstrumentKind` insérée au milieu** | Les indices sont sérialisés dans `track-layout-v1` → toujours ajouter **à la fin**. |
| **`DrumVoice::COUNT` utilisé pour la persistance** | Gelé à 13 via `LEGACY_VOICE_COUNT` dans `sound_settings.rs`. |
| **Oublier un match `DrumVoiceKind`** | Le compilateur le rappelle (exhaustivité) — les 9 matchs + `create_voice_for_kind`. |
| **Allocation dans `create_voice_for_kind`** | Appelé depuis `process()` via `reinitialize_slot` → données lourdes derrière un `OnceLock` pré-chauffé. |
| **Nouvel instrument silencieux sur GENERATE** | Pas de rôle dans le générateur → mapper vers un rôle existant. |
| **Listes UI hardcodées oubliées** | Dropdown Type, analog-fixed, is_bass_drum (Sound Panel + plock). |
| **Mauvais ordre `sound_settings_default`** | Ordre strict des 13 champs (voir étape 5). |
| **VST3 cache** | Fermer Studio One avant `build.ps1 -Install` (lock DLL). |

---

## 7. Build & Test

```powershell
cd drum-pattern-vst
cargo test
# Fermer Studio One avant l'install (lock DLL)
.\build.ps1 -Install
```

Puis dans Studio One : insérer le plugin, vérifier que :
1. Le nouveau kind apparaît dans le dropdown **Type** (onglet Track) et dans
   le popup **Add Module** d'une lane vide,
2. Le Sound Panel affiche les bons sliders groupés par famille,
3. Le son ne coupe pas quand on bouge un slider,
4. Le plock menu propose les standard + special params,
5. Changement de kind à chaud pendant la lecture : pas de clic, pas de
   silence bloqué,
6. Sauvegarde/reload de la song : kind + réglages conservés,
7. GENERATE écrit une ligne sur la lane du nouvel instrument.
