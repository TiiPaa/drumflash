# Handoff 2026-08-20 — SDrex, clears mute/solo, graphes unifiés

**Date:** 2026-08-20 · **Branch:** `main` · **Latest installed build:** `20260820-083705` · **HEAD:** `8ebf004` (commité + pushé)
**Read [`CLAUDE.md`](../../CLAUDE.md) first** — it is the canonical architecture + invariants + workflow reference for all AI agents. This file is a session-state handoff, not a replacement for it.

---

## 1. Snapshot / working-tree state

- Branch **`main`**, working tree **propre**, tout est commité ET pushé (`8ebf004`).
- Build **green** : `cargo test` = **487 tests** (297 lib + 1 + 189 integration), warning-clean, `build.ps1 -Install` OK.
- **Deploy caveat (rappel)** : Studio One locke la DLL — le fermer avant `build.ps1 -Install`. Lancer le build **plainly** (pas de `2>&1` sur PS 5.1).

## 2. Last completed (sessions 2026-08-19 → 2026-08-20)

| Task | Résumé | Build |
|---|---|---|
| **[175]** | **Nouvel instrument SDrex** (kind 15 / voice 17, catégorie SD, note 48, rôle Snare) — recette « drex_snare » : corps sine (drop +95 Hz) + noise HP + metal ring-mod 620×910, mix 0.5/0.8/0.18 ; **section Modulation** autonome avec switch Flanger/Filter Mod (Rate/Depth/Wet partagés, Delay/Fdbk réservés au flanger, Free Phase) ; enveloppe volume **A-H-D** + Attack/Decay Curve ; filtre LP + env A-H-D bipolaire (decays jusqu'à 1,5 s) ; drive tanh fixe. Correctifs RT : buffer flanger fixe, snapshot Pattern Bank sans pointeur brut, sérialisation hors callback audio. | 20260819-202022 |
| **[174]** | Graphes/DSP env filtre (span normalisé Toms, `FILTER_ENV_CURVE = 6.0` smp+Perc1, graphe ampli smp A-H-D) + **plocks sound perdus au chargement pattern** (`set_raw` field masks) + filtre Tom (double avance pitch_env, plancher cutoff 100 Hz, biquad 12 dB/oct, sweep exponentiel 20 kHz). Commit `400a534`. | 20260819-114620 |
| **[176]** | **Fix solo/mute invisibles après les clears** — un solo sur une lane désactivée continuait de muter tout le kit. Helper `controls::clear_all_mutes_solos` appelé par : Clear All header, presets de layout (Clear All/4/12 Lanes + kits), presets de style Generate, Delete Lane. | 20260820-082201 |
| **[178]** | **Graphes d'enveloppe unifiés** — socle commun `prep_graph` (LCD + padding 12/10 + grille) dans `envelope_viz.rs` ; **couleurs de stages partout** : attaque ambre / hold vert / decay bleu (filtre A-H-D Buzz/SDrex + ampli samplers désormais segmentés ; courbes mono-stage en bleu, l'orange `envelope_curve` supprimé du thème) ; trait 2 px, `draw_cutoff_line` factorisée. | 20260820-083705 |
| **[177]** | **Perc1 « sature / FM » → FAUX POSITIF** : le fix Algo [175] faisait enfin jouer l'algo Saw stocké en session (ignoré avant). Pas de régression DSP ; validé par l'utilisateur. ⚠️ Une migration auto (forcer Sine) a été envisagée puis **rejetée** — elle écraserait les choix Saw volontaires à chaque chargement. | — |

**Builds 20260819/20260820 en attente de validation Studio One** (checklists données en fin de chaque build).

## 3. Pending tasks (TODO.md)

**Resume point (`REPRENDRE ICI`) = [166].** L'utilisateur choisit — présenter la liste, ne pas auto-démarrer.

