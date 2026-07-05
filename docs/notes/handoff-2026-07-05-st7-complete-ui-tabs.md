# Handoff — 2026-07-05 — Passation à un nouvel agent

## TL;DR — état du projet

**Flash Drum** (VST3 Rust nih-plug + egui, voir `CLAUDE.md` et `AGENTS.md`) est en **V1.5 —
grille modulaire 14 slots**, et ce chantier vient de franchir un jalon majeur : **chaque lane est
désormais une instance d'instrument totalement indépendante** (deux Kicks ne partagent plus rien).

- **Build installé et VALIDÉ par l'utilisateur dans Studio One : `20260705-122315`.**
- Dernier commit : `1de4529` (main, **5 commits d'avance sur origin/main — push non fait**).
- `cargo test` : 106 (lib) + 72 (standalone) — tout vert. 37 warnings préexistants de
  scaffolding UI (documentés dans `docs/design/UI-REDESIGN-HANDOFF.md` §4), aucun nouveau.

## Ce qui a été livré sur les 3 derniers jours (sessions 2026-07-03 → 07-05)

1. **Stabilisation 14 slots** (`50ea359`) : crashs lane 14 (tableaux taillés à 13 indexés par
   slot), menus plock par slot, settings/plocks appliqués par slot au trigger.
2. **Défaut 4 lanes + grille fixe** (`17278b8`) : nouvelle instance = BD/SD/HH/Tom ; la grille
   rend TOUJOURS 14 rangées (lanes actives + vides cliquables) — **règle UI absolue : aucune
   ligne conditionnelle qui décale les zones** (voir AGENTS.md et la mémoire `ui-stable-zones`).
3. **ST-7 — instances par slot** (`1de4529`, validé) :
   - `special[32]` + mode Hz/Notes **par slot** dans `SoundSettingsState`
     (`src/sound_settings.rs`), seedés depuis les défauts du registre.
   - Persistance `sound-settings-v2` **format v3 = 46 floats/slot** (la longueur du blob fait
     office de version). Migration automatique des anciennes sessions depuis les params
     nih-plug par voix (`needs_param_seed`, seed one-shot RT-safe dans `process()`).
   - `voice_settings_for(slot_idx, voice_idx, …)` : specials + algo par slot.
   - Ranges algo unifiés (« Slot N Algo », `instrument_registry::max_algo_index()`).
   - ⚠️ Les special params ne sont **plus automatisables par le DAW** (restent plockables).
     Les params legacy (`kick_click`, `freq_mode_kick`, …) et `special_param()` ne servent
     QU'À la migration — ne jamais les lire ailleurs, ne pas en ajouter.
4. **Retours UI utilisateur** (même commit, validés) :
   - Deux onglets fixes **Sound Editor | Track** (plus de boutons par instrument ; la lane
     éditée se sélectionne en cliquant dans la grille, l'en-tête affiche « Slot N - nom »).
   - Onglet Track : instrument, note MIDI, routing Main/Out, Humanize, Push/Pull, Length.
   - Pastille `+N` d'une lane vide → **menu de choix parmi les 11 instruments**.

## Invariant central à retenir (résumé de AGENTS.md « Per-slot instances »)

> Tout ce qui est par lane (settings, specials, plocks, seq-plocks, algo, locks de longueur,
> mute/solo/mix) est indexé par **slot** (0..MAX_TRACKS=14). Seuls les lookups de
> schéma/registre utilisent l'index de **voix** dérivé du kind du slot
> (`schema_voice_idx()` dans ui.rs, `kind.drum_voice_index()` ailleurs).
> Toute la famille de bugs des 3 derniers jours venait de confusions slot/voix.

## Prochaines étapes (voir TODO.md, marqueur REPRENDRE ICI)

1. **[MG-10] Générateur par types de pistes** — le plus urgent depuis le défaut 4 lanes :
   les générateurs supposent encore les rôles legacy par rangée (rangée 3 = OpenHH alors que
   la lane 4 du template est un Tom) → GENERATE écrit des patterns incohérents.
2. **[92]** Valeurs par défaut du menu plock (P1) — probablement en partie résolu par ST-7
   (les défauts viennent maintenant des atomics du slot) : **re-vérifier avant de coder**.
3. **[MG-9]** Comportement note/canal MIDI par spec — à revalider.
4. Reste UI : [100o] Song arranger, [100q] animations, [100aa] nettoyage final
   (`StyledButton`, `allocate_ui_at_rect` déprécié — c'est de là que viennent les 37 warnings).
5. ST-7 phase 2 (optionnel, plus tard) : suppression définitive des params specials legacy
   (rupture de compat state à assumer explicitement).
6. Pas encore fait : rename/suppression/réordonnancement de slot (drag), duplicate de slot.

## Pièges connus pour le prochain agent

- **`build.ps1 -Install` doit tourner SANS pipe ni `2>&1`** (PowerShell 5.1 transforme le
  stderr de cargo en fausse erreur) et **Studio One fermé** (DLL lockée).
- **PowerShell 5.1 + git commit** : pas de guillemets doubles dans les messages de commit
  passés en here-string (bug d'échappement des arguments natifs) — utiliser « » ou rien.
- **Après CHAQUE build installé** : terminer le compte-rendu par la checklist numérotée
  « À tester dans Studio One » — règle OBLIGATOIRE, format dans `AGENTS.md` → Deployment Rule.
- **Workflow « next »/« on continue »** : ne pas coder ; présenter les tâches du TODO et
  laisser l'utilisateur choisir (règle dans CLAUDE.md/AGENTS.md).
- **L'utilisateur ne reconnaît pas toujours les boîtes de permission Claude Code** (il les
  refuse en croyant que rien ne tourne) : annoncer qu'une boîte de validation va apparaître
  avant chaque commande longue (mémoire `permission-prompts-confusion`).
- `test_standalone.rs` inclut `track.rs` via `#[path]` : un test ajouté/supprimé dans
  track.rs compte dans les DEUX cibles de test.
- `ADDING_AN_INSTRUMENT.md` : les étapes « déclarer des FloatParam pour les specials » sont
  marquées obsolètes (bannière ST-7) — un nouvel instrument n'a plus besoin d'aucun param
  nih-plug pour ses specials.

## Validation à refaire si reprise

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
cargo test          # attendu : 106 + 72, 0 échec
cargo check         # 0 erreur, 37 warnings scaffolding
.\build.ps1 -Install   # plainement, Studio One fermé
```

Test S1 de référence (déjà validé sur 20260705-122315) : 2 BD sur 2 slots → Click Type,
Saturation, Hz/Notes, algo et plocks indépendants ; pastille `+N` → menu d'instruments ;
onglets Sound Editor|Track ; ancienne song → réglages conservés (migration).

## Documents de référence

- `AGENTS.md` — architecture + invariants (source de vérité, inclut « Per-slot instances »).
- `CLAUDE.md` — résumé + règles de workflow (renvoie à AGENTS.md).
- `TODO.md` — tâches (marqueur REPRENDRE ICI), `CHANGELOG.md` — historique par build.
- Handoffs précédents : `handoff-2026-07-04-st7-per-slot-specials.md` (détails techniques
  ST-7), `handoff-2026-07-02-mg7a2-14-slots.md` (activation des 14 slots).
- Mémoire persistante agent : `ui-stable-zones`, `permission-prompts-confusion`,
  `win-kbd-fix`, `kick-click-parasite-fix`, `search-before-fixing`, `ui-redesign-2026`.
