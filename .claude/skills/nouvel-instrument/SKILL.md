---
name: nouvel-instrument
description: Ajouter un nouvel instrument (kind + voix DSP) au plugin Flash Drum, en respectant les trois espaces d'index, les contrats de persistance et les regles du thread audio. A utiliser des que l'utilisateur demande d'ajouter/creer un instrument, une voix, un moteur de synthese ou un sampler dans Flash Drum.
---

Tu ajoutes un instrument a **Flash Drum** (`drum-pattern-vst/`). Ce fichier est la
procedure **executable**, a jour du code. `ADDING_AN_INSTRUMENT.md` donne le fond
architectural ; **en cas de desaccord, le code fait foi, puis ce fichier.**

---

## 0. Avant d'ecrire une ligne : cadrer avec l'utilisateur

Ne devine pas ces cinq points, demande-les (en une seule fois, avec une
recommandation par point) :

1. **Nom et famille** : nom complet, label 2 caracteres (ASCII), et **categorie**
   (`BassDrum`, `Snare`, `HiHat`, `Perc`, `Fx`, `Other`) - c'est elle qui range
   l'instrument dans les menus.
2. **Origine du son** : synthese pure, portage d'un moteur existant, ou
   **multisample** (WAV embarque). Le cas sample a sa propre section (§5).
3. **Note MIDI par defaut** (General MIDI, sans collision avec les kinds
   existants - verifie avec `default_midi_note`).
4. **Parametres specifiques** : lesquels, leurs plages, leurs defauts. Ils vivent
   dans `special[0..32]`. Precise ceux qui sont **discrets** (listes, types) :
   ils se declarent avec `sp_discrete` et ne sont pas morphables.
5. **Mono ou stereo**, et **nombre d'algorithmes** (1 = « Standard »).

Ajoute aussi : **choke group** souhaite (0 = aucun, 1..4) et **role generateur**
a emprunter (§3.7) - sans role, GENERATE laissera la lane muette.

---

## 1. Les trois espaces d'index

Un instrument touche **trois** numerotations distinctes. Les melanger est
l'erreur classique du projet.

| Enum | Fichier | Valeur actuelle | Role |
|---|---|---|---|
| `TrackInstrumentKind` | `src/track.rs` | `COUNT = 23`, dernier `Oh6smp = 22` | Le kind choisi par l'utilisateur. **Persiste dans `track-layout-v1`.** |
| `DrumVoice` | `src/synthesis/mod.rs` | `COUNT = 25` | Espace d'index du registre `INSTRUMENTS`. |
| `DrumVoiceKind` | `src/synthesis/mod.rs` | enum de wrappers | La voix DSP concrete, pre-allouee par slot. |

Le pont : `TrackInstrumentKind::drum_voice_index()` -> index `DrumVoice`/registre.

