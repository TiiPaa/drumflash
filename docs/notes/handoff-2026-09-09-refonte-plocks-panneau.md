# Handoff — Refonte de l'édition des p-locks dans le panneau Sound (tâche [184])

**Écrit le:** 2026-09-09 · **Branch:** `main`
**Lire [`CLAUDE.md`](../../CLAUDE.md) d'abord** — c'est la référence canonique (architecture, invariants, workflow) pour tous les agents. Ce fichier est un état de session, pas un remplacement.

> **Périmètre.** Ce document couvre les tâches **[179] à [187]**, dont la refonte [184]. Il a été rédigé après coup et **ne couvre pas [195] à [206]** — les six instruments AC606, les quick wins du Lane Editor et le bandeau d'actions — développés entre-temps : pour ceux-là, le `CHANGELOG.md` est la source. La section 3 (ce que [184] a introduit dans l'architecture) reste la partie utile à lire avant de toucher au panneau Sound, aux p-locks ou au morphing.

---

## 1. État de l'arbre de travail au moment de la rédaction

- Tests **verts** : `cargo test` = **340 (lib) + 1 + 205 (test_standalone)** pour le périmètre [179]-[187], warning-clean.
- **Rappel deploy** : fermer Studio One avant `-Install` (verrou DLL) ; lancer `build.ps1` **plainly**, jamais avec `2>&1` / `2>$null` sur PS 5.1.

## 2. Ce qui a été fait dans cette session

