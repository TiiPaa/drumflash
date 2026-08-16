# Handoff — Flash Drum VST3

**Date:** 2026-08-16 · **Branch:** `main` (unique — skeuo-vector supprimée) · **Latest installed build:** `20260816-185337`
**Read [`CLAUDE.md`](../CLAUDE.md) first** — it is the canonical architecture + invariants + workflow reference for all AI agents. This file is a session-state handoff, not a replacement for it.

---

## 1. Snapshot / working-tree state

- Branch **`main`** (skeuo-vector fusionnée puis supprimée le 2026-08-13 — travailler sur `main`).
- Build **green** : `cargo test` = **450 tests** (277 lib + 1 + 172 integration), `build.ps1 -Install` OK.
- **Deploy caveat (rappel)** : Studio One locke la DLL — le fermer avant `build.ps1 -Install`. Lancer le build **plainly** (pas de `2>&1` sur PS 5.1).

## 2. Last completed (cette session, builds 20260816)

| Task | Résumé | Build |
|---|---|---|
| **[150]** | **Gestion des presets** : modal « Presets » (bouton header entre MIDI Pat et Settings, vbars 2 px) — 4 onglets **Instruments / Patterns / Grid / Songs**, JSON versionné sous `Documents/Flash Drum/presets/`, factory embeddés via `include_str!` (`factory_presets.rs`, vides), outil dev « Export factory » → staging `_factory/`. Grid = kits de lanes (les ex Clear All/4/12 de la page-bar y vivent ; dropdown page-bar supprimé). | 20260814-171844 |
| **[171]** | **MIDI Pat sans retrig** : `pending_song_pattern_restart` non levé sur switch MIDI → reprise à la volée ; page absente → page 1 via le resync host modulo nouvelle longueur. | 20260816-151800 |
| **[172]** | Temps forts 1/5/9/13 éclaircis (voile `white_a(6)` sur cellules OFF — itéré 26→12→6). | 20260816-173323 |
| **[169]** | Clap volume par défaut 0.7 → 1.0. | 20260816-151800 |
| **[167]** | Randomize Lane : slider **Density** (5-100 %) dans le menu lane (`randomize_density`, fallback 30 %). | 20260816-183613 |
| **[170]** | Curves bipolaires renforcées : exposant `1+3|c|` → `1+5|c|` (dsp + buzz + graphes). | 20260816-183613 |
| **[168]** | **Stéréo 2 samples** voix smp : switch **Stereo sous le sélecteur Sample** (infobulle EN) ; sélecteur en **paires** 1+2/3+4/5+6/7+8 quand Stereo ON ; **compatible Analog Mode** (paire aléatoire par coup) ; DSP filtre+DC par canal. | 20260816-185337 |

**Tout ce batch attend validation Studio One** (checklists données en fin de chaque build).

## 3. Pending tasks (TODO.md)

**Resume point (`REPRENDRE ICI`) = [166].** L'utilisateur choisit — présenter la liste, ne pas auto-démarrer.

- **[166]** Stutter × fusions (exclusifs aujourd'hui) — **étude d'abord**.
- **[173]** Presets d'usine de départ (composer + embarquer via l'outil factory).
- Backlog : [144] [146] [152] [94] [95] [69] [27] [84] [56] [41] + [BUG-LANE-DESYNC].

Rappel : « next » / « on continue » → présenter la liste TODO, ne pas coder.

## 4. Gotchas / lessons learned

- **PowerShell 5.1** : jamais `2>&1`/`2>$null` sur un exe natif ; jamais d'édition de source via `Set-Content` (encodage) — outil Edit uniquement.
- **Studio One DLL lock** : `-Install` échoue si S1 ouvert.
- **Anti-click** : pas de recréation d'enveloppe dans `set_settings()` ; `DecayReleaseEnvelope` est `Copy` (`with_*` en statement = no-op).
- **Blobs positionnels** : la longueur EST la version (`sound-settings-v2`, `plock-v1`, `pattern-v5`). Les presets JSON (user/factory) ont un champ `version` propre — format texte, migration explicite.
- **Labels boutons ASCII only** ([73]/[164]).
- **UI zones stables** : griser, ne pas cacher ; pas de ligne conditionnelle qui décale.
- **Skeuo rendering** : tout dans `src/ui/skeuo.rs`. **Keycaps 3D : trop lourds dans les sous-menus imbriqués** → `context_menu_row_plain` (lignes plates) pour les menus cascadés.
- **`EditorUIState` dérive `Default`** : un `f32` ajouté vaut 0.0 par défaut — prévoir un accesseur avec fallback (cf. `randomize_density()`).
- **Graphes Sound Editor** : empiler un 2e LCD dans une famille peut faire grandir la section et décaler le bloc — préférer une sous-rangée dédiée (cf. gate Buzz [160]).

## 5. Key file map (features récentes)

- Presets : `src/presets.rs`, `src/factory_presets.rs`, `src/ui/preset_browser.rs` ; bouton header dans `src/ui/header.rs`.
- Stéréo smp : `src/synthesis/{bd606,sd606,ch606}.rs` (`stereo_pair`, `process_sample_stereo`), UI dans `src/ui/sound_editor.rs` (sous le select Sample, paires).
- Microtiming : `src/sequencer/mod.rs` (early/late fire), `src/groove.rs` (`step_start_beat`).
- Catégories instruments : `src/track.rs` (`InstrumentCategory`), menu cascadé dans `src/ui/grid.rs`, lignes plates dans `src/ui/menus.rs`.

## 6. If you're picking this up

1. Read `CLAUDE.md`. 2. Tout est **commité** sur `main` (push à confirmer par l'utilisateur) ; les builds 20260814/16 attendent validation Studio One. 3. Resume point = **[166]** mais attendre le choix explicite de l'utilisateur. 4. Build+install → terminer chaque rapport par la checklist « À tester dans Studio One ».
