# Guide : ajouter un nouvel instrument dans Flash Drum

> **Ce document décrit l'architecture.** La *procédure* pas à pas, avec les
> questions à poser avant de coder et les vérifications à lancer après, vit dans
> la skill **`/nouvel-instrument`** (`.claude/skills/nouvel-instrument/SKILL.md`).
> Les deux sont tenus ensemble ; en cas de désaccord, **le code fait foi**.
>
> Références vivantes dans le code : **OH6smp** ([208], build 20260909-144136,
> un sampler qui réutilise un moteur existant) et la famille **AC606** ([195],
> six voix portées d'un moteur externe). Relisez leurs commits avant de commencer.
>
> Dernière mise à jour : **2026-09-13**.

---

## 1. Architecture en 30 secondes

**Stack :** Rust + `nih-plug` (VST3, vendoré et patché) + `egui` (UI intégrée).

**Il n'y a pas de voix fixes.** Le plugin expose un pool de **14 slots**
(`MAX_TRACKS`, `src/track.rs`). Chaque slot porte un **kind** d'instrument choisi
par l'utilisateur (`TrackInstrumentKind`), avec ses propres réglages, son
routing, sa note MIDI, son choke group et sa ligne de pattern.

```
Le DAW appelle process()
  → AtomicTrackLayout diff → reinitialize_slot() si le kind a changé (sans alloc)
  → Sequencer déclenche les steps (par slot)
  → DrumSynthesizer : une voix pré-allouée par slot (DrumVoiceKind)
  → mix + 14 sorties stéréo aux
```

**Règle d'or :** aucune allocation, aucun lock bloquant, aucun panic dans
`process()`.

---

## 2. Les trois espaces d'index

Ajouter un instrument, c'est toucher **trois numérotations distinctes**. Les
confondre est l'erreur classique du projet.

| Enum | Fichier | État au 2026-09-13 | Rôle |
|------|---------|--------------------|------|
| `TrackInstrumentKind` | `src/track.rs` | `COUNT = 23`, dernier `Oh6smp = 22` | Le kind exposé à l'UI et au track. **Sérialisé dans `track-layout-v1`** → nouvelles variantes **à la fin**. |
| `DrumVoice` | `src/synthesis/mod.rs` | `COUNT = 25`, dernier `Oh606 = 24` | L'espace d'index du registre `INSTRUMENTS`. Garde `Tom1/2/3` séparés pour raisons historiques. |
| `DrumVoiceKind` | `src/synthesis/mod.rs` | enum de wrappers | La voix DSP concrète, pré-allouée par slot (enum, pas de `dyn`). |

Le pont entre les deux premiers : `TrackInstrumentKind::drum_voice_index()`.

Les deux listes ne coïncident pas : un même moteur peut servir plusieurs kinds
(`Ch6smp` et `Oh6smp` partagent `Ch606Voice`, chargé sur un banc différent), et
le kind `Tom` couvre à lui seul les trois rôles `Tom1/2/3` du générateur.

---

## 3. Fichiers clés

| Fichier | Rôle |
|---------|------|
| `src/track.rs` | `TrackInstrumentKind`, `InstrumentCategory`, `TrackSlot`, `TrackLayoutState`, `AtomicTrackLayout` (vue lock-free côté audio). |
| `src/synthesis/mod.rs` | `DrumVoice`, `DrumVoiceKind`, trait `Voice`, `DrumSynthesizer`, `VoiceSettings`, `create_voice_for_kind()`, `reinitialize_slot()`. |
| `src/synthesis/<voix>.rs` | Implémentation du trait `Voice` (ex. `bd606.rs`, `perc1.rs`, `ac_voice.rs`). |
| `src/synthesis/settings/<voix>.rs` | Struct de réglages typée + conversions `From`/`Into<VoiceSettings>`. |
| `src/synthesis/dsp.rs` | Briques DSP : enveloppes, filtres, oscillateurs, smoothers, `AnalogDrift`, `DcBlocker`, `RetrigDeclick`. |
| `src/synthesis/sample_bank.rs` | Banque de samples embarqués — le patron pour tout multisample. |
| `src/synthesis/ac606/` | Moteurs portés d'analogcode + `ac_voice.rs` (wrapper commun aux six voix AC). |
| `src/synthesis/special_params.rs` | `algos_for` — définitions d'algorithmes par voix. |
| `src/instrument_registry.rs` | **Source de vérité de l'UI** : `INSTRUMENTS`, `is_sampler()`, `param_default()`. |
| `src/sound_settings.rs` | `SoundSettingsState` — atomiques par slot (13 standards + `special[32]` + `freq_mode`), persistance `sound-settings-v2`. |
| `src/lib.rs` | Plugin : `DrumFlashParams`, boucle `process()`, changement de kind à chaud, `apply_choke_groups()`. |
| `src/ui/menus.rs` | Sélecteurs d'instrument — **data-driven** (`InstrumentCategory::ALL` + `kinds_in`). |
| `src/ui/sound_editor.rs` | Onglet Track + Sound Panel (data-driven, sauf deux listes, cf. §7). |
| `src/generator/mod.rs` | `remap_roles_to_slots()` — mappe les kinds vers les rôles du générateur. |

---

## 4. Les invariants qui cassent des sessions

**Persistance.** Tous les formats sont **positionnels** et **la longueur du blob
EST la version** : `pattern-v5`, `sound-settings-v2`, `plock-v1`,
`track-layout-v1`. On ne réutilise jamais une longueur, on ne renumérote jamais
un champ.

- `TrackInstrumentKind::index()` vaut `self as usize` et il est **écrit dans les
  sessions et les presets**. Insérer une variante au milieu renumérote tout ce
  qui suit et change silencieusement l'instrument de chaque lane sauvegardée.
- **On ne supprime jamais une variante.** On la *retire* : `selectable()` la
  cache des sélecteurs, `retired_replacement()` dit vers quel kind migrer, et
  `migrate_retired_kinds()` — appelée depuis `PersistentField::set`, l'unique
  entonnoir de toute écriture de layout — convertit les lanes sauvegardées. C'est
  ce qui a été fait pour Snare606 ([203]) et OpenHiHat ([204]) : les variantes et
  leurs voix DSP existent toujours.
- `LEGACY_VOICE_COUNT = 13` dans `sound_settings.rs` est **gelé**. N'utilisez
  jamais `DrumVoice::COUNT` pour de la persistance : il grandit à chaque ajout et
  casserait la détection des anciens formats.
- Tant que les paramètres tiennent dans `special[32]`, **aucun changement de
  format n'est nécessaire**. S'il en faut un, c'est une décision à prendre avec
  l'utilisateur, pas un détail d'implémentation.

**Thread audio.** `create_voice_for_kind()` est appelé **depuis `process()`** via
`reinitialize_slot()` quand l'utilisateur change le kind d'une lane en pleine
lecture. Toute donnée lourde (samples, tables) doit donc vivre derrière un
`OnceLock` **préchauffé** dans `DrumSynthesizer::initialize_with_layout()` —
voir `sample_bank::bank()` et `ac606::prewarm()`.

---

## 5. Les contrats de son

- **Retrigger ([179]).** Chaque coup repart d'un **état neuf**, la discontinuité
  étant absorbée par `dsp::RetrigDeclick` (3 ms). C'est le contrat actuel, et il
  remplace volontairement l'ancien comportement « phase continue » : **ne le
  réintroduisez pas**. Modèles : `kick.rs`, `kick_808.rs`, `ac_voice.rs`.
- **Enveloppe d'ampli** = `DecayReleaseEnvelope`, modèle **A-H-D**
  (Attack-Hold-Decay, **sans release**), avec courbes bipolaires indépendantes
  sur l'attaque et le decay. Les noms sont conservés pour la persistance :
  `decay_curve` = courbe de decay, `release_curve` = courbe d'**attaque**,
  `set_release`/`release_time` = **no-op**.
- **Jamais recréer une enveloppe dans `set_settings()`** : utilisez les setters
  (`set_decay`, `set_attack_ms`, `set_hold`, `set_curve`…). Recréer remet l'état
  interne à zéro et coupe le son à chaque mouvement de slider.
- **Live vs trigger-latch ([243], retour utilisateur 2026-09-23).** Deux
  catégories de paramètres, à trancher explicitement pour chaque voix :
  - **Paramètres continus** — pitch, filtre, enveloppes, saturation, niveau :
    ils doivent s'appliquer **au fil du jeu**, car les macros [242] et
    l'automation DAW poussent de nouvelles valeurs à chaque sous-bloc via
    `set_settings()` (nih-plug découpe le buffer aux points d'automation).
    Un pitch relu seulement dans `trigger()` produit un « décalage » : le
    knob glisse, le son suit par à-coups au coup suivant. Recalculez dans
    `set_settings()` tout état dérivé (pas de lecture, taux, fenêtre de
    temps…) — changer un **incrément** de lecture est continu en phase, sans
    clic ; changer une **position** ne l'est pas. Modèles :
    `oneshot.rs` / `rift.rs` (`base_step` recalculé si la voix est active).
  - **Paramètres de structure du coup** — choix du sample, offset de départ,
    reverse, loop, fenêtre de grain : verrouillés au **trigger** par design
    (comportement sampler standard ; les déplacer en plein jeu déchirerait
    la lecture). Les documenter comme tels dans la voix.
- **Saturation** : uniquement via `SaturationConfig::process_at(pre_stage, x)`,
  appelée deux fois (pré et post filtre), le drapeau `pre_filter` routant
  laquelle agit. N'appelez jamais `process()` directement depuis une voix.
- **`settings.volume` multiplie APRÈS la saturation** — la dérive de niveau
  analogique reste pré-sat.
- `DcBlocker` sur les voix à retrigger asymétrique (kick).

---

## 6. Ce qu'il faut écrire

L'ordre compte : à partir du point 3, les matchs exhaustifs du compilateur
deviennent le filet.

1. **Réglages typés** — `src/synthesis/settings/<voix>.rs` (modèle :
   `settings/perc1.rs`). Les 13 champs standard + les spéciaux **nommés**, les
   conversions dans les deux sens, le test
   `crate::settings_roundtrip_test!(…)`, et `pub mod <voix>;` dans
   `settings/mod.rs`.
2. **Voix DSP** — `src/synthesis/<voix>.rs`, trait `Voice`, §5 respecté à la
   lettre.
3. **`synthesis/mod.rs`** — `DrumVoice::X` (à la fin) + `COUNT` + `from_index` ;
   `VoiceSettings::x()` ; `DrumVoiceKind::X(XVoice)` avec son arm dans les **neuf
   méthodes** du trait (`trigger`, `trigger_hard`, `process_sample`,
   `process_sample_stereo`, `is_active`, `reset`, `set_settings`, `set_algo`,
   `set_special_param`) ; l'arm dans `create_voice_for_kind()` ; le préchauffage
   si données lourdes.
4. **`track.rs`** — variante **à la fin** de `TrackInstrumentKind`, puis `COUNT`,
   `from_index`, **`category()`**, `default_label` (2 caractères ASCII),
   `default_name`, `default_midi_note`, `drum_voice_index`,
   `from_drum_voice_index`.
5. **Registre** — `src/instrument_registry.rs` : la table de `StandardParamDef`
   et l'entrée `InstrumentDef` **à l'index `drum_voice_index()`**. Les spéciaux
   se déclarent avec `sp` (continu, morphable), `sp_unit` (avec unité
   d'affichage), `sp_curved` (loi de réponse non linéaire) ou `sp_discrete`
   (listes et types — **non morphable**). L'ordre de `sound_settings_default` est
   **strict** :

   ```
   [freq, decay, vol, filter_freq, attack, release, decay_curve,
    release_curve, hold, filter_env_amount, filter_env_decay, analog, stereo]
   ```

   Puis : l'index ajouté à la liste **mono** ou **stéréo** des tests en bas du
   fichier, et à `is_sampler()` si c'en est un.
6. **Algorithmes** — `special_params.rs` : si `algo_count > 1`, la const
   `*_ALGOS` et l'arm dans `algos_for`. Sinon une entrée « Standard ».
7. **Générateur** — `generator/mod.rs`, match `base_voice` de
   `remap_roles_to_slots`. **Sans rôle, GENERATE n'écrit rien sur la lane** et
   l'instrument passe pour cassé. Les voix 606 et AC606 empruntent toutes le rôle
   de leur cousin acoustique.

> **Convention « pack saturation »** : des paramètres sur des `special[i]`
> consécutifs — type (`sp_discrete`), amount (`sp_curved` avec
> `SAT_AMOUNT_CURVE`), mix, output_gain, et pre_filter (`sp_discrete`) quand la
> voix route la saturation des deux côtés du filtre. Voir BD606
> (`special[4..8]`) et les six voix AC606 (`special[10..13]`, sans pre_filter).

---

## 7. Côté UI : presque tout est automatique

Les **sélecteurs d'instrument** (popup « Add Module », menu Instrument, dropdown
Type) sont data-driven depuis [203]/[204] : ils bouclent sur
`InstrumentCategory::ALL` puis `TrackInstrumentKind::kinds_in(cat)` dans
`ui/menus.rs`. Renseigner `category()` suffit à faire apparaître l'instrument au
bon endroit — **il n'y a plus de liste `kinds` à éditer**. Le Sound Panel, le
menu de p-lock et le morphing sont eux aussi pilotés par le registre.

Il reste **deux listes codées en dur**, toutes deux dans `ui/sound_editor.rs` :

- `matches!(voice_idx, 2 | 3 | 7 | 8 | 10 | 12) || is_sampler(voice_idx)` — les
  voix **sans dérive analogique**, où le champ `analog` n'est qu'un remplissage
  à 0,0.
- `is_bass_drum` (`voice_idx == 0 || 11 || 18`) — l'affichage **Hz / Notes**.

---

## 8. Cas particulier : instrument multisample

Patron complet : `sample_bank.rs` + `bd606.rs` ; variante « un moteur, deux
bancs » : `ch606.rs` avec `Ch606Voice::with_bank` ([208]).

- WAV embarqué par `include_bytes!`, décodé **une seule fois** dans un `OnceLock`
  global → `&'static SampleBank`.
- Préchauffé dans `initialize_with_layout()` (hors temps réel) ; sur le thread
  audio, `bank()` n'est plus qu'une lecture atomique.
- **Pas de rééchantillonnage au chargement** : la lecture à position
  fractionnaire (interpolation linéaire) absorbe le rapport
  `source_rate / session_rate`.
- Fichier illisible → hits vides (voix inerte), **jamais de panic**.
- RNG de sélection de layer (xorshift) seedé à la construction, **jamais reseedé
  au trigger**.
- L'index va dans `instrument_registry::is_sampler()`.

---

## 9. Pièges déjà rencontrés

| Piège | Conséquence |
|-------|-------------|
| Variante insérée au milieu de l'enum | Toutes les lanes sauvegardées changent d'instrument. |
| Variante supprimée au lieu d'être retirée | Idem — passer par `selectable()` + `retired_replacement()`. |
| `DrumVoice::COUNT` utilisé pour la persistance | Les anciennes sessions ne sont plus relues (détection par longueur). |
| Enveloppes recréées dans `set_settings` | Le son se coupe à chaque mouvement de slider. |
| Phase conservée au retrigger | Contredit [179] : l'état repart neuf + `RetrigDeclick`. |
| Allocation dans `create_voice_for_kind` | Craquement ou underrun : il tourne sur le thread audio. |
| Rôle générateur oublié | Instrument muet sur GENERATE, ce qui passe pour un bug. |
| Mauvais ordre de `sound_settings_default` | Paramètres mélangés, souvent silencieusement. |
| `category()` oublié | Le compilateur le signale (match exhaustif) — ne pas le contourner par un `_ =>`. |
| Studio One ouvert au build | `-Install` échoue en « accès refusé » (lock DLL). |

---

## 10. Build & test

```powershell
cd drum-pattern-vst
cargo test          # dont le roundtrip de settings et les tests du registre
cargo check         # doit rester warning-clean
# Fermer Studio One avant l'install (lock DLL)
.\build.ps1 -Install
```

Les tests de `sound_settings.rs` itèrent sur `TrackInstrumentKind::COUNT` : un
défaut de fréquence ≤ 0 les fait échouer.

Puis, dans Studio One, vérifier que :

1. le nouveau kind apparaît dans le popup **Add Module** d'une lane vide et dans
   le sélecteur **Type**, **dans la bonne catégorie** ;
2. le Sound Panel affiche les bons sliders, groupés par famille ;
3. bouger un slider ne coupe pas le son ;
4. le menu de p-lock propose les paramètres standard **et** spéciaux ;
5. changer de kind **pendant la lecture** ne produit ni clic ni silence bloqué ;
6. sauvegarde puis réouverture de la song : kind et réglages conservés ;
7. GENERATE écrit bien une ligne sur la lane du nouvel instrument.

Enfin : une entrée dans `CHANGELOG.md` avec le build ID, la tâche sortie vers
`DONE.md`, et la liste numérotée « À tester dans Studio One » remise à
l'utilisateur.