| Task | Résumé | Build |
|---|---|---|
| **[179]** | **Attaque identique quel que soit l'écart entre deux cellules — Kick + BD808 uniquement.** Contrat de retrigger : chaque coup repart d'un état neuf (phase, filtres, DC blocker) + `dsp::RetrigDeclick` (fondu demi-cosinus 3 ms du dernier échantillon émis) pour que la remise à zéro ne soit pas un click. `trigger_hard()` délègue à `trigger()`. 6 tests dans `src/synthesis/retrig_tests.rs`, seuils auto-calibrés sur la même frappe jouée seule. | 20260820-162036 |
| **[181]** | Fine-tune des sliders réparé, switch Modulation SDrex explicite (Flanger / Filter LFO), Delay → **Fade-in**, plages d'enveloppes resserrées (BD/BD8 decay 2 s, clap 1,5 s, holds SDrex 1 s). | 20260820-172925 |
| **[182]** | Unités affichées sur les paramètres spéciaux et dans le menu plock (elles n'étaient portées que par une partie des définitions du registre). | 20260820-184818 |
| **[183]** | **Filter LFO SDrex unipolaire** : la modulation part de la base et monte, au lieu d'osciller autour — à cutoff 20 Hz et depth au maximum, on n'entendait plus rien. | 20260821-091344 |
| **[184]** | **REFONTE TERMINÉE — l'édition des p-locks sound et du morphing se fait dans l'onglet Sound.** 5 phases, chacune validée dans S1 (détail §3). `ui/plock.rs` **1257 → 616 lignes**, ~930 lignes supprimées au total. | 20260821-101410 → 20260823-091039 |
| **[185]** | **L'algo p-locké ne durait que quelques millisecondes** — il était appliqué puis écrasé par le réglage global au bloc suivant. | 20260821-111726 |
| **[186]** | **CRASH de l'hôte au rechargement du plugin** (session pleine → fermeture → session vierge → ajout : crash déterministe). Classe de fenêtre Win32 fantôme dans le pont clavier vendoré. Voir §5, c'est la leçon la plus coûteuse de la session. | 20260821-120829 |
| **[187]** | **Le paramètre que l'`Attack` masquait redevient verrouillable par pas** — le spécial d'indice 4 entrait en collision avec `ATTACK_FIELD = 18` et était silencieusement ignoré dans 5 endroits (le `saturation_output_gain` du Kick, entre autres, ne pouvait ni être verrouillé ni morphé). | 20260821-160115 |
| **[180]** | **FERMÉ SANS SUITE (2026-08-26), aucun changement de code.** Étendre le contrat [179] aux autres voix a été **mesuré puis écarté** par décision utilisateur. Détail complet dans `TODO.md` ; résumé §4. | — |

## 3. Ce que [184] a introduit dans l'architecture

À lire avant de toucher au panneau Sound, aux p-locks ou au morphing — c'est la partie la plus structurante de la session.

**`src/param_id.rs`** — l'identité canonique d'un paramètre de son, indépendante du magasin qui le stocke. `ParamId::{Std, Algo, Special, FreqMode}`. Elle a absorbé **six mappings `StandardField` dupliqués** qui devaient rester synchronisés à la main. Elle porte aussi les constantes de disposition du p-lock (`FIELD_COUNT = 46`, `ALGO_FIELD`, `SPECIAL_FIELD_START`, `ATTACK_FIELD = 18`, `SPECIAL_4_FIELD`, `LEGACY_CLAP_ECHO_FIELD`, `ADDRESSABLE_MASK`, `LEGACY_ALL_BITS`, `sanitize_field_mask`). `plock.rs` les **ré-exporte**, donc les chemins `crate::plock::X` existants continuent de marcher. La dépendance va identité → stockage, ce qui permet au binaire headless `test_standalone` d'inclure `param_id` sans tirer tout le plugin.

**`src/ui/param_source.rs`** — le trait `ParamSource`, avec les trois sources **composées** (et non dupliquées) :

```
GlobalSource  (atomiques SoundSettingsState + AlgoSink)
  └── PlockSource   (valeurs + masques de champs ; délègue à Global pour l'hérité)
        └── MorphSource  (cibles dans le FusedGroup ; délègue à Plock)
```

`Support::{Editable, Disabled(&str)}` porte la **raison** du grisage, ce qui rend l'UI explicable (règle des zones stables : griser avec infobulle, jamais masquer). `AlgoSink` est la couture qui rend l'algo — seul paramètre écrit via `ParamSetter` nih-plug et non par atomique — testable headless. **18 tests** dans ce fichier.

**Le point subtil du morphing** : une seule extrémité est stockée dans le `FusedGroup`, l'autre est résolue au déclenchement, et `MorphDirection::{Target, Source}` dit de quel côté. Le routage est donc une table **2×2 onglet × direction** (`MorphSource::writes_group`), ce qui fait fonctionner les cibles héritées **sans migration de données**. Ne pas « simplifier » en supposant que `End` écrit toujours le groupe.

**`src/ui/editor_state.rs`** — `SelectedCell`, `EditScope {LaneGlobal, StepPlock, Morph}`, `resolve_edit_scope()` (pure, 9 tests), `FusionTab {Step (défaut), Start, End}`. `Step` est le défaut délibérément : créer une rampe doit être un acte volontaire, pas ce qui arrive au premier slider touché.

**Plafond de 4 cibles de morph par fusion** : c'est la capacité du format empaqueté (3 × u64 = 192 bits, 186 utilisés). Au-delà, les rangées se grisent avec une raison et un bandeau flottant apparaît. **Passer à 5 cibles demande un 4ᵉ u64 par groupe, donc un `pattern-v6`** — hors périmètre, ne pas s'y lancer sans décision explicite.

**Ce qui reste dans le menu contextuel** : p-locks séquenceur (probabilité, stutter, nudge, condition, solo), menu de groupe de fusion, et les actions structurelles du p-lock sound (Link to Global / Snapshot / Copy / Paste / Clear + indicateur de mode) + une rangée « Edit In Panel ». **Les rangées de valeurs n'existent plus qu'une fois**, dans le panneau.

## 4. [180] — pourquoi c'est fermé (à ne pas redécouvrir)

Sonde temporaire sur les 16 voix : deux frappes espacées de 500/250/125/83/62/31 ms, comparées à la même frappe jouée seule.

- **Aucun click résiduel nulle part** : raideur au retrigger ≤ 1,1x celle d'une attaque à froid, sur toutes les voix. Les kicks étaient un cas particulier (corps asymétrique + DC blocker + reset de phase). `RetrigDeclick` n'a rien à réparer ailleurs.
- **Artefact résiduel à `analog = 0`** (écart d'énergie du transitoire, RMS 5 ms, répétition rapprochée vs frappe à froid) : Cymbal 2,41 dB, HiHat 1,61, Ride 1,27, Snare606 1,17, OpenHiHat 1,08 ; puis Tom 0,80, Buzz 0,79, BD808 0,78, Sdrex 0,58, Snare 0,54, Clap 0,17, Perc1 0,11, Kick 0,02, samplers 606 ≈ 0.
- **Au défaut livré (`analog = 0.5`), la dérive volontaire domine** (1,8 à 4 dB) : corriger l'artefact reviendrait à supprimer un défaut plus petit que le « breathing » voulu.
- Les 5 voix concernées sont exactement celles dont `trigger()` garde bruit et filtre **continus par choix documenté** ; un reset changerait le timbre des roulements de charley.
- **Deux pièges de méthode** rencontrés : (a) `t_peak` est inutilisable sur les voix à base de bruit — les attaques font 0,3 à 2 ms, donc le pic mesuré est un échantillon de bruit tiré au hasard ; utiliser le **RMS** du transitoire. (b) Toujours **couper la dérive analog** avant de mesurer un artefact : à `analog = 0.5` la référence isolée tombe par chance sur une frappe douce et gonfle les rapports (la Cymbal affichait 5,1x d'« excès » qui n'existe pas).

**Conséquence à connaître** : le contrat de retrigger n'est gardé par des tests que sur **Kick et BD808**. Une modification future d'un `trigger()` sur HiHat/Cymbal/Ride/Snare606/OpenHiHat ne sera **pas** détectée par la suite.

## 5. Gotchas — nouveaux de cette session

- **Inscriptions Win32 = portée processus, pas DLL** ([186]). Toute inscription qui stocke un pointeur vers notre code (classe de fenêtre, sous-classe `GWLP_WNDPROC`, `SetTimer`, hook) survit au `FreeLibrary`. Trois règles : nom **unique par chargement** (`GetModuleHandleExW` + `FROM_ADDRESS`), `HINSTANCE` de **notre** DLL jamais celui de l'hôte, et `uninstall()` depuis `Drop` avec comptage de références. **Ne jamais tolérer un `RegisterClassW` qui renvoie 0.** Piège de diagnostic : l'adresse fautive du minidump peut tomber dans une DLL **tierce** chargée à l'emplacement libéré par la nôtre — le module fautif n'est pas le coupable (je m'y suis fait prendre, il a fallu que l'utilisateur reproduise le crash pour que j'y revienne).
- **Continuations de ligne dans les littéraux Rust écrits par script** : un `\` de continuation passé par heredoc/python est avalé et laisse des **paquets d'espaces** au milieu du texte. Ça compile, les tests passent, seul l'écran le montre. Ça a touché **7 chaînes d'un coup** dont 5 infobulles. Écrire les littéraux **sur une seule ligne**, et pour les chemins Windows utiliser des **slashes**. Vérifier après coup : `grep -rn '"[^"]*[a-z] \{3,\}[a-zA-Z]' --include=*.rs src/`.
- **Une infobulle sur un widget de taille nulle est inatteignable** : `ui.label("")` ne se survole pas. Attacher la raison à la `InnerResponse.response` de `add_enabled_ui`.
- **Collision d'ids egui panneau/popup** : le même paramètre rendu dans les deux endroits produisait des ids identiques → `ParamSource::salt()`.
- **Emprunts et fermeture de menu** : dans `ui/plock.rs`, les branches Paste empruntent le presse-papier depuis `state` ; poser un drapeau `close_after_action` **dans** la fermeture et l'appliquer **après**.
- **`build.ps1` archive désormais le PDB** dans `build/symbols/drum_pattern_vst-<buildId>.pdb` (10 plus récents conservés) — indispensable pour lire un futur minidump.
- **Ne pas demander « ok » avant chaque build** : lancer `build.ps1 -Install` directement, ne signaler que si l'install échoue (« accès refusé » = S1 ouvert, ou verrou transitoire de l'antivirus → réessayer).

Rappels toujours valables : blobs positionnels, la **longueur EST la version** (`pattern-v5`, `sound-settings-v2`, `plock-v1`) ; zones stables (griser, jamais masquer, aucune ligne conditionnelle qui décale) ; rendu skeuo **uniquement** via `src/ui/skeuo.rs` ; libellés de boutons **ASCII only** ; `panic = "abort"` en release, donc tout panic tue l'hôte.

## 6. Tâches en attente

**Point de reprise (`REPRENDRE ICI`) = [166].** Sur « next » / « on continue » : **présenter la liste TODO et attendre le choix explicite**, ne pas commencer à coder.

- **[166]** Stutter × cellules fusionnées (exclusifs aujourd'hui) — **étude d'abord** : sémantique temporelle, rendu audio, export MIDI [158], UI/plocks.
- **[173]** Presets d'usine de départ (composer + embarquer via « Export factory (dev) » → `assets/presets/` + `factory_presets.rs`).
- **Suite optionnelle de [184]** : remonter les widgets et les listes d'options des paramètres **discrets** dans le registre — supprime ~87 lignes de devinette par sous-chaîne de libellé dans le panneau, et corrige `Sample` (606) et `Saturation Pre-Filter`. L'utilisateur a explicitement dit « tâche séparée ».
- **[BUG-LANE-DESYNC]** Décalage de tête de lecture entre lanes au changement Song/Pattern — **en attente de l'isolation du déclencheur par l'utilisateur**.
- Backlog : [144] Snare algo, [146] env exponentielles négatives, [152] instrument Ambiant, [94] [95] [69] [27] [84] [56] [41].
- **Notes utilisateur non ticketées** (`docs/notes/notes.txt`) : refaire la représentation graphique du link lane · virer open-hihat · hihat decay 1,5 s · router plusieurs tracks vers une même output · déconnecter le main quand on assigne une output · améliorer le visuel du sound editor (barre de scroll) · rendre le slider saturation amount plus progressif.
- Une variante de [180] reste documentée dans `TODO.md` si le sujet revient : exposer le comportement en paramètre (`Retrig: Sum / Choke`) plutôt que d'imposer un choix. **Pas de ticket ouvert**, l'utilisateur ne l'a pas demandé.

## 7. Carte des fichiers (fonctionnalités récentes)

- **Identité de paramètre** : `src/param_id.rs` (constantes de disposition p-lock ré-exportées par `src/plock.rs`).
- **Sources de valeurs** : `src/ui/param_source.rs` (`GlobalSource` / `PlockSource` / `MorphSource`, `AlgoSink`, `Support`).
- **Portée d'édition** : `src/ui/editor_state.rs` (`SelectedCell`, `EditScope`, `resolve_edit_scope`, `FusionTab`).
- **Panneau** : `src/ui/sound_editor.rs` (badge de portée + segmenté `Step | Start | End`, `row_scoped`, gouttière de 14 px, bandeau flottant de notice).
- **Menu contextuel réduit** : `src/ui/plock.rs` (616 lignes : création/actions/indicateur, menu de fusion, p-locks séquenceur).
- **Contrat de retrigger** : `src/synthesis/dsp.rs` (`RetrigDeclick`), `src/synthesis/{kick,kick_808}.rs`, tests dans `src/synthesis/retrig_tests.rs`.
- **Pont clavier Win32** : `drum-pattern-vst/vendor/nih-plug/nih_plug_egui/src/editor.rs` (`our_hinstance`, `class_name`, `INSTALL_COUNT`, `install`/`uninstall`).
- **Plan de la refonte** (pour le raisonnement complet) : `~/.claude/plans/j-aimerais-ajouter-une-autre-mellow-wilkes.md`.

## 8. Si vous reprenez ici

1. Lire `CLAUDE.md`. 2. Pour l'état courant du dépôt, se fier à `git log` et `git status` plutôt qu'à ce document : il décrit le périmètre [179]-[187] et a été rédigé après coup. 3. Le `CHANGELOG.md` liste chaque build installé et son état de validation. 4. Sur « next » / « on continue » : présenter la liste de `TODO.md` et attendre le choix explicite de l'utilisateur, ne pas coder. 5. Tout build installé se termine par la checklist numérotée « À tester dans Studio One (build \<ID\>) ».
