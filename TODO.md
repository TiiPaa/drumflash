## Nouvelles tâches — session 2026-08-20

### Bugs
- [ ] [184] **Refondre l'édition des p-locks sound dans l'onglet Sound** — **phase 0 FAITE et validée S1** (build 20260821-101410 : `src/param_id.rs` + accesseurs `VoiceSettings::get/set` et `InstrumentSettingsState::get/set`, 3 des 6 mappings dupliqués supprimés, `PlockState::clear_field`, `StandardField::ALL`, 4 tests). Reste : phase 1 (panneau sur `ParamSource` en portée globale), phase 2 (mode p-lock + sélection), phase 3 (morphing Start/End), phase 4 (nettoyage des menus). — remplacer le menu contextuel par le panneau de droite, pour n'avoir plus qu'UNE implémentation de rangées de paramètres au lieu de trois (panneau global `sound_editor.rs` ~745 l. / menu p-lock `ui/plock.rs:21-448` / menu morph `ui/plock.rs:497-820`). Clé : nouveau `src/ui/param_target.rs` avec `ParamId` (identité canonique, absorbe les 5 mappings `StandardField` dupliqués) + `ParamCtx` (portée global / p-lock du pas / extrémité de morph, écritures groupées). Sélection par clic droit, switch **Début/Fin** pour le morphing, ↺ par rangée, les 32 spéciaux verrouillables. Livraison en 4 phases testables. **Plan complet** : `~/.claude/plans/j-aimerais-ajouter-une-autre-mellow-wilkes.md`.
- [x] [183] **Filter LFO SDrex inaudible à base basse** — Filter à 20 Hz + Depth à fond ne donnait plus de son : la modulation était bipolaire et multiplicative *autour* de la base, donc la moitié de chaque cycle tombait sous le clamp 20 Hz et le sommet (160 Hz) restait sous le corps de la voix (mesuré −15,8 dB sous le filtre ouvert). Fix : le LFO ouvre le filtre **vers le haut depuis la base** (unipolaire, « Filter » = plancher du balayage) + échelle élargie à 6 octaves (`FILTER_MOD_OCTAVE_SCALE = 2.0`) → −4,8 dB à base 20 Hz. Mode Flanger inchangé. Test de non-régression sur les 3 propriétés (build 20260821-091344, à valider dans S1).
- [x] [182] **Unités manquantes sur certains paramètres** — `SpecialParamDef` n'avait aucun champ d'unité (le Sound Panel passait `None` en dur), et le menu plock n'affichait jamais d'unité même pour les paramètres standard qui en ont une. Ajout de `unit: Option<&'static str>` + helper `sp_unit()` ; 12 spéciaux renseignés (Hz : Gate Rate, Rate SDrex, Click Tone 808, Shimmer Freq ; s : Filter Attack/Hold Buzz et SDrex ; ms : Fade-in SDrex ; ct : Pitch Fine des 3 samplers 606) ; les deux formateurs du menu plock prennent l'unité. Test snapshot + règle par mot-clé pour les futurs paramètres (build 20260820-184818, à valider dans S1).
- [x] [181] **Quick wins UI/plages** — (a) **fine-tune des sliders réparé** : `draw_track` ne faisait que du positionnement absolu depuis l'unification des sliders ; Shift/Alt+glisser fait un déplacement relatif ~4× plus fin, avec détection clavier plateforme (les hôtes qui interceptent le clavier masquaient le modificateur à egui) ; maths isolée dans `apply_fine_drag` + 4 tests. (b) **switch Modulation SDrex** → choix explicite « Flanger / Filter LFO » (helper `segmented_row`, libellé « Modulation »). (c) **Delay de modulation SDrex → Fade-in** (0–300 ms, défaut 0) : le Wet monte progressivement après le coup, actif dans les deux modes donc plus grisé en Filter LFO ; délai min du flanger figé à `FLANGER_MIN_DELAY_MS = 0.7`. (d) **plages** : Kick + BD808 decay 5 s → 2 s, Clap 5 s → 1,5 s (nouveau `CLAP_STD`, le Cymbal garde 5 s), SDrex Hold + Filter Hold 2 s → 1 s (build 20260820-172925, à valider dans S1).
- [x] [179] **Attaques des BD de synthèse qui changent selon l'écart entre 2 cellules** — la queue du coup précédent contaminait l'attaque du suivant : phase des oscillateurs jamais remise à zéro, deux chemins de code (cold start vs retrigger), enveloppes repartant de leur valeur courante, smoothers non réinitialisés. Mesuré en mode digital (aucun drift), réglages identiques : Kick 3,71 dB de dispersion de pic, polarité de la 1re demi-période inversée, temps de crête de 1,5 à 8,3 ms ; BD808 temps de crête 12,7 ms isolé vs 1,5 ms rapproché, et en analog (défaut) 2,6 dB de dispersion même sur des coups isolés + discontinuité 0,244 (vrai clic du drift de niveau appliqué sur la queue). Fix : **`dsp::RetrigDeclick`** — chaque coup repart d'un état neuf identique (phase 0, filtre/smoothers/DC, enveloppes depuis zéro) et un fondu raised-cosine de 3 ms du dernier échantillon garde la sortie continue. Résultat : dispersion **0,00 dB**, temps de crête constant, step max 0,014 (Kick) / 0,044 (BD808) — 4× plus propre que le retrigger à phase continue (0,058). Contrat verrouillé par `src/synthesis/retrig_tests.rs` (6 tests). Appliqué au Kick + BD808 uniquement ; les autres voix tonales restent à faire (build 20260820-162036, validé S1 2026-08-20).
- [x] [176] **Solo/mute invisibles après les clears** — un solo enclenché survivait à Clear All et continuait de muter le kit depuis une lane désactivée (invisible). Fix : helper `controls::clear_all_mutes_solos` appelé par Clear All header, presets de layout (Clear All/4/12 Lanes + kits), presets de style Generate, et Delete Lane (build 20260820-082201, validé S1 2026-08-20).
- [x] [177] **Perc1 sature / artefact type FM** — FAUX POSITIF : c'était le fix Algo du build 20260819-145510 ([175]) qui faisait enfin jouer l'algo **Saw** stocké en session (ignoré → Sine forcé avant). Pas de régression DSP. Validé par l'utilisateur sur le build 20260820-083705 (« c'est bon dans le dernier »). Migration automatique envisagée puis rejetée (elle aurait écrasé les choix Saw volontaires à chaque chargement).
- [x] [178] **Uniformiser les graphes d'enveloppe** — socle commun `prep_graph` (LCD + padding + grille) + couleurs de stages partout (A ambre / H vert / D bleu), y compris filtre A-H-D Buzz/SDrex et ampli samplers ; courbes mono-stage en bleu ; token `envelope_curve` supprimé (build 20260820-083705, validé S1 2026-08-20).

## Nouvelles tâches — session 2026-08-14

### Features
- [x] [175] **Nouvel instrument SDrex** (snare métallique, recette utilisateur) — corps sine (drop +95 Hz) + noise HP coloré (niveau/type) + metal ring-mod 620×910, mix 0.5/0.8/0.18, **section Modulation autonome** avec switch Flanger/Filter Mod (Rate/Depth/Wet partagés, Delay/Fdbk réservés au flanger, Free Phase du LFO), drive tanh fixe, **enveloppe volume A-H-D** (Attack/Hold/Decay jusqu’à 1,5 s + Attack/Decay Curve), filtre LP + env A-H-D bipolaire (Filter Hold, Decay jusqu’à 1,5 s), Analog = drift standard. Kind 15 / voice 17, catégorie SD, note MIDI 48, rôle Snare au générateur, mono. Correctifs stabilité : buffer flanger fixe sans allocation audio, snapshot Pattern Bank sans pointeur brut, sérialisation sortie du callback audio, conflit p-lock Attack/special[4] neutralisé. Tests complets 297+1+189 (build 20260819-202022, validé S1 2026-08-20).

> **Ordre de priorité (2026-08-14)** — comportement/bugs d'abord, quick wins, features, gros chantiers.

### P1 — Comportement / quick wins
- [x] [174] **Graphe/DSP env filtre défectueux** — F1 Toms : span du graphe normalisé puis fenêtre fixe 1 s ; F2 smp + Perc1 : `FILTER_ENV_CURVE = 6.0` dédié par voix (régression [159]) ; F3 graphe ampli smp en A-H-D bipolaire. **Bonus bugs** : plocks sound perdus au chargement de pattern (`set_raw` sur field masks, bank slots + presets) ; filtre Tom : double avance pitch_env, pitch_env recréé au drag, stick contournant le filtre, plancher cutoff 100 Hz, 1 pôle → biquad 12 dB/oct, sweep exponentiel vers 20 kHz (loi Buzz), graphe = vrai sweep cutoff (build 20260819-114620, à valider dans S1).
- [x] [171] **Retirer le retrig en mode MIDI Pat** — le `pending_song_pattern_restart` n'est plus levé sur un switch MIDI : le nouveau pattern reprend à la position courante (à la volée) ; si la longueur change, le resync host existant (`sync_to_host` modulo la nouvelle longueur) ramène la lecture dans le pattern (page 1 si la page courante n'existe plus) (build 20260816-151800, à valider dans S1).
- [x] [172] **Temps forts 1/5/9/13 en plus clair dans la grille** — voile clair (`white_a(6)`, itéré 26→12→6) sur les cellules OFF des temps forts (build 20260816-173323, validé dans S1).
- [x] [169] **Clap pas assez fort** — volume par défaut 0.7 → 1.0 (registry `sound_settings_default` + `VoiceSettings::clap`) (build 20260816-151800, à valider dans S1).

### P2 — Features moyennes
- [x] [167] **Densité réglable pour Randomize Lane** — slider « Density » (5-100 %, défaut 30 %) dans le menu clic droit lane, au-dessus de « Randomize Lane » ; persisté (`randomize_density`, fallback 0.3 si 0/legacy) (build 20260816-183613, à valider dans S1).
- [x] [170] **Pousser les curves plus loin (tous les instruments)** — exposant bipolaire `1+3|c|` → `1+5|c|` partout : `dsp::shape_curve` (amp A-H-D), `buzz::shape_curve` (filtre), `buzz_shape_curve` (graphes envelope_viz) (build 20260816-183613, à valider dans S1).
- [x] [168] **Mode stéréo 2 samples pour les voix multisamplées** — checkbox **Stereo** (BD6/SD6/CH6smp) placée **sous le sélecteur Sample** avec infobulle EN ; le selecteur affiche des **paires** (1+2, 3+4, 5+6, 7+8) quand Stereo est ON : L = 1er de la paire, R = 2e ; **compatible Analog Mode** (paire aléatoire à chaque coup). DSP : envs partagées, filtre + DC blocker par canal ; dual mono sinon. Tests ×3 voix + analog+stereo (build 20260816-185337, à valider dans S1).

### P2/P3 — Gros chantiers
- [ ] [166] **REPRENDRE ICI** — **Mixer le stutter avec les cellules fusionnées** — étudier attentivement l'interaction stutter × fusion (actuellement exclusifs) : sémantique temporelle, rendu audio, export MIDI [158], UI/plocks. **Bien étudier le point avant de coder.**

### Idées notées (2026-08-16)
- [ ] [180] **Étendre le contrat de retrigger [179] aux autres voix** — Tom, Perc1, Snare, Snare606, SDrex, Buzz (mêmes causes : phase continue + enveloppes repartant de leur valeur courante), puis les 3 samplers 606 (ampli seulement). Vigilance sur HiHat/OpenHiHat/Clap/Cymbal/Ride : filtres résonnants, un reset peut changer le timbre → valider une voix à la fois avec `retrig_tests.rs`.
- [ ] [173] **Presets d'usine de départ** — composer et embarquer les premiers presets factory (instruments/patterns/grids/songs) via l'outil « Export factory (dev) » + `assets/presets/` + `factory_presets.rs`.

---

## Nouvelles tâches — session 2026-08-12 (notées à la passation)

> **Ordre de priorité revu (2026-08-12)** — quick wins d'abord, features ensuite, gros chantiers en fin.

### P1 — Quick wins UI
- [x] [164] **Caractère bizarre sur le bouton de reset du morphing** — glyphe « × » corrompu (« Ã— », UTF-8 relu en Windows-1252) remplacé par « X » ASCII (convention [73]) dans `plock.rs` (build 20260812-111741, à valider dans S1).
- [x] [162] **Griser les enveloppes quand One-Shot est actif (samplers smp)** — sur BD6smp/SD6smp/CH6smp, en mode One-Shot l'ampli est bypassée → désactiver visuellement (griser) les contrôles + graphe d'enveloppe concernés. ~~Lié à [155]~~ (annulé).
- [x] [165] **Pouvoir déplacer une lane vide** — grip des lanes vides câblé comme les lanes actives (`lane_drag_source` + curseurs Grab/Grabbing) ; `apply_lane_reorder_move` est déjà slot-générique, rien d'autre à toucher (pas de sélection de track pour un slot inactif).

### P2 — Features moyennes
- [x] [160] **Graphe pour le Gate Shape (Buzz)** — `draw_buzz_gate_graph` sous l'enveloppe d'ampli (famille Env) : fenêtre fixe 60 ms (Rate visible), Smooth cosinus / Razor spike fidèles au DSP, plancher de Depth en pointillé, tag « GATE ».
- [x] [161] **Remettre le microtiming par cellule dans le plock séquenceur** — nudge **±100 ms** par cellule, appliqué DANS le séquenceur : `groove::step_start_beat` (inverse de `beat_to_step`), late-fire (nudge > 0, diffère tout le trigger) + early-fire avec peek de la cellule suivante (nudge < 0, `suppress_next` à la frontière, flag `early_next_loop` pour les conditions à la boucle) ; données copiées des atomics 1×/buffer (`set_microtimings`) ; UI row « Nudge » dans le menu seq-plock ; export MIDI décale les notes (clamp tick 0). Tests : +25/−75 ms sample-accurate, wrap, zéro, inverse groove, export MIDI. ⚠️ Limites : conditions First/NotFirst à la boucle avec push/pull ≠ 0 = approximatif ; collision même sample = report d'1 sample.

### P2/P3 — Gros chantiers
- [x] [163] **Catégories d'instruments + changement de type via clic droit sur le nom de lane** — `InstrumentCategory` (BD, SD, HH, PERC, FX, OTHER) sur `TrackInstrumentKind` (`category()` + `kinds_in()` + `ALL`) ; sous-menu « Instrument » groupé par catégorie dans le menu clic-droit du nom de lane (même sémantique que le dropdown Type : nom + note MIDI + reset défauts, `change_slot_kind`) ; popup Add Module aussi groupé par catégorie. Tests partition + spot-checks (build 20260813-153921, à valider dans S1).

---

## Nouvelles tâches — session 2026-08-07

### Régressions / bugs (P1)
- [x] [153] **Étoile "non sauvegardé" fantôme** — le positionnement sur un slot vide (action UI-only) ne publiait pas le slot vers `audio_last_loaded_slot` ; la resync UI ramenait alors le slot sauvé + grille vidée → faux dirty. Fix : publier `audio_last_loaded_slot = i` sur slot vide (build 20260807-105334).
- [x] [154] **Hold manquant dans le graphe d'enveloppe** — tracé A-H-D explicite : palier plat au sommet pendant le Hold (couleur dédiée) + légende « H », Hold intégré à l'échelle temporelle (build 20260807-111016).
- [x] ~~[155]~~ **ANNULÉ (2026-08-12)** — Attaque des instruments smp quasi inaudible + graphe décalé — annulé par l'utilisateur.

