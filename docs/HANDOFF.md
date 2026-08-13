# Handoff — Flash Drum VST3

**Date:** 2026-08-13 · **Branch:** `main` (skeuo-vector fusionnée dedans — ne plus l'utiliser) · **Latest installed build:** `20260813-174717`
**Read [`CLAUDE.md`](../CLAUDE.md) first** — it is the canonical architecture + invariants + workflow reference for all AI agents. This file is a session-state handoff, not a replacement for it.

---

## 1. Snapshot / working-tree state

- Branch **`main`** @ latest ; `skeuo-vector` a été fusionnée (ff) dans `main` le 2026-08-13 — travailler sur `main` désormais.
- Build **green** : `cargo test` = **436 tests pass** (267 lib + 1 + 168 integration), `cargo check` warning-clean (sauf le `kick_out` unused-var pré-existant dans un test d'intégration), `build.ps1 -Install` OK.
- **Tout le batch [164]/[162]/[165]/[160]/[161]/[163] est validé dans Studio One (2026-08-13).**
- **Deploy caveat (rappel)** : Studio One locke la DLL VST3 — il doit être fermé avant `build.ps1 -Install`. Lancer le build **plainly** (pas de `2>&1`/`2>$null` — PowerShell 5.1 transforme le stderr de cargo en faux `NativeCommandError`).

## 2. Last completed — [163] validé + batch [164]/[162]/[165]/[160]/[161] validé (builds 20260813)

### [163] Catégories d'instruments + type via clic droit lane — DONE & validé (build 20260813-174717)
`InstrumentCategory` (BD/SD/HH/PERC/FX/OTHER) sur `TrackInstrumentKind` (`category()`/`kinds_in()`/`ALL`, `track.rs`). Sous-menus **cascadés** « Instrument ▸ Catégorie ▸ kind » dans le menu clic-droit du nom de lane (`grid.rs`), kind courant « > » bleu non cliquable ; `change_slot_kind()` = même sémantique que le dropdown Type (nom + note MIDI + reset défauts). Popup Add Module groupé par catégorie (plat, headers). **Hover highlight** sur `context_menu_button` (`menus.rs`) → tous les menus contextuels. Tests partition + spot-checks.

### Retours post-batch (build 20260813-143901)
[160] graphe gate **replacé à droite des sliders Gate Rate/Depth/Shape** (l'empilement sous le graphe d'ampli décalait le bloc) ; [161] nudge étendu à **±100 ms** (UI, clamps sequencer/midi_export, test −75 ms).

### Batch 20260812-123906 (détail dans CHANGELOG)
[164] glyphe morphing « X » · [162] env grisées One-Shot (smp) · [165] drag lane vide · [160] graphe Gate Shape Buzz · [161] microtiming seq-plock (séquenceur early/late fire, `groove::step_start_beat`, UI Nudge, export MIDI).

## 3. Pending tasks (TODO.md)

**Resume point (`REPRENDRE ICI`) = [166].** L'utilisateur choisit la tâche — présenter la liste, ne pas auto-démarrer.

- **[166]** Mixer le **stutter avec les cellules fusionnées** (aujourd'hui exclusifs). **Bien étudier avant de coder** : sémantique temporelle, audio, export MIDI, UI/plocks.
- ~~**[163]**~~ FAIT & validé (build 20260813-174717).
- **[155] ANNULÉ** par l'utilisateur (2026-08-12).
- Backlog P2/P3 : [144] [146] [150] [152] [69] [27] [56] [41] [84] [83cont] [94] [95] et **[BUG-LANE-DESYNC]**.

Rappel : quand l'utilisateur dit « next » / « on continue », **ne pas coder** — présenter la liste TODO et le laisser choisir.

## 4. Gotchas / lessons learned (aussi en mémoire agent)

- **PowerShell 5.1** : jamais `2>&1`/`2>$null` sur un exe natif (cargo/build.ps1) — stderr devient `NativeCommandError`. Et ne JAMAIS éditer un fichier source via `Set-Content` (risque d'encodage) — utiliser l'outil Edit.
- **Studio One DLL lock** : `-Install` échoue si S1 est ouvert (`Get-Process "Studio One"` pour vérifier). Build plainly, ne remonter S1 que si la copie échoue.
- **« Aucune différence après un build »** = souvent une DLL cachée dans Studio One — vérifier le build ID dans le header du plugin, retirer/réajouter l'instance.
- **Anti-click** : ne jamais recréer une enveloppe dans `set_settings()` ; utiliser les setters. `DecayReleaseEnvelope` est `Copy` — `x.with_attack_ms(..)` en statement = no-op silencieux.
- **Blobs positionnels** : la longueur EST la version — ne jamais renuméroter un champ ni réutiliser une longueur. `sound-settings-v2`, `plock-v1`, `pattern-v5`.
- **UI zones stables** : pas de ligne conditionnelle qui décale l'UI — réserver l'espace, griser plutôt que cacher.
- **Labels de boutons : ASCII only** (tâche [73]) — les glyphes UTF-8 finissent corrompus en Windows-1252 ([164] en est un exemple résiduel).
- **Skeuo rendering** : tout vit dans `src/ui/skeuo.rs`, une fonction par élément.
- **Ne pas ajouter de scope non demandé** : implémenter exactement la demande ; proposer des idées de TODO, ne pas les construire en silence.
- **Graphes dans le Sound Editor** : un LCD par famille avec graphe (Env/Filter) ; pour empiler un 2e graphe (Buzz gate), tag texte en coin pour les distinguer.

## 5. Key file map (features de la session)

- Microtiming : `src/sequencer/mod.rs` (`TrackState::{late_trigger, suppress_next, early_fired}`, `classify_cell`, `eval_trigger`, `set_microtimings`), `src/groove.rs` (`step_start_beat`), `src/lib.rs` (copie 1×/buffer + `early_next_loop` → conditions), `src/ui/plock.rs` (row Nudge), `src/midi_export.rs` (nudge en ticks).
- One-Shot greying : `src/ui/sound_editor.rs` (`env_disabled`), `src/ui/envelope_viz.rs` (`draw_sample_amp_graph` one-shot gris).
- Gate graph Buzz : `src/ui/envelope_viz.rs` (`draw_buzz_gate_graph`), appelant `src/ui/sound_editor.rs`.
- Drag lane vide : `src/ui/grid.rs` (`draw_empty_slot_lane_v2`).
- Glyphe morphing : `src/ui/plock.rs` (`draw_morph_target_action_buttons`).

## 6. If you're picking this up

1. Read `CLAUDE.md`. 2. Tout est **commité** sur `main` ; le batch [164]/[162]/[165]/[160]/[161] + [163] sont **validés dans Studio One**. 3. Resume point = **[166]** mais attendre le choix explicite de l'utilisateur avant de coder. 4. Build+install et terminer chaque rapport de build par la checklist « À tester dans Studio One ».