Un nouvel instrument prend donc **kind 23** et **voix 25** (verifie les valeurs
dans le code avant d'ecrire - elles bougent a chaque ajout).

---

## 2. Invariants non negociables

Les enfreindre casse des sessions utilisateur deja enregistrees, ou fait
craquer le thread audio. Aucun n'est negociable sans decision explicite.

**Persistance**

- Les variantes s'ajoutent **a la fin** de l'enum. `index()` vaut `self as usize`
  et il est ecrit dans les sessions : inserer au milieu renumerote tout ce qui
  suit et change silencieusement l'instrument de chaque lane sauvegardee.
- On ne **supprime jamais** une variante. On la **retire** des selecteurs via
  `selectable()` + `retired_replacement()` (modele : [203] Snare606, [204]
  OpenHiHat).
- `LEGACY_VOICE_COUNT = 13` dans `sound_settings.rs` est **gele**. N'utilise
  jamais `DrumVoice::COUNT` pour de la persistance : il grandit, et la detection
  des anciens formats se fait **par longueur de blob**.
- Tant que les parametres tiennent dans `special[32]`, **aucun changement de
  format n'est necessaire**. S'il en faut un, arrete-toi et pose la question.

**Thread audio** (`process()` et tout ce qu'il appelle)

- Aucune allocation, aucun lock bloquant, aucun `unwrap()` sur des donnees hote,
  aucun panic (`panic = "abort"` en release).
- `create_voice_for_kind()` est appele **depuis `process()`** via
  `reinitialize_slot()` quand l'utilisateur change le kind d'une lane en pleine
  lecture. Toute donnee lourde (samples, tables) doit donc etre derriere un
  `OnceLock` **pre-chauffe** dans `DrumSynthesizer::initialize_with_layout()`
  (modeles : `sample_bank::bank()`, `ac606::prewarm()`).

**Son**

- **Contrat de retrigger [179]** : chaque coup repart d'un **etat neuf**, avec
  `dsp::RetrigDeclick` (3 ms) pour absorber la discontinuite. **Ne reviens pas a
  la phase continue** - c'est l'ancien comportement, supprime volontairement.
  Modeles : `kick.rs`, `kick_808.rs`, `ac_voice.rs`.
- **Jamais recreer une enveloppe dans `set_settings()`** : utilise les setters
  (`set_decay`, `set_attack_ms`, `set_hold`, `set_curve`…). Recreer coupe le son
  a chaque mouvement de slider.
- **Saturation** : uniquement via `SaturationConfig::process_at(pre_stage, x)`,
  appelee deux fois (pre et post filtre), le drapeau `pre_filter` route laquelle
  agit. N'appelle jamais `process()` directement.
- **`settings.volume` multiplie APRES la saturation** (la derive de niveau
  analogique reste pre-sat).
- Enveloppe d'ampli = `DecayReleaseEnvelope`, modele **A-H-D** (Attack-Hold-Decay,
  **pas de release**). Les noms `release_curve` (= courbe d'**attaque**) et
  `set_release` (= no-op) sont conserves pour la persistance.
- **Live vs trigger-latch ([243])** : les parametres **continus** (pitch,
  filtre, enveloppes, saturation, niveau) doivent s'appliquer au fil du jeu via
  `set_settings()` — les macros [242]/l'automation poussent une valeur a chaque
  sous-bloc ; un pitch relu seulement dans `trigger()` decale le son par
  a-coups. Recalcule dans `set_settings()` l'etat derive (increments de
  lecture, taux — continu en phase, sans clic ; jamais une position). Modeles :
  `oneshot.rs`, `rift.rs`. Les parametres de **structure du coup** (sample,
  offset, reverse, loop, grain) restent verrouilles au trigger par design.

---

## 3. Ordre de travail

Fais les etapes **dans cet ordre** : le compilateur devient ton filet a partir de
l'etape 3 (les matchs exhaustifs signalent tout ce qui manque).

### 3.1 Reglages types - `src/synthesis/settings/<voix>.rs`

Modele : `settings/perc1.rs`. Struct avec les 13 champs standard + les champs
speciaux **nommes**, plus `From<VoiceSettings>` et `From<XSettings> for
VoiceSettings` (les speciaux vivent dans `special[0..N]`).

- Test obligatoire : `crate::settings_roundtrip_test!(x_settings_roundtrip, x, XSettings);`
- Declarer `pub mod x;` dans `settings/mod.rs`.

> **Convention pack saturation** : 5 params sur des `special[i]` consecutifs -
> type (`sp_discrete`), amount, mix, output_gain, pre_filter (`sp_discrete`).

### 3.2 Voix DSP - `src/synthesis/<voix>.rs`

Implemente le trait `Voice`. Modeles : `perc1.rs` (synthese), `bd606.rs`
(sample), `ac_voice.rs` (moteur porte). Respecte le §2 « Son » a la lettre.

### 3.3 `src/synthesis/mod.rs`

1. `mod x;` + `pub use x::XVoice;` + `pub use settings::x::XSettings;`
2. `DrumVoice::X = 25`, incremente `COUNT`, ajoute l'arm dans `from_index()`.
3. `VoiceSettings::x()` - defauts **identiques** au `sound_settings_default` du
   registre, puis les defauts des speciaux dans l'ordre `special[i]`.
4. `DrumVoiceKind::X(XVoice)` + l'arm dans les **9 methodes** du trait :
   `trigger`, `trigger_hard`, `process_sample`, `process_sample_stereo`,
   `is_active`, `reset`, `set_settings`, `set_algo`, `set_special_param`.
5. L'arm dans `create_voice_for_kind()`.
6. Si donnees lourdes : pre-chauffage dans `initialize_with_layout()`.

### 3.4 `src/track.rs`

Variante **a la fin** de `TrackInstrumentKind`, puis : `COUNT`, `from_index`,
`category()` (**c'est elle qui fait apparaitre l'instrument dans les menus**),
`default_label` (2 chars ASCII), `default_name`, `default_midi_note`,
`drum_voice_index`, `from_drum_voice_index`.

`selectable()` renvoie `true` par defaut - n'y touche que pour un retrait.

### 3.5 Registre - `src/instrument_registry.rs`

C'est **la source de verite de l'UI** : le Sound Panel et les p-locks sont
data-driven, donc bien remplir cette entree suffit a les cabler.

1. Table `X_STD` de `StandardParamDef` (ou reutilise `FULL_STD` / `TOM_STD` / …).
2. Entree `InstrumentDef` **a l'index `drum_voice_index()`** : `name`, `label`,
   `full_name`, `midi_note`, `algo_count`, `standard_params`, `special_params`
   (`sp` = continu et morphable, `sp_discrete` = pas de morph), `filter_type_label`,
   `freq_display_ratio`, et `sound_settings_default` dans **l'ordre strict des 13
   champs** :

   ```
   [freq, decay, vol, filter_freq, attack, release, decay_curve,
    release_curve, hold, filter_env_amount, filter_env_decay, analog, stereo]
   ```

3. Ajoute l'index a la liste **mono** ou **stereo** des tests en bas du fichier.
4. Si c'est un sampler : ajoute l'index a `is_sampler()`.

### 3.6 Algorithmes - `src/synthesis/special_params.rs`

Si `algo_count > 1` : const `X_ALGOS` + arm dans `algos_for` (match exhaustif sur
`DrumVoice`). Sinon une seule entree « Standard ».

### 3.7 Generateur - `src/generator/mod.rs`

Dans `remap_roles_to_slots`, ajoute l'arm `TrackInstrumentKind::X => <role>` du
match `base_voice`. **Sans role, GENERATE n'ecrit rien sur la lane** et
l'instrument parait casse. Emprunte le role de son cousin acoustique (les voix
606 et AC606 font exactement ca).

