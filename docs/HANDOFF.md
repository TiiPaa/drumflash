# Handoff — Flash Drum VST3

**Date:** 2026-08-12 (soir) · **Branch:** `skeuo-vector` · **Latest installed build:** `20260812-123906`
**Read [`CLAUDE.md`](../CLAUDE.md) first** — it is the canonical architecture + invariants + workflow reference for all AI agents. This file is a session-state handoff, not a replacement for it.

---

## 1. Snapshot / working-tree state

- Branch `skeuo-vector` @ latest, **committed & pushed** ; `main` fast-forwardé sur `skeuo-vector` ce jour (les deux branches sont alignées).
- Build **green** : `cargo test` = **432 tests pass** (265 lib + 1 + 166 integration), `cargo check` warning-clean (sauf le `kick_out` unused-var pré-existant dans un test d'intégration), `build.ps1 -Install` OK.
- **Deploy caveat (rappel)** : Studio One locke la DLL VST3 — il doit être fermé avant `build.ps1 -Install`. Lancer le build **plainly** (pas de `2>&1`/`2>$null` — PowerShell 5.1 transforme le stderr de cargo en faux `NativeCommandError`).

## 2. Last completed — batch [164]/[162]/[165]/[160]/[161] (build 20260812-123906)

- **[164]** Glyphe « × » corrompu (« Ã— ») du bouton reset morphing → « X » ASCII (`ui/plock.rs`, convention [73]).
- **[162]** Voix smp (BD6/SD6/CH6smp) : **One Shot ON grise** les sliders Attack/Decay/Decay Curve (`add_enabled_ui` dans `sound_editor.rs` — le switch One Shot est un special param, reste actif) + ligne d'enveloppe grise dans `draw_sample_amp_graph`. On grise, on ne cache pas (règle zones stables).
- **[165]** **Drag d'une lane vide** : grip câblé comme les lanes actives dans `draw_empty_slot_lane_v2` (pas de `select_legacy_track` — un slot inactif n'a pas d'onglet). `apply_lane_reorder_move` était déjà slot-générique.
- **[160]** **Graphe Gate Shape (Buzz)** : `draw_buzz_gate_graph` (`envelope_viz.rs`) placé **à droite des sliders Gate Rate/Depth/Shape** (sous-rangée dédiée dans la famille Env — l'empilement sous le graphe d'ampli décalait le bloc, corrigé). Fenêtre **fixe 60 ms** (Rate visible), Smooth cosinus `^(1+4·shape)`, Razor rampe 0,3 ms + spike expo, plancher Depth, tag « GATE ».
- **[161]** **Microtiming par cellule (seq plock), ±100 ms complet** — n' avait JAMAIS été câblé au moteur (stockage/persistance seuls). Désormais appliqué **par le séquenceur** :
  - `groove::step_start_beat()` = inverse exacte de `beat_to_step` (paires swing/shuffle/MPC).
  - Nudge **positif** → `late_trigger`/`late_fire_beat` (tout le train stutter/fusion décalé d'un bloc).
  - Nudge **négatif** → peek de la cellule du prochain boundary à chaque sample ; fire en avance quand temps restant ≤ −nudge ; `classify_cell`/`eval_trigger` partagés avec le chemin normal (masque/humanize/fusions/morphs identiques) ; `suppress_next` au boundary réel ; `early_next_loop` quand le fire croise le wrap → conditions évaluées avec `loop_count + 1` dans `lib.rs`.
  - `Sequencer::set_microtimings()` copie les atomics 1×/buffer ; `clear_microtiming_state()` sur play/stop/reset/seek/sync.
  - UI : row **Nudge** (−100..+100 ms) dans le menu Seq Plock, entre Stutter et Condition.
  - Export MIDI : notes décalées du nudge (clamp tick 0).
  - Tests : inverse step_start_beat, ±25 ms sample-accurate, wrap + flag, zéro nudge, export MIDI.
  - ⚠️ Limites assumées : conditions à la boucle avec push/pull ≠ 0 approximatives (préexistant : `loop_count` wrap sur la timeline non shiftée) ; collision late+transition même sample → report d'1 sample.

**Non validé en main** : le batch entier attend la checklist « À tester dans Studio One » (voir section 4 du CHANGELOG / fin de session).

## 3. Pending tasks (TODO.md)

**Resume point (`REPRENDRE ICI`) = [166].** L'utilisateur choisit la tâche — présenter la liste, ne pas auto-démarrer.

- **[166]** Mixer le **stutter avec les cellules fusionnées** (aujourd'hui exclusifs). **Bien étudier avant de coder** : sémantique temporelle, audio, export MIDI, UI/plocks.
- **[163]** **Catégories d'instruments** (BD, SD, HH, PERC, FX, OTHER) + changement de type via clic droit sur le nom de lane.
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

1. Read `CLAUDE.md`. 2. Tout est **commité & poussé** (`skeuo-vector` = `main`) ; le batch [164]/[162]/[165]/[160]/[161] attend validation Studio One (checklist en fin de session précédente / CHANGELOG build `20260812-123906`). 3. Resume point = **[166]** mais attendre le choix explicite de l'utilisateur avant de coder. 4. Build+install et terminer chaque rapport de build par la checklist « À tester dans Studio One ».