### Quick wins UI (P1/P2)
- [x] [156] **Bouton "Save" à gauche des patterns** — Save déplacé avant la rangée de slots (build 20260807-123130).
- [x] [157] **Augmenter le max du Gate Rate (Buzz)** — 150 → 500 Hz (`GATE_RATE_MAX` + slider registry) (build 20260807-111951).

### Features moyennes (P2)
- [x] [158] **Export MIDI : inclure les notes des cellules fusionnées et des stutters** — fusion = `step_count` pulses uniformes sur le span ; stutter = N notes sur un pas ; réplique le séquenceur (les deux ne se combinent pas). `SequencerPlockState` passé à l'export. Tests fusion/stutter (build 20260807-140101).
- [x] [159] **Enveloppes d'ampli en A-H-D bipolaire (Release retiré) sur toutes les voix** — réécriture interne de `DecayReleaseEnvelope` (A-H-D piloté par le temps, signatures conservées) ; `decay_curve`/`release_curve` réinterprétés en courbes **decay**/**attack** bipolaires (−1..1) ; Release retiré des tables + du graphe ; Buzz ampli migré vers `DecayReleaseEnvelope` ; bugs `with_attack_ms` (`Copy` no-op) et recréation d'env corrigés (open_hihat/cymbal). Persistance : clamp gracieux des vieilles valeurs à +1, sans migration (build 20260807-170048).
- [x] [160]/[161] — déplacés dans la liste priorisée 2026-08-12 en haut du fichier (faits).

---

## Nouvelles tâches — session 2026-07-29

### Régressions / bugs (P1)
- [x] [135] **Le paramètre Stereo a disparu des instruments pouvant être stéréo** — régression : la checkbox avait été perdue des schémas FULL_STD (Snare, Perc1) et HIHAT_STD (HiHat). Restaurée + 2 tests de régression (stereo_capable/mono_voices).
- [x] [136] **Saturation Pre-Filter ne semble pas fonctionner** — le flag `pre_filter` était écrit mais JAMAIS lu par le DSP. Nouveau helper `SaturationConfig::process_at(pre_stage, x)` câblé dans les 11 voix saturées : pre = saturation avant le filtre de la voix, post = après (défaut). Bonus : la saturation du **HiHat n'était pas câblée du tout** (settings/UI mais aucun appel DSP) — corrigé ; toggle Pre-Filter ajouté au B8 (special index 8, param legacy `b8_sat_pre`).
- [x] [137] **S'assurer que le Volume est post-saturation** — `settings.volume` déplacé APRÈS `saturation.process` sur Kick, Snare, Tom1-3, Clap, Snare606, B8, Perc1 (était pré-sat → le volume changeait le drive). Le drift analog de niveau reste pré-sat (caractère du hit). Tests `test_kick/snare_volume_is_post_saturation` + `test_hihat_saturation_is_wired` + `process_at_routes_by_stage`.
- [x] [138] **Playhead sur cellule fusionnée** — quand la tête de lecture passe sur une fusion, entourer toute la cellule fusionnée (actuellement entoure les steps individuels) (build 20260729-143825 : ring sur le `block_rect` de la cellule de départ uniquement).
- [x] [139] **Warning pattern non sauvegardée** — plaque skeuo « The current pattern has unsaved changes » au clic sur un slot P1-P8 quand la grille est dirty (même slot = reload, slot vide = positionnement) ; boutons Discard & Load / Cancel. Dirty check factorisé dans `pattern_is_dirty` (build 20260729-174208).

### Quick wins UI (P1/P2)
- [x] [140] **Onglet Track : aligner le champ Name** avec la largeur des dropdowns Type et Aux Out (box keycap 146×26, TextEdit sans frame — build 20260729-100737).
- [x] [141] **Design des lanes vides plus discret** — cellules plates sans pointillés, chips sans bordure ni `--`, `+N` assombri (build 20260729-100737).
- [x] [142] **Différencier visuellement les slots Pattern Bank vides des remplis** — pastille blanche sur occupés, contour fantôme sur vides (build 20260729-100737).
- [x] [143] **Volumes par défaut : HiHat trop fort, BD pas assez fort** — Kick/808→1.0, HH→0.2, OH→0.3 (build 20260729-100737).

### Features moyennes (P2)
- [ ] [144] **Snare : améliorer l'algo** — pas assez de corps (enrichir la synthèse : body oscillator, tuning, noise blend).
- [x] [145] **Plock séquenceur : solo paramétré** — sémantique **finale = par-step / span de fusion** : le solo mute les autres lanes UNIQUEMENT pendant que la tête est sur la cellule soloée (1 step, ou toute la durée d'une fusion) ; hors fenêtre tout rejoue. Toggle **par cellule** ; désactiver clear le seq-plock si c'était le dernier param. Marqueur « S ». `SequencerPlockState::solo_window(fusion_span_len)` + gating `solo_window.bit(step) && !is_solo(slot,step)` ; persistance `solo_masks` appended rétro-compatible ; copy/paste + reorder de lane emportent le solo. **Validé en S1 (build 20260802-133013).**
  - ⚠️ **Historique d'itérations** : per-step (171811, jugé trop subtil) → solo-de-lane-tout-le-pattern (174923/180017, rejeté) → **retour au per-step/fusion-span** (final). Le bug « plock fantôme à un endroit aléatoire » n'était PAS le solo : le **clic-droit démarrait un step-drag** (`is_pointer_button_down_on` vrai aussi sur bouton droit) ; lire le menu >0,5 s l'activait, cliquer un contrôle du popup le relâchait → **déplacement silencieux du step**. Corrigé : le step-drag ne démarre que sur bouton **primaire** + `plock_popup.is_none()` (build 20260802-125810).
  - Le **crash à l'instanciation sur projet vide** (apparu pendant le dev de [145]/[149]) **a disparu** avec le retour au per-step + le fix du drag ; traçage diagnostic **retiré** (build 20260802-133013). ⚠️ Si le crash réapparaît : bisect [149] vs [145] (la base commitée `20260801-201613` ne crashait pas → race d'instanciation audio↔UI).
