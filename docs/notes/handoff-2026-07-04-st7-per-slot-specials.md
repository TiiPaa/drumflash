# Handoff — Session 2026-07-04 — ST-7 : special params par slot (EN COURS)

## Contexte de la session

Suite des tests Studio One de l'utilisateur sur la grille modulaire 14 slots :

1. **Livré et validé** (commit `50ea359`, builds 20260704-165252/-173043/-174006) : stabilisation
   ST-1..ST-4c — crash lane 14, crash menus plock, settings/plocks appliqués par slot au trigger,
   pastille `+N` cliquable. L'utilisateur a confirmé « les BD sont bien instanciés ».
2. **Livré, à valider** (build 20260704-195335, non commité) : défaut 4 lanes (BD/SD/HH/Tom) +
   grille à hauteur fixe 14 rangées (règle UI : aucune ligne conditionnelle qui décale les zones ;
   mémorisée dans la mémoire persistante `ui-stable-zones`).
3. **EN COURS — ST-7** : l'utilisateur a constaté que le Click Type de la lane 5 (BD) changeait
   celui de la lane 1 (BD) et a demandé une refonte complète de l'instanciation. Code écrit,
   **compile (cargo check 0 erreur)**, mais **non testé, non buildé, non installé, non validé**.

## ST-7 — architecture mise en place (code écrit cette session)

Les special params (click, saturation, noise type, …) et le mode d'affichage Hz/Notes étaient des
paramètres nih-plug déclarés **une fois par voix legacy** (`kick_click_type`, `freq_mode_kick`, …)
→ physiquement partagés entre slots du même kind. Ils sont maintenant stockés **par slot** :

### `src/sound_settings.rs`
- `InstrumentSettingsState` + `special: [AtomicU32; SPECIAL_SLOT_COUNT=32]` + `freq_mode: AtomicU32`.
- API : `special_value(idx)`, `set_special(idx, v)`, `load_specials() -> [f32;32]`,
  `freq_mode() -> bool`, `set_freq_mode(bool)`, `reset_specials_for_voice(voice_idx)` (défauts registre).
- `SoundSettingsState::new(layout)` seed les specials par slot selon le kind.
- `reset_slot_to_defaults(slot, kind)` resette aussi specials + freq_mode.
- **Persistance** : champ DAW `sound-settings-v2` inchangé, nouveau format « v3 » détecté par longueur :
  `14 × FIELDS_PER_INSTRUMENT_V3 (13 standards + 32 specials + 1 freq_mode = 46) = 644 floats`.
  Les longueurs legacy (156/169/182) restaurent les standards et posent
  `needs_param_seed: AtomicBool = true`.
- `read_all()` émet toujours le format v3.