- **[166]** Stutter × fusions (exclusifs aujourd'hui) — **étude d'abord** (sémantique temporelle, rendu audio, export MIDI [158], UI/plocks).
- **[173]** Presets d'usine de départ (composer + embarquer via « Export factory (dev) » → `assets/presets/` + `factory_presets.rs`).
- Backlog : [144] Snare algo (partiellement couvert par SDrex ?), [146] env exponentielles négatives, [152] instrument Ambiant, [94] [95] [69] [27] [84] [56] [41].
- Notes utilisateur dans `docs/notes/notes.txt` non encore ticketées : fine-tune slider (Alt+?) cassé, switch Modulation SDrex pas clair (« Filter LFO / Flanger »), remplacer Delay modulation SDrex par un fadein (+ ne pas griser en Filter LFO), holds SDrex à 1 s, clap decay max 1,5 s.

Rappel : « next » / « on continue » → présenter la liste TODO, ne pas coder.

## 4. Gotchas / lessons learned

- **PowerShell 5.1** : jamais `2>&1`/`2>$null` sur un exe natif ; jamais d'édition de source via `Set-Content` (encodage) — outil Edit uniquement.
- **Studio One DLL lock** : `-Install` échoue si S1 ouvert (vérifier `Get-Process "Studio One"`).
- **Anti-click** : pas de recréation d'enveloppe dans `set_settings()` ; `DecayReleaseEnvelope` est `Copy` (`with_*` en statement = no-op).
- **Blobs positionnels** : la longueur EST la version (`sound-settings-v2`, `plock-v1`, `pattern-v5`). Les presets JSON ont un champ `version` propre.
- **Plock Attack/special[4]** : le champ 18 est réservé à Attack ; la boucle specials skip `ATTACK_FIELD` — ne jamais mapper un special sur l'index 4 dans le layout plock (test `morphable_specials_never_use_the_reserved_attack_field`).
- **Mute/solo = BoolParams par slot** : toute action qui désactive/remplace des lanes doit les remettre à off via `controls::clear_all_mutes_solos` ([176]).
- **Labels boutons ASCII only** ([73]/[164]).
- **UI zones stables** : griser, ne pas cacher ; pas de ligne conditionnelle qui décale.
- **Skeuo rendering** : tout dans `src/ui/skeuo.rs`. Keycaps 3D trop lourds en sous-menus → `context_menu_row_plain`.
- **Graphes Sound Panel** ([178]) : tout graphe = `prep_graph` + `draw_grid_lines` + stages `stage_attack/hold/decay` ; waveform sous la grille pour les samplers. Ne plus réintroduire de cadre LCD inline.
- **`EditorUIState` dérive `Default`** : un `f32` ajouté vaut 0.0 — accesseur avec fallback (cf. `randomize_density()`).

## 5. Key file map (features récentes)

- SDrex : `src/synthesis/sdrex.rs`, `src/synthesis/settings/sdrex.rs` (graphes via `draw_buzz_filter_envelope`, specials par slot).
- Graphes : `src/ui/envelope_viz.rs` (socle `prep_graph`, stages colorés), appels dans `src/ui/sound_editor.rs` (~l.1430-1570).
- Clears mute/solo : `src/ui/controls.rs` (`clear_all_mutes_solos`), appelé dans `header.rs`, `sound_editor.rs` (`apply_lane_layout_preset`), `bottom_panel.rs` (`apply_style_preset`), `grid.rs` (`deactivate_slot`).
- Presets : `src/presets.rs`, `src/factory_presets.rs`, `src/ui/preset_browser.rs`.
- Stéréo smp : `src/synthesis/{bd606,sd606,ch606}.rs` ; microtiming : `src/sequencer/mod.rs` + `src/groove.rs`.

## 6. If you're picking this up

1. Read `CLAUDE.md`. 2. Tout est **commité et pushé** sur `main` ; les builds 20260819/20260820 attendent validation Studio One. 3. Resume point = **[166]** mais attendre le choix explicite de l'utilisateur. 4. Build+install → terminer chaque rapport par la checklist « À tester dans Studio One ».