- [ ] [146] **Enveloppes exponentielles négatives** — pour des attaques plus claquantes (courbe d'attaque exp inversée, par voix ou global ?).
- [x] [147] **Choke groups** — 4 choke groups assignables par slot dans l'onglet Track (dropdown None/1-4), tous instruments. Quand un slot trigger, les autres slots actifs du même groupe sont silencés (`apply_choke_groups`, lock-free via le routing byte atomique bits 4-6). Remplace le choke global HH→OH : param `hihat_chokes_oh` masqué (conservé pour les vieilles sessions), toggle header retiré, migration automatique HH/OH→groupe 1 (sentinel serde 0xFF), presets 12 lanes + legacy 13 avec HH/OH en groupe 1 (build 20260729-174208).
- [x] [148] **Presets de style pour le Generator** — 16 styles au total. Les 10 initiaux (Rock, Funk, Techno, Hip-Hop, Jazz, Metal, Latin, Disco, Trap, Reggae) via [113] + 6 ajoutés : **Bossa Nova, House, Drum'n'Bass, Afrobeat, Dub, Breakbeat** (build 20260805-175817). Data-driven via `Style::variants()`. Extensible : d'autres genres (Samba, Cumbia, UK Garage, Soul/Motown, Punk, Downtempo…) sur simple ajout de `MusicalTemplate`.

### Grosses features (P2/P3)
- [x] [149] **16 patterns au lieu de 8** — `SLOT_COUNT 8→16`, migration `deserialize_with` tolérant (vieux blob 8-slots → P1-P8 préservés, P9-P16 vides, plus de perte silencieuse), UI 16 slots sur 1 rangée (26px), MIDI switch notes 60-75, 2 tests migration/round-trip (build 20260801-154337).
- [x] [150] **Gestion des presets dans un modal** — modal « Presets » (bouton dans la barre Pattern Bank) : 3 onglets Instruments/Patterns/Songs ; save du courant (slot sélectionné / pattern / song), listes Factory (embarqués, read-only) + User (Load/Del 2-clics) ; presets patterns embarquent le kit de lanes avec option « Load lanes too » au chargement ; load instrument change le kind de la lane si besoin. Fichiers JSON versionnés sous `Documents/Flash Drum/presets/` ; factory embeddés via `include_str!` (`factory_presets.rs`, vides pour l'instant) ; outil d'authoring : bouton « Export factory (dev) » (debug) → staging `_factory/` → copier dans `assets/presets/` + enregistrer. Modules `presets.rs` + `ui/preset_browser.rs` (build 20260814-112155, à valider dans S1).
- [x] [151] **Linker 2 lanes adjacentes** (layering) — une lane linke celle du dessus et partage ses **steps + fusions** (son/plocks/routing indépendants). `TrackSlot.linked_up` + `AtomicTrackLayout.slot_linked`, helper `grid_slot()` (chaîne, rompt si maître inactif), audio `set_grid_slots`/bloc, UI affichage+édition bidirectionnelle redirigés, menu clic-droit Link/Unlink, indicateur barre+point. **À valider en S1 (build 20260804-105255).** Limites v1 : morph des fusions reste au maître ; reorder ne réajuste pas les liens.
- [ ] [152] **Instrument Ambiant** — voix jouant des bouts de samples d'ambiances noisy avec offset aléatoire (dépend de l'infra sampler [83] ?).

---

## [SKEUO] Refonte visuelle « hardware » (pack designer RustDesign_Flash Drum, 2026-07-23)
> Pack de référence : `design-pack/RustDesign_Flash Drum/flash-drum-source/`.
> Docs autoritaires : `HANDOFF.md` (index), `SPEC-COMPUTED.md` ⭐ (cotes mesurées), `RADIUS.md`, `SKEUO.md` (recettes), `rust/skeuo_theme.rs` + `rust/skeuo_widgets.rs` ⭐ (code egui clé en main), `png/` (textures + `reference-full-ui.png` = cible).
> Stratégie : porter les 2 fichiers Rust du designer comme module `skeuo` (theme + widgets), garder notre layout, remplacer le *rendu* de chaque élément par ses fonctions (`pad`, `keycap`, `generate_button`, `hslider`, `led`, `lcd_frame`, `well`). Le module « ne fait que le look ».

### Fondations — FAIT
- [x] [SK-0a] Fenêtre 1480×800 + mise en page verticale resserrée pour tenir dans 800 (build 20260722-161751)
- [x] [SK-0b] Pads affichés via les textures PNG du designer (`assets/pads/pad-*.png`) — build 20260722-180702
- [x] [SK-0c] Rayons harmonisés au nouveau `RADIUS.md` : panneaux 7, keycaps 5, tags 3, nom de lane 4, pads 4, ADSR 4 (build 20260723-102824)

### Blocage — coins des pads
- [x] [SK-1] **Coins des pads carrés qui dépassaient de l'anneau de lecture.** egui (egui-baseview) ne sait PAS arrondir une texture. Solution retenue (sur remarque utilisateur) : **adapter nos overlays vectoriels au coin réel du PNG** au lieu de vouloir arrondir la texture. Le coin baké mesure ~13 texels sur 176 → **~2 px à l'écran** (`RADIUS_PAD_TEX = 2.0`). L'anneau de lecture et la surbrillance de survol passent de 4 px à 2 px pour épouser le pad (build 20260723-155528). À valider dans S1.

### Look Skeuo à porter — ⚠️ APPROCHE CHANGÉE : PUR VECTORIEL egui, module centralisé
> **Branche `skeuo-vector`** (repartie de `backup/skeuo-redesign`, build 20260726-184543). Fini les textures PNG : tout le rendu des éléments vit dans **`src/ui/skeuo.rs`** (une fonction par élément : `keycap`/`pad`/`slider_track`/`well_recess`/`lcd_bg`), appelée partout. Rendu pré-validé au labo `egui_kittest`+wgpu (crate `egui_lab`, sorties `ui-lab/`). Voir CHANGELOG 2026-07-26.
- [x] [SK-3] Puits de grille encastré → refait en vectoriel `skeuo::well_recess` (ombres haut + gauche + droite, corner-safe). Reste : plaques bottom-panel / pattern-bank / sound-editor.
- [x] [SK-4] Boutons / pages / slots / selects / segmented → **`skeuo::keycap`** (vectoriel, build 20260726-184543).
- [x] [SK-5] **GENERATE** → keycap ambre `skeuo::keycap(PressedAmber)` (build 20260726-184543).
- [x] [SK-6] Sliders → **`skeuo::slider_track`** (sillon creusé + fill pilule + capuchon strié sur les gros, pas les mini) (build 20260726-184543).
- [x] [SK-7] Switches (`ToggleSwitch`) → `skeuo::switch` (glissière encastrée + bouton rond métal glissant) (build 20260728-094309).
- [x] [SK-8] LED des toggles header (`ToggleLED`) → `skeuo::led` (verre radial + reflet, SANS halo) + pilule keycap grise (build 20260727-124102).
- [x] [SK-9] Écran ADSR → **`skeuo::lcd_bg`** (verre vert CRT + creux + scanlines) (build 20260726-184543).
- [x] [SK-10] Tags M/S/T (17×17 r3) + nom de lane (52×21 r4) → `skeuo::tag` / `skeuo::lane_name` (build 20260728-094309).
- [x] [SK-cleanup] Supprimer ~13 warnings (code textures/handles obsolète) ; centraliser `local_param_slider` vers `skeuo::slider_track` (build 20260729-090446). SK-2 et SK-14 annulés à la demande.

### Deltas comportement / layout restants (`CHANGES.md`, hors look)
- [x] [SK-11] Header : groupe « Seq Mode » (Internal/Ext MIDI + MIDI Pat collés), **Auto-Edit → ⚙ Settings**, segmented à moitiés symétriques (`segmented_equal`) (build 20260728-102242).
- [x] [SK-12] Page bar : LED rouge de lecture **dans le coin haut-droit** du bouton (au lieu de sous le bouton) + même LED dans les blocs du Song (`skeuo::play_led`, build 20260727-142106).
- [x] [SK-15] Generator sur **2 rangées** (R1 Type/A-Mix-B, R2 Dens-Var-GENERATE) (build 20260728-102242).
- [x] [SK-16] Lane Editor : dropdowns **ouverture vers le haut** près du bord bas ; mode Notes avec flèches **◂ ▸** peintes ; **A/D/R retirées** du graphe → légende en bas (build 20260728-111256).
- [x] [SK-17] Modals Lot 1 — popups « maison » (Plock son/Morph/Seq, Add Module, Page, Settings, Warning preset) en **plaque skeuo** (`skeuo::plate_shape` via slot réservé) ; header `×` → ✓ discret + trait d'accent ; rangées d'action → keycap (build 20260728-162504).
- [x] [SK-18] Modals Lot 2 — **menus contextuels egui natifs** (nom de lane, lane vide, longueur de lane, block Song) : frame relevé (Visuals) + rangées **keycap** (`menus::context_menu_button`), largeur fixée par menu (build 20260728-165711).

### Ordre proposé (1 build testable par étape)
1. SK-1 (débloquer les pads) → 2. SK-2/SK-3 (palette + fonds) → 3. SK-4/SK-5 (keycaps) → 4. SK-6..SK-10 (contrôles) → 5. SK-11..SK-16 (comportement).

---

## Modular Grid Redesign (active — V1.5)

- [x] [MG-1] Internal track model: `TrackSlot`, `TrackInstrumentKind`, `TrackRouting`, `TrackLayoutState`
- [x] [MG-2] Persist track layout in `track-layout-v1`
- [x] [MG-3] Migrate legacy 13-voice sessions to 14-slot layout
- [x] [MG-4] Adapt sequencer to iterate active tracks
- [x] [MG-5] Adapt audio engine to 14 independent synth instances + routing
- [x] [MG-6] Adapt pattern bank to store only musical data (no layout)
- [x] [MG-7] Refactor UI grid for modular lanes (active tracks, add/remove/change) — rollback 20260701: reverted after Studio One startup crash, puis complété via MG-7a.2 (`+ Add Module`) et MG-8
  - [x] [MG-7.1] Checkpoint sûr : ajouter `selected_track_slot` dans l'état UI, synchronisé avec les 13 lanes fixes, sans changement VST3/state audio (build 20260701-172602)
  - [x] [MG-7.2] Checkpoint sûr : sélectionner le slot via les interactions de grille/lane restantes (volume, Hum, Push, Len, fusion double-clic/shift-clic, plock clic-droit) sans changement VST3/state audio (build 20260701-173832)
  - [x] [MG-7.2a] Fix compat audio : tant que l'UI affiche 13 lanes fixes, le layout par défaut et le template 4 slots buggué sont migrés vers les 13 voix legacy pour éviter les lanes 5+ silencieuses (build 20260701-174700)
  - [x] [MG-7.3] Checkpoint sûr : introduire le bridge `slot_idx -> voice_idx` dans la boucle de grille, sans changement visuel ni VST3/state audio (build 20260701-175321)
  - [x] [MG-7.4] Checkpoint sûr : extraire le rendu d'une lane dans `draw_legacy_slot_lane_v2(slot_idx, voice_idx, ...)`, sans changement visuel ni VST3/state audio (build 20260701-183243)
  - [x] [MG-7.4a] Fix Len individuel : une lane lockée utilise sa propre longueur 1..64 même au-delà de la longueur globale, UI et playhead inclus (build 20260701-201011)
- [x] [MG-7a] Move `+ Add module` under lanes + styled empty lanes — rollback 20260701
  - [x] [MG-7a.1] Checkpoint visuel sûr : afficher le slot 14 vide et `+ Add Module` sous les lanes, sans activer l'ajout de piste ni changer audio/VST3/state (build 20260701-205643)
  - [x] [MG-7a.1a] Fix layout : passer la fenêtre fixe de 1480x800 à 1480x900 pour rendre visibles les options/panneaux bas après ajout du slot vide (build 20260701-230011)
  - [x] [MG-7a.2] Activer `+ Add Module` avec sélection d'instrument et mutation contrôlée du `track-layout-v1` (build 20260702-215053)
- [x] [MG-8] Sound editor tabs per track (Sound / Track) + instrument selector + per-slot routing — rollback 20260701 (build 20260702-215053)
- [x] [MG-9] MIDI note/channel behavior per spec — needs revalidation after rollback
- [x] [MG-10] **Adapt generator to track types and duplicate variations** — corrigé (build 20260707-161620)
  - Le générateur prend désormais le `track_layout` courant en entrée et mappe les rôles musicaux par `kind.drum_voice_index()` au lieu de l’index de rangée.
  - Jusqu’à 3 slots `Tom` utilisent les rôles Tom1/Tom2/Tom3 existants ; au-delà (ou pour toute autre duplication de kind) une variation déterministe est appliquée.
  - Les slots vides restent vides après `GENERATE`.
  - Tests déterministes ajoutés : mapping 4 lanes par défaut, rôles Tom distincts, variation des duplicates, slots vides silencieux.
- [x] [MG-11] Build, test, install, update CHANGELOG — done (build 20260702-215053)

## [P0] Stabilisation modular grid 14 slots (session 2026-07-03)

> Constats du test Studio One post-MG-7a.2 (build 20260702-215053) : crash à la 14e piste,
> son défectueux sur piste ajoutée, type non modifiable via TRK, layout 4 lanes au démarrage.
> Diagnostic code fait le 2026-07-03 — références de lignes valables sur le working tree non commité.

- [x] [ST-1] **Crash S1 en ajoutant la 14e piste** : `fusion_selection_start` taillé à 13 mais indexé par slot (0..14) dans la boucle de grille → index out of bounds dès que la lane 14 est dessinée. Corrigé : taillé à `MAX_TRACKS` (build 20260704-165252).
- [x] [ST-2] **Crash clic droit (menu plock) sur la lane 14** : `INSTRUMENTS[slot_idx]` dans les menus Plock/Morph/Seq Plock + `DrumVoice::from_index(slot).expect(...)` dans le dropdown Algo. Corrigé : schéma résolu via `schema_voice_idx(params, slot)` dérivé du kind, stockage plock inchangé (par slot) (build 20260704-165252).
- [x] [ST-3] **Son défectueux sur un slot ajouté** : `reset_slot_to_defaults()` n'était jamais appelé — un slot activé/rekindé gardait les settings d'init de la voix legacy de même index (ex. Tom pour le slot 5). Corrigé : reset aux défauts du kind à l'activation (`+ Add Module`) et au changement d'instrument (onglet TRK) (build 20260704-165252).
- [x] [ST-4] **Sound Panel confond index de slot et index de voix** : `selected_instrument` est désormais un index de slot (0..14), le schéma (registre, special params, filter label, checks Kick/B8, algos) est dérivé du kind du slot ; changer le type dans TRK ne fait plus sauter la sélection ; onglets du Sound Editor = slots actifs ; `effective_lane_length_for_ui` aligné sur l'indexation slot du moteur audio (build 20260704-165252).
- [x] [ST-4b] **Settings/plocks appliqués par voix au trigger** (trouvé par test S1 : "la freq de la lane 1 change celle de la lane 14") : `voice_settings_at_step()` indexait `sound_settings_state.instruments[]` et `plock_state.get_settings()` par voix au lieu du slot → chaque trigger d'un slot dupliqué réappliquait les settings et plocks du premier slot du même kind. Corrigé : signature `(slot_idx, voice_idx, step)`, settings + plocks par slot (build 20260704-173043).
- [x] [ST-4c] **Pastille `+14` de la lane vide non cliquable** (rapporté test S1 : "quand je clique sur +14 rien ne se passe") : la lane vide affichait `+N` avec highlight au survol mais `Sense::hover` seulement — seule la rangée `+ Add Module` était active. Corrigé : pastille cliquable (curseur main + tooltip), activation factorisée dans `activate_next_free_slot()` (build 20260704-174006).
- [x] [ST-7] **Special params par slot (instances vraiment indépendantes)** — **FAIT (build 20260705-122315), à valider dans S1.**
  - [x] ST-7a : `special[32]` + `freq_mode` par slot dans `InstrumentSettingsState` ; persistance `sound-settings-v2` format v3 (644 floats) + flag `needs_param_seed` pour migration
  - [x] ST-7b : moteur — `voice_settings_for(slot, voice, …)` lit specials + algo par slot ; seed one-shot des params legacy dans `process()`
  - [x] ST-7c : Sound Panel — widgets specials + Hz/Notes sur les atomics par slot (plus de ParamSetter)
  - [x] ST-7d : menus plock/morph — défauts specials, Snapshot, morph, toggle Display par slot
  - [x] ST-7e : ranges algo unifiés (`max_algo_index()`), renommés "Slot N Algo", fix ranges 0..0 crashogènes
  - [x] ST-7f : warnings nettoyés, 3 tests unitaires persistance v3/migration/reset, `cargo test` OK (106+72), `build.ps1 -Install` OK, CHANGELOG + AGENTS.md + CLAUDE.md + ADDING_AN_INSTRUMENT.md mis à jour (specials par slot, special_param() = migration only)
- [x] [ST-9] **Retours UI 2026-07-05** (build 20260705-122315, à valider dans S1) :
  - [x] Onglets fixes `Sound Editor` | `Track` — suppression des boutons par instrument (sélection de la lane via la grille, en-tête "Slot N - nom")
  - [x] Onglet Track complet : instrument, note MIDI, routing, Humanize, Push/Pull, Length
  - [x] Pastille `+N` → menu de choix parmi les 11 instruments à la création
  - [x] Fix lock de longueur de lane indexé par voix côté UI (aligné slot, comme l'audio)
- [x] [ST-5] **Layout 4 lanes au démarrage** : résolu par décision produit 2026-07-04 — le défaut EST maintenant 4 lanes (BD/SD/HH/Tom, `modular_default_layout()`), migration anti-template supprimée. ⚠️ Songs pré-`track-layout-v1` s'ouvrent en 4 lanes (build 20260704-195335).
- [x] [ST-8] **Règle UI zones stables** : la grille rend toujours 14 rangées (lanes actives + vides cliquables), rangée `+ Add Module` supprimée — plus aucune ligne conditionnelle qui décale les panneaux du bas (build 20260704-195335). Règle générale à respecter dans toute l'UI.
- [x] [ST-6] **Revalidation S1 après fixes** : instances BD indépendantes confirmées par l'utilisateur (2026-07-04) ; reste à re-vérifier après le passage au défaut 4 lanes : activation de chaque lane vide, 14 pistes, Out 14 audible.

## Plan d'action — Audit code review 2026-07-05

### [AUDIT-1] P0 — Eliminer les verrous Mutex bloquants sur le thread audio
- [x] Remplacer `PatternBank::lock()` par `try_lock()` dans `process()` (lib.rs:2750, 1954, 1981)
- [x] Si contention, reporter save/load/song au bloc suivant
- [x] Option : double-buffer atomique + file SPSC UI→audio pour le song mode — **Fait (build 20260716-111228)** : `SongStateController` (SPSC `ArrayQueue<SongSequence>`) dans `src/atomic_song.rs`, publié par l'UI et consommé par le thread audio, plus de `try_lock` sur `PatternBank` en lecture du song.

### [AUDIT-2] P0 — Supprimer les allocations sur le thread audio
- [x] `Box::new` dans `reinitialize_slot()` (synthesis/mod.rs:846) → réutilise la `Box` existante du slot au lieu d'allouer/désallouer sur changement de kind
- [x] `Vec::with_capacity`/`push` dans `save_pattern_to_slot` (pattern_bank.rs:223) et `restore_from_buffers` (pattern_bank.rs:585) → tableaux fixes `[FusedGroup; MAX_FUSIONS]`
- [x] Conditionner `nih_log!` dans `process()` à `#[cfg(debug_assertions)]` (lib.rs:2012, 2036, 2066, 2083, 2100, 2116, 2290) + `println!` de `fire_voice_trigger`

### [AUDIT-3] Important — Corriger l'export MIDI 14 slots + note par slot
- [x] Itérer `0..MAX_TRACKS` au lieu de `INSTRUMENTS` (midi_export.rs:81)
- [x] Lire `track_layout[slot].midi_note` plutôt que `def.midi_note`
- [x] Ajouter un test couvrant le 14e slot et une note personnalisée

### [AUDIT-4] Important — Adapter le générateur aux kinds réels des slots (MG-10)
- [x] Mapper rôles musicaux par `kind.drum_voice_index()` / `track_layout`, pas par index de rangée (déjà en place via `remap_roles_to_slots`)
- [x] Ajouter `seed: u64` à `GeneratorParams` et rendre `generate()` déterministe
- [x] Ajouter des tests déterministes seedés : même graine = même pattern, graine différente = pattern différent, Kick mappé par kind (slot 1 et 13), aucun Kick quand le layout n’en a pas

### [AUDIT-5] Important — Tests mute/solo/mix routing
- [x] Extraire la logique de gating (effective_mutes/mix_gains) en fonction pure (lib.rs:2271)
- [x] Ajouter des cas : 1 mute, 1 solo, mute+solo, plusieurs solos, aucun


### [AUDIT-6] Suggestion — Tests round-trip synth settings
- [x] Généraliser le test de round-trip à toutes les voix (macro `settings_roundtrip_test!` dans `synthesis/settings/mod.rs`, appliquée à chaque fichier settings)
- [x] Corriger les défauts `special[]` des saturations non-entiers (Kick, Snare, HiHat, OpenHiHat, Ride) pour que le round-trip soit stable

### [AUDIT-7] Infrastructure
- [x] Committer `Cargo.lock` et le retirer de `.gitignore`
- [x] Supprimer `fix_roles.pdb` et les `.zip` redondants du suivi git
- [x] Retirer `.claude/settings.local.json` du suivi
- [x] Corriger docs : `13 voix/aux` → `14 slots`, `pattern-v1` → `pattern-v5`

### [AUDIT-8] Dette UI / qualité
- [x] Nettoyer échafaudage UI mort (`design_system.rs`, `StyledButton`, `allocate_ui_at_rect`) → tâche [100aa]
- [x] Renommage ports auxiliaires génériques `Out 1..14` — build 20260706-173427
- [x] Documenter invariants `// SAFETY:` dans `native_drag.rs` + test `build_hdrop_medium`

## Feedback utilisateur — 2026-07-05 post build 20260705-150850

### Bugs / régressions P1
- [x] [117] **P0 — Gros bug de son distordu lors de l'activation/désactivation d'une output dans le DAW** — corrigé côté écriture aux défensive (build 20260706-141836) + routing Track par slot (build 20260706-172704) + sorties auxiliaires exclusives par lane (build 20260706-175157) + mapping sparse VST3 Studio One (build 20260706-185857) + init synth sur layout courant (build 20260706-190624, à valider dans Studio One)
  - Reproduire dans Studio One : activer/désactiver une sortie auxiliaire du plugin pendant que le séquenceur joue.
  - Vérifier routing main/aux, buffers non activés, état de bus VST3 côté vendor nih-plug et écriture dans `aux.outputs`.
  - Attendu : aucune distorsion, aucun burst, aucun signal corrompu lors du changement d'activation de sortie.
  - Régression associée corrigée : changer `Track > Out` ne doit plus changer le son entendu ; le slot sélectionné est réellement routé vers la sortie choisie.
  - Régression associée corrigée : assigner un Tom à `Out 2` ne doit plus laisser un HH caché sur le même bus ; un `Out N` est maintenant exclusif à une lane.
  - Cause profonde corrigée : Studio One peut fournir des buffers auxiliaires compactés pour des sorties sparse ; le wrapper VST3 remappe maintenant ces buffers vers le vrai `Out N`.
  - Cause profonde corrigée : à l'activation, le synthé ne doit plus recréer le slot Tom avec la voix legacy OpenHH.
  - UX routing corrigée : la liste `Out` affiche `No Aux` au lieu de `Main`, car le Main Mix est déjà contrôlé par le switch `Main` (build 20260706-192033).
- [x] [103] **Régression : le drag & drop MIDI a disparu** — corrigé (build 20260707-091113)
  - Le bouton `Drag MIDI` est à nouveau sensible au glisser (`Sense::click_and_drag`).
  - Le helper OLE démarre automatiquement si le bouton gauche est déjà enfoncé.
  - L’export temporaire MIDI utilise le `track-layout` courant (14 slots + note par slot).

### Retours Studio One — 2026-07-07 post build 20260707-094444
- [x] **Ext MIDI : tête de lecture interne masquée** — corrigé (build 20260707-103907)
  - En mode `Ext MIDI` la grille ne surligne plus de step ; le playhead interne est gelé.
- [x] **Ext MIDI : flash visuel du `T` par lane** — corrigé (build 20260707-103907)
  - Chaque lane dont la note MIDI est reçue fait clignoter sa pastille `T`.
  - Couleur ajustée en AMBER/texte noir dans la build 20260707-111442.
- [x] **Export MIDI : swing/groove appliqué** — corrigé (build 20260707-103907)
  - Les fichiers MIDI exportés (Export + Drag) respectent le Swing et le Groove sélectionnés.

- [x] [118] **Morphing : Saturation Amount / Mix reveniennent à la valeur de base + cohérence tous instruments** — corrigé (build 20260707-155108)
  - Popup Morph élargi (284 → 350 px) et sliders réduits (104 → 96 px) pour éviter que les longs labels ne poussent le slider hors du cadre sur tous les instruments.
  - Clamp systématique de la valeur morph affichée/stockée à min..max (Volume, standard params, specials continus).
  - `morphable_fields()` inclut désormais les champs standard de type checkbox (ex. `Stereo`) pour correspondre au menu Morph ; test de régression ajouté.
  - Correction similaire s'applique à tous les instruments : Kick, Snare, HiHat, OpenHiHat, Tom1/2/3, Clap, Ride, Cymbal, Snare606, 808 Kick, Perc1, Zap.

- [x] [104] **Ligne avec le bloc Fusion décalée / perte de place** — corrigé (build 20260707-125743)
  - Revoir le placement du panneau Fusion sous la grille : il ne doit pas décaler inutilement les zones ni consommer de hauteur excessive.
  - La Fusion box (380 px) est maintenant affichée sur la même ligne que le sélecteur P-Lock Mode (Sound/Sequencer), alignée à droite. Elle ne pousse plus la Pattern Bank et le Bottom Panel vers le bas.
  - Fix : allocation de taille exacte (`allocate_exact_size`) pour que la hauteur de la box soit identique en idle et en édition, évitant tout saut de l’interface.
- [x] [105] **Plock sound : Frequency à 0 par défaut sur certains instruments** — vérifié + tests de régression (build 20260707-113932)
  - Probablement lié à [92] ; vérifier tous les instruments, notamment B8/HH/Tom et les slots dupliqués.
  - Attendu : le menu plock doit initialiser `Frequency` avec la valeur globale courante du slot/instrument, jamais `0` sauf si c'est réellement la valeur globale.
  - Résultat : les défauts du registre et le reset aux défauts du kind (`ST-3`/`ST-7`) garantissent une fréquence > 0 ; tests `default_frequency_is_nonzero_for_every_instrument_kind` et `duplicate_slots_keep_nonzero_default_frequency` ajoutés.

### UX grille / lanes P1
- [x] [106] **Retirer Hum et Push de l'onglet Track** — corrigé (build 20260707-164821)
  - Décision utilisateur 2026-07-07 : conserver `Humanize` et `Push/Pull` dans la grille pour l'instant, et les retirer seulement de l’onglet `Track`.
  - L’onglet `Track` garde `Instrument`, `Routing`, `MIDI Note` et `Length`.
- [x] [107] **Cellules hors longueur en pointillé** — corrigé (build 20260707-174844)
  - Quand `Len` global ou individuel est inférieur au maximum affiché, rendre les cellules non jouées en pointillé.
  - Attendu : distinguer clairement les steps visibles mais inactifs à cause de la longueur.
  - Résultat : les cellules hors longueur et les lanes non activées utilisent le même design inactif : fond très sombre + bordure segmentée épaisse/sombre.
  - Fix layout : le bloc `Len` global conserve une largeur fixe quand la valeur passe sous 10 ; l’indicateur `N steps` est dessiné dans un rectangle fixe pour que les boutons `16/32/48/64` ne se décalent plus.
- [x] [108] **Réarranger les lanes avec la poignée** — corrigé (build 20260708-162542)
  - Activer le drag de lanes via la poignée prévue dans le design.
  - Préserver instrument, paramètres, séquence, plocks, longueur, mute/solo/routing lors du déplacement.
  - Résultat : drag depuis la poignée vers une autre rangée active ou vide ; détection immédiate au clic via `is_pointer_button_down_on` + geste drag classique ; curseur Grab/Grabbing.
  - Feedback visuel : trait bleu horizontal (2 px) affiché à la limite exacte de la lane cible (haut de la ligne cible, ou bas de la dernière ligne pour le drop final), calculé par rapport au centre des lanes.
  - Déplacement complet de `track-layout`, pattern, fusions, plocks sound/seq, sound settings, algo, mute/solo/mix, Hum/Push/Len, locks et sélection UI.

### Presets / gestion lanes P1
- [x] [109] **Boutons de presets de lanes** — corrigé (build 20260708-145331)
  - Ajouter `Clear All Lanes`.
  - Ajouter `Preset 12 Lanes`.
  - Ajouter `Preset 4 Lanes`.
  - Vérifier que les zones UI restent stables : aucune ligne conditionnelle qui décale les panneaux.
  - Résultat : dropdown global `Preset` centré entre `Follow` et `Len` dans la page-bar avec marge dédiée avant `Len`, options `Clear All`, `Preset 4`, `Preset 12`; warning de confirmation avant application ; les presets ne sont plus dans l’onglet `Track`; tests `empty_layout_has_no_active_lanes` et `preset_12_layout_uses_core_legacy_kit_without_perc1` ajoutés.
- [x] [111] **Revoir le preset du Tom** — ajusté (build 20260707-120216)
  - Ajuster les valeurs par défaut Tom pour un rendu plus musical/utilisable dès création de lane.
  - Changements : Tom1 (lane Tom par défaut) **196 Hz** / 0.35 s / vol 0.7 / filter 600 Hz / release 0.25 ; Tom2 150 Hz / 0.30 s ; Tom3 100 Hz / 0.45 s. Stick Attack ramené à 0.3. `VoiceSettings::tom1/2/3()` alignés sur les défauts du registre.

### Song / Generator P1
- [x] [112] **Revoir le Song Editor, actuellement peu pratique** — corrigé (build 20260708-164626)
  - **Répétitions par step :** champ `repeats` ajouté à `SongSequence` (compatibilité `pattern-bank-v1` via `serde(default)`), moteur audio `lib.rs` lit `repeat_at()` et reste sur le step courant le nombre de boucles demandé avant d’avancer.
  - **Hauteur du panneau Song/Generator augmentée** de 144 px à 180 px pour accueillir une rangée d’inspection.
  - **Rangée d’inspection de la step sélectionnée :** dropdown `P1-P8` / vide, compteur de répétitions `1..64`, boutons `Copy` / `Paste` / `Dup` / `Clear`.
  - **Grille 4×16 conservée :** clic gauche sélectionne la step, clic droit contextuel `Copy / Paste / Duplicate / Clear`.
  - **Raccourcis globaux :** bouton `Reset` pour remettre la position song à 0, `Clear All` pour vider la song, `Len` via `DragValue` (0-64), `Loop` conservé.
  - **Suppression du toggle `Song Enabled` redondant ;** le playhead song est maintenant suivi dès que l’onglet `Song` est actif (`params.song_mode`).
  - **Reset automatique de la position song** quand on quitte le mode Song ou quand le transport s’arrête.
  - Tests ajoutés : `song_sequence_repeat_clamps_and_defaults`, `pattern_bank_legacy_load_without_repeats_defaults_to_one`, `pattern_bank_persistence_roundtrips_song` mis à jour avec les répétitions.
  - [x] Corrections UI post-build : dropdown d’inspection inscriptif, répétition affichée `P1xN`, step courant bleu lisible, hauteur de grille fixée pour les 4 rangées (build 20260708-171322).
  - [x] Refonte UX : 16 blocks fixes, onglet Song = vue uniquement, checkbox `Song Mode` dans le panneau, suppression de `Loop`/`Len`, cellule en deux parties (pattern en haut, repeat en bas), retour au début sur block vide (build 20260708-182802).
  - [x] Ajustement : panneau Song/Generator agrandi de 30 px (210 px total), suppression de la rangée d'inspection, édition directe du pattern et du repeat dans chaque block (build 20260708-183824).
  - [x] Polish : marge interne aux blocks pour éviter les débordements, blocks vides assombris, retrait de `Reset`, confirmation sur `Clear All` (build 20260708-185335).

- [x] [113] **Revoir le Generator : HiHats trop similaires entre styles** — corrigé (build 20260707-163927)
  - Les rôles HiHat sont maintenant différenciés par style : Rock 8ths, Funk offbeats, Techno/Metal/Disco 16ths, Hip-Hop sparse/swung, Jazz skip-beats, Latin clave-like, Trap dense rolls, Reggae one-drop.
  - Test `hihat_roles_are_style_specific` ajouté pour éviter un retour au motif quasi identique sur tous les styles.
  - Attendu : varier densité, accents, ouvertures, syncopes et probabilités selon style.

### Sound Editor / synthèse P2
- [x] [114] **Clarifier et enrichir HiHat / OpenHiHat** (build 20260709-121611)
  - Renommer `Frequency` → `Tone` (range 100–20000 Hz) et `Filter` → `Cutoff`.
  - Ajouter `Resonance`, `Noise Type` (White/Pink/Brown/Blue), `Shimmer`.
  - Supprimer l’algorithme `Bright` inutilisé.
- [x] [115] **Mettre Analog au milieu pour tous les instruments** (build 20260709-141013)
  - Création d’une famille `ParamFamily::Analog` dédiée.
  - Déplacement du champ `Analog` depuis `Output` vers la nouvelle section `Analog` pour tous les instruments.
  - Ordre UI : `Osc` → `Env` → `Analog` → `Filter` → `Sat` → `Output`.
  - Pour les instruments tonaux (Kick, Snare, Tom, Kick808, Perc1, Snare606), le slider `Analog` pilote `AnalogDrift` (pitch/level/time par hit).
  - Pour les instruments non tonaux (HiHat, OpenHiHat, Clap, Ride, Cymbal), le slider `Analog` module désormais le **tone** :
    - HiHat / OpenHiHat : `Tone` (centre du peaking filter), dérive **±25 %**.
    - Ride : `Frequency` (base des oscillateurs inharmoniques), dérive **±7.5 %**.
    - Clap / Cymbal : `Cutoff` / `Filter` (highpass cutoff), dérive **±25 %**.
  - `Zap` n’existe pas dans le code actuel ; la 13e voix est `Perc1`.

### Copier / coller lanes P2
- [x] [116] **Copier/coller une lane vers une autre** — corrigé (build 20260709-182258)
  - Menu contextuel sur le nom d'une lane active : `Copy Lane`, `Paste Lane`, `Paste Grid`.
  - Menu contextuel sur le nom d'une lane active : `Clear Grid` avec confirmation en deux clics.
  - Menu contextuel sur une lane vide : `Paste Lane` si un clipboard existe.
  - `Paste Lane` copie instrument, réglages sonores (standard + specials + Hz/Notes), algo, steps, fusions, sound plocks, seq plocks, Humanize, Push/Pull, Len et lock Len.
  - `Paste Grid` copie uniquement les steps on/off de la grille sur une lane active cible ; instrument, réglages sonores, algo, fusions, plocks, Hum/Push/Len, lock Len, routing, mute/solo/mix et note MIDI restent inchangés sur la cible.
  - `Clear Grid` efface uniquement les steps/fusions/sound plocks/seq plocks de la lane active ; instrument, réglages sonores, algo, Hum/Push/Len, lock Len, routing, mute/solo/mix et note MIDI restent inchangés.
  - Routing, Main/Out, note MIDI source personnalisée, mute/solo/mix ne sont pas copiés ; un changement d'instrument remet la note MIDI du slot cible sur le défaut du kind.

## Feedback utilisateur — 2026-07-13

### Bugs / régressions
- [x] [119] **Enlever le hover "empty slot" sur la cellule vide** — retirer le tooltip inutile sur les cellules hors longueur de pattern.
- [x] [124] **L'édition des patterns semble bloquée en mode song** — auto-save des edits vers le slot de Pattern Bank courant quand `Song Mode` est actif (build 20260713-162128).
- [x] [129] **Bug dans le graph du filter decay du T1** — le graphique d'enveloppe de filtre utilise maintenant la courbe fixe du moteur par instrument (6.0 pour Tom, 8.0 pour Kick/Snare/HiHat/Snare606, `decay_curve` pour Perc1) au lieu de l'amplitude `decay_curve` (build 20260713-171854).
- [x] [132] **Crash plock Kick 808 en mode Song** — la création de plock est désormais interdite en mode Song ; garde-fous ajoutés sur `note_name`, `freq_to_note`, `PlockValues` (build 20260714-135251).
- [x] [130] **Améliorer les différences de volumes avec/sans saturation** — compensation automatique du gain appliquée au signal saturé, et `saturation_output_gain` par défaut à 1.0 (build 20260714-141959).
- [x] [133] **Analog à 50% par défaut pour tous les instruments** — `VoiceSettings::*` ET les tableaux `sound_settings_default` de `instrument_registry` passent `analog: 0.5` (build 20260714-192351).
- [x] [134] **Cacher le paramètre Algo en automation pour les instruments mono-algo** — `Slot 3/4/10/11/12/14 Algo` masqués dans le DAW (build 20260714-190609).

### UI / UX
- [x] [121] **Bouton droit sur le titre du lane → Clear lane ou aléatoire** — `Clear Lane` et `Randomize Lane` ajoutés dans le menu contextuel du nom de lane (build 20260714-193958).
- [x] [123] **Bouton Follow off dans le panel song** — retiré (build 20260714-114115).
- [x] [128] **Option supprimer un lane** — permettre de passer un slot en inactif (lié à [MG-7]) (build 20260714-194847).
- [x] [131] **Les cellules fusionnées ne changent pas de couleur quand elles ont un plock** — inclure la modulation (morph) comme indicateur visuel de plock (build 20260715-085935).

### Features
- [x] [120] **Changer les patterns en MIDI temps réel en mode Seq Internal / Pattern** — permettre au séquenceur interne de recevoir des notes MIDI pour changer de pattern P1-P8 en cours de lecture (hors mode Song) (build 20260715-093205).
- [x] [122] **Drag cell long press 2s** — quand on laisse le bouton gauche appuyé ~2s sur une cellule, permettre de la déplacer (avec ses plocks) à gauche/droite avec la souris (build 20260715-163647).
- [x] [125] **Morphing : choisir origine vs cible** — pouvoir décider si les paramètres de morphing d'une cellule fusionnée restent sur les valeurs d'origine ou atteignent les valeurs cibles (build 20260715-203803).
- [x] [126] **Retirer le paramètre mix du sound editor, remplacer par main dans track editor** — déplacer le contrôle de routage Main du panneau Sound vers l'onglet Track (build 20260715-163647).
- [x] [127] **Menu settings global** — ajouter un menu de paramètres globaux (dérives du paramètre analog, MIDI global, etc.).
  - Valeur analog par défaut variabilisée via le menu Settings.
  - Persistance dans `Documents/Flash Drum/config.json`.

---

## Court terme (Stabilisation V1 — En cours)


- [x] **[DEBUG]** Routing `Out 1` silent in Studio One while Main Mix works
  - Check host output enable / aux routing
  - Review audio-thread routing code for off-by-one or output-activation issue

- [x] **[FIX]** New tracks silent + solo shared by instrument family + all UI interactions now track-based — rollback 20260701: redo with Studio One compatibility preserved

- [x] [69] Vrai fix du click parasite BD (changement de hauteur/plock) : chemin digital = reset de phase + crossfade cass� supprim�s ; phase reset�e au cold-start uniquement ; plancher d'attaque anti-click (MIN_AMP_ATTACK_MS) ; bug sweep digital +1 Hz corrig� (build 20260531-155232)
- [x] [70] Mode analog/digital BD re-rendu audible : digital = identique au bit pr�s, analog = drift par coup (hauteur �3.5 %, niveau �10 %, temps d'enveloppe �20 %)
- [x] [71] S�curis� les autres voix : perc1 (reset phase inconditionnel ? cold-start only), snare/tom/snare606 (reset digital ? cold-start only + enveloppes recr��es ? setters), hihat (enveloppe recr��e ? setters + biquad peaking recalcul� seulement si freq change). Plancher d'attaque + DC-blockers partout ; drift analog sur snare & tom (sliders expos�s) ; helper partag� `AnalogDrift`. ride/cymbal/clap/open_hihat/kick_808 d�j� click-safe, non modifi�s. (build 20260531-184528)
- [x] [71a] Ajout du drift analogique sur Snare606 + Perc1 (sliders Analog inactifs ? fonctionnels). Audit complet de tous les instruments avec slider Analog.
- [x] [72] Nettoyer les fichiers de cruft h�rit�s de la r�paration ui.rs + .gitignore
- [x] [81] **Bug P1 � Plock li� � la page et pas � la position grid** (FAUX POSITIF � code d�j� correct)
  - Le code utilise `global_step = page_offset + local_step` partout (affichage, clic, x2, copier/coller)
  - Les plocks sont bien index�s par step absolu (0-63) et suivent correctement la pagination
  - Tests confirment le bon fonctionnement des steps 16-63

## Nouveaux bugs & Feedback (Session 2026-06-01)

### Bugs P1 (Critiques � � traiter en priorit�)

- [x] [73] **Corruption caract�res UTF-8 r�currente** dans les boutons/texte UI (CORRIG� - build 20260601-163923)
  - Remplacement des �mojis corrompus (??, ??, ??, ??) par du texte ASCII (Link, Snapshot, Random, Clear)
  - Remplacement des symboles de navigation (?, ?, ?) par des caract�res ASCII (<, >, R)
  - Remplacement des s�parateurs box-drawing (-) et em-dashes (�) par des tirets simples
  - **Cause** : encodage UTF-8 ? Windows-1252 lors de manipulations PowerShell
  - **Pr�vention** : utilisation exclusive de caract�res ASCII dans les labels de boutons pour �viter les probl�mes d'encodage
- [x] [74] **Focus fen�tre plugin bloque Windows** � impossible de switcher vers une autre fen�tre (CORRIG� - build 20260601-170350)
  - Quand la fen�tre Flash Drum est ouverte, le focus revient automatiquement vers Studio One/Flash Drum
  - Bloque l'utilisation d'autres applications (navigateur, explorateur, etc.)
  - Potentiellement li� au workaround focus clavier (SetFocus sur HWND)
  - **Action** : identifier et corriger le hook/m�canisme qui force le focus
  - [x] Regression Studio One menus corrigee (build 20260609-094555) : le workaround clavier ne refocalise plus le VST a chaque frame hors saisie texte
- [x] [88] **Crash Studio One en manipulant le slider Master Volume dB** (CORRIGE - build 20260609-114803)
  - Cause probable : `master_volume` autorise `0.0` (`-inf dB`) mais utilisait `SmoothingStyle::Logarithmic`, incompatible avec un range passant par zero.
  - Correction : passage a `SmoothingStyle::Exponential(50.0)` pour conserver le lissage sans produire de valeurs non finies.
  - Test : `master_volume_smoothing_stays_finite_from_silence`.

### Bugs P1 (UI/UX)

- [x] [75] **Incoh�rence des ranges de volume** dans l'interface (CORRIG� - builds 20260601-171606, 20260609-152742, 20260609-160617)
  - Slider en haut du Sound Editor : affiche en dB (`-inf dB` a `+6.0 dB`), stockage interne gain lineaire `0.0..2.0`.
  - Slider dans la lane de la grille : courbe dB coherente, stockage interne gain lineaire `0.0..2.0`.
  - Ancien slider Volume data-driven en bas du Sound Editor supprime.
  - **Action** : uniformiser � 0.0�2.0 partout (coh�rent avec le gain de sortie)
  - Regression corrigee : Sound Editor garde uniquement le Volume du haut ; `StandardField::Volume` aligne a `0.0..2.0`.
  - UX corrigee : double-clic sur un volume local reset a `0 dB`.
- [x] [89] **Hauteur VST fixe pour eviter les sauts d'interface** (CORRIGE - builds 20260609-141438, 20260609-144118, 20260609-145809, 20260609-150545)
  - `EguiState::from_size` passe a `1480x800`.
  - `ResizableWindow::min_size` passe a `1480x800` avec `resizable(false)`.
  - Fix Studio One : `ResizableWindow::fixed_size(1480x800)` force la taille effective et bloque l'ancien auto-resize par contenu a `850px`.
  - Sound Editor : ajout d'un scroll interne pour les controles de synthese, avec le titre et les onglets instruments hors scroll.
  - Objectif : conserver une hauteur stable lors des changements d'instruments.

### Features P1 (Parit� PoC / Impact fort)

- [x] [76] **Longueur globale du pattern ajustable 1 ? 64 steps** (CORRIG� - build 20260601-175002)
  - 4 pages de 16 steps maximum
  - Pr�voir un switch "Follow lecture" (la grille suit le playhead ou reste fixe)
  - Complexit� : Moyenne-�lev�e
  - **Note** : cela implique de revoir la logique `SharedPattern` (actuellement 16 steps) et l'UI de pagination

### Features P2 (Am�lioration)

- [x] [77] **3 types de clicks pour la Bass Drum** (CORRIG� - build 20260602-174136)
  - Soft : click subtil, rond
  - Medium : click standard (actuel)
  - Hard : click agressif, transitoire pointu
  - Complexit� : Moyenne, 3-5 jours
  - **Fix bug** : `set_settings()` ne recr�ait pas le `ClickGenerator` quand `click_type` changeait
  - Valeurs exag�r�es pour diff�renciation audible (Soft/Medium/Hard)
- [x] [79] **D�placer le slider de longueur � c�t� de la pagination**
  - Slider "Len" retir� du header bar, positionn� avec les boutons de page
  - Ajout de boutons rapides 16/32/48/64
  - Ajout du bouton x2 pour doubler le pattern (avec copie des plocks)
  - Grisage du bouton x2 quand len > 32
- [x] [80] **LED rouge sous la page en cours de lecture**
  - Petit cercle rouge sous le bouton de page active dans le s�quenceur
  - Ind�pendant du highlight bleu de la page affich�e
- [x] [78] **Clarifier/documenter le mode Analog** � Document cr�� dans `docs/analog-mode.md`
  - Diff�renciation claire entre instruments avec drift op�rationnel (Kick, Snare, Tom, Cymbal, B8) et analog fix� (HiHat, OpenHH, Clap, Ride, Snare606, Zap)
  - Amplitude du drift document�e : Kick �3.5% pitch / �10% niveau, autres ~7.5% pitch max
  - Valeurs par d�faut 0.3 vs 1.0 expliqu�es
  - Recommandations par style musical et conseils de d�pannage inclus
  - La section "Analyse Technique (Reference)" de ce fichier reste disponible pour les d�tails d'impl�mentation
- [x] [67] Positionner le volume en haut du sound editor + ajouter un controle de volume sur chaque lane de la grille (Complexité: Faible, P1)
- [x] [68] Couleurs differentes pour plock link global vs full snapshot (orange / rouge) pour distinguer visuellement les modes (Complexité: Faible, P1)
- [x] [55] Ameliorer le rendu Snare 606 (plus percutant, plus proche TR-606)
  - raw noise excite le resonator directement
  - snap envelope ultra-court (0.2ms attack, 3ms decay)
  - defaults ajustes : decay 0.25s, filter_freq 8000Hz, tone 0.4, snap 0.6
- [x] [1] Corriger la double instanciation du `Sequencer` dans `lib.rs` (lignes 250 + 256)
- [x] [2] Revalider dans Studio One la sauvegarde/reouverture d'une song avec grille modifiee (build `20260511-091259`)
- [x] [3] Tester un projet Studio One neuf : insertion, sorties Kick/Snare/HH/Open HH/Toms, audio sur chaque bus, sauvegarde/reouverture
- [x] [4] Verifier que le Main Mix reste audible quand les sorties separees sont activees
- [x] [5] Verifier que mutes/solos affectent correctement Main Mix et sorties separees

## Refactoring & Qualite de code

- [x] [6] Remplacer `Vec<Box<dyn Voice>>` par une `enum` pour eliminer le dynamic dispatch dans le moteur audio
- [x] [7] Nettoyer les parametres legacy `st01` a `st16` dans `DrumFlashParams` (garder uniquement la logique de migration dans `filter_state`)
- [x] [8] Corriger le warning clippy dans `test_standalone.rs` (needless_range_loop)
- [x] [9] Passer `cargo clippy --all-targets` proprement sans warnings

## Tests & Validation

- [x] [10] Tester le plugin dans au moins un autre DAW (Reaper recommande)
  - chargement OK dans Reaper
  - sauts de volume initialement suspectes : RMS du plugin mesure stable a ~0.4 dB pres
  - confirme reproduit avec un autre plugin dans Reaper → pb driver audio, plugin innocent
- [x] [10a] Corriger les defaults de decay (snare 0.47, hihat 0.36, open_hh 0.66)
- [x] [10b] Corriger le choke qui ne fonctionne plus
- [x] [10c] Corriger les step skips rares (sync_to_host moins agressif)
- [x] [10d] Remplacer sliders par checkbox/combobox pour paramètres bool/enum/algos
- [x] [13] Verifier la precision du timing du sequencer (compteur d'echantillons vs transport hote, correction continue)

## Fonctionnalites P1 (Parite PoC — Impact fort)

- [x] [14] Editer les reglages de synthese par instrument dans l'UI (frequence, decay, volume, filter)
- [x] [15] Connecter `filter_freq` dans `SnareVoice` (actuellement ignore)
- [x] [16] Ajouter un bouton "Test" par instrument pour declencher le son isole
- [x] [17] Ajouter export MIDI fichier depuis le plugin
- [x] [17a] Corriger l'export MIDI fichier/drag-drop pour inclure les 13 instruments
  - `midi_export.rs` utilise encore deux tableaux `midi_notes` hardcodes a 12 notes
  - deriver les notes depuis `instrument_registry::INSTRUMENTS` ou une constante unique partagee avec `MIDI_NOTE_MAP`
  - verifier que Perc1 (note 37) est exporte en fichier MIDI et dans `export_pattern_to_midi_bytes`
  - ajouter un test unitaire couvrant au moins le 13e instrument
- [x] [18] Ajouter sortie MIDI temps reel vers hardware externe
- [x] [19] Ajouter la generation de pattern aleatoire (grille + option Random BPM + option Random Sounds)

## Fonctionnalites P2 (Post-V1 — Nice to have)
- [x] [66b] Correction focus clavier Windows (SetFocus sur HWND plugin) � build 20260529-124136\n
- [x] [20] Ajouter swing
- [x] [21] Ajouter un facteur de groove parametrable (Straight/Swing16/Shuffle/MPC)
- [x] [22] Ajouter un parametre analogique pour legeres variations aleatoires (humanize per track)
- [x] [23] Permettre un mode stereo analogique avec variation gauche/droite (push/pull per track)
- [x] [24] Ajouter song mode (placeholder UI P1-P8, backend à câbler)
- [x] [24a] Ajouter modularité des instruments (algos + special params)
- [x] [24b] Ajouter Kick algos (Sine/Square/FM) + click transient
- [x] [24c] Ajouter Snare algos (Synth/Noise/Layered) + snap param
- [x] [24d] Ajouter Clap, Ride, Cymbal voices
- [x] [25] Labels complets des instruments dans l'UI et couleurs par instrument (labels courts BD/SD/HH, couleurs blocs de 4 steps, grisage len)
- [x] [26a] Per-instrument Mix Bus checkbox (exclure du Main Mix)
- [x] [26b] Clap Echo plockable par step
- [x] [26c] Masquer les paramètres inutiles par instrument dans le Sound Panel
- [x] [26d] Nouvel instrument B8 (TR-808 Bass Drum)
- [x] [26e] Slider Analog actif pour B8
- [x] [26f] Release fonctionnel pour B8 (DecayReleaseEnvelope)
- [x] [26g] Attack ramp 1.5 ms sur B8 (élimine click de démarrage)
- [x] [26h] Filtre LP dédié Click Tone sur B8 (100-8000 Hz)
- [x] [26i] Plock B8 fix : special params (accent/snap/pitch_drop/click_tone) stockés
- [x] [26j] Finaliser les réglages de synthèse par instrument
  - `standard_params` data-driven avec ranges UI (min/max, log, suffix, checkbox)
  - Ranges corrigés pour éviter le clamp involontaire (ex: Ride decay 1.2s > slider 0.5s)
- [x] [51] Ajouter un paramètre Attack réglable par instrument (graphique AHDSR complet A-H-D-R)
- [x] **[EN COURS � Phase 1a OK]** [66] Presets d'instruments — sauvegarder/charger des réglages de synthese par voix (Complexité: Moyenne, P2)
- [x] [26k] Refonte UI Phase 1 (grid intégré, sound panel ongleté, auto-edit)
  - Sound Panel regroupé par familles data-driven (OSC/ENV/FILTER/OUTPUT)
  - Visualisations interactives d'enveloppe (Amp AHDSR + Filter Env)
  - Layout horizontal : params à gauche, graph à droite
- [x] [26l] Corriger le toggle stereo pour certains instruments
  - exposer/finir les toggles stéréo pour les voix où la largeur apporte une vraie valeur : Snare, HiHat, OpenHH, Clap, Ride, Cymbal, Snare606, Perc1
  - garder Kick, B8 et Toms mono par défaut pour préserver le centre et la compatibilité mono
  - priorité technique : finir stereo Snare606 et vérifier que les toggles UI ne sont visibles que sur les voix concernées
- [x] [55] Saturation / distortion par instrument (tous les 13 voix)
  - Module `saturation.rs` avec 5 algorithmes distincts (SoftClip, Valve, Transistor, HardClip, Tape)
  - Paramètres exposés dans le Sound Panel : Type, Amount, Mix, Output Gain, Pre-Filter
  - Drive d'entrée mappé 1×..20× pour effet audible
  - Pre-Filter comme checkbox toggle (post-filter par défaut)
  - Section SAT dédiée dans le Sound Panel (ParamFamily::Saturation)
  - Combobox affichant les noms d'algorithmes (SoftClip, Valve, etc.)
  - Saturation appliquée sur 8/13 instruments : Kick, Snare, Snare606, B8, Tom1-3, Perc1
  - ~~Saturation sur les 5 restants (HiHat, OpenHH, Clap, Ride, Cymbal)~~ — pas prioritaire
  - Special params augmentés de 8 à 32 slots (`special: [f32; 32]`)
  - Plock field masks passés de u32 à u64 (46 fields total, 32 special params plockables)
  - Auto-edit activé par défaut
- [x] [62] Cymbal : retirer frequency inutilisé, ajouter Shimmer Freq + Noise Type
  - `frequency` retiré du Sound Panel (paramètre inutilisé sur un bruit)
  - `Shimmer Freq` (1-50 Hz) : module la fréquence du FM shimmer (était hardcodé à 15 Hz)
  - `Noise Type` : White / Pink / Brown / Blue — générateurs Voss-McCartney dans dsp.rs
  - Combobox UI pour sélectionner le type de bruit
- [x] [63] Bug B8 se coupe quand on modifie CY : corrigé division par zéro dans `ExpDecayEnvelope::set_attack_ms`
  - Quand attack_time passe à 0 pendant un ramp actif → snap à peak immédiat pour éviter NaN
  - Bouton "T" (Test) : appelle maintenant `set_voice_settings` avant `trigger`
- [x] [82] **Intégrer les éléments graphiques définis avec Claude Design**
  - Phase 1a : Widgets custom (ToggleLED, ToggleSwitch, StyledButton, SegmentedControl)
  - Phase 1b : Header redesign (fond PANEL, bordure LINE, séparateurs DIVIDER, padding 14px)
  - Phase 1c : Style global sombre (BG, PANEL2, P_HOVER, P_ACTIVE, BLUE via egui::Visuals)
  - Phase 1d : Boutons page 1-4 stylisés + glow LED lecture
  - Phase 2 reprise : layout fixe 2 colonnes, grille custom, Sound Editor et panneaux bas rapprochés du design pack (builds 20260611-184532, 20260611-194657, 20260611-201611)
  - Assets UI (icônes, couleurs, fonts, layout) produits par Claude Design
  - Remplacer les widgets egui basiques par des widgets custom avec le design system
  - **Complexité : Moyenne-Élevée, 1-2 semaines, P2**

## Fonctionnalites P3 (Avancees / Complexes)

- [ ] [69] Creer un instrument percussif a base de wavetables — phase recherche et prototypage (Complexité: Élevée, 2-4 semaines, P3)
- [ ] [27] Generation IA de patterns par style (rock, techno, rap, jazz, reggae, metal, funk, latin, disco, trap)
- [~] [83] **Instruments sampler TR-606 multisamplé** — **1er instrument livré : BD6smp (build 20260802-160117)**
  - [x] Nouveau type de voix "Sampler" (multisample) — `synthesis/sample_bank.rs` + `synthesis/bd606.rs`
  - [x] Sélection aléatoire du layer à chaque trigger (sans répétition immédiate) pour simuler l'imperfection analogique
  - [x] Chargement de samples WAV embarqués (`include_bytes!` + `OnceLock`, zéro alloc audio thread)
  - [~] Étendre aux autres instruments de la 606 (SD, HH, …) — même infra, il suffit d'ajouter les WAV
    - [x] SD6smp (Snare 606)
    - [x] CH6smp (Closed Hi-hat 606, note MIDI 42) — build 20260804-170126, `wav/CH.wav` → `assets/ch606.wav`
    - [ ] OH (Open Hi-hat), Cymbal, … restants
- [ ] [84] **Instruments sampler Yamaha RX11**
  - Même architecture sampler que [83] avec le kit RX11
  - 4 layers par son pour l'effet analog random
  - Dépend de [83] (infrastructure sampler)
  - **Complexité : Moyenne, 2-3 semaines, P3**
- [x] [28] Drag & drop MIDI directement vers le DAW — helper externe validé dans Studio One
  - [x] remplacer l'ancien `dnd_set_drag_payload(bytes)` interne egui par un drag fichier OS natif Windows (`CF_HDROP` via OLE `DoDragDrop`)
  - [x] garder l'export fichier OK via bouton MIDI dans `Documents/Flash Drum/exports`
  - [x] isoler `DoDragDrop` hors process DAW via `drum-pattern-midi-drag-helper.exe`
  - [x] réactiver le bouton `Drag` : export MIDI puis ouverture d'une petite poignée de drag externe
  - [x] valider dans Studio One : cliquer `Drag`, puis glisser la fenêtre `Drag MIDI` vers une piste/instrument et vérifier qu'un clip MIDI est créé sans crash
- [x] [29] Parameter locks (plocks) façon Elektron — changer un paramètre de synthese par step
  - 14 champs plockables (12 sound settings + clap_echo + algo)
  - special params (accent/snap/pitch_drop) propagés uniquement au trigger (fix echo perdu)

## Nouveaux éléments (À prioriser)

- [ ] [56] Ajouter une percussion de type Tom Simmons (Complexité: Moyenne, 3-5 jours)
  - Créer un nouveau module de synthèse
  - Ajouter l'instrument dans le registre des instruments
  - Créer les paramètres spécifiques et l'interface utilisateur
  - Intégrer dans le système de mixage et de sortie audio

  - Refonte majeure de l'architecture du séquencer
  - Système de tracks fixes à 14 slots, actives visibles dans l'UI
  - Gestion de l'ajout/suppression d'instruments à chaud
  - Interface utilisateur pour la configuration modulaire
  - Système de sauvegarde/restoration des configurations

- [x] [58] Gestion des patterns et song (Complexité: Moyenne-Élevée, 3-5 semaines) � **FAIT (build 20260604-175459)**
  - [x] Pattern bank P1-P8 : sauvegarde/chargement de patterns complets (grid + plocks + seq plocks)
  - [x] **Stabilisation pattern bank** (build 20260604-170055) : preallocation buffers, pas d'alloc audio thread, pas de panic mutex
  - [x] Song mode : chaînage séquentiel des patterns via `SongSequence` (64 steps max)
  - [x] **Refonte UX Pattern Bank** (build 20260604-175459) : boutons Save/Load explicites, indicateurs d'occupation, tooltips, sync pattern_length au load
  - [x] UI Song Editor : grille de steps avec sélection P1-P8, loop toggle, longueur ajustable
  - [x] Persistance DAW : `SongSequence` intégrée à `PatternBank` (champ `pattern-bank-v1`)
  - [x] Playback : détection de wrap via `loop_count`, avance auto song_position, chargement pattern

- [x] [59] **Gestion des plocks de type séquenceur � COMPLET** (FAIT build 20260603-205246)
  - [x] Architecture `SequencerPlockState` lock-free (probabilité, stutter, condition, microtiming)
  - [x] Switch UI "Plock mode: Sound / Sequencer" sous la grille
  - [x] Menu contextuel adaptatif (mode Seq = paramètres séquenceur)
  - [x] Probabilité 0-100% par step � instrument
  - [x] Skip aléatoire dans le callback audio (LCG)
  - [x] Persistance DAW (`seq-plock-v1`)
  - [x] **Phase 2:** Couleurs violet pour les plocks séquenceur (visibles uniquement en mode Seq)
  - [x] **Phase 3:** Switch avec label "Plock mode" + couleur orange (Sound) / violet (Sequencer)
  - [x] **Phase 4:** Stutter (1-8x) � déclenche multiple fois le son
  - [x] **Phase 5:** Conditions (Always, 1st, Not 1st, 1/2, 2/2, 1/3, 2/3, 3/3, 1/4, 2/4, 3/4, 4/4)
  - [x] **Fix build 20260603-211721:** `SequencerStepParams::default()` probability=1.0, stutter_count=1 ; retrait Fill/NotFill ; stutter avec espacement temporel

- [x] [60] Désactivation du séquenceur interne et pilotage MIDI depuis le DAW (Complexité: Moyenne) — **FAIT (build 20260604-141711)**
  - [x] Ajout d'un paramètre `use_internal_sequencer` (ID: `int_seq`) pour activer/désactiver le séquenceur interne
  - [x] Mode "MIDI thru" : les NoteOn reçus déclenchent les voix via `instrument_registry::voice_idx_from_midi_note()`
  - [x] Mapping complet GM Drum Map pour les 13 voix
  - [x] UI: checkbox "Seq" dans la header bar

- [x] [61] Pour les BD, ajouter un switch de tuning entre Hz et Notes (Complexité: Faible, 2-3 jours)
  - Ajouter un paramètre booléen pour basculer entre les modes de tuning
  - Implémenter la conversion Hz ↔ Notes (standard MIDI)
  - Mettre à jour l'interface utilisateur pour afficher le bon format
  - S'assurer que la valeur est correctement sauvegardée/restaurée
  - Appliquer aux instruments Kick et B8 (et potentiellement autres bass drums)
- [x] [61b] Ajouter copier/coller un plock dans le menu bouton droit � **FAIT (build 20260603-142316)**
  - Stocker le plock copié dans l'état de l'éditeur (`EditorUIState.plock_clipboard` via `SinglePlockClipboard`)
  - Boutons "Copy Plock" / "Paste Plock" dans le menu contextuel de la grid
  - Coller écrase le plock existant sur la step cible
  - Support multi-instrument : on ne colle que si l'instrument correspond
  - Disponible à la fois en mode création (step vide) et édition (step avec plock)
- [x] [29a] Refactor plock UI data-driven depuis `instrument_registry`
  - remplacer les branches hardcodees par instrument dans `draw_plock_menu`
  - exposer automatiquement les `special_params` de Clap, Snare606, B8, Perc1 et futurs instruments
  - aligner les champs plock stockes/lus (`FIELD_COUNT = 18`) avec les special params reels
  - clarifier/corriger l'incoherence Clap Echo : UI lit le champ 12 alors que `PlockState::set_settings()` stocke les specials en 14..17
  - ajouter tests unitaires sur `PlockState::set_settings/get_settings` pour Clap Echo, B8 specials et Perc1 specials
- [x] [39] Refactor : paramètres dédiés par instrument (au lieu du `VoiceSettings` partagé + `special[8]`). Permet labels, ranges et défauts spécifiques par voix.
  - [x] Prototype Kick : `KickSettings` struct typée, conversion `VoiceSettings ↔ KickSettings`, tests passents
  - [x] Généraliser aux 12 autres instruments (Snare, HiHat, OpenHH, Tom1-3, Clap, Ride, Cymbal, Snare606, B8, Perc1)
- [x] [40] Filter envelope (cutoff modulé par AD/ADSR) — Kick, Snare, Tom, HiHat, Snare606
- [ ] [41] Émulation circuit-exact TR-606 (WDF, modèle non-linéaire VCA, oversampling) — vs grey-box actuelle
- [x] [54] Saisie clavier de valeurs précises + Shift+mouse pour affiner les sliders de paramètres
  - LocalParamSlider créé pour remplacer egui::Slider dans les plocks et paramètres spéciaux
  - Shift+drag implémenté pour le fine-tuning sur tous les sliders
  - Hauteurs de sliders harmonisées pour une expérience visuelle cohérente

## Dette technique & Documentation

- [x] [30] ~~Clarifier si `index.js` doit etre conserve ou archive~~ � **ARCHIV�**
  - Les fichiers `index.html` et `index.js` (PoC web React) ont �t� d�plac�s dans `archive/web-poc/`
  - Le plugin VST3 est d�sormais le seul produit actif
- [x] [31] ~~Revoir l'organisation du repo pour separer clairement PoC web et plugin~~ � **FAIT**
  - Le PoC web est archiv� dans `archive/web-poc/`
  - La racine du repo contient uniquement le plugin (`drum-pattern-vst/`), la doc et les fichiers de suivi
- [x] [31a] Clarifier l'emplacement des docs produit actives
  - `AGENTS.md` cite `PROJECT_BRIEF.md` et `BACKLOG_VST.md`, mais les fichiers presents sont sous `docs/historique/`
  - decider si ces docs doivent revenir a la racine, etre remplacees par `TODO.md`/`README.md`, ou etre explicitement marquees comme archivees
  - mettre a jour `README.md`, `AGENTS.md` et les references croisees en consequence
- [x] [32] Synchroniser `BACKLOG_VST.md` avec `TODO.md`
- [x] [33] Reduire les warnings Rust inutiles (0 warning sur lib + bin + tests, release inclus)
- [x] [34] Garder les fichiers de sauvegarde hors de `src/` — Dossier `drum-pattern-vst/backups/` créé, `.gitignore` déjà configuré
- [x] **[87] Step Fusion V2** — Fusion de cellules pour tuplets/micro-rhythmes (build 20260607-131747)
  - **Spécifications:**
    - Shift+clic début → Shift+clic fin = sélection plage à fusionner
    - Double-clic sur groupe fusionné = édition inline du nombre de steps
    - Limites: 1-64 steps, par instrument, indépendant par ligne
    - Générateur/Clear: suppriment les fusions (reset)
    - Plocks: appliqués par cellule de départ (tous les pulses partagent le même plock sonore)
    - Stutter seq-plock: désactivé/ignoré sur une fusion
  - **Implémentation:**
    - [x] Data model `FusedGroup { start_cell, end_cell, step_count }` par instrument
    - [x] UI: grille fixe 16 colonnes/page, Shift+clic sélection page-local, double-clic édition pulses
    - [x] Séquenceur: cellule de départ uniquement, cellules internes supprimées, métadonnées fusion vers audio
    - [x] Audio: pulses régulièrement espacés sur la durée de la fusion via queue préallouée
    - [x] Rendu visuel: cellules fixes avec bordure/couleur fusion + texte "pulses/cells" sur la cellule de départ
    - [x] Fix UI build 20260608-190515: rendu en vrai bloc graphique unique, sans subdivisions internes visibles
    - [x] Fix UX build 20260608-191352: style aligne cellules standard, edition inline du nombre de pulses, creation active par defaut
    - [x] Fix build 20260608-192613: creation de fusion supprime les plocks sound/seq des cellules internes couvertes
    - [x] Fix UI build 20260608-193357: indicateur "Maj for fusion mode" gris/bleu + "Select 2 cells" sous la grille
    - [x] Fix build 20260608-194139: detection Maj robuste via Win32 `GetAsyncKeyState()` pour l'indicateur et Shift+clic fusion
    - [x] Fix build 20260608-195857: `Copy/Paste Page` et `x2` copient aussi les groupes Step Fusion ; `Clear Page` supprime les fusions de la page
    - [x] Fix UI build 20260609-100205: edition inline du nombre de pulses sans decalage de ligne ; clic exterieur ferme l'edition et garde la fusion active
    - [x] Fix UI build 20260609-102249: panneau `Fusion x-y (cells) Steps` deplace dans une box Fusion reservee stable ; clic sur son champ `Steps` ne ferme plus l'edition
    - [x] Fix UI build 20260609-112628: double-clic sur cellule fusionnee traite avant le clic simple, ouvre l'edition sans desactiver la fusion et sans toggle differe
    - [x] Fix UI build 20260609-121512: premier Maj+clic de fusion colore le point central de la cellule source en bleu ; relacher Maj annule la selection et restaure la couleur normale
    - [x] Fix UI build 20260609-124302: cellule source de selection Fusion rendue comme active temporaire (`X` + fond bleu clignotant + bordure bleue) pour etre plus visible
    - [x] Fix UI build 20260712-103414: valider le champ `Steps` avec `Enter` applique la valeur 1..64 puis ferme l'edition inline de la fusion
    - [x] Fix UI build 20260712-104124: regression freeze Studio One apres `Enter` corrigee en relachant le focus clavier et en fermant l'edition a la frame suivante
    - [x] Fix UI build 20260712-110414: remplacement du `TextEdit` par un `DragValue` natif egui pour le champ `Steps`; validation `Enter` geree en interne sans freeze
    - [x] Persistance DAW (champ `fusion-v1`) — réalisée via `pattern-v5` + `pattern-bank-v1`
    - [x] Tests: filtrage invalides + suppression triggers internes + métadonnées pulses
- [x] [34a] Corriger le click de retrigger kick (2 steps BD proches)
- [x] [34b] Nettoyer le code mort dans `special_params.rs` (struct `SpecialParamDef`, tous les `*_SPECIALS`, helper `specials_for`, methodes trait `supported_algos`/`special_params`)
- [x] [34c] Corriger les libelles obsoletes multi-out dans le code
  - `AUX_OUT_COUNT` vaut 13 mais `lib.rs` parle encore de "10 stereo drum outs"
  - corriger le commentaire "Frozen at 10" et le `PortNames.layout`
  - verifier que la doc Studio One reste alignee avec Main Mix + 13 sorties aux

## Bugs a corriger
- [x] [70] Kick : click de retrigger quand la queue percute l'attaque du suivant � corrig� (ne pas retrigger le click pendant la tail) � build 20260529-172133\n
- [x] [64] Revoir l'algo de polyrythmie (lane length) — comportement bizarre, longueurs mal synchronisées (Complexité: Moyenne, P1)
  - Build 20260609-185930 : fix affichage valeur effective dans l'UI.
  - Par defaut suit Pattern Length. Drag = verrouille sur cette valeur.
  - Si Pattern > valeur verrouillee → garde valeur (polyrythmie).
  - Si Pattern <= valeur verrouillee → suit pattern (trop court).
  - Clic droit = deverrouille. Persistance DAW via `lane-locks-v1`.
- [x] [65] Revoir les algos de generation pattern avec les nouveaux instruments (13 voix) — tous les générateurs gèrent 13 instruments; rôles musicaux enrichis pour Snare 606, B8, Perc1 dans le style Rock (démonstration)
- [x] [45] Sauts de volume general dans Reaper — diagnostique externe (driver audio, reproduit avec d'autres plugins)
- [x] [46] Revert du code Perc1 au commit 5ae1286 (Zap) — build stable réinstallé
- [x] [47] Refaire Perc1 proprement : ne pas recréer les enveloppes dans `set_settings`
- [x] [48] Refaire Perc1 proprement : utiliser `DecayReleaseEnvelope` pour le slider Release
- [x] [50] Diagnostiquer pourquoi la moitié des paramètres Perc1 ne sont pas actionnables (faux positif — tests unitaires confirment que decay/release fonctionnent)
- [x] [49] Refaire Perc1 proprement : rendre le plock menu data-driven (plus de hardcode par index)
- [x] [53] Plock Snapshot vs Link : choix à la création du plock (snapshot fige tout, link ne stocke que les champs modifiés)
- [x] [35] Diagnostiquer la sauvegarde/reouverture Studio One
- [x] [35a] Plock B8 : accent/snap/pitch_drop/click_tone plockables
- [x] [36] Corriger la persistance de grille via `pattern-v1`
- [x] [37] Migration legacy depuis les parametres caches `st01` a `st16`
- [x] [38] ~~Ecart entre documentation et code reel a surveiller~~ � **DOCUMENTATION � JOUR**
  - `README.md` : mis � jour avec la structure du repo (archive/web-poc/)
  - `docs/infrastructure.md` : cr�� � guide build, architecture, tests, d�ploiement
  - `docs/user-guide.md` : cr�� � guide utilisateur complet (UI, plocks, export, multi-out)
  - `docs/analog-mode.md` : cr�� pr�c�demment � documentation technique du mode Analog
- [x] [38b] Supprimer les `unwrap()` evitables du chemin audio/UI sensible
  - `lib.rs::process()` utilise `DrumVoice::from_index(...).unwrap()` sur des index bornes par `DrumVoice::COUNT`
  - risque faible aujourd'hui, mais non conforme a la regle stricte "audio thread sans panic"
  - remplacer par API interne sans `Option`, ou par branche defensive sans panic
- [x] [38a] Fusionner `CLAUDE.md` dans `AGENTS.md` (13 instruments, AUX_OUT_COUNT = 13, Zap ajouté)
- [x] [42] Crash a l'instanciation avec 11e voix (cause: `IntRange { min:0, max:0 }` → div par zéro nih-plug)
- [x] [43] Index out of bounds dans UI (`hums`/`pushes`/`lengths` taille 10 vs INSTRUMENT_LABELS taille 11)
- [x] [44] Step mask hardcode `0x3ff` (10 bits) — extensible via `INSTRUMENT_COUNT`

## Bugs a corriger (Nouveaux)

- [x] **[86] Plocks restent du pattern precedent au changement de slot** (CORRIGE — build 20260605-094135)
  - **Symptome :** charger un pattern depuis P1→P2 laissait les plocks de P1 visibles
  - **Cause double :**
    1. `restore_from_buffers()` skipait le restore si `plock_bytes.len()` < taille attendue (FIELD_COUNT=46). Les anciens slots (FIELD_COUNT=18) n'etaient pas restaures
    2. Aucun `clear_all()` avant restauration → plocks residuels du pattern precedent persistaient
  - **Fix :**
    - Detection automatique du format (18 vs 46 fields) dans `restore_from_buffers()` et `PatternSlot::restore()`
    - `PlockState::clear_all()` + `SequencerPlockState::clear_all()` : vident tous les plocks avant restauration
    - `load_pattern_from_slot()` appelle `clear_all()` sur les deux types de plocks avant `restore_from_buffers()`

- [x] **[85] CRASH — Retour � P1 apr�s avoir sauvegard� 2 patterns fait crasher Studio One** (CORRIG� — build 20260605-090814)
  - **Cause :** `MAX_PLOCK_BYTES` utilisait `18` (ancien `FIELD_COUNT`) au lieu de `46` (actuel)
  - Buffer sous-allou� de 66 664 → 159 848 bytes ; `copy_data_for_restore()` overflow → crash
  - **Fix :** calcul dynamique depuis `FIELD_COUNT`/`INSTRUMENT_COUNT`/`STEP_COUNT`, plus de hardcode
  - **S�curit� :** `copy_data_for_restore()` prot�g� par `.min()` pour �viter tout overflow futur

- [x] [71] Longueur globale du pattern ajustable 1 => 64 avec 4 pages de 16 steps max. Prevoir un switch de follow de la lecture ou pas (Complexite: Moyenne-Elevee, 1-2 semaines, P1) � **DOUBLON de [76], D�J� CORRIG�**
- [x] [72] Probleme d'affichage du volume : slider en haut de l'editor (1.5 max) et en bas (1) et dans la lane (1.5) � incoherence de range a uniformiser (Complexite: Faible, 1-2 jours, P1) � **DOUBLON de [75], D�J� CORRIG�**
- [x] [73] caracteres esoteriques ont remplace aleatoirement les caracteres normaux dans les boutons/texte UI � CORRIGE (restauration UTF-8 via script Python) � build 20260529-174106 (Complexite: Faible, 1 jour, P1)
- [x] [74] Proposer 3 types de clicks pour la BD (Kick) : soft/medium/hard ou impulse/noise/transient (Complexite: Moyenne, 3-5 jours, P2)

## Bugs a corriger (Actifs)

- [x] [102] **Song mode : changement de pattern de longueur différente ne remet pas la tête à zéro** (P1, Sequencer/Song)
  - Constat utilisateur 2026-07-05 : en song-mode, après un pattern dont la longueur diffère des autres, la tête de lecture continue au lieu de repartir à step 0.
  - Cause : le load de slot envoyait la longueur à l'UI via `pending_pattern_length`, mais l'audio utilisait encore temporairement l'ancien paramètre ; la resynchro hôte pouvait ensuite recaler la position sur la timeline DAW absolue.
  - Correctif : longueur audio-local mise à jour immédiatement au load, redémarrage song programmé à step 0 après transition, resync hôte continue désactivée pendant le song-mode.

- [x] [101] **Regression Push/Pull apres correction playhead** (P1, Sequencer/UI)
  - Constat utilisateur fin de session 2026-06-12 : avec du Push, le decalage devient enorme et impossible a annuler correctement.
  - Dernier changement suspect : build `20260612-210534`, UI playhead decouplee de `current_steps` et basee sur `current_step` global.
  - Correctif (build `20260613-105028`) :
    - `sync_to_host` recalcule `step_counter` depuis la timeline shifted (position hote - push/pull) au lieu de la timeline master.
    - UI grille : playhead alignee sur `current_step` global ; Push/Pull ne deplace plus l'anneau de lecture, seul le timing audio est decale.
  - Objectif atteint : timing audio Push/Pull correct et annulable, playhead visuelle stable quand on module Push.

- [x] [91] **Sortir automatiquement du mode edit quand on selectionne en dehors de la cellule** (P1, UI/UX)
  - Actuellement, le mode edit reste actif meme si on clique ailleurs
  - Comportement attendu : deselection de la cellule = sortie du mode edit
  - Complexite : Faible
  - Correctif (build `20260623-120806`) : lors d'un clic normal, si le clic ne porte pas sur le groupe fusionné en cours d'édition, `finish_fusion_editing_for_ui` est appelé avant de traiter le toggle.

- [x] [92] **Valeurs du menu plock sound par defaut = valeurs globales de l'instrument** (P1, Donnees) — résolu par ST-7 + tests de régression (build 20260707-113932)
  - Constate : la frequence de BD8 (BassDrum808) est a 0 dans le plock au lieu de la valeur globale
  - Verifier que tous les instruments initialisent correctement les valeurs par defaut des plocks
  - Complexite : Faible

## [100] Redesign UI complet (design pack 2026-06-11) — EN COURS

> **Livrable designer** : `design-pack/Flash_Drum_design_11062026/flash-drum-source/`
> Fichiers clés : `DESIGN-SYSTEM.md` (tokens), `LAYOUT.md` (architecture), `assets/fd-data.js` (schémas moteurs)

### Architecture (invariants du design)
- **Système de lanes modulaires** : 4 lanes au départ (BD/SD/HH/TOM), ajoutables jusqu'à 14, réordonnables par drag
- **Registre de moteurs** : Synth (kick/snare/tom/hat/cymbal/clap/perc), Sample, Sample FX, MIDI Out
- **Éditeur dynamique** : contenu reconstruit selon le moteur assigné, aucun paramètre codé en dur
- **Séparation données ↔ rendu** : ajouter instrument/paramètre = éditer une donnée

### Phases d'implémentation

#### Phase 1 — Fondations (structure + tokens)
- [x] [100a] **Mettre à jour `design_system.rs`** avec nouveaux tokens (palette IBM Plex, rayons, gaps, strokes)
- [x] [100b] **Intégrer polices IBM Plex** (Sans + Mono) via `FontDefinitions` egui (build 20260612-090421)
- [x] [100c] **Créer `theme.rs`** — constants `Color32` et helpers (`blue_glow`, `white_a`)
- [x] [100d] **Créer `widgets.rs`** — widgets custom coordonnés (Slider, Freq, Select, Switch, ToggleLED, Knob)
- [x] [100e] **Créer `engine_registry.rs`** — struct `Engine`, `EngineGroup`, `schema_for_engine()`, registre `ENGINES`

#### Phase 2 — Layout général (header + colonnes)
- [x] [100f] **Header redesign** — Brand + Transport (▶/■/●) + Master/Swing/Groove + Seq source (Internal/Ext MIDI segmented) + toggles LED
- [x] [100g] **Layout 2 colonnes** — Gauche (~910px) : séquenceur + page-bar + p-lock-bar + patterns + generator/song | Droite (~568px) : Sound Editor
- [x] [100h] **Sound Editor** — En-tête dynamique (nom + Engine selector) + onglets instruments (14) + zone scroll avec sections

#### Phase 3 — Séquenceur (grille + lanes)
- [x] [100i] **Lane modulaire** — Poignée drag, nom cliquable, menu clic-droit (rename, assign engine, remove), tag M/S/T — rename fait dans l'onglet Track (build 20260713-143422) ; assign engine / remove font partie de la phase modulaire B (reporté, cf. [57])
- [x] [100j] **Grille de steps** — 16 colonnes visibles, états p-lock (Sound/Sequencer exclusifs), playhead, fusion
- [x] [100k] **Page/Length bar** — Pages 1-4, Follow ON/OFF, Len slider 1-64, presets 16/32/48/64, ×2
- [x] [100l] **P-lock modes** — Toggle segmented Sound/Sequencer, menus contextuels (Volume en premier, undo ↺)

#### Phase 4 — Panneaux bas (patterns + generator/song)
- [x] [100m] **Pattern Bank** — Save/Load, slots P1-P8, Clear, Export MIDI, Drag MIDI
- [x] [100n] **Generator/Song panel** — Segmented toggle Generator|Song, Generator = type + A/B + Mix/Dens/Var + Random + GENERATE
- [x] [100o] **Song arranger** — Chaîne de blocs pattern × répétitions, toggle Song Enabled (réalisé via le Song Editor 16 blocs + Song Mode, build 20260708-164626)

#### Phase 5 — Polish & validation
- [x] [100p] **ADSR visualization** — graphe inline réécrit (modèle 3 segments colorés A/D/R, cadre #0c0c11, espacé)
- [x] [100q] **Animations** — Hover transitions 0.14s (build 20260716-142342), step playback glow (build 20260716-142342), toggle LED state transitions (build 20260718-083031 : ToggleLED fades, ToggleSwitch slide, led_segmented cross-fade).
- [x] **Polish Pattern Bank** — Suppression de l'indicateur de debug `[P:X S:X]` et alignement des hauteurs des boutons (Export/Drag/Save/P1-P8/Clr) à 26 px (build 20260716-144332).
- [x] **Double-clic reset sur tous les sliders** — Sliders d'en-tête (`header_param_slider`), Sound Editor, menus P-lock/Morph/Seq P-lock, et config Default Analog : double-clic retourne à la valeur par défaut (build 20260716-150443). `master_volume` repasse à `1.0` (0 dB).
- [x] **Mini sliders de lane : poignée + reset** — Ajout d'une petite poignée blanche au hover/drag et double-clic reset sur Volume (→1.0), Humanize et Push/Pull (build 20260716-152038).
- [x] [100r] **Tests** — Vérifier que tous les moteurs rendent correctement, pas de régression audio (mode léger : tests `all_voices_render_finite_non_silent_output` + `all_voices_stay_finite_on_retrigger`, build 20260716-114252)
- [x] [100s] **Build + install** — VST3 fonctionnel avec nouveau design

### Tâches découvertes pendant la reprise UI 2026-06-11
- [x] [100t] Nettoyer le code UI legacy (~1300 lignes : `draw_grid` & helpers morts + modules `schema.rs` et `engine_registry.rs` supprimés). Restent des warnings de scaffolding (`design_system.rs`, `StyledButton`) — cf. `docs/design/UI-REDESIGN-HANDOFF.md` §4.
- [x] [100u] Polish pixel : Sound Editor (sliders/labels/sections/ADSR), combos → Select stylé, page-bar, bloc Generator réorganisé en 2 rangées + knob non tronqué (jusqu'au build 20260614-205742).
- [x] [100v] **OBSOLÈTE** — Engine selector fonctionnel + registre de moteurs : redondant avec l'architecture modulaire actuelle (types d'instruments fixes par slot + algorithme par type). Le selector inerte a été retiré du Sound Editor.
- [x] [100w] Bouton GENERATE invisible après refonte en 2 lignes — corrigé en revenant à une seule ligne horizontale avec le bouton poussé à droite (build 20260614-092628).

#### Reste à faire (worklist détaillée : `docs/design/UI-REDESIGN-HANDOFF.md` §4)
- [x] [100x] Menus clic-droit p-lock → style `.plk` (284px, fond P_ACTIVE, r9, bordure LINE2), Volume en tête, mode Sound=orange / Sequencer=violet (build 20260616-203617).
- [x] [100y] Recâbler le menu page Copy/Paste/Clear sur la page-bar (helpers conservés sous `#[allow(dead_code)]` : `clear_page_fusions_for_ui`, `replace_page_fusions_for_ui`).
  - Bouton droit sur les numéros de page pour ouvrir le menu.
  - Actions : Copy Page, Paste Page, Clear Page.
  - Warnings de confirmation avant Paste (écrase la page cible) et Clear (supprime grille + plocks + fusions de la page).
  - Build `20260623-124600`.
- [x] [100aa] ~~Nettoyage final : adopter `StyledButton` (hover chrome), retirer `design_system.rs`/`SegmentedControl` non câblés, remplacer `allocate_ui_at_rect` (déprécié) par `allocate_new_ui`~~ — **OBSOLÈTE (2026-07-21)** : `allocate_ui_at_rect` déjà remplacé (build précédente), `design_system.rs`/`SegmentedControl` déjà supprimés ; seul `StyledButton` resterait (helper bouton partagé, peu prioritaire).
- [x] [100ab] Dropdown Algo dynamique dans le menu p-lock (plage selon algo_count, nom affiché, masquage si 1 algo) - build 20260624-171823.
- [x] [100ac] Morphing par pulse sur les cellules fusionnees (select Morph + slider End, interpolation lineaire, params continus + special params continus, persistence DAW pattern-v3 + pattern bank) - build 20260629-160624.

### Notes
- **Volume** : range -60 dB à +6 dB (actuellement 0..2 linéaire, à convertir)
- **Norme de casse** : Title Case partout
- **Pas de gradients** : aplats + ombres/glow subtils
- **Contrainte egui** : tout en primitives (rect, cercle, texte), pas d'images

---

## Investigation & Features (A prioriser)

- [x] [93] **Son tres ecourte interessant quand on maintient un slider** (P2, Audio/Design) — mécanisme identifié + transformé en instrument (build 20260806-161402)
  - **Mécanisme** : nécessite une voix qui « ring » (pattern en lecture). Bouger un slider ré-applique `set_settings` à chaque frame ; sur l'Open-Hat ça **recrée l'enveloppe** → la queue est hachée ~60 Hz. Pas de preview au drag ; artefact anti-clic (famille du clic kick).
  - **Livré comme feature** : nouvel instrument **Buzz** (percussion tonale + bruit réglable + gate/retrigger d'enveloppe à taux réglable) qui reproduit l'effet délibérément (Gate Rate/Depth/Shape, modes Smooth/Razor). Voir CHANGELOG.
  - (Le fix anti-clic de l'Open-Hat lui-même — pour ne PLUS l'avoir involontairement — reste optionnel/non fait, à la demande de l'utilisateur.)

- [ ] [94] **Ajouter un parametre pitch LFO sur les Toms** (P2, Synthese)
  - Intensite, Rate, Type de LFO (sine/triangle/square/saw), arrivee progressive
  - Permet des variations de hauteur dynamiques sur les toms
  - Complexite : Moyenne, 3-5 jours

- [ ] [95] **Ajouter un instrument de type MIDI (avec MIDI out)** (P2/P3, Architecture MIDI)
  - Voix virtuelle qui envoie des NoteOn/NoteOff MIDI sur une sortie MIDI externe
  - Pas de synthese interne, juste du routage MIDI
  - Permet de declencher des instruments externes depuis le sequencer
  - Complexite : Moyenne-Elevee, 1-2 semaines

## Tests avances (Post-V1)

- [x] [12] Ajouter un test de stress du sequencer (longue session, stabilite du timing) - 6 tests impl�ment�s

## Analyse Technique (Reference)

### Mode Analog vs Digital - Comportement par Instrument

**Fonctionnement du mode Analog (`analog >= 0.5`)** :
- Oscillateurs conservent leur phase actuelle (kick.rs:142-148)
- Enveloppes relanc�es depuis leur valeur actuelle via `trigger_at_peak()`
- Son organique et continu, comme un vrai circuit analogique
- Retriggers pendant une queue ajoutent de l'�nergie plut�t que de r�initialiser
- Comportement similaire aux drum machines analogiques (Roland TR-808/909)

**Mode Digital (`analog < 0.5`)** :
- Oscillateurs r�initialis�s � phase = 0.0 avec crossfade sur 2 samples (kick.rs:150-165)
- Enveloppes repartent de z�ro via `trigger()`
- Son propre et r�p�table, id�al pour l'EDM et le techno
- Chaque hit sonne identique, m�me sur des retriggers rapides
- Comportement similaire aux drum machines num�riques (Roland TR-626, LinnDrum)

**Impl�mentation technique par instrument** :

**Kick (kick.rs)** :
- Analog: `self.osc.phase` pr�serv�, `self.noise_osc.phase` pr�serv�
- Digital: Crossfade entre ancienne et nouvelle phase sur 2 samples
- Impact sonore: Analog = plus de "punch" sur les retriggers, Digital = plus pr�cis

**Kick 808 (kick_808.rs)** :
- Analog: Phase pr�serv�e, simulate le comportement du circuit original
- Digital: R�initialisation compl�te ("cold start" comme l'original 808)
- Impact sonore: Analog = plus chaud, Digital = plus cliquety

**Snare (snare.rs)** :
- Analog: Phase pr�serv�e + noise generator NON reseed�
- Digital: Phase r�initialis�e + noise generator reseed�
- Impact sonore: Analog = plus de variation naturelle, Digital = plus constant

**Snare 606 (snare606.rs)** :
- Analog: Comportement similaire au snare mais avec envelope diff�rente
- Digital: R�initialisation compl�te comme le 606 original
- Impact sonore: Analog = plus organique, Digital = plus m�canique

**Tom (tom.rs)** :
- Analog: Phase pr�serv�e pour un son plus naturel
- Digital: R�initialisation pour un son plus synth�tique
- Impact sonore: Analog = comme des toms acoustiques, Digital = comme des toms �lectroniques

**Instruments SANS mode Analog/Digital** (toujours "analog") :
- Clap: Toujours analog (0.3) - n�cessite la continuit� pour le son r�aliste
- HiHat: Toujours analog (1.0) - les retriggers doivent �tre fluides
- OpenHiHat: Toujours analog (1.0) - m�me raison que HiHat
- Ride: Toujours analog (1.0) - n�cessite un decay naturel
- Cymbal: Toujours analog (1.0) - le shimmer n�cessite la continuit�
- Perc1: Valeur interm�diaire (0.3) - comportement hybride
- Zap: Valeur basse (0.0) - mais toujours trait� comme analog

**Valeurs par d�faut et plage typique** :
- Analog pur: 1.0 (Kick, Snare, Tom, HiHat, etc.)
- Digital pur: 0.0 (utilis� pour les sons �lectroniques pr�cis)
- Hybride: 0.3-0.7 (pour un m�lange des deux caract�res)

**Impact CPU par mode** :
- Analog: L�g�rement plus �lev� (calculs de phase pr�serv�e)
- Digital: L�g�rement plus bas (r�initialisations simples)
- Diff�rence: <2% sur un Core i7 (mesur� avec `test_high_cpu_load_patterns`)

**Quand utiliser chaque mode** :
- Analog: Sons organiques, patterns denses (>120 BPM), caract�re vintage
  Ex: House, Disco, Funk, Drum & Bass
- Digital: Sons propres, patterns clairsem�s (<110 BPM), caract�re moderne
  Ex: Techno, Minimal, Electro, Trance
- Hybride (0.3-0.7): Pour un m�lange des deux caract�res
  Ex: Progressive House, Melodic Techno

**Guide pratique par instrument** :

**Kick** :
- Analog (1.0): Id�al pour House/Disco - retriggers ajoutent du punch
- Digital (0.0): Parfait pour Techno - chaque hit identique
- Test: Essayez un pattern 16e notes � 125 BPM avec release=300ms

**Snare** :
- Analog (1.0): Son r�aliste comme une vraie caisse claire
- Digital (0.0): Son �lectronique pr�cis pour l'EDM
- Astuce: En mode analog, activez le noise pour plus de r�alisme

**Tom** :
- Analog (1.0): Sons comme des toms acoustiques
- Digital (0.0): Sons synth�tiques style 808
- Conseil: Utilisez analog pour les fills, digital pour les riffs

**HiHat/OpenHiHat** :
- Toujours analog (1.0) - ne peut pas �tre chang�
- Pourquoi: Les retriggers rapides n�cessitent une continuit� parfaite
- Astuce: Utilisez le param�tre "Tight" pour ajuster le caract�re

**Clap** :
- Toujours analog (0.3) - valeur fixe
- Pourquoi: Le son r�aliste n�cessite la continuit� des oscillateurs
- Alternative: Utilisez le Snare en mode digital pour un clap �lectronique

**Ride/Cymbal** :
- Toujours analog (1.0) - pour le shimmer naturel
- Astuce: Ajustez le param�tre "Shimmer" pour plus/moins de brillance

**Perc1** :
- Valeur interm�diaire (0.3) - comportement hybride
- Utilisation: Pour des sons de percussion interm�diaires
- Exp�rimentation: Essayez entre 0.1 et 0.5 pour diff�rents caract�res

**Zap** :
- Valeur basse (0.0) mais trait� comme analog
- Comportement: Son �lectronique avec une touche organique
- Utilisation: Pour des effets sp�ciaux et transitions

**Recettes par style musical** :

**1. Classic House (� la Kerri Chandler)** :
- Kick: 0.9 (l�g�rement digital pour la pr�cision)
- Snare: 1.0 (full analog pour le groove)
- HiHat: 1.0 (toujours analog)
- Tom: 0.8 (presque analog)
- Clap: 0.3 (d�faut)
- Groove: Swing16 � 55%

**2. Detroit Techno (� la Jeff Mills)** :
- Kick: 0.2 (tr�s digital pour la pr�cision)
- Snare: 0.3 (l�g�rement analog pour le corps)
- HiHat: 1.0 (toujours analog)
- Tom: 0.4 (mi-chemin)
- Clap: 0.3 (d�faut)
- Groove: Straight (pas de swing)

**3. Drum & Bass (� la LTJ Bukem)** :
- Kick: 0.7 (analog pour les retriggers rapides)
- Snare: 0.8 (presque analog pour le groove)
- HiHat: 1.0 (toujours analog)
- Tom: 0.9 (presque analog)
- Clap: 0.3 (d�faut)
- Groove: Shuffle � 40%

**4. Minimal Techno (� la Richie Hawtin)** :
- Kick: 0.1 (tr�s digital)
- Snare: 0.2 (tr�s digital)
- HiHat: 1.0 (toujours analog)
- Tom: 0.3 (digital)
- Clap: 0.3 (d�faut)
- Groove: Straight (pas de swing)

**Conseils avanc�s** :

1. **Automatisation du param�tre analog** :
   - Automatisez le param�tre analog pendant un breakdown
   - Passez de digital (pr�cis) � analog (organique) pour un effet dramatique

2. **Per-instrument settings** :
   - Chaque instrument peut avoir sa propre valeur analog
   - Ex: Kick digital (0.2) + Snare analog (1.0) = combo puissant

3. **Pattern density** :
   - Patterns denses (>120 BPM, 16e notes) ? privil�giez analog
   - Patterns clairsem�s (<110 BPM, 8e notes) ? digital fonctionne bien

4. **Velocity interaction** :
   - En mode analog: la velocity affecte plus le timbre
   - En mode digital: la velocity affecte plus le volume

**D�pannage** :

Probl�me: "Mon kick sonne diff�rent � chaque hit"
- Solution: Passez en mode digital (0.0) pour une consistance parfaite

Probl�me: "Mon pattern dense sonne m�canique"
- Solution: Passez en mode analog (1.0) pour plus de groove

Probl�me: "Je veux un m�lange des deux"
- Solution: Essayez des valeurs entre 0.3 et 0.7

**Exemples de r�glages par style** :
- TR-808 style: Kick=1.0, Snare=1.0, Tom=1.0 (full analog)



## Plan d'action — Audit code review 2026-07-18

### [AUDIT-CR-1] P0 — Correctifs RT/stabilité
- [x] **[AUDIT-RT1]** Éliminer l'alloc heap dans `reinitialize_slot` (`synthesis/mod.rs:867`) — pré-allouer les 14 voix ou différer la création hors thread audio.
- [x] **[AUDIT-RT2]** `initialize()` : remplacer `lock().unwrap()` (`lib.rs:2347`) par `try_lock` + repli (pattern déjà utilisé dans `process()`).
- [x] **[AUDIT-RT3]** Zéro panic UI/audio : `expect("valid voice index")` (`ui.rs:7210`) → `let-else`, `unreachable!()` (`generator/mod.rs:70`) → fallback, `lock().unwrap()` constructeur (`lib.rs:1862`) — critique car `panic = "abort"` tue Studio One.

### [AUDIT-CR-2] P1 — Industrialisation
- [x] **[AUDIT-INF1]** CI minimale GitHub Actions (Windows) : `cargo check` + `cargo test` + `cargo build --release` + artefact bundle.
- [x] **[AUDIT-INF2]** Versioning : passer de `0.1.0` à `0.2.0` (V1 modulaire) et l'afficher dans l'UI à côté du build ID.

### [AUDIT-CR-3] P2 — Quick wins docs/code mort (< 30 min chacun)
- [x] **[AUDIT-QW1]** Supprimer `println!` debug du chemin audio (`lib.rs:2228`) + nettoyer les 17 warnings (`clear_plocks_request`, `led_toggle`, constantes theme non utilisées).
- [x] **[AUDIT-QW2]** Corriger docs : README (`13 voix/aux` → `14 slots`, `pattern-v1` → `pattern-v5`) ; infrastructure.md (`sound-settings-v2`, 175 tests) ; AGENTS.md (PoC dans `archive/`) ; supprimer `fix_roles.pdb` et le build ID obsolète de `test-verification.ps1`. (build 20260718-125431)

### [AUDIT-CR-4] P3 — Dette structurelle (à planifier)
- [x] **[BUG-PLOCK-STEP]** Corriger la désactivation involontaire d'une step active lors de la création/édition/clear de sound p-locks et sequencer p-locks ; rendu p-lock actif restauré en full orange/violet. (build 20260719-112838)
- [x] **[AUDIT-Q3]** Sérialisation JSON de la Pattern Bank hors verrou (`pattern_bank.rs:716`) : snapshot JSON atomique rafraîchi hors verrou, `map()` ne touche plus le `Mutex<PatternBank>`. (build 20260720-154200)
- [x] **[AUDIT-Q4]** Unifier les 4 implémentations de sliders (LocalParamSlider / editor / mini / header) : module `src/ui/slider.rs` (logique + track core partagés), wrappers editor/mini, LocalParamSlider sur helpers communs. (build 20260721-142452)
- [x] **[SKIN-1]** Système de skins : `Theme` runtime dans `theme.rs` (const → accessors), migration des couleurs hardcodées vers des tokens, skins intégrés (Dark/Midnight/Ember), sélecteur dans Settings, persistance `GlobalConfig`. (build 20260721-150308)
- [x] **[AUDIT-Q5]** Valider le chemin de `DRUM_FLASH_MIDI_DRAG_HELPER` : nom exact + prefix bundle canonisé, 5 tests Windows. (build 20260721-152237)
- [x] **[AUDIT-Q6]** Éclater `ui.rs` (~8 060 → 366 lignes) en 13 modules thématiques (`editor_state`, `menus`, `fmt`, `controls`, `midi`, `header`, `pattern_bank`, `bottom_panel`, `song`, `popups`, `sound_editor`, `grid`, `plock`). (build 20260721-170521)
- [ ] **[BUG-LANE-DESYNC]** Décalage de tête de lecture entre lanes au changement Song/Pattern + changement de pattern — en attente de l'isolation du déclencheur par l'utilisateur.
- [ ] [94] Ajouter un paramètre pitch LFO sur les Toms (P2, synthèse).
- TR-909 style: Kick=0.8, Snare=0.7, Tom=0.9 (l�g�rement digital)
- Modern Techno: Kick=0.2, Snare=0.3, Tom=0.4 (plus digital)
- Acoustic simulation: Tous � 1.0 avec long decay