### `src/lib.rs`
- `voice_settings_for(slot_idx, voice_idx, …)` : specials via `instruments[slot_idx].load_specials()`,
  algo via `params.algos()[slot_idx]` clampé au `algo_count` du kind (l'ancien `match voice_idx` a été supprimé).
- **Migration one-shot dans `process()`** (avant le poll de version) : si `needs_param_seed`,
  copie des valeurs des params legacy par voix (`special_param(voice_idx, …)`, `freq_mode_kick/b8`)
  vers les atomics du slot correspondant, puis `bump_version()`. RT-safe (stores atomiques uniquement).
- Les params legacy par voix **restent déclarés** uniquement comme source de migration
  (compat sessions existantes). Plus aucune lecture ailleurs.
- **Ranges algo** : les 14 params `algo_*` (positionnels par slot) partagent maintenant
  `IntRange 0..instrument_registry::max_algo_index()` et sont renommés « Slot N Algo »
  (IDs de state inchangés). Corrige au passage `algo_cymbal`/`algo_s13` qui avaient `max: 0`
  (range crashogène, cf. bug [42]) et le fait qu'un Kick sur un slot ≠ 0 ne pouvait pas changer d'algo.

### `src/ui.rs`
- Sound Panel : widgets specials (switch pre-filter, selects saturation/noise/click type, sliders)
  lisent/écrivent `inst.special_value/set_special` + `sound_settings.bump_version()` (plus de ParamSetter).
- freq_mode (Hz/Notes) : `inst.freq_mode()/set_freq_mode()` dans le Sound Panel, le menu plock et
  le menu morph. Les checks bass-drum utilisent `voice_idx` (0|11), plus l'index de slot.
- Menu plock : valeur par défaut d'un special = `inst.special_value(…)` ; « Snapshot Current
  Settings » = `inst.load_specials()`.
- `current_field_value_for_fusion` : specials depuis `sound_settings.instruments[slot]`.
- Dev preset dumps : lecture/écriture via les atomics.

### `src/instrument_registry.rs`
- + `pub fn max_algo_index() -> i32` (max des `algo_count`, ≥ 1).

## REPRENDRE ICI — ce qui reste à faire

1. **3 warnings `unused variable` introduits** par le refactor : `params` ×1, `setter` ×2
   (arguments devenus inutiles dans des fonctions ui.rs — probablement `draw_plock_menu` /
   menu morph / une fonction du Sound Panel). Localiser :
   `cargo check 2>&1 | Select-String "unused variable" -Context 0,4`, préfixer `_` ou retirer l'argument.
2. **Tests unitaires à ajouter** (sound_settings.rs) : roundtrip v3 `read_all`/`write_all`
   (specials + freq_mode), blob legacy 182 floats → standards restaurés + `needs_param_seed == true`,
   `reset_slot_to_defaults` : specials = défauts du kind.
3. `cargo test` (attendu 103 lib + 72 standalone + nouveaux tests).
4. `.\build.ps1 -Install` — **sans pipe** (piège PowerShell 5.1 NativeCommandError) et Studio One fermé.
5. CHANGELOG + TODO (cocher ST-7) + mettre à jour `AGENTS.md` : l'invariant « every new special param
   needs a match arm in special_param() » devient « special_param() = source de migration uniquement ;
   les specials vivent par slot dans SoundSettingsState » + procédure ADDING_AN_INSTRUMENT à relire.
6. **Validation S1** :
   - Click Type lane 1 vs lane 5 (deux BD) → indépendants (le bug rapporté).
   - Saturation, mode Hz/Notes → indépendants par slot.
   - Changement d'algo sur une BD placée sur un slot ≠ 1 (avant : impossible, range 0..0/0..1).
   - Charger une song sauvegardée AVANT cette build → les réglages click/saturation existants
     doivent être conservés (migration `needs_param_seed`).
   - Valider aussi le point 2 (défaut 4 lanes + grille fixe 14 rangées, build 20260704-195335).
7. **Après validation** : commit (le checkpoint WIP de fin de session contient déjà tout),
   et envisager ST-7 phase 2 : suppression définitive des params specials legacy (rupture state).

## Points d'attention

- **Automation DAW des specials** : en déplaçant les specials hors des params nih-plug, ils ne sont
  plus automatisables par l'hôte (ils restent plockables par step, à la Elektron). Assumé ; à
  documenter dans le CHANGELOG de la build.
- La longueur du blob `sound-settings-v2` fait office de version : ne jamais réutiliser 644 floats
  pour un autre layout de champ ; prochaine évolution → nouvelle longueur ou champ v3 explicite.
- `test_standalone.rs` inclut `track.rs` (et non `sound_settings.rs`) via `#[path]` : les tests de
  track comptent dans les deux cibles (c'est pour ça que 104/73 → 103/72 après suppression d'un test).

## Validation à refaire si un agent reprend

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
cargo check    # 0 erreur attendu, 37 warnings préexistants + 3 à nettoyer
cargo test
.\build.ps1 -Install    # plainement, S1 fermé
```