### 3.8 Listes encore codees en dur - `src/ui/sound_editor.rs`

Les selecteurs d'instrument sont data-driven (`InstrumentCategory::ALL` +
`kinds_in(cat)` dans `ui/menus.rs`) : **rien a y faire**. Il reste deux listes a
verifier :

- `matches!(voice_idx, 2 | 3 | 7 | 8 | 10 | 12) || is_sampler(voice_idx)` - les
  voix **sans derive analogique** (le champ `analog` y est un remplissage a 0,0).
  Les samplers sont deja couverts par `is_sampler`, donc il n'y a rien a ajouter
  pour eux.
- `is_bass_drum` (`voice_idx == 0 || 11 || 18`) - ajoute l'index si l'affichage
  **Hz / Notes** a du sens pour cet instrument.

---

## 4. Verification avant de builder

Lance ces controles et **montre le resultat** ; ils attrapent ce que le
compilateur ne voit pas :

```bash
cd drum-pattern-vst
cargo test                 # dont le roundtrip de settings et les tests du registre
cargo check                # doit rester warning-clean
grep -n "TrackInstrumentKind::X" src/generator/mod.rs   # role generateur present ?
grep -n "X" src/instrument_registry.rs | head           # entree registre a l'index attendu ?
```

Les tests de `sound_settings.rs` iterent sur `TrackInstrumentKind::COUNT` : un
defaut de frequence <= 0 les fait echouer.

---

## 5. Cas particulier : instrument multisample

Modele complet : `sample_bank.rs` + `bd606.rs`.

- WAV embarque par `include_bytes!`, decode **une seule fois** dans un `OnceLock`
  global -> `&'static SampleBank`.
- Pre-chauffe dans `initialize_with_layout()` (hors temps reel) ; sur le thread
  audio, `bank()` n'est plus qu'une lecture atomique.
- **Pas de resampling au chargement** : la lecture a position fractionnaire
  (interpolation lineaire) absorbe le rapport `source_rate / session_rate`.
- Fichier illisible -> hits vides (voix inerte), **jamais de panic**.
- RNG de selection de layer (xorshift) seede a la construction, **jamais reseede
  au trigger**.
- Ajoute l'index a `instrument_registry::is_sampler()`.

---

## 6. Livraison

```powershell
cd drum-pattern-vst
cargo test
# Studio One DOIT etre ferme : il verrouille la DLL
.\build.ps1 -Install
```

Puis, obligatoirement :

1. Une entree dans `CHANGELOG.md` avec le build ID.
2. La tache cochee et **sortie vers `DONE.md`** (voir la skill `matodo`).
3. **Une liste « A tester dans Studio One (build <ID>) » numerotee**, une action
   par ligne, qui couvre au minimum :
   - l'instrument apparait dans le menu **Add Module** d'une lane vide et dans le
     selecteur **Type**, dans la bonne categorie ;
   - le Sound Panel affiche les bons sliders, groupes par famille ;
   - bouger un slider ne coupe pas le son ;
   - le menu de p-lock propose les parametres standard **et** speciaux ;
   - changer de kind **pendant la lecture** : pas de clic, pas de silence bloque ;
   - sauvegarde puis reouverture de la song : kind et reglages conserves ;
   - GENERATE ecrit bien une ligne sur la lane du nouvel instrument.

---

## 7. Pieges deja rencontres

| Piege | Consequence |
|---|---|
| Variante inseree au milieu de l'enum | Toutes les lanes sauvegardees changent d'instrument. |
| `DrumVoice::COUNT` utilise pour la persistance | Les anciennes sessions ne sont plus relues (detection par longueur). |
| Enveloppes recreees dans `set_settings` | Le son se coupe a chaque mouvement de slider. |
| Phase conservee au retrigger | Contredit [179] : l'etat repart neuf + `RetrigDeclick`. |
| Allocation dans `create_voice_for_kind` | Craquement/underrun : il tourne sur le thread audio. |
| Role generateur oublie | Instrument muet sur GENERATE, qui passe pour un bug. |
| Mauvais ordre de `sound_settings_default` | Parametres melanges, souvent silencieusement. |
| `category()` oublie | Le compilateur le signale (match exhaustif) - ne le contourne pas par un `_ =>`. |
| Studio One ouvert au build | `-Install` echoue en « acces refuse ». |

---

## Demande de l'utilisateur

{ARGUMENTS}
