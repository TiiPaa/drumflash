# Changelog

## 2026-08-21 — [183] Filter LFO SDrex : modulation vers le haut depuis la base (build 20260821-091344)

**Branche:** `main` · **Build:** `20260821-091344`
**Validation:** `cargo test` 310+1+199 OK, `build.ps1 -Install` OK. **À valider dans Studio One.**

- **Symptôme** : en mode Filter LFO, avec Filter au minimum (20 Hz) et Depth à fond, on n'entendait plus rien — l'inverse de ce qu'une modulation à pleine profondeur devrait donner.
- **Cause** : le cutoff était modulé de façon **bipolaire et multiplicative autour** de la base (`filter × 2^(sin × depth × wet)`). À 20 Hz de base et 3 octaves, le balayage allait de 2,5 Hz à 160 Hz : **la moitié basse de chaque cycle était écrasée par le clamp à 20 Hz**, et le sommet de l'autre moitié (160 Hz) restait sous le corps de la voix (185 Hz), loin du metal (620/910 Hz). Mesuré : **−15,8 dB** sous la voix filtre ouvert, contre −35,4 dB filtre fermé sans modulation. La profondeur étant exprimée en octaves *relatives à la base*, aucun réglage de Depth ne pouvait ouvrir assez depuis une base basse.
- **Correctif** : le LFO ouvre le filtre **vers le haut depuis la base** (LFO unipolaire) — « Filter » devient le **plancher** du balayage au lieu d'en être le centre, donc plus aucun demi-cycle perdu contre le clamp. Passer unipolaire ne suffisait pas (−15,1 dB), donc l'échelle est élargie : `FILTER_MOD_OCTAVE_SCALE = 2.0`, soit **6 octaves** à Depth × Wet au maximum (20 Hz → 1280 Hz au bas de la plage Filter). Mesuré après : **−4,8 dB** à base 20 Hz, −2,2 dB à 100 Hz, −0,7 dB à 500 Hz. À 9 ou 12 octaves l'effet s'aplatit (filtre quasi ouvert en permanence), d'où le choix de 6.
- **Conséquence assumée** : un LFO qui n'ouvre que vers le haut rend l'ensemble plus brillant — les réglages SDrex existants en Filter LFO sonneront plus ouverts. Le mode Flanger est inchangé (il lit le même Depth comme des millisecondes de délai).
- Test `filter_lfo_at_the_lowest_base_stays_audible_at_full_depth` : vérifie les trois propriétés — un passe-bas à 20 Hz seul mute bien la voix (< −25 dB), Depth à fond la ramène à moins de 6 dB du filtre ouvert, et le LFO continue de façonner le son (> 1 dB sous l'ouvert, donc pas un bypass).

## 2026-08-20 — [182] Unités affichées sur les paramètres spéciaux et dans le menu plock (build 20260820-184818)

**Branche:** `main` · **Build:** `20260820-184818`
**Validation:** `cargo test` 309+1+198 OK, `build.ps1 -Install` OK. **À valider dans Studio One.**

- **Cause du manque d'unités** : les deux catégories de paramètres sont deux structures distinctes. `ParamWidget::Slider` (paramètres standard) porte un `suffix`, mais **`SpecialParamDef` n'avait aucun champ d'unité** — le Sound Panel passait `None` en dur pour tous les spéciaux, donc aucun ne pouvait afficher son unité, même quand il s'agissait de Hz, de secondes ou de ms.
- **`SpecialParamDef` gagne `unit: Option<&'static str>`** + un helper `sp_unit(...)`. `sp()` et `sp_discrete()` restent pour les grandeurs sans dimension (depth, wet, mix, amount…), donc seules les 12 lignes concernées changent : **Hz** — `Gate Rate` (Buzz), `Rate` (LFO SDrex), `Click Tone` (BD808), `Shimmer Freq` (Cymbal) ; **s** — `Filter Attack` et `Filter Hold` (Buzz et SDrex) ; **ms** — `Fade-in` (SDrex) ; **ct** (cents) — `Pitch Fine` des trois samplers 606.
- **Le menu plock affiche désormais les mêmes unités que le Sound Panel** — il formatait tout en `{:.2}` nu, y compris pour les paramètres standard qui ont une unité (« 0.50 » dans le menu contre « 0.50 s » dans le panneau). Les deux formateurs de `ui/fmt.rs` prennent l'unité ; le nombre reste produit par les mêmes règles d'arrondi qu'avant (fonctions internes `format_plock_number` / `format_plock_special_number`).
- Test `physical_special_params_declare_their_unit` : snapshot exact des 12 paramètres portant une unité (pour qu'une grandeur sans dimension n'en gagne pas une par erreur) **et** règle par mot-clé (`_rate`, `_freq`, `_attack`, `_hold`, `_fade`, `_fine_tune`) pour que le prochain paramètre de fréquence ou de durée ne puisse pas l'oublier. `snare606_tone` (mélange 0..1) et les `_atk_curve` ne matchent volontairement pas.

## 2026-08-20 — [181] Fine-tune des sliders, Modulation SDrex explicite, Fade-in, plages d'enveloppes (build 20260820-172925)

**Branche:** `main` · **Build:** `20260820-172925`
**Validation:** `cargo test` 308+1+197 OK, `build.ps1 -Install` OK. **À valider dans Studio One.**

- **Fine-tune des sliders réparé** — la modulation fine avait disparu lors de l'unification des sliders : `slider::draw_track` ne faisait que du positionnement absolu (saut à la position du curseur). **Shift ou Alt + glisser** fait maintenant un déplacement *relatif* à la valeur courante, ~4× plus fin (0,0015 unité normalisée par pixel, la même sensibilité que le slider du menu plock). Un simple Shift/Alt+clic ne saute plus. Détection du modificateur via `controls::fine_tune_modifier_pressed`, qui double le test egui d'une lecture clavier plateforme (`GetAsyncKeyState`) — les hôtes qui interceptent le clavier (Studio One, REAPER) empêchaient egui de voir le modificateur, ce qui est très probablement la cause de la panne. Le slider du menu plock utilise désormais la même détection. Maths du drag fin isolée dans `apply_fine_drag` + 4 tests unitaires (relatif, clamp aux bornes, mapping log, respect du pas de quantification).
- **SDrex : le switch « Filter Mod » devient un vrai choix « Flanger / Filter LFO »** — un interrupteur on/off ne disait pas entre quoi il choisissait. Nouveau helper `segmented_row` (label + sélecteur segmenté aligné à droite), le libellé du paramètre passe à « Modulation ».
- **SDrex : le Delay de modulation devient un Fade-in** — le slider « Delay » (délai minimum du flanger, 0–3 ms) est remplacé par **« Fade-in » (0–300 ms, défaut 0)** : le Wet de la modulation monte progressivement après chaque coup, donc le flanger ou le LFO de filtre **s'installe** au lieu d'être présent dès l'attaque. Actif dans **les deux modes**, donc plus grisé en Filter LFO (seul Feedback reste spécifique au flanger). Le délai minimum du flanger devient la constante `FLANGER_MIN_DELAY_MS = 0.7` — la valeur du défaut précédent, le caractère du flanger est inchangé. Les sessions existantes stockaient 0,7 sur ce champ → interprété en ms, soit un fade-in imperceptible : pas de rupture.
- **Plages d'enveloppes resserrées** là où la course du slider ne servait à rien : **Kick** et **BD808** decay 5 s → **2 s**, **Clap** decay 5 s → **1,5 s** (nouveau jeu `CLAP_STD` : le Cymbal, qui partageait `NO_FREQ_STD`, garde ses 5 s), **SDrex** Hold et Filter Hold 2 s → **1 s** (clamps DSP alignés).
- Tests : 4 nouveaux pour le drag fin, 1 pour le fade-in dans les deux modes, 1 pour les plafonds de decay (avec garde sur le Cymbal) ; les 3 tests SDrex existants mis à jour (le Fade-in n'est plus ignoré en Filter LFO, holds à 1 s, composition de la famille Modulation).

## 2026-08-20 — [179] Attaque identique quel que soit l'écart entre deux cellules (Kick + BD808) (build 20260820-162036)

**Branche:** `main` · **Build:** `20260820-162036`
**Validation:** `cargo test` 302+1+195 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-08-20).**

- **Le problème, mesuré** : la queue du coup précédent contaminait l'attaque du suivant. En mode digital (donc sans aucun drift aléatoire) et à réglages strictement identiques, seul l'écart entre deux cellules variant : le **Kick** montrait **3,71 dB** de dispersion de pic, une polarité de première demi-période qui s'inversait et un temps de crête errant de 1,5 à 8,3 ms ; la **BD808** passait d'un temps de crête de 12,7 ms (coup isolé) à 1,5 ms (coup rapproché), et en mode analog — le défaut — même ses coups isolés variaient de **2,6 dB** avec une discontinuité de **0,244** (le drift de niveau appliqué d'un coup sur la queue encore sonnante).
- **Quatre causes** : phase des oscillateurs jamais remise à zéro au retrigger (choix anti-clic historique) ; deux chemins de code distincts (cold start vs retrigger sur queue, bascule au seuil d'extinction de l'enveloppe) ; toutes les enveloppes repartant de leur valeur courante (ampli, pitch, filtre, snap/drop du 808) ; smoothers de fréquence et de cutoff non réinitialisés.
- **Fix — nouveau primitif `dsp::RetrigDeclick`** : chaque coup repart d'un **état neuf identique** (phase 0, filtre, smoothers et DC blocker vidés, enveloppes redémarrées depuis zéro), et un **fondu raised-cosine de 3 ms** du dernier échantillon émis garde la *sortie* continue pendant ce reset. Le reset de phase seul cliquait (step 0,35) : c'est le fondu qui le rend propre.
- **Résultat mesuré** : dispersion de pic **0,00 dB** sur 7 espacements (500 → 15 ms), temps de crête constant, polarité constante, step max au retrigger **0,014** (Kick) et **0,044** (BD808) — soit **4× plus propre** que le retrigger à phase continue qu'il remplace (0,058). Le clic du drift analog de la BD808 disparaît (plus de queue à re-scaler).
- La distinction analog/digital vit désormais **entièrement dans le drift par coup** (pitch/niveau/durée de queue) ; le sweep de pitch est déterministe dans les deux modes. Les stutters (`trigger_hard`) suivent le même contrat.
- **Contrat verrouillé par des tests** : nouveau module `src/synthesis/retrig_tests.rs` (6 tests) — dispersion ≤ 0,3 dB, temps de crête ≤ 0,5 ms d'écart, polarité constante, absence de discontinuité calibrée sur le même coup joué isolément, et coups isolés bit-identiques en digital. Le test de plock existant a été reformulé sur la même base auto-calibrée (un seuil absolu mesurait en réalité la raideur d'attaque légitime).
- **Portée : Kick et BD808 uniquement.** Les autres voix tonales (Tom, Perc1, Snare, Snare606, SDrex, Buzz) et les samplers 606 restent sur l'ancien comportement → tâche [180].

## 2026-08-20 — [178] Graphes d'enveloppe unifiés et factorisés (build 20260820-083705)

**Branche:** `main` · **Build:** `20260820-083705`
**Validation:** `cargo test` 297+1+189 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-08-20).**

- **Tous les graphes du Sound Panel sont construits de la même manière** : socle commun `prep_graph` (cadre LCD encastré + padding unifié 12/10 + hauteur 104, gate Buzz 72) et grille de quarts partagée, utilisés par les 6 graphes (`envelope_viz.rs`).
- **Couleurs de stages partout** : attaque = ambre, hold = vert, decay = bleu (helpers `stage_attack/hold/decay`). Le graphe filtre A-H-D (Buzz/SDrex) et le graphe ampli des samplers 606 colorent désormais leurs segments comme le graphe ampli des synthés ; les courbes filtre mono-stage (Toms, samplers) passent de l'orange au bleu decay. Trait de courbe unifié à 2 px ; ligne de cutoff factorisée (`draw_cutoff_line`).
- Token de thème `envelope_curve` (orange) devenu inutile → supprimé des 3 skins.

## 2026-08-20 — Fix solo/mute invisibles après les clears (build 20260820-082201)

**Branche:** `main` · **Build:** `20260820-082201`
**Validation:** `cargo test` 297+1+189 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-08-20).**

- **Bug** : un solo (ou mute) enclenché sur une lane survivait aux clears ; une fois la lane désactivée, le solo devenait invisible et continuait de muter tout le kit → silence total sans cause apparente.
- Nouveau helper partagé `controls::clear_all_mutes_solos(setter, params)` (14 mutes + 14 solos à off via `ParamSetter`).
- Appelé dans tous les chemins de clear destructifs : **Clear All** du header, **presets de layout** du navigateur de presets (Clear All / 4 Lanes / 12 Lanes + kits pattern/grid), **presets de style** du panneau Generate (House/Dub/DnB/Bossa/Afro/Break), et **Delete Lane** (reset du mute/solo du slot désactivé).
- Non touchés (les lanes restent visibles) : Generate, chips Rock/Funk/Disco/Random, Clear Lane, Clear All du Song.

## 2026-08-19 — SDrex : Flanger / Filter Mod + Decays 1,5 s (build 20260819-202022)

**Branche:** `main` · **Build:** `20260819-202022`
**Validation:** `cargo test` 297+1+189 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-08-20).**

- **Decay maxima ajustés** : Amp Decay et Filter Decay passent de 2 s à **1,5 s** ; les Holds restent à 2 s.
- **Section Flanger → Modulation** : ajout du switch `Filter Mod`. OFF = flanger classique ; ON = le même LFO module le cutoff du filtre LP et le délai flanger est bypassé.
- **Mapping Filter Mod** : `Rate` = fréquence du LFO, `Depth` = excursion bipolaire jusqu’à ±3 octaves, `Wet` = intensité de cette excursion. `Delay` et `Feedback` sont grisés dans l’UI et strictement ignorés par le DSP.
- **Changement de cible sûr** : le buffer de délai est vidé lors du passage Flanger ↔ Filter Mod pour empêcher la réapparition d’un ancien feedback. `Free Phase` contrôle le même LFO dans les deux modes.
- Tests dédiés : routage Depth/Wet vers le cutoff, indépendance bit-identique vis-à-vis de Delay/Feedback en Filter Mod, différence de rendu entre les deux cibles, nettoyage du buffer et roundtrip du switch.

## 2026-08-19 — SDrex : Holds, Decays 2 s, Free Phase flanger (build 20260819-165918)

**Branche:** `main` · **Build:** `20260819-165918`
**Validation:** `cargo test` 295+1+187 OK, `build.ps1 -Install` OK. **À valider dans Studio One.**

- **Enveloppe volume A-H-D** : ajout de `Hold` (0-2 s) entre Attack et Decay ; `Decay` monte désormais jusqu’à **2 s**. Le hold retarde les trois décroissances body/noise/metal sans figer le pitch drop.
- **Enveloppe filtre A-H-D** : ajout de `Filter Hold` (0-2 s) ; `Filter Decay` monte désormais jusqu’à **2 s**. DSP et graphe utilisent Attack → Hold → Decay avec les courbes bipolaires existantes.
- **Free Phase corrigé** : le switch quitte Oscillator et rejoint la section **Flanger**. OFF remet la phase du LFO flanger à zéro à chaque trigger ; ON conserve sa phase courante. Il n’agit plus sur les oscillateurs body/metal, qui retrouvent leur reset normal au cold start.
- Tests dédiés : plages UI à 2 s, Holds audibles, roundtrip des nouveaux champs, reset/conservation directe de `flanger_phase`, absence d’effet de Free Phase sur `body_phase`.

## 2026-08-19 — SDrex : section Flanger + enveloppe volume A-D (build 20260819-164808)

**Branche:** `main` · **Build:** `20260819-164808`
**Validation:** `cargo test` 292+1+184 OK, `build.ps1 -Install` OK. **À valider dans Studio One.**

- **Filter et Flanger séparés** : nouvelle famille data-driven `ParamFamily::Flanger`; Rate, Delay, Depth, Feedback et Wet apparaissent désormais dans une section **Flanger** autonome. Les paramètres cutoff et enveloppe LP restent seuls dans **Filter**.
- **Enveloppe volume SDrex enrichie** : ajout de `Attack`, `Attack Curve` et `Decay Curve` dans la section **Envelope**, avec graphe A-D. Attack applique une rampe commune aux couches body/noise/metal ; Attack Curve façonne cette rampe et Decay Curve façonne les trois décroissances caractéristiques sans ajouter une seconde enveloppe qui raccourcirait la recette.
- **Défaut neutre préservant le son** : Decay Curve SDrex passe à `0.0` (linéaire/neutre dans `shape_curve`), Attack reste à 0,5 ms et Attack Curve à `0.0`. Le reset par double-clic utilise les défauts SDrex du registry.
- Tests dédiés : séparation exacte des 5 paramètres Flanger, effet audible Attack/Attack Curve/Decay Curve, plus toute la suite de stabilité SDrex.

## 2026-08-19 — Correctif stabilité Clear All / SDrex (build 20260819-163329)

**Branche:** `main` · **Build:** `20260819-163329`
**Validation:** `cargo test` 290+1+182 OK, `build.ps1 -Install` OK. **À valider dans Studio One.**

- **Risque de crash natif supprimé dans la persistence Pattern Bank / Clear All** : l'ancien snapshot JSON publié par `AtomicPtr<Vec<u8>>` libérait immédiatement l'ancien buffer pendant qu'un autre thread pouvait encore le lire. Remplacé par un snapshot partagé protégé par `RwLock`, avec test de lecture/rafraîchissement concurrents.
- **Thread audio assaini** : une sauvegarde de pattern ne clone/sérialise plus toute la banque dans `process()` ; elle pose uniquement un drapeau atomique, consommé au prochain accès de persistence hors callback audio.
- **SDrex temps réel sécurisé** : buffer du flanger `Vec` remplacé par un tableau fixe (aucune allocation lors d'un changement de kind à chaud), longueur active bornée jusqu'à 192 kHz avec fallback sûr au-delà, délai interpolé clampé, phases oscillateurs bornées à `2π`. Test extrême fini à 8/44,1/192/384 kHz.
- **Preset Song** : suppression d'un auto-deadlock (`refresh_snapshot()` était appelé alors que le mutex Pattern Bank était encore détenu).
- **P-lock/morph** : `special[4]` ne peut plus écraser le champ réservé `Attack` (18). Pour SDrex, **Flanger Wet reste éditable globalement mais n'est pas proposé en p-lock/morph** tant que le format persistant n'a pas un champ distinct.

## 2026-08-19 — [175] Nouvel instrument SDrex + fix Algo Perc1 + légendes/grilles graphes (build 20260819-145510)

**Branche:** `main` · **Build:** `20260819-145510`
**Validation:** `cargo test` 284+1+178 OK, `build.ps1 -Install` OK. **À valider dans Studio One.**

- **[175] SDrex** (kind 15 / voice 17, catégorie SD) : recette « drex_snare » de l'utilisateur portée en voix temps réel — corps sine (pitch drop +95 Hz → base, env rate 32), noise HP par soustraction LP (env 18), metal ring-mod 620×910 Hz (env 25), mix 0.50/0.80/0.18, **flanger** (Rate 0.1-20 Hz / Delay 0-3 ms / Depth 0-3 ms / Fdbk 0-0.9 / Wet 0-1 — les 5 params demandés, en special params famille Filter), drive tanh ×2.2×0.8 fixe dans la chaîne de saturation standard. `Frequency` = base du corps (déplace aussi la paire metal), `Decay` scale les 3 enveloppes, `Analog` = drift par coup. Note MIDI 48, rôle Snare au GENERATE, mono. Enveloppes en formules temporelles → `set_settings` sans recréation (anti-click natif). Tests : son/fini/silence, wet flanger audible, decay étire la queue, roundtrip settings.
- **Fix Perc1 Algorithm** : `set_algo()` ne recréait pas les oscillateurs et faisait échouer la détection de changement dans `set_settings` → Perc1 jouait toujours Sine. `set_algo` reconstruit maintenant les 4 oscs sur changement réel. Test de régression (`perc1_algo_changes_the_output`, chemin moteur bit-identique à une voix Saw fraîche).
- **Graphes** : légende A/H/D supprimée (graphe ampli pleine hauteur) ; barres verticales de quart **sur tous les graphes** via `draw_grid_lines` partagé, atténuées (`white_a(9)`).

## 2026-08-19 — Filtre Tom : câblage corrigé + sweep exponentiel 20k (build 20260819-114620)

**Branche:** `main` · **Build:** `20260819-114620`
**Validation:** `cargo test` 278+1+172 OK. **À valider dans Studio One.**

Diagnostic utilisateur (« le filtre du Tom ne fonctionne pas ») — 5 corrections en chaîne sur `tom.rs` :
- **Bug pitch** : `pitch_env.next()` appelé 2× par sample (top + branches algo) → sweep de pitch à **double vitesse**. Corrigé (1 appel, réutilisé).
- **Anti-click** : `set_settings()` recréait `pitch_env` à chaque paramètre → sweep redémarré au drag. Nouveau `PitchEnvelope::set_sweep_time` (mutation sans reset).
- **Stick attack contournait le filtre** (ajouté après) → routé à travers le même cutoff via un filtre dédié (`stick_filter`).
- **Plancher de cutoff 100 Hz** (50 en Deep) supprimé → 20 Hz (un Filter à 20 Hz donnait 100 Hz réels — le bas du slider était inopérant).
- **Filtre 1 pôle (6 dB/oct) → biquad 12 dB/oct** (RBJ Butterworth, Q=0.707, sans résonance) pour un filtrage radical en bas de course.
- **Loi du sweep changée** : `cutoff × (1 + env×amount×4)` (Filter 20 Hz + Env max = sweep 100→20 Hz, invisible) → **sweep exponentiel vers 20 kHz** `cutoff × (20000/cutoff)^(env×amount)` (même loi que Buzz) — DSP + graphe alignés.
- **Graphe filtre refait** : affiche le **vrai sweep du cutoff** sur axe log Hz (ligne de repos = Filter, courbe = balayage, fenêtre fixe 1 s) ; Filter Env à 0 = ligne plate honnête.

## 2026-08-18 — [174] Fixes graphes/DSP env filtre + BUG plocks sound perdus au chargement de pattern (build 20260818-182234)

**Branche:** `main` · **Build:** `20260818-182234`
**Validation:** `cargo test` 278+1+172 OK. **À valider dans Studio One.**

- **BUG (P1) — plocks sound perdus au chargement d'un pattern** (bank slots ET presets) : `restore_from_buffers` écrivait le field mask via `field_masks.set(inst, step, mask as usize)` — or `set()` attend un **index de champ** (`1 << field`), pas un masque → le masque était corrompu à chaque restore (snapshot `(1<<46)-1` → no-op → masque vide → le plock devenait un link sans champ = muet). Remplacé par `set_raw`. La persistence projet (`plock-v1`) utilisait déjà `set_raw` — seul le chargement de patterns était cassé. Test de régression `pattern_preset_roundtrip_preserves_sound_plock`.
- **[174/F1] Toms** : `draw_filter_envelope` normalise la courbe sur toute la largeur (avant : plancher 100 ms sur l'axe X → courbe écrasée à gauche quand Filter Decay < 100 ms).
- **[174/F2] smp (BD6/SD6/CH6) + Perc1** : courbe de l'enveloppe de filtre = **constante dédiée** `FILTER_ENV_CURVE = 6.0` par voix (comme Kick/Snare/Tom). Avant, ces voix lisaient `decay_curve` — devenu **bipolaire (−1..1)** en [159] — comme raideur exp (2..12) → sweep de filtre quasi plat (vieilles sessions clampées à +1). Régression [159] corrigée côté **DSP et graphe** (`filter_env_curve()` étendu). ⚠️ Effet audible : le sweep de filtre de ces 4 voix redevient punchy.
- **[174/F3] Graphe ampli smp** : `draw_sample_amp_graph` réécrit en **A-H-D bipolaire** fidèle au DSP (`shape_curve` attack + decay ; proportion attack/decay approximative, shapes exactes) ; l'attack curve (`release_curve` repurposée) est passée au graphe.

## 2026-08-16 — [167] densité Randomize Lane + [170] curves renforcées + [168] stéréo 2 samples smp (build 20260816-185337)

**Branche:** `main` · **Build:** `20260816-185337` (retour utilisateur intégré : paires + compatible Analog)
**Validation:** `cargo test` 277+1+172 OK, `build.ps1 -Install` OK. **À valider dans Studio One.**

- **[167] Densité réglable pour Randomize Lane** : slider « Density » (5-100 %, défaut 30 %) dans le menu clic droit du nom de lane, au-dessus de « Randomize Lane ». Persisté dans l'état éditeur (`randomize_density`, fallback 30 % si 0/legacy).
- **[170] Courbes bipolaires renforcées** (tous les instruments) : exposant `1+3|c|` → `1+5|c|` dans `dsp::shape_curve` (enveloppes d'ampli A-H-D), `buzz::shape_curve` (enveloppe de filtre Buzz) et le graphe `envelope_viz`. Les réglages de courbe existants sonnent plus extrêmes aux bords (voulu).
- **[168] Mode stéréo 2 samples (voix multisamplées)** : switch **Stereo** placé **directement sous le sélecteur Sample** (famille Osc) avec infobulle EN expliquant la relation. Quand Stereo est ON, le sélecteur affiche des **paires** (« 1+2 », « 3+4 », « 5+6 », « 7+8 ») : L = 1er sample de la paire, R = 2e. **Compatible avec l'Analog Mode** : la paire est alors tirée au hasard à chaque coup. DSP : enveloppes partagées (1 avancée/sample), filtre et DC blocker indépendants par canal ; dual mono quand OFF. Tests ×3 voix (paire distincte / dual mono / analog+stereo) ; tests registry stereo/mono mis à jour (13|14|15 → stereo-capable).

## 2026-08-16 — [171] MIDI Pat sans retrig + [172] temps forts éclaircis + [169] Clap plus fort (build 20260816-151800)

**Branche:** `main` · **Build:** `20260816-151800`
**Validation:** `cargo test` 273+1+168 OK, `build.ps1 -Install` OK. **À valider dans Studio One.**

- **[171] MIDI Pat : plus de retrig au changement de pattern.** Le switch par note MIDI (60-75) ne lève plus `pending_song_pattern_restart` : le nouveau pattern **reprend à la volée** (position conservée). Si la longueur change, le resync host existant (`sync_to_host`, `rem_euclid` sur la nouvelle longueur) ramène la lecture dans le pattern — une page courante qui n'existe plus retombe dans le pattern (page 1 pour un pattern 1 page). Le mode Song conserve son restart (avancée par bloc).
- **[172] Temps forts 1/5/9/13 éclaircis** : voile `white_a(26)` sur les cellules OFF des temps forts dans `draw_step_cell_v2` (le sprite `pad-off-beat` seul était trop subtil).
- **[169] Clap plus fort** : volume par défaut 0.7 → 1.0 (`sound_settings_default` registry + `VoiceSettings::clap`). N'affecte que les nouvelles lanes / resets (les sessions existantes gardent leur valeur).

## 2026-08-15 — Sélecteur d'instrument : vrai flyout natif identique partout + rendu propre (build 20260815-181345)

**Branche:** `main` · **Build:** `20260815-181345`
**Validation:** `cargo check` warning-clean, `build.ps1 -Install` OK.

- **Refonte de l'unification** (la précédente était bâclée) : les trois points d'entrée ouvrent maintenant **exactement le même menu natif egui** (flyout cascade catégorie ▸ kind), donc rendu et comportement rigoureusement identiques.
  - **Cause des bugs précédents** : `ui.menu_button` brut posé dans un `Area` popup custom (lane vide) n'a pas de racine de menu → les sous-menus (catégories) ne s'ouvraient pas ; et dans l'onglet Track il produisait un petit bouton étroit disgracieux.
  - **Correctif** : passage à `egui::menu::menu_custom_button` (déclencheur custom + **vraie racine de menu native** → sous-menus fonctionnels) pour la lane vide et l'onglet Track. Le clic-droit reste natif. Le flyout est donc le même partout (celui, validé, du clic-droit).
  - **Lane vide** : le « +N » devient un bouton stylé (fill/bordure/coins arrondis) qui ouvre la cascade ; l'ancien popup custom `AddModulePopup` est **supprimé** (struct + champ d'état + `draw_add_module_popup_if_any` + remaps).
  - **Onglet Track « Type »** : champ 146×CTL_HEIGHT (même gabarit que Aux Out / Choke, fill P_ACTIVE + bordure LINE2 + police mono) ouvrant la cascade — fini le carré étroit.
  - Helper `menus::instrument_category_menu` : ferme désormais le menu à la sélection (les lignes custom ne déclenchent pas la fermeture auto d'egui).
- Nettoyage : `draw_empty_lane_name_v2` retiré, imports orphelins supprimés.

## 2026-08-15 — Sélecteur d'instrument unifié partout (cascade par catégorie) (build 20260815-171606)

**Branche:** `main` · **Build:** `20260815-171606`
**Validation:** `cargo check`/build warning-clean, `build.ps1 -Install` OK.

- **Un seul sélecteur d'instrument, identique aux trois endroits** : lane vide (popup « Add Module »), clic-droit sur le nom de lane, et champ « Type » de l'onglet Track. Tous ouvrent désormais la **même cascade par catégorie** (BD/SD/HH/PERC/FX/OTHER ▸ kind), avec le kind courant préfixé « > » et surligné.
- Helper partagé unique `menus::instrument_category_menu(ui, current) -> Option<kind>` (un sous-menu par catégorie, lignes plates) — les trois appelants s'y branchent, plus de duplication.
  - Lane vide : le popup « Add Module » passe de la liste plate groupée à la cascade.
  - Onglet Track : le dropdown plat `styled_select` (15 kinds) est remplacé par un menu-bouton `<kind> ▾` ouvrant la cascade.
  - Clic-droit : inchangé fonctionnellement (déjà en cascade), factorisé sur le helper.
- Nettoyage des imports devenus inutilisés (`InstrumentCategory` dans grid.rs, `TrackInstrumentKind` dans popups.rs/sound_editor.rs).

## 2026-08-15 — Onglet Track : loader de presets d'instrument pour la lane sélectionnée (build 20260815-161113)

**Branche:** `main` · **Build:** `20260815-161113`
**Validation:** `cargo test` 273+1+168 OK, `build.ps1 -Install` (install manuelle — verrou AV transitoire sur le DLL, cf. note).

- **Nouvelle section « Preset » dans l'onglet Track** (sous *Instrument*, avant *Routing*) : un menu **« Load »** liste les presets d'instrument (factory + utilisateur) **du type de la lane sélectionnée** et les applique en un clic (standards + algo + specials) sur le slot courant.
  - Menu « action » : le bouton reste sur « Load preset… » ; après sélection il applique puis revient au placeholder. « No presets » si aucun preset pour ce type (zone stable).
  - Liste mise en cache par (slot, kind) → aucune I/O par frame ; rebâtie au changement de lane/type et après sauvegarde d'un preset d'instrument.
  - `presets::list_instrument_presets(kind)` / `load_instrument_preset(entry)` ; application via `preset_browser::apply_instrument_preset_to_slot` (factorisé avec `apply_instrument`, helper `write_slot_sound` partagé).
- **Note install** : l'écriture dans `C:\Program Files\...\VST3\` a échoué 2× sur un « accès refusé » alors que Studio One était fermé — verrou transitoire de l'antivirus (BitDefender) scannant le DLL fraîchement compilé. Le fichier était libre juste après ; bundle copié manuellement. À surveiller si ça se reproduit.

## 2026-08-15 — Preset de pattern : sauvegarde/restauration du son de chaque instrument (build 20260815-124040)

**Branche:** `main` · **Build:** `20260815-124040`
**Validation:** `cargo test` 273+1+168 OK (2 nouveaux tests presets), `build.ps1 -Install` OK.

- **Le preset de pattern ([150] « Patterns ») embarque désormais le son de chaque lane active** (13 standards + algo + specials), en plus de la grille / fusions / plocks / kit déjà capturés. Avant : charger un preset de pattern restaurait la grille et le kit (types d'instruments) mais **pas** les réglages sonores → les instruments sonnaient avec leurs valeurs courantes/par défaut.
- **Capture** : `capture_pattern` prend `sound_settings` + les algos par slot et sérialise un `PatternSlotSound { slot, kind, standards[13], algo, specials }` par lane active (réutilise l'extraction de `capture_instrument`).
- **Restauration** : `apply_pattern` réapplique chaque son **sur le slot qui porte encore le même kind** (garanti après pose du kit ; les lanes de kind différent sont ignorées quand on charge sans le kit, pour ne jamais écraser un autre instrument). Helper `write_slot_sound` factorisé et partagé avec `apply_instrument`.
- **Rétro-compat** : champ `sounds` en `#[serde(default)]` → les presets enregistrés avant ce changement se chargent avec une liste vide (grille seule, sons inchangés). Tests : capture des sons + chargement d'un preset legacy sans `sounds`.

## 2026-08-15 — Fix : les cellules fusionnées sont bien enregistrées dans un slot de pattern (build 20260815-121906)

**Branche:** `main` · **Build:** `20260815-121906`
**Validation:** `cargo test` 272+1+168 OK (nouveau test `pattern_slot_capture_restore_preserves_fusions`), `build.ps1 -Install` OK.

- **Bug** : sauvegarder le pattern courant dans un slot (P1–P16) **perdait les cellules fusionnées** au rechargement du slot.
- **Cause** : dans `deserialize_fusions` (`pattern_bank.rs`), le gate `expected_new` calculait la taille attendue comme `INSTRUMENT_COUNT × MAX_FUSIONS × FUSION_SLOT_COUNT × 8`, alors que `PatternSlot::capture` sérialise en réalité, par lane, **un compteur u64 nu (8 o) + (MAX_FUSIONS−1) groupes de FUSION_SLOT_COUNT u64** (16 o de moins par lane). Le vrai blob (5152 o) étant plus court que le gate (5376 o), il était **rejeté puis relu en format legacy** → toutes les fusions tombaient.
- **Fix** : gate corrigé à la taille réelle `INSTRUMENT_COUNT × (8 + (MAX_FUSIONS−1) × FUSION_SLOT_COUNT × 8)`. S'applique aux deux chemins de chargement (`PatternSlot::restore` et `restore_from_buffers`). Test de non-régression ajouté.
- N'affectait pas la persistance projet (`pattern-v5`, autre chemin) — d'où des fusions conservées à la sauvegarde du projet mais perdues au save-slot.

## 2026-08-14 — [150] Gestion des presets (instruments / patterns / grid / songs) + outil factory (build 20260814-171844)

**Branche:** `main` · **Build:** `20260814-171844`
**Validation:** `cargo test` 271+1+168 OK, `build.ps1 -Install` OK. **À valider dans Studio One.**

- **Retours post-v1** : bouton **« Presets » déplacé dans le header**, entre « MIDI Pat » et « Settings », encadré de barres de séparation (vbar épaissies 1→2 px) ; le bouton de la barre Pattern Bank est retiré.
- **Type de preset « Grid »** (4e onglet du modal) : capture/charge le kit de lanes (kinds par slot, `.fdgrid.json`). Les 3 layouts d'usine de l'ancien dropdown **« Preset » de la page-bar — supprimé** — y vivent désormais : **Clear All** (2 clics, efface la grille) / **4 Lanes** / **12 Lanes**. `LanePresetAction`, le dropdown et le warning popup associés sont supprimés ; `apply_lane_layout_preset` est partagé avec le modal.

- **Modal « Presets »** (nouveau bouton dans la barre Pattern Bank, plaque skeuo centrée) avec 3 onglets :
  - **Instruments** : capture les 13 standards + specials + algo du slot sélectionné. Au load, si le preset vise un autre kind, la lane change de type d'abord (`change_slot_kind`), puis les valeurs s'appliquent.
  - **Patterns** : capture grille 64 steps + fusions + sound plocks + seq plocks (blobs hex, même layout que la Pattern Bank → tolérance legacy) + **kit de lanes**. Toggle **« Load lanes too »** : avec kit = installe les lanes du preset (`apply_lane_layout_preset`), sans kit = steps/plocks sur les lanes actuelles.
  - **Songs** : capture/publie la `SongSequence` (bank + snapshot + `song_controller`).
- **Fichiers JSON versionnés** (`version: 1`) sous `Documents/Flash Drum/presets/{instruments,patterns,songs}/` (extensions `.fdinst.json` / `.fdpat.json` / `.fdsong.json`). Listes Factory (read-only) + User (Load / Del en 2 clics).
- **Presets d'usine embarqués** : `factory_presets.rs` expose `INSTRUMENTS`/`PATTERNS`/`SONGS` en `include_str!` depuis `assets/presets/` (vides pour l'instant).
- **Outil d'authoring factory** : en build debug, bouton « Export factory (dev) » dans le modal → écrit le JSON dans `presets/_factory/<kind>/` ; workflow : copier dans `assets/presets/<kind>/`, ajouter la ligne `include_str!`, commit (documenté dans `factory_presets.rs`).
- Modules : `src/presets.rs` (types, capture, fs, hex) + `src/ui/preset_browser.rs`. L'ancien outil dev `preset_dumps.rs` reste en place (sound editor debug).
- Tests : roundtrips JSON/hex, sanitize, capture pattern (masks + kit + blobs décodables).

## 2026-08-13 — [163] Catégories d'instruments + type via clic droit sur la lane (build 20260814-090820)

**Branche:** `main` · **Build:** `20260814-090820`
**Validation:** `cargo test` 267+1+168 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-08-14).**

- **`InstrumentCategory` (BD, SD, HH, PERC, FX, OTHER)** sur `TrackInstrumentKind` (`track.rs`) : `category()`, `kinds_in(cat)`, `ALL`. Mapping : BD = Kick/808 Kick/BD6smp, SD = Snare/Snare 606/SD6smp/Clap, HH = HiHat/Open Hi-Hat/CH6smp, PERC = Tom/Perc1, FX = Buzz, OTHER = Ride/Cymbal.
- **Le menu clic-droit sur le nom de lane permet de changer le type d'instrument** : sous-menus **cascadés** « Instrument ▸ Catégorie ▸ kind » (2 niveaux), kind courant marqué « > » en bleu et non cliquable. `change_slot_kind()` applique la même sémantique que le dropdown Type de l'onglet Track : nom par défaut + note MIDI du kind + reset des réglages aux défauts (réinit audio via le watch `last_slot_kinds`). **Hover highlight** ajouté sur `context_menu_button` (tous les menus contextuels : fond `P_HOVER` + label blanc au survol, rangées désactivées sans highlight). Lignes d'instruments en version **plate** (`context_menu_row_plain` — pas de keycap 3D dans les sous-menus imbriqués, trop lourd visuellement).
- **Popup Add Module groupé par catégorie** (même regroupement, headers de catégorie).
- Tests : `categories_partition_all_kinds` (chaque kind dans exactement une catégorie, aucune vide) + `category_spot_checks`.

## 2026-08-12 — [164]+[162]+[165]+[160]+[161] Batch de tâches (build 20260813-143901)

**Branche:** `main` · **Build:** `20260813-143901` (inclut les retours : graphe gate replacé à côté des sliders, nudge ±100 ms)
**Validation:** `cargo test` 265+1+166 OK, `build.ps1 -Install` OK. **Validé dans Studio One (2026-08-13).**

- **[164] Glyphe du bouton reset morphing corrigé** — « × » corrompu (« Ã— », UTF-8 relu en Windows-1252) remplacé par « X » ASCII dans le popup Morph (`ui/plock.rs`), conforme à la convention [73] (ASCII only dans les labels).
- **[162] Enveloppes grisées en One-Shot (voix smp)** — sur BD6smp/SD6smp/CH6smp, activer **One Shot** grise les sliders Attack/Decay/Decay Curve (`add_enabled_ui`, le switch One Shot reste actif car c'est un special param) et la ligne d'enveloppe du graphe passe en gris (`draw_sample_amp_graph`). Layout stable : on grise, on ne cache pas.
- **[165] Drag d'une lane vide** — le grip des lanes inactives est câblé comme celui des lanes actives (`lane_drag_source` + curseurs Grab/Grabbing). `apply_lane_reorder_move` était déjà slot-générique → rien d'autre à toucher ; pas de sélection de track pour un slot inactif (pas d'onglet Sound Editor).
- **[160] Graphe Gate Shape (Buzz)** — `draw_buzz_gate_graph` placé **à droite des sliders Gate Rate/Depth/Shape** (sous-rangée dédiée : sliders à gauche, graphe à droite — pas d'empilement sous le graphe d'ampli qui décalait le bloc). Fenêtre temps **fixe 60 ms** (le Rate est visible : ~3 cycles à 55 Hz, peigne dense à 500 Hz), Smooth = cosinus surélevé `^(1+4·shape)`, Razor = rampe 0,3 ms + spike expo (mêmes constantes que `BuzzVoice`), plancher de Depth en ligne fine, tag « GATE ».
- **[161] Microtiming par cellule (seq plock), ±100 ms complet** — jamais câblé au moteur auparavant (stockage/persistance seuls existaient). Le **séquenceur** décale désormais chaque trigger :
  - `groove::step_start_beat(step, swing, groove)` : inverse exacte de `beat_to_step` (paires swing/shuffle/MPC) pour connaître l'heure des frontières de step.
  - **Nudge positif** : le trigger est différé (`late_trigger`/`late_fire_beat`) → stutter/fusion pulses s'expandent depuis l'heure décalée, tout le train bouge d'un bloc.
  - **Nudge négatif** : peek de la cellule du prochain boundary à chaque sample ; quand le temps restant ≤ −nudge, le trigger part en avance (`classify_cell`/`eval_trigger` partagés avec le chemin normal → masque, humanize, fusions, morphs identiques) ; le boundary réel avance l'état mais reste muet (`suppress_next`). Flag `early_next_loop` sur un early-fire qui croise le wrap → les conditions (First/NotFirst/…) sont évaluées avec `loop_count + 1` côté `lib.rs`.
  - Données : `Sequencer::set_microtimings` copie les atomics seq-plock 1×/buffer (RT-safe) ; état microtiming purgé sur play/stop/reset/seek (`clear_microtiming_state`).
  - **UI** : row « Nudge » (−100..+100 ms, reset double-clic à 0) dans le menu Seq Plock, entre Stutter et Condition.
  - **Export MIDI** : les notes sont décalées du nudge (ms → ticks au tempo d'export, clamp tick 0).
  - Tests : inverse `step_start_beat`↔`beat_to_step` (4 grooves × 5 swings × 16 steps), ±25 ms sample-accurate (±2 samples), early-fire au wrap + flag, zéro nudge = grille inchangée, export MIDI ±50 ms.
  - ⚠️ Limites connues : conditions à la boucle avec push/pull ≠ 0 = approximatif (le wrap de `loop_count` suit la timeline non shiftée, préexistant) ; collision même sample late+transition = report d'1 sample plutôt qu'un hit perdu.

## 2026-08-12 — Passation : docs réorganisées + handoff (pas de build)

- **`CLAUDE.md` devient la référence canonique unique pour tous les agents IA** (Claude/Codex/Kimi/…) : compteurs mis à jour (17 voix, 14 slots, plocks 46×14×64), enveloppe d'ampli A-H-D documentée, et absorption du détail unique d'`AGENTS.md` (patches du fork nih-plug, chaîne de saturation, choke groups, règle checklist « À tester dans Studio One », portabilité, règle « next »).
- **`AGENTS.md` réduit à une redirection** vers `CLAUDE.md` (fini la duplication périmée). Cross-refs `README.md` / `drum-pattern-vst/README.md` mis à jour.
- **`docs/HANDOFF.md` ajouté** : état de session pour un autre agent dev (arbre git, tâches, gotchas, carte des fichiers).
- **[159] validé** dans Studio One. 3 nouvelles idées notées → TODO [162]/[163]/[164]. Point de reprise : **[155]**.

## 2026-08-07 — [159] Enveloppe d'ampli A-H-D bipolaire (retrait du Release) sur toutes les voix (build 20260807-170048)

**Branche:** `skeuo-vector` · **Build:** `20260807-170048`
**Validation:** `cargo test` 259+1+161 OK, `build.ps1 -Install` OK.

- **L'enveloppe d'AMPLITUDE de toutes les voix passe d'un modèle decay+release à un A-H-D (Attack-Hold-Decay) sans release**, avec des **courbes bipolaires concave/convexe indépendantes sur l'attaque et le decay** (comme l'env de filtre du Buzz). Généralise le retour positif de l'utilisateur sur le Buzz.
  - **Réécriture interne de `DecayReleaseEnvelope`** (`dsp.rs`) en A-H-D piloté par le temps, **signatures publiques conservées** → les 14 voix ne changent quasiment pas. `decay_curve` = courbe **decay** bipolaire ; `release_curve` **réutilisé** comme courbe **attack** bipolaire ; `set_release`/`release_time` = **no-op**. `shape_curve(e,c)` partagée : `c≥0 → e^(1+3c)` (convexe), `c<0 → 1-(1-e)^(1-3c)` (concave).
  - **Anti-clic préservé** : `trigger()` rampe depuis la valeur courante (queue vivante) ; `trigger_hard()` repart de zéro (machine-gun/stutter). Tests anti-clic kick/perc1 verts.
  - **Registry** : slider **Release retiré** de toutes les tables ; **« Release Curve » → « Attack Curve »** (−1..1) ; **« Decay Curve »** en −1..1. `morphable_fields` suit (plages dérivées des tables).
  - **Graphe** (`draw_amp_envelope`) : A-H-D bipolaire (attaque + decay façonnées par `shape_curve`), plateau Hold, plus de segment/légende Release.
  - **Buzz** : son ampli passe de `ExpDecayEnvelope` à `DecayReleaseEnvelope` (courbe de decay désormais bipolaire, cohérent avec le reste ; retrigger machine-gun conservé via `trigger_hard`).
  - **Bugs corrigés au passage** : `open_hihat` recréait son env dans `set_settings` (reset d'état → clic au drag de slider) → remplacé par des setters ; `open_hihat`/`cymbal` jetaient le résultat de `with_attack_ms` (`Copy` no-op) → `set_attack_ms` ; drift d'attaque dupliqué du cymbal nettoyé.
- **Persistance (dégradation gracieuse, sans migration)** : blobs positionnels inchangés → aucune casse. Les anciennes valeurs de courbe (0.1–20) relues sur −1..1 sont **clampées à +1** (max convexe ≈ decay exponentiel raide d'avant). Les défauts par voix ne sont pas retouchés → nouvelles instances = courbes max-convexes (sliders au max) ; à affiner par voix si besoin.

## 2026-08-07 — [158] Export MIDI : fusions + stutters inclus (build 20260807-140101)

**Branche:** `skeuo-vector` · **Build:** `20260807-140101`
**Validation:** `cargo test` 259+161 OK, `build.ps1 -Install` OK.

- **L'export/drag MIDI inclut désormais les notes des cellules fusionnées et des stutters** (avant : 1 note par step actif, fusions/stutters ignorés). Réplique la logique du séquenceur audio.
  - **Fusion** : la cellule START d'un groupe émet `step_count` notes réparties uniformément sur toute la durée du span (`cell_span()` pas de 1/16, PPQ 480 → 120 ticks/step) ; les cellules couvertes n'émettent rien.
  - **Stutter** : une cellule non fusionnée avec un p-lock séquenceur `stutter_count = N` émet N notes réparties sur un pas. Fusion et stutter ne se combinent jamais (comme l'audio).
  - Durée de note-off raccourcie (`min(10, spacing-1)`) pour éviter le chevauchement des sous-notes rapprochées.
- `export_pattern_to_midi[_data|_bytes]` + `export_midi_to_documents` prennent un `&SequencerPlockState` ; call sites Export/Drag passent `params.seq_plock_state.state`. Tests `midi_export_expands_stutter_into_multiple_notes` + `midi_export_expands_fusion_into_pulses`.

## 2026-08-07 — [156] Bouton "Save" à gauche des patterns (build 20260807-123130)

**Branche:** `skeuo-vector` · **Build:** `20260807-123130`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK.

- Le keycap **Save** est déplacé **avant** la rangée de slots (`Patterns  [Save]  P1…P16  [Clr]  …Export/Drag`). Comportement inchangé (armer puis cliquer un slot).

## 2026-08-07 — Pas de fausse alerte sur projet rouvert déjà sauvé dans un slot (build 20260807-121446)

**Branche:** `skeuo-vector` · **Build:** `20260807-121446`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK.

- **Nuance** : projet sauvé avec le pattern courant stocké dans P3, quitté puis rouvert → `last_loaded = None` (association runtime perdue) mais la grille **correspond** à P3. Le test précédent (`None + non-vide = dirty`) déclenchait une **fausse alerte** au changement de slot alors que le pattern était bien sauvegardé.
- **Correctif** : sur `None`, dirty **seulement si** la grille a du contenu **ET** ne correspond à **aucun** slot occupé (mêmes `step_masks` + longueur). Un projet rouvert dont le pattern était sauvé dans un Pn matche ce slot → non dirty → pas d'alerte.

## 2026-08-07 — Warning "unsaved" aussi pour un projet chargé sans slot (build 20260807-115649)

**Branche:** `skeuo-vector` · **Build:** `20260807-115649`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK.

- **Suite** : un projet rouvert avec un pattern de travail non associé à un slot a `last_loaded = None` (l'atomique `audio_last_loaded_slot` n'est pas persistée) ; `pattern_is_dirty` renvoyait `false` sur `None` → pas de warning au changement de slot.
- **Correctif** : `None` + grille non-vide = dirty (travail non sauvé sans slot). Le warning « unsaved changes » s'affiche au changement de slot ; aucune étoile (aucun slot n'est marqué chargé). Grille vide → toujours pas dirty.

## 2026-08-07 — Fix régression : warning "unsaved" restauré pour un nouveau pattern (build 20260807-114735)

**Branche:** `skeuo-vector` · **Build:** `20260807-114735`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK.

- **Régression du fix [153]** : avant, se positionner sur un slot vide laissait (par bug) `last_loaded` sur le slot occupé précédent → un pattern construit de zéro était comparé à ce slot → jugé dirty → warning au changement. Le fix [153] a corrigé `last_loaded` (pointe bien le slot vide), mais `pattern_is_dirty` retournait `false` dès que le slot n'était pas occupé → **plus aucun warning** pour un nouveau pattern non sauvé.
- **Correctif** : `pattern_is_dirty` considère désormais un slot **vide avec une grille non-vide** comme dirty (travail non sauvé) → le warning « unsaved changes » réapparaît proprement au changement de slot, et une étoile marque le slot vide en cours de construction. Grille vide → toujours pas dirty (donc [153] préservé : après save + sélection d'un slot vide, la grille est vidée → aucune étoile fantôme).

## 2026-08-07 — [157] Buzz : max Gate Rate 150 → 500 Hz (build 20260807-111951)

**Branche:** `skeuo-vector` · **Build:** `20260807-111951`
**Validation:** `build.ps1 -Install` OK.

- **Gate Rate max relevé de 150 à 500 Hz** (`GATE_RATE_MAX` + max du slider registry) — buzz plus aigus, jusqu'à un caractère AM/tonal. Le clamp audio reste aligné sur la constante.

## 2026-08-07 — [154] Hold visible dans le graphe d'enveloppe (build 20260807-111016)

**Branche:** `skeuo-vector` · **Build:** `20260807-111016`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK.

- **Bug** : `draw_amp_envelope` recevait `hold` mais le repliait dans `decay_time` — aucun palier de maintien n'était dessiné, donc les instruments avec un Hold (Snare, etc.) ne le voyaient pas dans le graphe.
- **Correctif** : tracé A-H-D explicite — rampe d'attaque ↗, **palier plat au sommet pendant le Hold** (couleur teal dédiée), puis decay ↘. Le Hold compte dans l'échelle temporelle (`total = attack + hold + decay [+ release]`). Légende « H » ajoutée quand un Hold est réglé.

## 2026-08-07 — [153] Fix étoile "non sauvegardé" fantôme (build 20260807-105334)

**Branche:** `skeuo-vector` · **Build:** `20260807-105334`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK.

- **Bug** : sauver un pattern puis sélectionner un slot **vide** faisait réapparaître le pattern sauvé avec une étoile (dirty) alors qu'il n'avait pas été modifié.
- **Cause** : l'UI resynchronise `state.last_loaded_slot` **depuis** `audio_last_loaded_slot` à chaque frame (ui.rs). Se positionner sur un slot vide est une action **UI-only** (pas de requête de load) qui vidait la grille + posait `last_loaded_slot` localement, mais **sans** publier le slot vers l'audio → au frame suivant la sync ramenait le slot sauvé, et la grille étant vidée ≠ pattern sauvé → `is_dirty` vrai → étoile fantôme.
- **Correctif** : le positionnement sur slot vide publie maintenant `audio_last_loaded_slot = i` (l'atomique n'est qu'un hint lu par l'UI — l'audio ne l'écrit que sur save/load, jamais ne le lit pour sa logique). La sync est cohérente → plus d'étoile fantôme.

## 2026-08-07 — Buzz : retrigger machine-gun de l'enveloppe d'ampli (build 20260807-093254)

**Branche:** `skeuo-vector` · **Build:** `20260807-093254`
**Validation:** `cargo test` buzz OK, `build.ps1 -Install` OK.

- **Fix « sur cellules consécutives, l'env de volume n'est plus prise en compte »** : le retrigger de l'ampli utilisait `trigger_at_peak` (rampe depuis la valeur courante, anti-clic), donc une queue en cours « absorbait » la nouvelle enveloppe → l'A-H-D de volume ne se ré-articulait pas pleinement par cellule.
- **Correctif** : ampli passé en **retrigger machine-gun** (`trigger_from_zero`) dans `trigger()` et `trigger_hard()` → chaque cellule redémarre **toute** l'enveloppe A-H-D de volume depuis zéro (comportement standard des boîtes à rythme). Déviation volontaire de la convention anti-clic (la rampe d'attaque ≥0.3 ms adoucit le redémarrage).

## 2026-08-06 — Buzz : le gate se resynchronise à chaque hit (build 20260806-203401)

**Branche:** `skeuo-vector` · **Build:** `20260806-203401`
**Validation:** `cargo test` buzz OK, `build.ps1 -Install` OK.

- **Fix « l'attaque de volume n'est pas triggée à chaque cellule »** : l'enveloppe d'ampli se ré-attaquait bien (vérifié en test : queue 0.10 → 1.52 après re-trigger), mais le **gate** ne réinitialisait sa phase qu'au cold-start (anti-clic osc). Sur des hits qui se chevauchent, chaque cellule attrapait donc le gate free-run à une phase aléatoire → pas d'attaque de volume gatée cohérente par cellule.
- **Correctif** : `gate_phase` remis à 0 + `gate_env` ré-articulé à **chaque** trigger (chaque cellule démarre sur un pic de gate = burst d'amplitude identique et net). La phase de l'oscillateur tonal reste réinitialisée **au cold-start uniquement** (anti-clic de la partie tonale).

## 2026-08-06 — Buzz : courbes filtre attack/decay dissociées (build 20260806-201745)

**Branche:** `skeuo-vector` · **Build:** `20260806-201745`
**Validation:** `cargo test` 257+161 OK, `build.ps1 -Install` OK.

- **Filter Atk Curve + Filter Dec Curve** (au lieu d'un unique « Filter Curve ») : la montée et la descente de l'enveloppe de filtre se façonnent indépendamment, chacune bipolaire -1..+1.
- **Enveloppe de filtre passée en AHD manuelle** (compteur de temps depuis le trigger) au lieu d'un `ExpDecayEnvelope` — nécessaire pour connaître la phase (attack vs decay) et appliquer la bonne courbe. Bases **linéaires** façonnées par `shape_curve` bipolaire (0 = linéaire, +1 = convexe, -1 = concave). Défaut Dec Curve = 0.6 (garde le punch), Atk Curve = 0.
- Specials : `buzz_filter_curve` (index 15) renommé « Filter Dec Curve » ; nouveau `buzz_filter_atk_curve` (index 16). Graphe `draw_buzz_filter_envelope` mis à jour pour les deux courbes.

## 2026-08-06 — Buzz : graphe de l'enveloppe de filtre reflète l'AHD réelle (build 20260806-200700)

**Branche:** `skeuo-vector` · **Build:** `20260806-200700`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK (changement UI pur).

- **Fix graphe filtre statique** : `draw_filter_envelope` ne dessinait qu'une décroissance exponentielle simple (curve = decay_curve de l'ampli, car `filter_env_curve()` = None pour Buzz), ignorant Filter Attack / Hold / Filter Curve / amount. Nouveau tracé dédié **`draw_buzz_filter_envelope`** qui reproduit exactement le DSP : rampe d'attaque → hold → decay (raideur interne 4.0) avec la **courbe bipolaire** appliquée, et le cutoff balayé **exponentiellement** (base → base·(20000/base)^(env·amount) → base), axe Y en Hz log + ligne du cutoff au repos. Le graphe réagit désormais à Filter Attack, Hold, Decay, Curve et Filter Env.

## 2026-08-06 — Buzz : Filter Curve bipolaire (concave ↔ convexe) (build 20260806-172100)

**Branche:** `skeuo-vector` · **Build:** `20260806-172100`
**Validation:** `cargo test` 257+161 OK, `build.ps1 -Install` OK.

- **Nouveau contrôle « Filter Curve »** (special index 15, famille Filter, bipolaire **-1 → +1**, défaut 0). Façonne le contour du decay de l'enveloppe de filtre : **-1 = concave** (le filtre tient puis chute vite), **0 = naturel** (exp), **+1 = convexe** (chute rapide puis lente = snappy). Implémenté par une transformation puissance bipolaire de la sortie de l'env (`e^(1+3c)` côté +, `1-(1-e)^(1-3c)` côté -), l'`ExpDecayEnvelope` ne faisant que de la raideur convexe.

## 2026-08-06 — Buzz : env filtre exponentielle + vrai sélecteur LP/HP/BP (build 20260806-171049)

**Branche:** `skeuo-vector` · **Build:** `20260806-171049`
**Validation:** `cargo test` 257+161 OK, `build.ps1 -Install` OK.

Retours utilisateur :
- **Env de filtre repensée (percussive)** : le mapping additif (`base + env·amount·12 kHz`) était peu intuitif et, avec Filter Attack à 2 ms, le filtre s'ouvrait *après* le transitoire. Désormais **balayage exponentiel** `cutoff = base·(20000/base)^(env·amount)` (0 = base, 1 = base→20 kHz→base) + **Filter Attack défaut = 0** (instantané). À amount plein + Filter au minimum → filtre percussif franc (20 Hz→20 kHz→20 Hz). Défauts revus : base 1200 Hz, amount 0.6, decay 0.12 s.
- **Vrai sélecteur Filter Type** : le special `buzz_filter_type` s'affichait en slider (le dropdown n'avait pas été câblé) → branche dropdown **LP / HP / BP** ajoutée dans `sound_editor`.
- **« (LP) » retiré** à côté du slider Filter : `filter_type_label` de Buzz vidé, et le label n'ajoute plus le suffixe quand il est vide (le type est maintenant piloté par le dropdown).

## 2026-08-06 — Buzz : ampli AHD (sans release) + enveloppe AHD de filtre + type LP/HP/BP (build 20260806-170024)

**Branche:** `skeuo-vector` · **Build:** `20260806-170024`
**Validation:** `cargo test` 257+161 OK, `build.ps1 -Install` OK.

- **Ampli en AHD** : l'enveloppe d'ampli passe de decay+release à **Attack-Hold-Decay pure** (`ExpDecayEnvelope`) — le **Release est retiré**. Nouvelle table de params standard `BUZZ_STD` (sans Release / Release Curve).
- **Enveloppe AHD de filtre** : le cutoff est balayé par sa propre enveloppe A-H-D. Contrôles : **Filter Env** (amount) + **Filter Decay** (standard, section Filter) ; **Filter Attack** + **Filter Hold** (specials index 12/13, famille Filter). Modulation additive `cutoff = base + env·amount·12 kHz`.
- **Type de filtre** : nouveau dropdown **Filter Type** (special index 14) → **LP / HP / BP**. Le filtre de base passe d'un one-pole 6 dB à un **Biquad 2 pôles** ; ajout de `Biquad::set_lowpass` / `set_highpass` (RBJ) à `dsp.rs` (Q 0.9 LP/HP, 2.5 BP). Recalcul des coefficients par sample (négligeable pour une voix).
- Défauts : Filter Env amount 0.4, Filter Decay 0.15 s, Filter Attack 2 ms (l'effet est audible d'emblée).
- Tests ajoutés : `filter_type_changes_the_output` ; garde `output_stays_finite_and_stops` OK avec l'ampli sans release.

## 2026-08-06 — Buzz : waveform Sine/Square/Saw + fixes gate/sweep/volume (build 20260806-164108)

**Branche:** `skeuo-vector` · **Build:** `20260806-164108`
**Validation:** `cargo test` 256+160 OK, `build.ps1 -Install` OK.

Retours utilisateur sur la voix Buzz :
- **Waveform Sine/Square/Saw** : oscillateur tonal passé à un accumulateur de phase manuel + sélecteur `Wave` (dropdown, special index 11, famille Osc). Square/Saw = plus riche/buzzant (aliasing naïf assumé pour le grain lo-fi).
- **Smooth vs Razor désormais audibles** : le modèle « fraction de période » rendait les deux identiques (l'enveloppe retombait à ~0 chaque cycle). Redéfinis : **Smooth = trémolo cosinus** (montée/descente douce, Shape resserre le pulse) ; **Razor = spike exponentiel** re-déclenché de zéro chaque cycle (chop franc). Test `smooth_and_razor_differ`.
- **Pitch Sweep audible** : le sweep n'était re-déclenché qu'au *cold-start* → inaudible quand les queues se chevauchent en lecture. Désormais **re-déclenché à chaque trigger** (comme Perc1), profondeur portée à ~2 octaves, absorbé par le smoother de fréquence (anti-clic).
- **Volume** : défaut monté de 0.6 → **1.3** (le gate abaisse le niveau perçu).
- Tests ajoutés : `smooth_and_razor_differ`, `waveform_changes_the_source`.

## 2026-08-06 — Nouvel instrument « Buzz » : percussion tonale + gate rapide (build 20260806-161402)

**Branche:** `skeuo-vector` · **Build:** `20260806-161402`
**Validation:** `cargo test` 254+158 OK, `build.ps1 -Install` OK. Issu de l'investigation [93].

- **Nouvelle voix de synthèse « Buzz »** (15e kind, index voix 16, note MIDI 44) : percussion **tonale** (oscillateur sinus pitché + Pitch Sweep percussif) + **couche de bruit réglable** (montant + couleur White/Pink/Brown/Blue), le tout haché par un **gate/retrigger d'enveloppe rapide** — l'effet observé en [93] rendu délibéré et contrôlable.
- **Module gate** : un phasor à taux réglable (`Gate Rate` 1–150 Hz) ré-articule une `ExpDecayEnvelope` **courte** qui multiplie l'amplitude (reproduit l'« effondrement d'enveloppe »). `Gate Depth` = dry/wet, `Gate Shape` = durée/courbe du decay relatif à la période, `algo` Smooth (ramp) / Razor (from-zero). Chemin : source → gate → LP → amp env → saturation → DC → volume ; stéréo à phase de gate partagée.
- **Anti-clic** respecté : phase osc/gate reset au cold-start uniquement, enveloppes via setters (jamais recréées) dans `set_settings`, Freq/cutoff/depth lissés (`OnePoleSmoother`), `DcBlocker` en sortie, taux plafonné 150 Hz.
- **Contrôles data-driven** (registry) : Gate Rate/Depth/Shape (Env), Noise/Noise Type/Pitch Sweep (Osc), pack Saturation. `FULL_STD` en params standard, `filter_type_label "LP"`.
- Fichiers : `synthesis/buzz.rs` + `synthesis/settings/buzz.rs` (nouveaux) ; `DrumVoice::Buzz`, `TrackInstrumentKind::Buzz`, `TrackLayoutState::from_kinds`-compatible, `BUZZ_ALGOS`, entrée registry index 16, remap generator (emprunte le rôle Perc1), dropdown Type. **Aucune édition de `lib.rs`** (archi par slot).
- **Tests** : `buzz_settings_roundtrip`, `produces_sound_on_trigger`, `output_stays_finite_and_stops`, `gate_depth_modulates_the_output`.

## 2026-08-06 — Generator : anchors intouchables (fix four-on-the-floor) (build 20260806-124254)

**Branche:** `skeuo-vector` · **Build:** `20260806-124254`
**Validation:** `cargo test` 250+154 OK, `build.ps1 -Install` OK.

- **Fix « House/House n'a pas de rythme house »** : le kick était bien en anchors `[0,4,8,12]`, mais la règle de cohérence kick/snare de `generate_from_template` **retirait le kick** partout où le snare tombait aussi (temps 2 & 4) → il ne restait que 0 & 8. Les frappes fondatrices (anchors) étaient détruites.
- **Correctif** : les **anchors sont désormais sacrées**. `generate_from_template` traque un `is_anchor[inst][step]` ; les 3 règles de cohérence (kick/snare stacking, closed/open hat, suppression densité) ne touchent plus que les frappes **candidates** (probabilistes), jamais les anchors. Si kick ET snare sont anchors sur le même step (four-on-the-floor sous le backbeat) → les deux sont conservés.
- Bénéficie à **tous** les styles four-on-the-floor (House, Disco, Techno) et rend chaque template fidèle à sa définition ; strictement plus conservateur (ne peut qu'ajouter des frappes voulues, jamais en retirer).
- **Test** : `four_on_the_floor_kick_survives_backbeat_overlap` (House → kick présent sur 0/4/8/12).

## 2026-08-06 — Presets de style : kit de lanes adéquat + grooves authentiques (build 20260806-102824)

**Branche:** `skeuo-vector` · **Build:** `20260806-102824`
**Validation:** `cargo test` 249+154 OK, `build.ps1 -Install` OK.

- **Les 6 chips de preset (House/Dub/DnB/Bossa/Afro/Break) installent désormais un KIT de lanes adéquat** en plus de charger le groove — avant, les grooves plaquaient sur les 4 lanes par défaut et « ne ressemblaient pas ». Action **destructive** (remplace lanes + sons + grille), via `PersistentField::set(track_layout)` + `reset_slot_to_defaults` par lane (même chemin que le changement de Type d'un slot ; l'audio réinitialise les voix).
- **Kits + grooves authentiques** (par genre) :
  - **House** : Kick · Clap · HiHat · OpenHat · Perc — 4-on-floor, clap 2&4, open-hat contretemps, perc syncopée.
  - **Dub** : Kick · Snare(rim) · HiHat · 808 · Perc — one-drop (temps 3), 808 sub sur 1&3, skank offbeat.
  - **DnB** : Kick · Snare · HiHat · Snare606 · 808 — two-step, ghost snares 606, sub qui suit le kick.
  - **Bossa** : Kick · Snare(cross-stick) · Ride · HiHat · Perc — surdo, clave 3-2, ride comping, pedal hat 2&4, shaker.
  - **Afro** : Kick · Snare(rim) · HiHat · Ride(bell) · Tom(conga) · Perc — kick syncopé, cloche, congas, shaker.
  - **Break** : Kick · Snare · HiHat · Snare606 · Ride — kick cassé, hats 16èmes funky, ghost snares.
- Nouveau `TrackLayoutState::from_kinds(&[kinds])` (active slots 0..n, choke 1 sur HH/OH) ; grooves `pattern.rs` réécrits pour l'ordre de lane de chaque kit ; test `new_style_presets_are_well_formed` mis à jour (hits ≥ 8, lanes dans la taille du kit).
- Rock/Funk/Disco/⟳ Random inchangés (grille seule).

## 2026-08-06 — Presets fixes pour les 6 nouveaux styles (build 20260806-091408)

**Branche:** `skeuo-vector` · **Build:** `20260806-091408`
**Validation:** `cargo test` 249+154 OK, `build.ps1 -Install` OK.

- **6 chips de preset fixe** ajoutés à la rangée **« Presets »** du bottom panel (à côté de Rock / Funk / Disco) : **Bossa, House, DnB, Afro, Dub, Break**. Comme les presets existants, ce sont des grooves **déterministes** (toujours identiques, contrairement à GENERATE qui tire un seed aléatoire).
- Nouveaux constructeurs `Pattern::{bossa,house,dnb,afrobeat,dub,breakbeat}_pattern()` dans `sequencer/pattern.rs` : grooves signature répétés par bar, n'utilisant que les **4 lanes cœur** (0 Kick, 1 Snare, 2 HiHat, 3 Open HH) — comme Rock/Funk/Disco — pour sonner sur le layout par défaut sans dépendre des slots aux.
  - House = four-on-the-floor + open-hat offbeat · DnB = kick two-step + snare 2&4 + ghosts · Dub = one-drop (beat 3) + skank offbeat · Bossa = surdo + cross-stick clave · Afrobeat = kick syncopé + hats busy · Breakbeat = kick cassé + hats funky.
- **Test** : `new_style_presets_are_well_formed` (kick + hats présents, uniquement lanes 0-3).

## 2026-08-05 — [148] Generator : +6 styles (16 au total) (build 20260805-175817)

**Branche:** `skeuo-vector` · **Build:** `20260805-175817`
**Validation:** `cargo test` 248+153 OK, `build.ps1 -Install` OK.

- **6 nouveaux styles** ajoutés à la palette du Generator (dropdowns Style A / B) → **16 styles** au total : **Bossa Nova, House, Drum'n'Bass, Afrobeat, Dub, Breakbeat** (en plus de Rock, Funk, Techno, Hip-Hop, Jazz, Metal, Latin, Disco, Trap, Reggae).
- Chaque style = un `MusicalTemplate` (rôles rythmiques des 14 instruments : anchors / candidates / prob / exclusions + plage BPM), écrit d'après le rythme caractéristique du genre : four-on-the-floor + open-hat offbeat (House), one-drop + gros sub (Dub), amen/ghost-snares (DnB, Breakbeat), clave 3-2 cross-stick (Bossa), tresillo + bell + percussions (Afrobeat).
- **Zéro édition UI** : les dropdowns sont pilotés par `Style::variants()`, ajouter un variant suffit.
- **Persistance** : nouveaux variants ajoutés à la fin de l'enum → indices 0-9 (styles existants) inchangés, les vieilles sessions rechargent leur style correctement.
- **Tests** : `every_style_template_is_well_formed` (garde : steps dans la page 0-15, prob ∈ [0,1], BPM sain, sur les 16 styles) ; les 6 nouveaux ajoutés à `hihat_roles_are_style_specific`.

## 2026-08-04 — Fix cellule fantôme après un tap rapide (step-drag) (build 20260804-224719)

**Branche:** `skeuo-vector` · **Build:** `20260804-224719`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK.

- **Bug** : après un clic bref sur une cellule active, une cellule « fantôme » avec marqueur jaune apparaissait parfois et suivait la souris.
- **Cause** : le déplacement de cellule est un *long-press* (~0,5 s). Le traitement du **relâchement** (`any_released`) était imbriqué dans `if drag.active` — donc un appui **relâché avant le seuil de 0,5 s** ne nettoyait jamais `state.step_drag`. Le compteur de temps continuait aux frames suivants et le drag « s'activait » ~0,5 s **après** le relâchement (bouton déjà relevé) → cellule fantôme qui suit le curseur jusqu'au prochain clic.
- **Fix** : le test de relâchement est sorti de `if drag.active`. Un relâchement termine **toujours** le geste : s'il n'a jamais franchi le seuil (tap rapide) il annule proprement le drag en attente et retombe sur un clic/toggle normal ; s'il était actif il applique le déplacement comme avant.

## 2026-08-04 — Samplers 606 : Attack en millisecondes absolues (build 20260804-174450)

**Branche:** `skeuo-vector` · **Build:** `20260804-174450`
**Validation:** `cargo test` 247+153 OK, `build.ps1 -Install` OK.

- **Fix Attack inutilisable sur les samples courts** (signalé sur CH6smp) : l'Attack des samplers 606 était une **fraction de la durée du sample** (`attack × played_secs`). Sur un charleston fermé (transitoire ~3,5 ms), même une petite fraction montait pendant que le sample était déjà éteint → au lieu d'un fondu d'attaque musical, ça **écrasait/effaçait** le son (fraction 0,1 → 50 ms → pic ÷5 ; 0,5 → 250 ms → quasi silence).
- **Nouveau modèle** : Attack = temps de montée **ABSOLU**, `attack × MAX_AMP_ATTACK_SECS` (**80 ms** plein-échelle), indépendant de la longueur du sample. Défaut 0,001 → ~0,08 ms (plein transitoire, inchangé). Appliqué aux **3 samplers** (BD6smp/SD6smp/CH6smp) pour rester cohérent.
- **Non touché** : le **Decay** et le **Filter Decay** restent des fractions de la durée jouée (ils suivent le sample, ce qui a du sens ; seule l'attaque posait problème).
- ⚠️ **Compat sessions** : les valeurs Attack **non par défaut** sauvegardées sur BD6smp/SD6smp sont réinterprétées (0–1 → 0–80 ms au lieu de 0–durée). Voix ajoutées le 2026-08-02, impact minime.
- **Tests** : `amp_decay_tracks_length_attack_is_absolute` (×3) — prouve que le decay suit le pitch/longueur mais que l'attack reste absolu.

## 2026-08-04 — CH6smp : sampler Closed Hi-hat TR-606 (build 20260804-170126)

**Branche:** `skeuo-vector` · **Build:** `20260804-170126`
**Validation:** `cargo test` 153 OK (dont `ch606::output_stays_finite_and_stops`), `build.ps1 -Install` OK.

- **Nouvelle voix `CH6smp`** (14e slot) : sampler à pitch relatif calqué sur `bd606`/`sd606`, alimenté par `wav/CH.wav` (copié en `assets/ch606.wav`, embarqué via `include_bytes!`).
  - Enum : `DrumVoice::Ch606` (index 15, `COUNT = 16`) + `TrackInstrumentKind::Ch6smp` (index 13, `COUNT = 14`).
  - **Note MIDI 42** (Closed Hi-hat GM), label `c6`, rôle générateur = **HiHat**.
  - Registre : 16e `InstrumentDef` (`SMP606_STD` + specials `ch606_*`, `algo_count = 1`, `filter_type_label "LP"`, root legacy 8000 Hz).
  - Banque d'échantillons : `sample_bank::ch606()` (`OnceLock`, 8 hits), pré-chauffée au `new()` du synthé ; graine RNG distincte (`0x6060_0003`).
  - UI (`sound_editor`) : la voix apparaît dans le dropdown Type, hérite du graphe waveform + End/Start/pitch des voix smp (sites `13|14` → `13|14|15`, sélection de banque 3-way).
  - `reset_specials_for_voice` : marqueur pitch (`special[10] = 1.0`) posé pour la voix 15 comme pour 13/14.
- **Tests** : `synthesis::ch606::tests::output_stays_finite_and_stops`.

## 2026-08-04 — [151] Linker 2 lanes adjacentes (layering) (build 20260804-121207)

**Branche:** `skeuo-vector` · **Build:** `20260804-121207` (fixes) · initial `20260804-105255`
**Validation:** `cargo test` 377 OK, `build.ps1 -Install` OK. **Validé en S1** (fonctionne).

**Fixes 121207 :**
- **Indicateur de lien** refait : la ligne+point qui dépassait de la poignée (effet « bug GUI ») remplacée par une **bande d'accent bleue 2px** propre sur le bord gauche de la lane.
- **Rafraîchissement après suppression de fusion** : l'éditeur est réactif (pas de repaint continu à l'arrêt) → supprimer une fusion (bouton « Del ») laissait la cellule affichée fusionnée. Ajout de `mark_pattern_dirty()` + `ctx().request_repaint()` sur le « Del » et sur les éditions de grille (toggle step/fusion).

- **Layering** : une lane peut « linker » celle **juste au-dessus** → elle partage ses **steps + fusions** (même rythme) tout en gardant **son propre son, plocks, algo, routing, note MIDI, mute/solo, Hum/Push/Len**.
- **Modèle** : nouveau champ `TrackSlot.linked_up` (`#[serde(default)]` → migration transparente) + `AtomicTrackLayout.slot_linked` (lu par l'audio). Helper central **`grid_slot(slot)`** qui remonte la chaîne de liens jusqu'au maître actif (C→B→A) ; le lien se **rompt** si le maître devient inactif. Adjacence garantie (« lien vers le haut »).
- **Audio** (`sequencer`) : nouveau `set_grid_slots` (calculé/bloc depuis le layout atomique) ; les lectures de step + fusion passent par `grid_slots[slot]` (mute/timing restent par lane).
- **UI** (`grid.rs`) : la lane linkée **affiche et édite** les steps/fusions du maître (édition bidirectionnelle) ; menu clic-droit **« Link steps to lane above » / « Unlink steps »** ; indicateur visuel (barre + point bleu dans la poignée).
- **Tests** : `grid_slot_resolves_link_chain_to_active_master`, `linked_up_defaults_false_and_survives_layout_roundtrip`.
- ⚠️ **Limites v1** : le **morph** des fusions reste appliqué au maître (la lane linkée joue le rythme fusionné avec son propre son statique) ; le reorder de lane ne réajuste pas les liens.

## 2026-08-02 — [83] Pitch Fine sous Pitch + fix fusion : actions restaurées en mode Sequencer P-Lock (build 20260802-212008)

**Branche:** `skeuo-vector` · **Build:** `20260802-212008`
**Validation:** `cargo test` 377 OK, `build.ps1 -Install` OK.

- **Pitch Fine déplacé directement sous le slider Pitch** dans la section OSC des voix smp (était après Sample/Start) ; retiré de la boucle des special params pour ces voix.
- **Fix régression fusions (depuis [145])** : en mode **Sequencer P-Lock**, le clic droit sur une cellule fusionnée n'affichait que le menu seq (solo/proba/stutter/…) — **Morphing / Edit Fusion Steps / Delete Fusion étaient devenus inaccessibles** (ils n'existaient plus qu'en mode Sound). Le bloc fusion est extrait en helper `draw_fusion_group_menu` et affiché **au-dessus du menu seq** dans les deux modes ; le sous-menu Morph y est aussi accessible (`popup.morph_menu` honoré en mode Sequencer).
- **Test d'intégration** : `bd606_fine_tune_changes_playback_rate_through_synthesizer` (prouve que Pitch Fine agit sur la vitesse de lecture via DrumSynthesizer — l'effet s'entend au **prochain trigger**, pas sur le hit en cours).

## 2026-08-02 — [83] Fix pitch smp : snapping par pas de 1 DANS le widget slider (build 20260802-204021)

**Branche:** `skeuo-vector` · **Build:** `20260802-204021`
**Validation:** `cargo test` 375 OK, `build.ps1 -Install` OK.

- Le snapping par pas de 1 semitone de la build précédente était appliqué **après** le dessin du slider → pendant le drag, l'affichage restait fractionnaire et le pas de 1 était invisible. Le pas est maintenant géré **dans le widget track** (`TrackStyle.step` + `with_step`, utilisé via `draw_editor_slider_row_full`) : la valeur ET l'affichage snappent à l'entier en temps réel pendant le drag. Comportement continu inchangé pour tous les autres sliders (`step = 0.0`).
- Rappel : **Pitch Fine** (±100 = ±1 semitone) et **End** sont dans la section OSC du Sound Panel (sous Sample/Start), déjà présents depuis la build `20260802-202744`.

## 2026-08-02 — [83] Lanes smp : pitch par pas de 1, param End, graphes waveform croppés (build 20260802-202744)

**Branche:** `skeuo-vector` · **Build:** `20260802-202744`
**Validation:** `cargo test` 375 OK, `build.ps1 -Install` OK.

- **Pitch par pas de 1 semitone** : le slider Pitch des voix smp snappe à l'entier (le special **Pitch Fine** couvre les cents, ±100 = ±1 semitone ; label renommé pour clarifier qu'il s'applique au pitch).
- **Nouveau paramètre End** (`special[11]`, fraction 0..1, défaut 1.0) : la lecture du sample s'arrête à End, symétrique de Start. Les anciennes sessions (blob sans End, `special[11] = 0`) jouent le sample entier — garde-fou legacy côté voix via le marqueur pitch + seeding à 1.0 lors de la migration UI.
- **Graphes waveform rework** (`ui/envelope_viz.rs`) : la waveform est **croppée sur [Start, End]** (les parties offsetées ne sont plus dessinées du tout), barres plus larges et plus lumineuses (64 colonnes, stroke 2). Les courbes amp/filtre sont mappées sur l'axe de la région jouée.
- **Fix courbe env filter tronquée** : après le sweep, la courbe est **tenue au cutoff** jusqu'au bord droit — la fin de la courbe reste visible (elle s'arrêtait net en plein graphe). Idem amp : la courbe atterrit visiblement sur la baseline.
- **Tests** : `end_truncates_playback` (×2 voix), `legacy_settings_without_end_play_the_full_sample`, roundtrips settings couvrant le nouveau champ.

## 2026-08-02 — [83] Lanes smp : pitch relatif ±24 + fine, graphes waveform, env 100 % relatives (build 20260802-200054)

**Branche:** `skeuo-vector` · **Build:** `20260802-200054`
**Validation:** `cargo test` 369 OK, `build.ps1 -Install` OK.

- **Pitch relatif** sur BD6smp/SD6smp : le slider devient **-24/+24 semitones** (0 = pitch natif) + nouveau special **Fine** (-100/+100 cents, `special[9]`). Le mode Hz/Notes est retiré pour ces voix (listes `is_bass_drum` revenues à Kick/B8).
- **Env amp entièrement relative** : **Release et Release Curve retirés** des tables smp ; **Attack** et **Decay** sont désormais des fractions 0..1 de la **durée jouée** (longueur du sample ÷ pitch, recalculées à chaque trigger), comme Filter Decay déjà.
- **Graphes waveform** (`ui/envelope_viz.rs`) : les sections Envelope et Filter des lanes smp affichent la **waveform du sample sélectionné** (normalisée) avec la zone **Start** grisée + ligne amber, et la courbe superposée — env amp bleue (ligne pleine en One Shot), sweep filtre orange + ligne de cutoff. Les autres voix gardent les graphes ADSR/classiques.
- **Migration du pitch Hz → semitones** (sessions des builds d'hier) : marqueur `special[10]` (1 = format relatif, 0 = legacy Hz, caché dans les 32 specials sans changer le blob `sound-settings-v2`). La voix **comprend** l'ancien format (ratio = freq/60 Hz BD, /200 Hz SD) → le son reste correct même sans ouvrir l'UI ; l'ouverture du Sound Panel **commit** la conversion en semitones. Nouvelles lanes et resets seedés à 1.
- **Reset sliders smp** : le double-clic revient aux defaults du registry (`0.0` semitone) et plus au `60 Hz` générique.
- **Tests** : `fine_tune_adds_cents_to_relative_pitch`, `amp_times_track_the_played_sample_length` (×2 voix), `legacy_hz_pitch_keeps_native_rate` (×2), `multisample_defaults_mark_relative_pitch_format`, test `default_frequency_is_nonzero` devenu data-driven (exempte les pitch relatifs à range négative).

## 2026-08-02 — [83] Nouvel instrument SD6smp : SD 606 multisamplée ×8 (build 20260802-165023)

**Branche:** `skeuo-vector` · **Build:** `20260802-165023`
**Validation:** `cargo test` 358 OK, `build.ps1 -Install` OK.

- **2ᵉ instrument multisample** : `TrackInstrumentKind::Sd6smp` (label grille `s6`). 8 coups de la SD d'une TR-606 embarqués (`assets/sd606.wav`, float32 mono 44,1 kHz, 4 s = 8 × 0,5 s). Même moteur de lecture que BD6smp : Analog Mode (random sans répétition / sample fixe 1-8), Pitch (200 Hz = natif, plage 50-1000), env amp, env filtre relative à la durée jouée, One Shot, Start relatif, pack saturation.
- **Sample bank généralisée** (`sample_bank.rs`) : `bd606()` / `sd606()` (deux `OnceLock`), décodeur partagé `load_bank(bytes)` — ajouter un prochain instrument = 1 WAV + 1 accesseur.
- **Enregistrement** : `DrumVoice::Sd606 = 14` (COUNT 15), `DrumVoiceKind::Sd606` (9 matchs + `create_voice_for_kind`), `TrackInstrumentKind::Sd6smp = 12` (COUNT 13, fin d'enum), registry `INSTRUMENTS[14]` (note MIDI 40). Persistance `sound-settings-v2` inchangée.
- **Générateur** : la lane SD6smp emprunte le rôle Snare.
- **UI** : les rendus spéciaux multisample (switch Analog Mode / One Shot, liste Sample grisée quand Analog ON) sont généralisés par suffixe de nom (`_analog_mode`, `_one_shot`, `_sample`) → automatiques pour les prochains instruments 606.
- **Tests** : décodage bank SD (8 hits de 22050 samples), roundtrip settings, voix (son, finie, silence final), sample fixe bit-identique, random sans répétition (×64), Start relatif, Filter Decay relatif (0,25 s natif / 0,125 s à l'octave), registry mono.

## 2026-08-02 — [83] BD6smp : Sample grisé (pas masqué), Start & Filter Decay relatifs au sample (build 20260802-163241)

**Branche:** `skeuo-vector` · **Build:** `20260802-163241`
**Validation:** `cargo test` 342 OK, `build.ps1 -Install` OK.

- **Sample grisé au lieu de masqué** quand Analog Mode est ON (`add_enabled_ui(false)`) → le bas du Sound Panel ne se décale plus au toggle.
- **Start relatif à la longueur du sample** : le paramètre devient une fraction 0..1 du coup sélectionné (était 0-0,5 s absolus) — 0,5 = démarrage au milieu du sample, quel que soit le pitch.
- **Filter Decay relatif à la longueur du sample** : le paramètre devient une fraction 0,01..1 de la **durée jouée** (longueur du sample ÷ pitch, recalculée à chaque trigger) — le balayage filtre suit le hit à tout pitch. Défaut 0,15.
- **Tests** : `start_offset_is_a_fraction_of_the_sample_length` (lecture démarre bien au milieu du hit), `filter_decay_tracks_the_played_sample_length` (0,5 × 1 s à pitch natif, 0,25 s à l'octave).

## 2026-08-02 — [83] BD6smp : Analog Mode en switch + liste Sample conditionnelle (build 20260802-161718)

**Branche:** `skeuo-vector` · **Build:** `20260802-161718`
**Validation:** `cargo test` 338 OK, `build.ps1 -Install` OK.

- **Analog Mode** et **One Shot** rendus comme des **switches** (étaient des sliders 0-1) dans le Sound Panel.
- **Sample** rendu comme une **liste 1-8** (était un slider) et **masqué tant qu'Analog Mode est ON** — il n'a de sens qu'en mode sample fixe. La valeur reste persistée quand la liste est cachée.

## 2026-08-02 — [83] Nouvel instrument BD6smp : BD 606 multisamplée ×8 (build 20260802-160117)

**Branche:** `skeuo-vector` · **Build:** `20260802-160117`
**Validation:** `cargo test` 338 OK, `build.ps1 -Install` OK.

- **Premier instrument à base de multisample** : `TrackInstrumentKind::Bd6smp` (label grille `B6`). 8 coups de la BD d'une TR-606 embarqués (`assets/bd606.wav`, float32 mono 44,1 kHz, 8 s = 8 × 1 s) pour reproduire la variabilité analogique hit-to-hit.
- **Sample bank** (`synthesis/sample_bank.rs`) : WAV embarqué via `include_bytes!`, décodé une fois dans un `OnceLock` global (pré-chauffé dans `initialize_with_layout`, zéro alloc sur le thread audio), split égal 8 × 44 100 samples. **Pas de resampling au chargement** : la lecture à position fractionnaire (interpolation linéaire) absorbe le ratio source/session.
- **Paramètres** (Sound Panel, data-driven) :
  - **Analog Mode** (special, défaut ON) : ON = tirage aléatoire **sans répétition immédiate** parmi les 8 coups (RNG xorshift seedé à la construction, jamais reseedé au trigger) ; OFF = toujours le même coup, choisi par **Sample** (1-8).
  - **Pitch** (slider Freq, 20-500 Hz, 60 Hz = pitch natif) = vitesse de lecture ; mode Hz/Notes actif (bass drum).
  - **Env amp** standard (Attack/Decay/Curve/Release) — contournée en mode **One Shot** (joue le sample entier).
  - **Env filtre** additive sur LP one-pole (Filter/Filter Env/Filter Decay).
  - **Start** (offset 0-0,5 s), **pack saturation** complet (5 params, routage `process_at`, volume post-sat).
- **Enregistrement** : `DrumVoice::Bd606 = 13` (COUNT 14) + `DrumVoiceKind::Bd606` (9 matchs + `create_voice_for_kind`) + `TrackInstrumentKind::Bd6smp = 11` (COUNT 12, ajouté **en fin d'enum** pour la compat `track-layout-v1`) + entrée registry `INSTRUMENTS[13]` (note MIDI 41).
- **Persistance intacte** : les longueurs legacy de `sound-settings-v2` sont gelées sur 13 voix via la nouvelle const `LEGACY_VOICE_COUNT` (ne plus jamais utiliser `DrumVoice::COUNT` là — il grandit désormais).
- **Générateur** : la lane BD6smp emprunte le rôle Kick (sinon silencieuse sur GENERATE).
- **UI** : dropdown Type (Track tab) + popup Add Module ; listes analog-fixed et is_bass_drum (Sound Panel + plock) étendues à l'index 13.
- **Doc** : `ADDING_AN_INSTRUMENT.md` réécrit pour l'architecture **modulaire** réelle (3 enums, `reinitialize_slot`, persistance par slot) — l'ancienne version décrivait encore les voix fixes.
- **Tests** : décodage bank (8 hits non vides, attaques présentes), roundtrip settings, voix (son produit, finie, silence final), sample fixe bit-identique, random sans répétition (×64), pitch raccourcit la durée, one-shot ignore l'env amp, registry mono.

## 2026-08-02 — [145] Solo par-step/fusion finalisé + fix step-drag fantôme (build 20260802-133013)

**Branche:** `skeuo-vector` · **Build:** `20260802-133013`
**Validation:** `cargo test` 320 OK, `build.ps1 -Install` OK. **Validé en Studio One** (solo OK, plus de fantôme, plus de crash projet vide).

- **Sémantique du solo finale = par-step / span de fusion** : le solo mute les autres lanes **uniquement pendant que la tête de lecture est sur la cellule soloée** (1 step, ou toute la durée d'une cellule fusionnée) ; hors de cette fenêtre tout rejoue. Pour soloer sur plusieurs steps → fusionner la cellule. `SequencerPlockState::solo_window(fusion_span_len)` + gating `solo_window.bit(step) && !is_solo(slot,step)`. Toggle « Solo » **par cellule** ; désactiver **efface le seq-plock** s'il ne reste aucun autre param (proba/stutter/condition/micro).
- **Fix bug « plock fantôme à un endroit aléatoire »** (racine réelle, trouvée au trace) : le **clic-droit** qui ouvre le popup démarrait aussi un **step-drag** (`response.is_pointer_button_down_on()` est vrai pour n'importe quel bouton) ; lire le menu >0,5 s activait le drag, et cliquer un contrôle du popup le **relâchait** → **déplacement silencieux** du step (avec son solo/plock) vers la position du popup. Corrigé : le step-drag ne démarre que sur le bouton **primaire** et jamais quand `plock_popup.is_some()`. Le déplacement légitime (glisser gauche) emporte toujours correctement le solo (via `get`/`set`).
- **Crash à l'instanciation sur projet vide** (apparu pendant le dev [145]/[149]) : **disparu** avec le retour au per-step + le fix du drag. **Traçage diagnostic entièrement retiré** (`diag_log`, `install_panic_logger`, compteurs process, marqueurs paint). Si le crash réapparaît → bisect [149] vs [145] (la base `20260801-201613` ne crashait pas).

## 2026-08-01 — [145] Solo de lane retirable depuis n'importe quelle cellule (build 20260801-180017)

**Branche:** `skeuo-vector` · **Build:** `20260801-180017`
**Validation:** `cargo test` OK, `build.ps1 -Install` OK.

- **Fix « impossible de retirer le solo »** : le solo étant par cellule mais l'effet sur toute la lane, retirer le solo depuis une autre cellule (toggle off) ajoutait en fait un 2ᵉ solo. Le toggle **« Solo (lane) »** reflète désormais l'état de la **lane** (`lane_soloed`) et non de la cellule ; le désactiver appelle `clear_lane_solo(slot)` qui efface **tous** les bits solo de la lane → retirable depuis n'importe quelle cellule.
- Test `clear_lane_solo_removes_every_soloed_cell_in_the_lane`.

## 2026-08-01 — [145] Solo de lane (seq-plocks) — solo pattern entier (build 20260801-174923)

**Branche:** `skeuo-vector` · **Build:** `20260801-174923`
**Validation:** `cargo test` 205 OK, `build.ps1 -Install` OK.

- **Nouveau toggle Solo** dans les p-locks séquenceur, par cellule. Sémantique : **dès qu'une cellule est Solo, sa lane joue seule sur TOUT le pattern** — toutes les autres lanes sont muettes. Plusieurs cellules solo dans des lanes différentes → ces lanes jouent ensemble, le reste muet. Indépendant du tag S de lane. *(Première itération 20260801-171811 était un solo par step, jugé trop étroit ; refait en solo de lane pattern-entier.)*
- **Modèle** (`plock.rs`) : `SequencerStepParams.solo` + `SequencerPlockState.solo_masks` (bitmask lock-free 1 bit/step). `set_solo`/`is_solo`, `lane_soloed(slot)` / `any_lane_soloed()`. Activer Solo marque la cellule seq-plock active.
- **Audio** (`lib.rs`) : `any_solo` calculé une fois par bloc (RT-safe) ; gating au trigger : `if any_solo && !lane_soloed(slot) → skip`.
- **Persistance** (`pattern_bank.rs`) : `solo_masks` **appended** en fin de `seq_plock_bytes` → vieux blob `pattern-bank-v1` rechargé tel quel avec solo=false (lecture conditionnée à la longueur). Les 2 chemins de restore mis à jour. Copy/paste lane + reorder de lane emportent le solo.
- **UI** : toggle **Solo** dans le menu seq-plock (violet, après « Mode ») ; marqueur **« S »** en coin haut-gauche des cellules solo (mode séquenceur).
- **Tests** : `lane_solo_marks_whole_lane_and_any_solo`, `set_solo_marks_active_and_roundtrips`, `clear_resets_solo`, `seq_plock_solo_survives_capture_restore`, `seq_plock_legacy_blob_without_solo_defaults_false`.

## 2026-08-01 — [149] Pattern Bank étendue à 16 slots (build 20260801-154337)

**Branche:** `skeuo-vector` · **Build:** `20260801-154337`
**Validation:** `cargo test` 314 OK, `build.ps1 -Install` OK.

- **`SLOT_COUNT: 8 → 16`** — la Pattern Bank passe de P1-P8 à P1-P16.
- **Migration sans perte** : `PatternBank::slots` (`[PatternSlot; 16]`) reçoit un `deserialize_with` **tolérant en longueur**. Une vieille session `pattern-bank-v1` à 8 slots remplit P1-P8 et laisse P9-P16 vides, au lieu de l'échec `from_slice` actuel qui rechargeait un bank **vide** (perte silencieuse de tous les patterns sauvegardés). La clé JSON et le `VST3_CLASS_ID` sont inchangés.
- **UI** (`ui/pattern_bank.rs`) : loop `0..8` → `0..SLOT_COUNT`, slots 30→26 px + espacement resserré (3 px) → 16 slots sur **une seule rangée** (hauteur inchangée, zones stables OK) ; Export/Drag restent collés à droite.
- **MIDI pattern-switch** (`lib.rs`) : notes 60-75 (au lieu de 60-67) → P1-P16, via `SLOT_COUNT`.
- Libellés « P1-P8 » → « P1-P16 » (UI + docstrings). `song.rs` utilisait déjà `SLOT_COUNT` → dropdown des blocks auto-adapté à 16.
- **Tests** : `pattern_bank_migrates_8_slot_blob_to_16` (P1-P8 préservés, P9-P16 vides) + `pattern_bank_roundtrips_16_slots`.

## 2026-07-29 — Logo : bas du logotype coupé (build 20260729-223750)

**Branche:** `skeuo-vector` · **Build:** `20260729-223750`
**Validation:** `cargo check` 0 warning, `build.ps1 -Install` OK.

- L'uv du logotype excluait la **dernière ligne de pixels** du contenu (glyph y12..36 **inclus**, mais l'uv max était `36/48` → la ligne 36 était rognée, idem colonne 162). uv corrigée à `163/164, 37/48` ; affichage recalé en 1:1 sur le contenu réel 160×25 (était 159×24).

## 2026-07-29 — Warning pattern : « Save & Load » + modal recentré (build 20260729-222818)

**Branche:** `skeuo-vector` · **Build:** `20260729-222818`
**Validation:** `cargo check` 0 warning, `build.ps1 -Install` OK.

- Retouche [139] (sur retour utilisateur) : le modal d'avertissement pattern est **descendu au centre de l'écran** (était collé sous le header) et propose un troisième bouton **`Save & Load`** (bleu) : sauvegarde d'abord le pattern courant dans SON slot, puis bascule sur le slot cible. `Discard & Load Pn` (rouge) et `Cancel` inchangés.

## 2026-07-29 — Warning pattern non sauvegardée + choke groups ×4 (build 20260729-174208)

**Branche:** `skeuo-vector` · **Build:** `20260729-174208`
**Validation:** `cargo check` 0 warning, `cargo test` OK (198+113+1), `build.ps1 -Install` OK.

- **[139] Warning pattern non sauvegardée** : cliquer un slot P1-P8 alors que la grille courante a des modifications non sauvegardées (`P1*`) ouvre une plaque skeuo « The current pattern has unsaved changes. Switching to Pn will discard them. » avec `Discard & Load Pn` (rouge) / `Cancel`. Couvre le load d'un slot occupé, le reload du même slot et le positionnement sur un slot vide. Dirty check factorisé dans `pattern_is_dirty()`.
- **[147] Choke groups ×4** : remplace le choke global HH→OH par **4 choke groups par slot**, assignables dans l'onglet Track (dropdown `Choke` : None/1/2/3/4, section Routing). Quand un slot déclenche, tous les autres slots actifs du même groupe sont silencés (`apply_choke_groups`, lecture lock-free du layout atomique — groupe packé dans les bits 4-6 du routing byte). Fonctionne pour le séquenceur interne ET les triggers MIDI externes, pour tous les instruments.
- **Migration** : les sessions sans champ `choke_group` (sentinel serde `0xFF`) récupèrent le comportement historique — HiHat et OpenHiHat en **groupe 1**. Les presets 12 lanes / legacy 13 initialisent HH+OH au groupe 1. Le param legacy `hihat_chokes_oh` est **masqué** du DAW (conservé pour le chargement des vieilles sessions) et le toggle `Choke` du header est retiré. ⚠️ Les utilisateurs qui avaient DÉSACTIVÉ le choke HH→OH retrouvent le groupe 1 assigné — mettre `Choke: None` dans l'onglet Track pour revenir à l'ancien comportement.
- Tests : `legacy_routing_without_choke_group_migrates_hats_to_group_1`, `routing_byte_packs_choke_group_and_output`, `atomic_layout_exposes_normalized_choke_groups`, `choke_group_silences_same_group_slots`.

## 2026-07-29 — Clr : choix Grid / Slot + positionnement sur slot vide (build 20260729-150219)

**Branche:** `skeuo-vector` · **Build:** `20260729-150219`
**Validation:** `cargo check` 0 warning, `cargo test` OK (194+110+1), `build.ps1 -Install` OK.

- **Clr repensé** (sur demande) : le premier clic arme la confirmation, qui propose désormais **trois keycaps** : `Grid` (bleu — vide la grille courante seule, le slot garde son pattern sauvegardé), `Slot` (rouge — vide la grille ET le slot de la bank, visible seulement si un slot est chargé), `X` (annuler).
- **Positionnement sur un slot vide** : cliquer un slot P1-P8 vide s'y **positionne** (keycap bleu = slot courant) avec une grille fraîche vide — on peut commencer un nouveau pattern directement à cet emplacement, `Save` y écrira. Factorisé dans `clear_current_grid()`.

## 2026-07-29 — Playhead : ring autour de toute la fusion (build 20260729-143825)

**Branche:** `skeuo-vector` · **Build:** `20260729-143825`
**Validation:** `cargo check` 0 warning, `cargo test` OK (194+110+1), `build.ps1 -Install` OK.

- **[138]** Quand la tête de lecture entre dans un groupe fusionné, le ring blanc pulsé entoure désormais **tout le bloc fusionné** (dessiné sur le `block_rect` de la cellule de départ). Avant : chaque step individuel était entouré au fil du jeu, y compris les cellules internes invisibles. Le marqueur ne vit plus que sur la cellule de départ du groupe.

## 2026-07-29 — Pattern Bank : keycap = occupé, creux = vide ; Clr vide le slot (build 20260729-140953)

**Branche:** `skeuo-vector` · **Build:** `20260729-140953`
**Validation:** `cargo check` 0 warning, `build.ps1 -Install` OK.

- Retouche [142] (sur retour utilisateur) : pastille blanche supprimée. Nouvelle différenciation par le **langage de forme** : slot **occupé** = keycap relevé + label lumineux (`INK_KEYCAP`) ; slot **vide** = box plate en creux (fond `BG()` + bordure `LINE()`, label `FAINT`). Le slot chargé (bleu) reste inchangé.
- **Clr vide désormais le slot de la bank** : confirmer `Clr` efface la grille/plocks/fusions courantes ET remet le slot d'origine (`last_loaded_slot`) à `PatternSlot::default()` → le slot redevient visuellement vide immédiatement. Tooltip mis à jour.

## 2026-07-29 — Slots vides sans marqueur (build 20260729-135911)

**Branche:** `skeuo-vector` · **Build:** `20260729-135911`
**Validation:** `build.ps1 -Install` OK.

- Retouche [142] : le fin contour fantôme des slots Pattern Bank vides (lu comme une « LED éteinte ») est retiré. Reste : pastille blanche sur les slots **occupés**, slots vides simplement assombris.

## 2026-07-29 — Quick wins UI : champ Name aligné, lanes vides discrètes, slots pattern, volumes (build 20260729-100737)

**Branche:** `skeuo-vector` · **Build:** `20260729-100737`
**Validation:** `cargo check` 0 warning, `cargo test` OK (194+110+1), `build.ps1 -Install` OK.

- **[140] Onglet Track — champ Name aligné** : le `TextEdit` (padding de frame egui) dépassait des dropdowns Type/Aux Out. Il est maintenant dessiné dans la même box keycap 146×26 (`keycap_tex` + `TextEdit` sans frame à l'intérieur) : bords droits alignés.
- **[141] Lanes vides plus discrètes** : cellules en fond sombre **plat** (plus de pointillés — les pointillés restent réservés aux cellules hors longueur), chips Vol/M-S-T/Hum-Push-Len **sans bordure** et sans texte `--`, pastille `+N` assombrie (fond `BG()` + bordure `LINE()`, survol inchangé).
- **[142] Slots Pattern Bank vides vs remplis** : slots occupés = **pastille blanche** en haut à droite du keycap ; slots vides = assombris + fin contour fantôme. Le slot chargé (bleu) reste inchangé.
- **[143] Volumes par défaut** : Kick 0.8→**1.0**, 808 0.9→**1.0**, HiHat 0.3→**0.2**, OpenHH 0.4→**0.3** (registry `sound_settings_default` + `VoiceSettings::*`). N'affecte que les **nouvelles lanes / resets** — les sessions existantes gardent leurs valeurs sauvegardées.

## 2026-07-29 — Stéréo restauré + refonte chaîne saturation (pre-filter réel, volume post-sat) (build 20260729-095139)

**Branche:** `skeuo-vector` · **Build:** `20260729-095139`
**Validation:** `cargo check` 0 warning, `cargo test` OK (194+110+1), `build.ps1 -Install` OK.

- **[135] Stéréo disparu** : la checkbox `Stereo` avait été perdue des schémas `FULL_STD` (Snare, Perc1) et `HIHAT_STD` (HiHat) dans `instrument_registry.rs`. Restaurée ; tests `stereo_capable_voices_expose_the_stereo_checkbox` / `mono_voices_do_not_expose_the_stereo_checkbox` ajoutés.
- **[136] Saturation Pre-Filter morte** : le flag `pre_filter` était écrit par toutes les voix mais **jamais lu** par `SaturationConfig::process()`. Nouveau helper `process_at(pre_stage, x)` : chaque voix appelle la saturation à deux points (avant son filtre, après) et le flag route l'un ou l'autre. Effet audible : pre = le filtre adoucit les harmoniques générées (drive devant), post = la saturation colore le signal filtré (défaut, comportement inchangé).
- **Bug bonus découvert** : la saturation du **HiHat n'était pas câblée du tout** (settings + UI présents, zéro appel DSP). Câblée (mono + stéréo).
- **B8** : toggle `Saturation Pre-Filter` ajouté (special index 8, param legacy `b8_sat_pre` pour le seed des vieilles sessions).
- **[137] Volume post-saturation** : `settings.volume` déplacé **après** la saturation sur Kick, Snare, Tom1-3, Clap, Snare606, B8, Perc1 (avant : il alimentait le drive, donc baisser le volume changeait le caractère). Le drift de niveau analog reste pré-sat (fait partie du caractère du hit). OpenHH/Ride/Cymbal étaient déjà corrects.
- **Attention au changement sonore** : avec saturation active, le niveau perçu peut différer des sessions précédentes (le volume ne drive plus). Sans saturation, rendu identique.
- Tests : `process_at_routes_by_stage`, `test_kick_volume_is_post_saturation`, `test_snare_volume_is_post_saturation`, `test_hihat_saturation_is_wired`.

## 2026-07-29 — [SK-cleanup] Zéro warning + sliders plock sur le track skeuo (build 20260729-090446)

**Branche:** `skeuo-vector` · **Build:** `20260729-090446`
**Validation:** `cargo check` 0 warning, `cargo test` OK (188+104+1), `build.ps1 -Install` OK.

- **18 warnings éliminés** : imports inutilisés (`cargo fix`), code mort de l'ère textures/anciens essais de rendu supprimé — `skeuo::pad`, `widgets::radial_rect` / `soft_glow`, `envelope_viz::draw_env_label`, helper `theme::blue_glow`, constantes `RADIUS_TAG` / `RADIUS_PAD_TEX` / `KEYCAP_BORDER`, tokens de skin jamais lus (`panel3`, `danger_dim`, `danger_soft`, `handle`, `mute_fill`, `envelope_bg` retirés du struct `Theme` + des 3 skins), champs `TrackStyle.corner` / `handle_r` jamais lus.
- **`LocalParamSlider` centralisé** : les sliders des menus P-lock / Morph / Seq P-lock ne dessinent plus leur propre fond/fill/bordure — ils passent désormais par **`skeuo::slider_track`** (sillon creusé + fill pilule bleue), même rendu que les sliders du Sound Editor, des lanes et du header. Le fond plein carré est remplacé par la groove encastrée dans ces menus.
- Annulées à la demande : SK-2 (intégration `skeuo_theme.rs`) et SK-14 (patterns sur plaque dédiée) — retirées du TODO.

## 2026-07-28 — Grid : suppression de tooltips (build 20260728-224103)

**Branche:** `skeuo-vector` · **Build:** `20260728-224103`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK.

- Retiré les `on_hover_text` du grid qui n'affichaient pas de paramètre : **grip** (« Drag to reorder lane »), **nom de lane** (nom complet de l'instrument), **lane vide** (« Choose an instrument for this slot »), **cellule vide** (« Empty slot - step N »). Les curseurs (`Grab` / `PointingHand`) sont conservés.
- Puis, sur demande, retiré aussi les tooltips **Volume**, **M** (Mute), **S** (Solo) et **T** (Test) — passage d'une chaîne vide aux helpers `draw_mini_value_slider` / `draw_tag_*` (build 20260728-224103). Restent uniquement les tooltips des mini-sliders **Hum / Push / Len**.

## 2026-07-28 — Couleur du panneau Lane Editor + biseau de jonction (build 20260728-175826)

**Branche:** `skeuo-vector` · **Build:** `20260728-175826`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK.

- Le panneau droit (Lane Editor) était resté peint à l'**ancien `PANEL()` flat** `rgb(20,20,25)` (quasi noir) → il ressortait comme un rectangle noir sur le châssis skeuo éclairci. Nouvelles constantes skin-indépendantes : `PANEL_SKEUO` `rgb(37,38,43)` (corps, = ton des plaques), `PANEL_SKEUO_HEADER` `rgb(44,45,51)` (bandeau titre + onglets inactifs), `PANEL_SKEUO_HOVER` `rgb(52,53,60)` (survol onglets). Appliqué au fond du panneau (`ui.rs`), au header et aux onglets Sound/Track (`sound_editor.rs`).
- **Biseau de jonction gauche/droite** : l'ancien séparateur `vline` était peint AVANT le fond du panneau → recouvert (invisible une fois les deux surfaces au même gris skeuo). Refait en biseau dessiné en dernier : creux sombre `PANEL_BORDER` `rgb(18,18,21)` + liseré clair `rgb(58,59,66)` côté panneau.

## 2026-07-28 — Menus contextuels egui à la norme skeuo (Lot 2 modals) (build 20260728-165711)

**Branche:** `skeuo-vector` · **Build:** `20260728-165711`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK.

- **Frame des menus egui natifs relevé** (Visuals globales) : `window_fill` plaque-mid `rgb(41,42,47)`, `window_stroke` bordure sombre `rgb(20,20,24)`, `popup_shadow` ombre douce. Ne touche QUE les menus contextuels + tooltips (nos popups maison utilisent `Frame::NONE` + plaque peinte à la main).
- **Nouveaux helpers `menus::context_menu_button` / `context_menu_separator`** : rangée **keycap pleine largeur** (label en accent, grisé + sans feedback quand `enabled=false`, chaînable `.on_hover_text`) et trait de séparation discret.
- **4 menus clic-droit convertis** : nom de lane (Copy/Paste Lane, Paste Grid, Clear/Delete Lane avec confirmation en rouge, Randomize), lane vide (Paste Lane), longueur de lane (Follow pattern length), block Song (Copy/Paste/Duplicate/Clear). Fini le look « gris brut » egui — relief + keycaps partout.
- **Largeur fixée à chaque menu** (`min`+`max` = largeur du plus long label) pour éviter que les keycaps ne s'étirent : lane 148, longueur 150, lane vide 118, Song 110 (build 20260728-165711).

## 2026-07-28 — Popups « maison » en plaque skeuo (Lot 1 modals) (build 20260728-162504)

**Branche:** `skeuo-vector` · **Build:** `20260728-162504`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK. Design validé en mockup local (`ui-lab/popup_frame.png`).

- **Nouveau `skeuo::plate_shape`** : plaque en relief (ombre douce empilée + fond + dégradé mesh + bordure + liseré) renvoyée comme `Shape` composite ; posée via un **slot réservé** (`painter.add(Noop)` → `painter.set`) pour être peinte DERRIÈRE le contenu du popup une fois son rect connu.
- **`menus.rs`** : `plock_menu_frame` / `page_menu_frame` → plaque skeuo (fini l'aplat `P_ACTIVE` + barre d'accent) ; header `×` → **✓ discret dessiné** (les changements plock sont live → « terminé », pas « annuler ») + fin trait d'accent ; `plock_menu_action_row` → **keycap** (label en accent, rouge pour destructif).
- Corrige d'un coup : **Plock son / Morph / Séquenceur** (via plock_menu_frame), **Add Module** + **Menu de page** (via page_menu_frame), **⚙ Settings** (frame). **Warning preset de lane** repassé aussi en plaque + boutons `chip_button`.

## 2026-07-28 — Logotype FLASH DRUM en bitmap baké, affiché 1:1 (build 20260728-155147)

**Branche:** `skeuo-vector` · **Build:** `20260728-155147`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK.

- **Logotype du header = bitmap baké** (`assets/logotype.png`, 164×48, FLASH gris oblique + DRUM blanc droit), croppé sur son contenu (x3..162 / y12..36) et **affiché 1:1** (24px → 24pt) : egui n'ayant PAS de mipmaps, toute réduction d'une texture plus grande que sa taille écran crénèle — d'où l'affichage à sa taille native. `v0.1.0 · build` reste en Plex Mono live à droite.
- **Abandon du texte oblique en mesh** (`skewed_text` retiré) : cisailler le galley tessellisé cassait l'anti-aliasing. Les 2 fontes condensées **retirées** du binaire.

## 2026-07-28 — Lane Editor SK-16 + norme du bouton/menu Settings (Lot 3) (build 20260728-111256)

**Branche:** `skeuo-vector` · **Build:** `20260728-111256`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK.

- **[SK-16] Mode Notes** : les boutons `-`/`+` du stepper de fréquence deviennent des **keycaps avec flèches ◂ ▸ peintes** (triangles, pas des glyphes).
- **[SK-16] Graphe ADSR** : plus de lettres A/D/R **sur la courbe** → **légende en bas** (carré coloré + lettre, A ambre / D bleu / R violet), 14px réservés en bas du LCD.
- **[SK-16] Dropdowns** (`styled_select`) : s'ouvrent **vers le haut** quand il n'y a pas la place en dessous (près du bord bas de la fenêtre).
- **Norme Settings** : bouton **Settings** du header → keycap ; **× de fermeture** du menu → keycap.

## 2026-07-28 — Generator 2 rangées + Header groupe Seq (Lot 2) (build 20260728-102242)

**Branche:** `skeuo-vector` · **Build:** `20260728-102242`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK. Designs validés en mockups locaux (`ui-lab/generator2.png`, `ui-lab/header_seq.png`).

- **[SK-15] Generator sur 2 rangées** : R1 `Type · A [style] · Mix · B [style]`, R2 `Density · Variation · GENERATE` (droite). Respire vs la rangée unique tassée.
- **[SK-11] Header** : groupe **Seq** = segmented Internal/Ext MIDI (**moitiés symétriques** via nouveau `skeuo::segmented_equal`) + **MIDI Pat** collé juste après ; **Auto-Edit retiré du header** → déplacé dans le menu **⚙ Settings** (toggle skeuo, `draw_settings_popup_if_any` reçoit désormais `setter`).

## 2026-07-28 — Switch + tags M/S/T + nom de lane en skeuo (Lot 1) (build 20260728-095822)

**Branche:** `skeuo-vector` · **Build:** `20260728-095822`
**Corrections post-test :** tag **M** en **rouge** actif (texte blanc, était ambre) ; **T** flashe ambre **au clic** aussi (plus seulement au MIDI externe) ; **nom de lane aplati** (dégradé doux + sans liseré → moins bombé).

**Validation:** `cargo check` OK, `build.ps1 -Install` OK. Designs validés en mockups locaux (`ui-lab/switches.png`, `ui-lab/tags_lane.png`).

- **[SK-7] `skeuo::switch`** : glissière encastrée (sombre → bleue) + bouton rond métal qui glisse (animé), sans ligne foncée en haut ni reflet interne. `ToggleSwitch` (Stereo, Saturation Pre-Filter, Main Mix) routé dessus.
- **[SK-10a] `skeuo::tag`** : tags M/S/T en mini-keycap relief, gris inactif / accent coloré actif (M rouge, S bleu, T ambre). `draw_tag_button_v2` routé dessus.
- **[SK-10b] `skeuo::lane_name`** : nom de lane = keycap (gris repos / bleu sélectionné) texte à gauche. `draw_lane_name_v2` routé dessus.

## 2026-07-27 — Renommage « Lane Editor » + bascule Sound/Track en onglets à ras (build 20260727-181804)

**Branche:** `skeuo-vector` · **Build:** `20260727-181804`
- **En-tête du panneau renommé « Sound Editor » → « Lane Editor »** (le contexte « Slot N – nom » reste).
- **Bascule Sound | Track** repensée en **onglets à ras** : pleine largeur 50/50, h30, **sans radius**, hairline entre les deux + bord bas, **bleu plein** actif / **couleur plaque** inactif (s'éclaircit au survol). C'est le seul segmenté sans keycap (volontaire). L'onglet « Sound Editor » devient « Sound ».
- Sélecteur d'instrument 14-keycaps **écarté** (décision : la sélection de lane via la grille suffit).

## 2026-07-27 — Pattern Bank + section Fusion en skeuo (build 20260727-175112)

**Branche:** `skeuo-vector` · **Build:** `20260727-175112`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK. Design validé en mockups locaux (`ui-lab/pattern_bank.png`, `ui-lab/fusion_section.png`).

### Pattern Bank (ligne du bas)
- **Save** et **Clr** passent de `egui::Button` bruts (fond plat clignotant) à des **keycaps** : Save armé = keycap bleu (`keycap_button`) ; Clr → `Sure?` rouge en confirmation (`chip_button`).
- **Disposition revue** : slots P1-P8 **groupés** juste après le label, Save+Clr collés après les slots, **Export/Drag repoussés à droite** (`right_to_left`).

### Section Fusion
- Refonte du `draw_fusion_edit_box` : ancien `Frame` PANEL3 + mini-boutons egui `Del`/`×` → **strip en relief** (`fusion_strip_bg` : dégradé + bordure + liseré) avec **contenu centré**.
- **Repos** : `FUSION` + touche **`Maj` ambre** + « + glisser pour fusionner » (ou « Sélectionne 2 cellules » en mode fusion).
- **Édition** : `Fusion N–M` + `Steps` (DragValue) + `Morph: Off`/liste + **Del** (rouge) + **×** en keycaps compacts (h19, ne touchent plus les bords).

## 2026-07-27 — Finitions blocs Song (build 20260727-163615)

**Branche:** `skeuo-vector` · **Build:** `20260727-163615`
- **LED des blocs Song** décalée vers l'intérieur (coin −11px) pour ne plus toucher le bord, comme la page-bar.
- **Sélecteur de pattern des blocs** : nouveau `styled_select_centered` (code centré, **sans flèche** ▾) — juste `P1`/`--` centré ; le clic ouvre toujours le picker.
- **Bouton `Clear All` du Song** passé à la norme keycap (`chip_button`) : label rouge en confirmation (`Confirm?`).

## 2026-07-27 — LED de lecture incrustée (page-bar + blocs Song) (build 20260727-142106)

**Branche:** `skeuo-vector` · **Build:** `20260727-142106`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK. Design validé en mockup local (`ui-lab/page_leds.png`).

- **Nouveau `skeuo::play_led`** : LED rouge de position de lecture (verre radial + reflet, sans halo), à incruster dans le coin haut-droit.
- **Page-bar** : la LED rouge passe de **sous** le bouton au **coin haut-droit** du bouton de page en lecture (grid.rs).
- **Blocs du Song** : le bloc en cours de lecture n'est plus rempli en bleu → même LED rouge dans son coin haut-droit ; la sélection (bloc édité) garde sa bordure bleue.

## 2026-07-27 — Boutons à LED du header en skeuo (build 20260727-124102)

**Branche:** `skeuo-vector` · **Build:** `20260727-124102`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK. Design validé en mockup local (`ui-lab/led_buttons.png`) avant câblage.

- **Nouveau `skeuo::led`** : pastille indicatrice rétroéclairée (verre radial en mesh éventail + reflet spéculaire), **sans halo lumineux** (retiré à la demande). Bleue allumée, sombre éteinte.
- **`ToggleLED` (Choke / Auto-Edit / MIDI Pat)** repensé : **pilule keycap grise** (le fond ne change plus selon l'état) + `skeuo::led` à gauche + label. Désactivé (MIDI Pat en Ext MIDI) = touche grisée + LED éteinte + label estompé. Suppression de l'ancien fond plat teinté + halo bleu.

## 2026-07-27 — Switches segmentés harmonisés au langage keycap (build 20260727-122735)

**Branche:** `skeuo-vector` · **Build:** `20260727-122735`
- **Ligne Frequency (Kick/808)** : le switch `Hz`/`Note` passe **avant** le slider (juste après le label, position fixe) au lieu d'être à droite — il ne saute plus quand on bascule Hz↔Note, et ça suit la maquette (switch puis slider ; valeur toujours alignée à droite).
**Validation:** `cargo check` OK, `build.ps1 -Install` OK. Design pré-validé au lab + montré à l'utilisateur (aperçu web du rendu egui réel) avant câblage.

- **Nouveau renderer unique `skeuo::segmented`** : puits creusé (rect arrondi + ombre haute corner-safe + bordure, **sans dégradé mesh** → pas de coin carré qui dépasse) + option active en **keycap bleu sans ombre portée** (l'ombre baverait hors du puits) + options inactives en texte discret cliquable. Segments dimensionnés au texte.
- **3 anciens renderers supprimés/remplacés** : `text_segmented` (controls.rs) → `p_lock_mode_segmented` (Sound/Sequencer) + `generator_song_segmented` (Generator/Song) appellent `skeuo::segmented` ; `led_segmented` (widgets.rs, LED bleue) supprimé → Seq Mode (Internal/Ext MIDI) dans header.rs ; `draw_note_freq_mode_toggle` (Hz/Note, aplat bleu) réduit à un appel `skeuo::segmented`.
- **`keycap` refactorée** en `keycap_body(p, rect, state, shadow)` : `shadow=false` quand la touche est dans un puits ; mesh de dégradé insetté à 2.2 (≥ r×0.42) pour ne jamais dépasser le rayon.

## 2026-07-27 — Grid en atlas bitmap + dropdowns skeuo + alignement onglets Sound/Track (build 20260727-112050)

**Branche:** `skeuo-vector` · **Build:** `20260727-112050`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK. Alignement pré-validé dans le lab egui headless (scènes `sound_panel` / `track_panel`, sorties `ui-lab/`).

### Alignement Sound / Track (comme la maquette `onglet01.png`)
- **Bug racine : colonne de labels non fixe.** `editor_label` utilisait `allocate_ui_with_layout(138, …)` qui **rétrécit à la largeur du texte** → les débuts de sliders étaient en escalier. Fix : `allocate_exact_size(138)` + `painter.text` → colonne vraiment fixe, tous les contrôles démarrent au même x.
- **Sliders pleine largeur** dans les sections sans graphe (valeurs/dropdowns/toggles calés à droite) ; sections à graphe (Envelope/Filter) gardent la colonne 340 pour loger le graphe.
- **Dropdowns calés à droite** (Click/Noise/Saturation Type, Algorithm) via `right_to_left`.
- **Onglet Track reconstruit** avec le même système de lignes (Name, Type, Main Mix, Aux Out, Channel, Note, Length) + en-têtes de section `editor_section_header`, au lieu de l'empilement vertical.

### Changements
- **Cellules du séquenceur = atlas bitmap du designer** (`assets/pads/atlas-pads.png` + `.json`, 69 sprites : 9 pas simples + 60 fusions N=2..16). Nouveau `src/ui/pads.rs` : parse le manifeste une fois (`OnceLock`), mappe (état + span de fusion) → sprite, blitte via UV. Appelé depuis `grid.rs::draw_step_cell_v2`. Overlays (playhead, chiffre de pulses) restent vectoriels par-dessus.
- **Bleed opaque contourné** : l'atlas a été baké avec un fond **opaque** `(11,11,14)` (et non alpha transparent comme annoncé au README) → le bleed d'une cellule écrasait le bord droit de sa voisine (troncature de quelques px). Fix : on blitte **uniquement la zone utile** (`ew×eh`, bleed rogné) dans la cellule, sans chevauchement. Compromis : ombre/halo bakés dans le bleed perdus (re-baker en alpha pour les récupérer).
- **Coins pads** : `Image::corner_radius(6)` — rogne légèrement dans le corps du pad pour casser les angles (les coins de l'atlas hors radius = fond sombre ≈ well, donc un petit radius était invisible).
- **Dropdowns → skeuo** (`styled_select`) : onglet Track (kind, out), preset de lane, sélecteurs de pattern du Song.

### Reste à faire
- Alignement des items dans les onglets Sound / Track (cf. `screenshots/onglet01.png`).
- Écrasement vertical léger des pads (cellules 21pt vs bake 26pt) — à trancher si gênant.
- LED / switch / tags → `skeuo::*` (encore versions backup).
- Nettoyer les ~13 warnings de code mort (textures/handles obsolètes).

---

## 2026-07-26 — Skeuo VECTORIEL dans le plugin + module `skeuo.rs` centralisé (build 20260726-184543)

**Branche:** `skeuo-vector` (repartie de `backup/skeuo-redesign`) · **Build:** `20260726-184543`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK. Rendu pré-validé via un previewer egui headless **pixel-fidèle** (`egui_kittest` + wgpu, crate scratch `egui_lab`, sorties `ui-lab/`) avant chaque build.

### Changements
- **Nouveau module `src/ui/skeuo.rs`** = maison unique des éléments graphiques : une fonction par élément (`keycap`, `pad`, `slider_track`, `well_recess`, `lcd_bg`), appelée par TOUS les sites d'appel. Changer un élément (vectoriel OU bitmap) = une seule édition. Primitives bas-niveau (dégradés mesh, ombres, radial…) dans `widgets.rs`.
- **Tout le look bascule en pur vectoriel egui** (fini les textures PNG) : dégradés en mesh à sommets colorés (lisses, sans banding), ombres douces empilées, ombres de creux qui épousent les coins.
- **Keycaps** (`skeuo::keycap`, relais `widgets::keycap_tex`) → boutons/pages/slots/GENERATE/segmented/dropdown.
- **Pads** (`skeuo::pad`) : minimal — rect arrondi plein + bordure, **sans glow ni radial** (qui bavaient sur les voisins / faisaient un blob).
- **Puits de grille** (`skeuo::well_recess`) : ombres haut + gauche + droite, fondues, corner-safe.
- **Sliders** (`skeuo::slider_track`, UNIQUE pour Len + Vol/Hum/Push + ENV) : sillon creusé + fill pilule ; **capuchon strié** pour les gros sliders, pas les mini.
- **Écran ADSR** (`skeuo::lcd_bg`) : verre vert CRT + creux + scanlines + bord vert-noir, courbe par-dessus.

### Reste à faire
- Nettoyer ~13 warnings (code textures/handles obsolète : `pads::pad_source_for`, `RADIUS_PAD_TEX`, `KEYCAP_BORDER`, `HANDLE`, `ENVELOPE_BG`, primitives pas encore câblées).
- **LED / switch / tags** → `skeuo::*` (encore versions backup).
- Châssis / plaques (fonds de panneaux).
- Centraliser `local_param_slider` (sliders menus p-lock) vers `skeuo::slider_track`.

---

## 2026-07-24 — Skeuo : keycaps en textures bakées (chips, pages, slots, dropdowns, GENERATE) (build 20260724-153847)

**Build:** `20260724-153847`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK. Texture keycap validée hors Studio One via un previewer maison (rendu PNG) ; assemblage à valider dans Studio One.

### Changements
- **Nouvelle méthode de travail UI** : les surfaces skeuo (dégradés) rendaient mal en vectoriel egui (banding, artefacts de coins, liseré = « trait blanc moche »). On bake désormais des **textures PNG haute-déf** que je valide **en image de mon côté** (outil `image` dans le scratchpad, sorties dans `ui-preview/` gitignoré) avant tout build — fin de la boucle d'itération aveugle dans Studio One.
- **Keycaps = textures bakées, posées en 3-tranches horizontal** (coins 1:1, milieu étiré → coins nets à toute largeur, hauteur 1:1 sur `CTL_HEIGHT`). 3 états : `keycap-rest/blue/amber.png` dans `assets/keycaps/`. Nouveau `widgets::keycap_tex()` + helper unifié `controls::keycap_button()`.
- **Éléments passés en keycap** : chips presets/actions + segmented (déjà faits), **pages 1-4** + Follow + longueurs 16/32/48/64 + x2, **slots P1-P8** de la banque (chargé=bleu, occupé=repos, vide=assombri), **dropdowns** (`styled_select`, liseré bleu au survol), **GENERATE** en **ambre** (`PressedAmber`).
- Feedback clic « enfoncé » (assombrissement) sur les keycaps ; pas de retour au survol (préférence utilisateur). Plus de liseré clair.
- Save/Clr de la banque et LED rouge de page inchangés (hors périmètre / effets dédiés).

---

## 2026-07-23 — Skeuo : pads en textures + rayons harmonisés (build 20260723-155528)

**Build:** `20260723-155528`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK, coins des pads validés dans Studio One.

### Changements
- **Pads du séquenceur rendus via les textures PNG du designer** (`assets/pads/pad-*.png`) au lieu d'un dessin vectoriel — chargées par le loader `egui_extras` + `include_image!` (installé au démarrage de l'éditeur). Mapping état→texture (hit/link/seq/off/off-beat/off-link/off-snap/seq-off) ; fusion/édition/sélection gardent le rendu vectoriel.
- **Rayons harmonisés au thème Skeuo** (`RADIUS.md` / `SPEC-COMPUTED.md`) : panneaux/popups 7, keycaps 5, tags 3, nom de lane 4, pads 4, ADSR 4.
- **Fit des coins de pad** : egui-baseview ne sait pas arrondir une texture, donc les overlays vectoriels d'un pad (anneau de lecture, surbrillance de survol) utilisent le coin RÉEL de la texture (`RADIUS_PAD_TEX = 2 px`, mesuré) au lieu de 4 px — l'anneau épouse le pad, plus de coin carré qui dépasse.
- Dépendances : `image` (png) + `egui_extras` (image loader).
- Plan complet de la refonte visuelle ajouté à `TODO.md` (section `[SKEUO]`, SK-0 → SK-16).

---

## 2026-07-22 — Fenêtre 1480×800 (cible designer) + mise en page resserrée (build 20260722-161751)

**Build:** `20260722-161751`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK, vérifié dans Studio One (rien de coupé).

### Changements
- **Fenêtre repassée de 1480×900 à 1480×800**, la cible du pack designer (`EguiState::from_size` + `ResizableWindow` min/fixed). Le 900 était un dépannage temporaire pour l'ancienne rangée « + Add Module », depuis supprimée.
- **Mise en page verticale resserrée pour tenir dans 800** (fondation du nouveau design, avant le look) :
  - Panneau bas Generator/Song : hauteur fixe 210 → **168** (Generator ~70px de corps, Song ~110px).
  - Rythme vertical de la colonne gauche : `item_spacing.y` 16 → **10**.
  - Marge interne de la plaque grille : `inner_margin` 11 → **8**.
- Aucun changement de comportement audio ni de persistance.
- Prochaine étape : le look « Skeuo » (matières hardware) puis les retouches restantes du tri `CHANGES.md`.

---

## 2026-07-21 — Pack designer revue UI (docs, pas de build)

**Type:** documentation uniquement (aucun changement de code, pas de nouveau build).

### Changements
- **`design-pack/ui-review-2026-07/`** : pack complet pour revue UI par le designer.
  - `README.md` — vue d'ensemble, évolutions depuis la maquette de juin, liste des 10 captures à fournir.
  - `UI-STATE.md` — inventaire de l'UI implémentée (header, pattern bank, page bar, grille 14 lanes, états cellules, interactions, bottom panel, Sound Editor, popups, skins).
  - `SKINS.md` + `skins.json` — palettes des 3 skins (Dark/Midnight/Ember) par token, version JSON.
  - `TOKENS.md` — tailles, rayons, colonnes de grille, typographie IBM Plex, règles visuelles.
  - `screenshots/` — dossier prêt pour les captures (1480×900).

---

## 2026-07-21 — [AUDIT-Q6] Découpage de `ui.rs` en modules thématiques (build 20260721-170521)

**Build:** `20260721-170521`
**Validation:** `cargo test` OK (188 lib + 1 midi_drag_helper + 104 test_standalone), `cargo check` OK (0 warning), `build.ps1 -Install` OK

### Changements
- **`src/ui.rs` réduit de ~8 060 à 366 lignes** : ne garde que les déclarations de modules, `install_egui_fonts` et l'orchestration `create_editor`. Aucun changement de comportement ni de rendu.
- **Nouveaux modules dans `src/ui/`** :
  - `editor_state.rs` — `EditorUIState`, structs popup/clipboard, opérations lane (copy/paste/clear/randomize), helpers plock par lane.
  - `menus.rs` — chrome des popups (frame/header/rows).
  - `fmt.rs` — formatage valeurs/notes/fréquences.
  - `controls.rs` — setters param, length control, modifier fusion, chips/combos/segmenteds/LED.
  - `midi.rs` — export MIDI + helper drag Windows (déplacé depuis ui.rs).
  - `header.rs` — barre d'en-tête + `header_param_slider`.
  - `pattern_bank.rs` — barre Patterns P1-P8 + save/load pattern.
  - `bottom_panel.rs` — panneau Generator | Song + contrôles générateur.
  - `song.rs` — éditeur de song chain.
  - `popups.rs` — Add Module, page menu, Settings, lane preset warning.
  - `sound_editor.rs` — Sound/Track tabs, lignes éditeur, presets de layout, `store_field`.
  - `grid.rs` — grille pattern, step cells, fusion, reorder de lanes, page bar, mixer_rows.
  - `plock.rs` — menus p-lock sound/morph/sequencer + popup.
- **`AGENTS.md`** : structure UI documentée.

### À tester dans Studio One (build 20260721-170521)
1. Ouvrir le plugin → l'interface est **visuellement identique** à la build précédente (header, grille, panneaux).
2. Grille : toggle steps, p-lock clic droit (Link/Snapshot), fusion (shift+clic), drag de step, reorder de lane, Len lock, pages 1-4.
3. Sound Editor : onglets Sound/Track, sliders toutes familles, Hz/Notes sur Kick/808, algo combo, presets de layout (Preset 4/12, Clear All).
4. Pattern bank : Save/Load P1-P8, copy/paste slot, Clear, Export/Drag MIDI.
5. Bottom panel : Generator (presets, GENERATE) + Song editor (blocks, repeats, clear).
6. Popups : Settings (analog, MIDI channel, skin), Add Module sur lane vide.
7. Régression : skins, song mode, multi-out, sauvegarde/rechargement projet OK.

---

## 2026-07-21 — [AUDIT-Q5] Validation du chemin du MIDI drag helper (build 20260721-152237)

**Build:** `20260721-152237`
**Validation:** `cargo test` OK (188 lib + 1 midi_drag_helper + 104 test_standalone), `cargo check` OK (0 warning), `build.ps1 -Install` OK

### Changements
- **`src/ui.rs`** : `find_midi_drag_helper()` ne fait plus confiance aveuglément à `DRUM_FLASH_MIDI_DRAG_HELPER`. Nouvelle validation `is_valid_helper_candidate()` : le fichier doit exister, porter le nom exact `drum-pattern-midi-drag-helper.exe`, et vivre sous un prefix bundle connu (`%CommonProgramFiles%\VST3\drum-pattern-vst.vst3` ou le bundle local `build/`), vérifié sur chemins **canonisés** (impossible de sortir du prefix via `..`).
- Tests ajoutés (Windows) : accepte helper dans le bundle, rejette mauvais nom de fichier, rejette fichier hors prefix, rejette traversée `..`, rejette fichier manquant.

### À tester dans Studio One (build 20260721-152237)
1. Bouton `Drag` (barre Patterns) → le helper de drag MIDI se lance et la fenêtre de drag apparaît comme avant.
2. Bouton `Export` → le fichier `.mid` est bien créé dans `Documents/Flash Drum/exports`.
3. (Optionnel) Positionner `DRUM_FLASH_MIDI_DRAG_HELPER` vers le helper du bundle installé → le drag fonctionne toujours.
4. (Optionnel) Positionner `DRUM_FLASH_MIDI_DRAG_HELPER` vers un autre exécutable → le drag est refusé (le bouton affiche une erreur au lieu de lancer le programme).
5. Régression : skins, plocks, song mode OK.

---

## 2026-07-21 — [SKIN-1] Système de skins UI (build 20260721-150308)

**Build:** `20260721-150308`
**Validation:** `cargo test` OK (183 lib + 1 midi_drag_helper + 104 test_standalone), `cargo check` OK (0 warning), `build.ps1 -Install` OK

### Changements
- **`src/ui/theme.rs`** réécrit : les couleurs passent de constantes compile-time à une struct `Theme` runtime (`static ACTIVE: RwLock<&'static Theme>`). Tokens accessibles via des fonctions du même nom (`BG()`, `BLUE()`, …). Nouveaux tokens sémantiques : cellules (`CELL_EMPTY_BEAT/OFF`, `CELL_CURRENT`, `CELL_DISABLED`, `FUSION_FILL`, `CELL_SEQPL_OFF`, `CELL_PL_SNAP/LINK_OFF`, `SONG_EMPTY`), feedback (`DANGER*`, `DRAG_TARGET`, `HANDLE`, `MUTE_FILL`, `SOLO_FILL`), graphes (`ENVELOPE_BG/CURVE`), surface `PANEL3`.
- **3 skins intégrés** : `Dark` (palette actuelle, rendu identique), `Midnight` (bleu nuit), `Ember` (chaud/ambré). `theme::set_skin()`, `skin_name()`, `SKINS`.
- **Migration** : ~34 couleurs hardcodées dans `ui.rs`, `slider.rs`, `envelope_viz.rs` remplacées par des tokens ; les animations Save/Clear dérivent désormais de `BLUE()`/`DANGER()` ; bordure noire des cellules désactivées en `Color32::BLACK`.
- **Persistance** : `GlobalConfig.skin` (JSON `Documents/Flash Drum/config.json`), appliqué à l'ouverture de l'éditeur.
- **Settings popup** : sélecteur `Skin` (Dark / Midnight / Ember), effet immédiat, sauvegardé.
- Tests : switch de skin met à jour les tokens, skin inconnu ignoré, `blue_glow` suit l'accent actif (comparaison `to_opaque()` car `Color32` stocke du prémultiplié linéaire).

### À tester dans Studio One (build 20260721-150308)
1. Ouvrir le plugin → l'interface est **identique à avant** (skin Dark par défaut).
2. Menu `⚙ Settings` → `Skin` → choisir `Midnight` : toute l'UI passe en bleu nuit, immédiatement.
3. Choisir `Ember` : accents chauds/ambrés ; revenir à `Dark` : rendu d'origine.
4. Fermer et rouvrir le plugin → le skin choisi est restauré (persistance config.json).
5. Vérifier les zones tokenisées : steps de la grille (vides/playhead/plock orange/seq violet), boutons rouges (Clear Plock, Delete Fusion), enveloppes ADSR, boutons M/S.
6. Régression : plocks, générateur, song mode, export MIDI OK.

---

## 2026-07-21 — [AUDIT-Q4] Unification des implémentations de sliders (build 20260721-142452)

**Build:** `20260721-142452`
**Validation:** `cargo test` OK (182 lib + 1 midi_drag_helper + 104 test_standalone), `cargo check` OK (0 warning), `build.ps1 -Install` OK

### Changements
- **Nouveau module `src/ui/slider.rs`** : logique partagée `normalize_value` / `denormalize_value` (linéaire + logarithmique avec fallback linéaire si domaine invalide) et track core `draw_track` avec style paramétrable (`TrackStyle::EDITOR` / `TrackStyle::MINI`). Tests unitaires : roundtrips lin/log, clamping, fallback, ancres endpoints.
- **`src/ui.rs`** : `draw_editor_slider_track` et `draw_mini_value_slider` sont maintenant des wrappers fins sur `slider::draw_track` (rendu visuel identique) ; les helpers dupliqués `normalize_slider_value` / `denormalize_slider_value` sont supprimés.
- **`src/ui/local_param_slider.rs`** : le mapping log/linéaire interne est remplacé par les helpers partagés (rendu et shift-drag granulaire inchangés).
- **`header_param_slider`** : inchangé (param-bound, logique déjà simple).

### À tester dans Studio One (build 20260721-142452)
1. Sound Editor : draguer les sliders d'une section (Volume, Decay, Filter...) → valeurs et fill suivent le curseur comme avant.
2. Double-clic sur un slider d'éditeur → reset à la valeur par défaut.
3. Sliders Hum/Push dans les lanes de la grille → drag + tooltip de valeur fonctionnent.
4. Menu p-lock (clic droit sur une step) : draguer un slider de paramètre (Volume, Freq log) → la valeur suit ; shift+drag = réglage fin granulaire.
5. Header Master/Swing → drag + double-clic reset inchangés.
6. Régression : plocks, générateur, song mode, export MIDI OK.

---

## 2026-07-20 — [AUDIT-Q3] Snapshot JSON de la Pattern Bank hors verrou (build 20260720-154200)

**Build:** `20260720-154200`
**Validation:** `cargo test` OK (177 lib + 1 midi_drag_helper + 104 test_standalone), `cargo check` OK (0 warning), `build.ps1 -Install` OK

### Changements
- **`src/pattern_bank.rs`** : `PersistentPatternBank` maintient maintenant un snapshot JSON atomique (`AtomicPtr<Vec<u8>>`). La sérialisation n'a plus lieu dans `map()` sous verrou : elle est faite dans `refresh_snapshot()` à partir d'une copie de la bank, puis publiée de façon atomique. `map()` lit ce snapshot sans toucher au `Mutex<PatternBank>`.
- **`src/lib.rs`** : après `save_pattern_to_slot()`, le snapshot de persistance est rafraîchi une fois la mutation appliquée.
- **`src/ui.rs`** : rafraîchissement du snapshot après save/paste de slot et après modification de la song chain.
- Tests ajoutés :
  - `persistent_bank_snapshot_tracks_explicit_refresh`
  - `persistent_bank_map_uses_snapshot_even_when_bank_is_locked`

### À tester dans Studio One (build 20260720-154200)
1. Sauvegarder un pattern dans un slot `P1`…`P8` puis le recharger → grille, plocks, fusions et length doivent être restaurés normalement.
2. Copier/coller un slot de pattern (clic droit sur slot occupé puis clic droit sur slot vide) → le slot collé doit être persisté correctement.
3. Modifier la song chain (steps, repeats, clear, duplicate) → sauvegarder/recharger le projet : la chaine doit être restaurée telle quelle.
4. Régression : lecture, song mode, pattern bank save/load en lecture continue ne doivent ni bloquer ni sauter de pattern.

---

## 2026-07-19 — Fix p-lock creation preserving active step (build 20260719-112838)

**Build:** `20260719-112838`
**Validation:** `cargo test` OK (175 lib + 1 midi_drag_helper + 104 test_standalone), `cargo check` OK (0 warning), `build.ps1 -Install` OK

### Changements
- **`src/ui.rs`** :
  - `PlockPopup` mémorise maintenant `step_was_active` au moment du clic droit.
  - `draw_plock_menu` appelle `preserve_step_active_from_plock_popup()` : si le clic sur `Link to Global` ou `Snapshot Current Settings` a éteint la step sous-jacente, le trigger est restauré immédiatement.
  - `draw_sequencer_plock_menu` applique le même garde-fou : créer/modifier/clear un sequencer p-lock ne peut plus désactiver la step active.
  - `step_colors_v2` : rendu plein restauré pour les p-locks actifs : sound p-lock = cellule pleine orange, seq p-lock = cellule pleine violette.
  - **`src/ui/theme.rs`** : suppression de `PL_SNAP`, devenu inutilisé après retour au rendu plein orange pour les sound p-locks.
  - `draw_legacy_slot_lane_v2` : le toggle d'une step cellule est désactivé tant que le popup p-lock est ouvert (`suppress_click` inclut `state.plock_popup.is_some()`). Quand le popup est affiché, tous les clics gauche de la grille sont ignorés pour laisser le popup traiter l'interaction.
  - `draw_grid_v2` : quand le popup p-lock est ouvert en début de frame et qu'un clic gauche arrive, on positionne `state.suppress_step_cell_click` en renfort.
  - Le toggle d'une step cellule ne réagit plus qu'au clic principal (`PointerButton::Primary`), aussi bien en mode normal qu'en mode fusion.
  - Le popup de p-lock passe de `Sense::hover()` à `Sense::click()` : les clics dans la bordure/marge du popup sont consommés au lieu de passer à la cellule sous-jacente.
  - Fermeture du popup lors d'un clic dans sa bordure, et maintien de la fermeture au clic extérieur (`clicked_elsewhere`).

### À tester dans Studio One (build 20260719-112838)
1. Activer une step dans la grille (clic gauche) → la cellule s'allume normalement.
2. Faire un clic droit sur une cellule active → le menu sound p-lock s'ouvre et la cellule reste active.
3. Sélectionner `Link to Global` ou `Snapshot Current Settings` → la cellule reste active et devient full orange.
4. Passer en mode sequencer p-lock, créer/modifier/clear un seq-p-lock sur la même step → la cellule reste active et devient full violette tant que le seq-p-lock existe.
5. Régression : génération de pattern, mute/solo, export MIDI et sauvegarde/rechargement projet fonctionnent normalement.

---

## 2026-07-18 — [AUDIT-QW2] Corrections docs & script de vérification (build 20260718-125431)

**Build:** `20260718-125431`
**Validation:** `cargo test` OK (175 lib + 1 midi_drag_helper + 104 test_standalone), `cargo check` OK (0 warning), `build.ps1 -Install` OK

### Changements
- Suppression du fichier obsolète `drum-pattern-vst/fix_roles.pdb`.
- **`README.md`** : description à jour (64 pas, 13 voix de synthèse dans 14 slots modulaires, Perc1).
- **`drum-pattern-vst/README.md`** : même mise à jour (13 voix dans 14 slots, Perc1, 14 sorties aux + Main Mix).
- **`docs/infrastructure.md`** :
  - 13 instruments → 13 voix de synthèse dans 14 slots modulaires.
  - `beat_position` décrit correctement (0.0 .. master_length × 0.25 beat, défaut 16, max 64).
  - `sound-settings-v1` → `sound-settings-v2` (46 floats/slot).
  - Suppression du champ `global-v1` inexistant ; ajout de `seq-plock-v1` et mention des paramètres nih-plug persistés par le DAW.
  - Nombre de tests unitaires : 76 → 175.
  - Version d'exemple : `0.1.0` → `0.2.0`.
- **`docs/user-guide.md`** : 13 instruments → 13 voix/14 slots, Zap → Perc1, 13 sorties → 14 sorties aux, réglages par instrument → par slot.
- **`docs/analog-mode.md`** : Zap → Perc1.
- **`AGENTS.md`** : chemin du PoC web corrigé (`index.html`/`index.js` → `archive/web-poc/index.html`/`archive/web-poc/index.js`).
- **`test-verification.ps1`** : suppression du build ID figé obsolète ; comparaison SHA-256 entre le bundle local et le binaire installé.

### À tester dans Studio One (build 20260718-125431)
1. Ouvrir le plugin → le header affiche `v0.2.0 · 20260718-125431`.
2. Lecture du séquenceur et déclenchement de chaque slot (1 à 14) → pas de crash, audio OK.
3. Régression : sauvegarde/rechargement d'un projet, plocks, mutes/solos, export MIDI fonctionnent normalement.

---

## 2026-07-18 — [AUDIT-QW1] Nettoyage `println!` + warnings (build 20260718-114531)

**Build:** `20260718-114531`
**Validation:** `cargo test` OK (175 lib + 1 midi_drag_helper + 104 test_standalone), `cargo check` OK (0 warning), `build.ps1 -Install` OK

### Changements
- **`src/lib.rs`** : suppression du `println!` debug dans `fire_voice_trigger`.
- **`src/ui.rs`** : suppression du paramètre `clear_plocks_request` mort dans `create_editor` / `draw_pattern_bank` et de la fonction `led_toggle` inutilisée.
- **`src/sequencer/pattern.rs`** : suppression de `PatternStateV3::expand` (jamais utilisé).
- **`src/synthesis/dsp.rs`** : `#[allow(dead_code)]` sur les méthodes `reseed` de `PinkNoise`/`BrownNoise`/`BlueNoise`.
- **`src/ui/local_param_slider.rs`** : `#[allow(dead_code)]` sur le builder `suffix`.
- **`src/ui/theme.rs`** : suppression des constantes theme inutilisées (`DIVIDER`, `BLUE_D`, `BLUE_DIM`, `BLUE_GLOW`, `RADIUS_PILL`, `STROKE_HAIR`, `STROKE_CURVE`, `GAP_SM`, `GAP_MD`, `GAP_LG`).
- **`src/sequencer/stress_tests.rs`** : `_new_shared` pour la variable inutilisée.
- **`src/synthesis/mod.rs`** : utilisation de `cy_idx` dans le test.

### À tester dans Studio One (build 20260718-114531)
1. Ouvrir le plugin → le header affiche `v0.2.0 · <build ID>` sans changement visuel.
2. Tester une génération de pattern (chaque type) → pas de crash.
3. Ouvrir le menu P-lock sur un instrument multi-algo → le sélecteur Algo fonctionne.
4. Régression : lecture, plocks, mute/solo, song, export MIDI fonctionnent normalement.

---

## 2026-07-18 — [AUDIT-INF1] CI GitHub Actions Windows (build 20260718-112910)

**Build:** `20260718-112910`
**Validation:** `cargo test` OK (175 lib + 1 midi_drag_helper + 104 test_standalone), `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Ajout de `.github/workflows/ci.yml`.
- Workflow Windows (`windows-latest`) : checkout, Rust stable, cache cargo, `cargo check`, `cargo test`, `cargo build --release`, `build.ps1` (bundle VST3), upload artifact `drum-pattern-vst-windows`.

### À tester dans Studio One (build 20260718-112910)
1. Pas de test fonctionnel spécifique (changement d'infrastructure uniquement).
2. Vérifier que le plugin local reste fonctionnel : version `v0.2.0`, build ID, lecture audio.
3. Régression : aucune autre fonctionnalité affectée.

---

## 2026-07-18 — [AUDIT-INF2] Version 0.2.0 affichée dans l'UI (build 20260718-112253)

**Build:** `20260718-112253`
**Validation:** `cargo test` OK (175 lib + 1 midi_drag_helper + 104 test_standalone), `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **`Cargo.toml`** : version passée de `0.1.0` à `0.2.0`.
- L'UI affichait déjà `env!("CARGO_PKG_VERSION")` à côté du build ID dans le header (`src/ui.rs:1464`) ; elle affiche donc maintenant `v0.2.0`.

### À tester dans Studio One (build 20260718-112253)
1. Ouvrir le plugin → dans le header, à côté de "FLASH DRUM", lire la version affichée `v0.2.0 · <build ID>`.
2. Vérifier que le build ID est toujours présent.
3. Régression : aucune autre fonctionnalité affectée.

---

## 2026-07-18 — [AUDIT-RT3] Zéro panic UI/audio (build 20260718-110145)

**Build:** `20260718-110145`
**Validation:** `cargo test` OK (175 lib + 1 midi_drag_helper + 104 test_standalone), `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **`src/ui.rs` (`draw_plock_menu`)** : remplacement de `expect("valid voice index")` par un `if let Some(voice) = ...` défensif ; le menu Algo est simplement ignoré si l'index est invalide.
- **`src/generator/mod.rs` (`generate_bar`)** : suppression du `unreachable!()` catch-all ; le match est maintenant exhaustif avec `GeneratorType::Euclidean => Pattern::empty()` comme fallback sûr.
- **`src/lib.rs` (`Default for DrumFlashVst`)** : remplacement de `lock().unwrap()` sur `pattern_bank.bank` par `try_lock()` + fallback `SongSequence::default()`.

### À tester dans Studio One (build 20260718-110145)
1. Ouvrir le menu P-lock (clic droit sur une step) pour un instrument avec plusieurs algos → le sélecteur Algo doit s'afficher et fonctionner normalement.
2. Générer un pattern avec chaque type de générateur (Probabilistic, Markov, Classic, Euclidean) → aucun crash, résultat conforme.
3. Ouvrir le plugin dans un projet neuf ou recharger un projet → chargement sans panic.
4. Régression : le mode Song, les plocks, le générateur et les algos fonctionnent toujours.

---

## 2026-07-18 — [AUDIT-RT2] `try_lock` dans `initialize()` pour éviter le blocage audio (build 20260718-105213)

**Build:** `20260718-105213`
**Validation:** `cargo test` OK (175 lib + 1 midi_drag_helper + 104 test_standalone), `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **`src/lib.rs` (`initialize()`)** : remplacement du `lock().unwrap()` sur `pattern_bank.bank` par `try_lock()` + fallback sur `SongSequence::default()`.
- Si le bank est verrouillé par l'UI pendant l'init, le thread audio ne bloque pas ; la song est rechargée via le snapshot publié par l'UI et consommé dans `process()`.

### À tester dans Studio One (build 20260718-105213)
1. Ouvrir le plugin dans un projet neuf → chargement sans freeze/timeout.
2. Recharger un projet sauvegardé avec une song (mode Song actif, blocks P1-P8) → la song doit être restaurée et jouer normalement.
3. Basculer rapidement entre morceaux/états du plugin → pas de blocage au chargement.
4. Régression : le mode Song fonctionne toujours (lecture, changement de pattern, repeats).

---

## 2026-07-18 — [AUDIT-RT1] Pré-allocation des voix pour supprimer l'alloc heap en RT (build 20260718-095728)

**Build:** `20260718-095728`
**Validation:** `cargo test` OK (173 lib + 1 midi_drag_helper + 102 test_standalone), `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **`src/synthesis/mod.rs`**: toutes les voix sont pré-allouées dans `initialize_with_layout`, même les slots inactifs.
  - Nouveau champ `active: [bool; MAX_TRACKS]` pour gérer l'activation sans libérer/allouer de `Box` sur le thread audio.
  - `reinitialize_slot` ne fait plus que remplacer l'enum `DrumVoiceKind` dans la `Box` existante ; la branche `Box::new` est supprimée du chemin RT.
  - `trigger`, `trigger_hard`, `set_voice_settings`, `set_algo`, `reset_voice` et les boucles `process_*` ignorent les slots inactifs.
- **`src/lib.rs`**: appel `set_slot_active(slot, false)` quand un slot devient inactif.

### Tests
- `reinitialize_inactive_slot_reuses_preallocated_voice` : vérifie qu'un slot inactif peut être activé sans nouvelle allocation.
- `set_slot_active_gates_trigger_and_output` : vérifie que `set_slot_active(false)` coupe le son et `true` le réactive.

### À tester dans Studio One (build 20260718-095728)
1. Ajouter une nouvelle lane via la pastille `+N` (activer un slot inactif) pendant la lecture → le slot doit sonner immédiatement, sans dropout/click.
2. Changer l'instrument d'une lane active (onglet Track → Instrument) → le son doit changer sans glitch/click.
3. Supprimer une lane (passer le slot inactif) pendant la lecture → plus de son sur ce slot, pas de rupture audio.
4. Sauvegarder/recharger le projet → le layout et les slots restent audibles.
5. Régression : toutes les lanes actives (jusqu'à 14) produisent du son, mutes/solos fonctionnent.

---

## 2026-07-18 — [100q] complet : transitions d'état on/off des toggles LED (build 20260718-083031)

**Build:** `20260718-083031`
**Validation:** `cargo test` OK (173 lib + 1 midi_drag_helper + 102 test_standalone), `build.ps1 -Install` OK

### Changements
- **Transitions d'état 0.14s sur les toggles (fin de [100q]).**
  - `src/ui/widgets.rs` (`ToggleLED`) : LED `FAINT→BLUE`, halo `blue_glow(0→90)`, fond `PANEL2→blue_glow(64)` et bordure `LINE2→BLUE` lissent à l'activation/désactivation via `animate_value_with_time` sur `response.id.with("state")` (ID distinct de celui du hover pour ne pas mélanger les deux animations).
  - `src/ui/widgets.rs` (`ToggleSwitch`) : la pastille **glisse** gauche↔droite (interpolation de `knob_x`) et fond/couleur/bordure fondent en douceur.
  - `src/ui/widgets.rs` (`led_segmented`) : cross-fade entre segments — l'ancien segment s'éteint en fondu pendant que le nouveau s'allume (bg glow, halo LED, couleur LED).

### À tester dans Studio One (build 20260718-083031)
1. Cliquer un bouton LED (Solo/Mute/Link, tags M/S/T…) → la LED, son halo, le fond et la bordure doivent **fondre** en ~0.14s au lieu de basculer instantanément ; re-cliquer → extinction en fondu.
2. Cliquer un `ToggleSwitch` (Sound Editor, ex. pré-filtre / stéréo) → la pastille doit **glisser** de gauche à droite (pas téléporter) avec fade de couleur.
3. Basculer un segmented control (Sound/Sequencer, Generator|Song, Hz/Notes…) → cross-fade entre l'ancien et le nouveau segment.
4. Cliquer rapidement plusieurs fois de suite → l'animation ne doit pas sauter (elle repart de sa valeur courante).
5. Régression : hover animé 0.14s, glow de playhead, double-clic reset et poignées mini sliders fonctionnent toujours ; aucune animation ne tourne en boucle (pas de repaint permanent quand l'UI est au repos).

---

## 2026-07-16 — TODO : [100v] marqué obsolète (docs)

**Modifications :** aucune (mise à jour de documentation uniquement).

### Changements
- **Suppression de la tâche [100v] de la liste active.**
  - `TODO.md` : `[100v]` marqué **OBSOLÈTE** car redondant avec l'architecture modulaire actuelle.
  - `TODO.md` ligne `[100i]` : référence à `[100v]` retirée, référence à `[57]` conservée.
  - Rationale : les 14 slots / types d'instruments par slot + sélecteur d'algorithme couvrent déjà le besoin V1 ; un "registre de moteurs" séparé n'apporte rien tant qu'on n'a pas de moteurs interchangeables (sampler, wavetable, etc.).

---

## 2026-07-16 — Mini sliders lanes : double-clic reset + poignée au hover (build 20260716-152038)

**Build:** `20260716-152038`
**Validation:** `cargo test` OK (173 lib + 1 midi_drag_helper + 102 test_standalone), `build.ps1 -Install` OK (Studio One fermé par l'utilisateur)

### Changements
- **Mini sliders : double-clic reset + poignée au hover.**
  - `src/ui.rs` (`draw_mini_value_slider`) : ajout d'un paramètre `default`, gestion du double-clic pour revenir à la valeur par défaut, et dessin d'une petite poignée ronde (Ø8) au hover/drag pour repérer la valeur.
  - `src/ui.rs` (`draw_param_mini_slider_with_value`) : propagation du `default` via `param.default_plain_value()` ; suppression de la logique de double-clic redondante.
  - `src/ui.rs` (lane volume) : slider de volume de lane raccourci passe `default = 1.0` (unity gain).
  - Les mini sliders Humanize / Push-Pull héritent automatiquement de la poignée et du reset à leur valeur par défaut.

### À tester dans Studio One (build 20260716-152038)
1. Survoler un mini slider de lane (Volume, Humanize, Push/Pull) → une petite poignée blanche doit apparaître à la position courante.
2. Double-cliquer sur le mini slider Volume d'une lane → retour à 1.0 (100%).
3. Double-cliquer sur le mini slider Humanize d'une lane → retour à la valeur par défaut du paramètre.
4. Double-cliquer sur le mini slider Push/Pull d'une lane → retour à la valeur par défaut du paramètre (0 ms).
5. Vérifier que drag et click normal sur les mini sliders fonctionnent toujours.
6. Régression : vérifier que la poignée ne masque pas la valeur affichée en tooltip au hover.

---

## 2026-07-16 — Double-clic reset sur tous les sliders (build 20260716-150443)

**Build:** `20260716-150443`
**Validation:** `cargo test` OK (173 lib + 1 midi_drag_helper + 102 test_standalone), `build.ps1 -Install` OK (Studio One fermé par l'utilisateur)

### Changements
- **Double-clic reset étendu à tous les sliders de l'interface.**
  - `src/lib.rs` : `master_volume` passe de `0.8` à `1.0` (0 dB par défaut).
  - `src/ui.rs` (`header_param_slider`) : déjà présent, reset à `param.default_normalized_value()` pour les sliders d'en-tête (Master, BPM, Swing, Pattern Length).
  - `src/ui.rs` (`draw_editor_slider_track` / `draw_editor_slider_row`) : ajout d'un paramètre `default` et gestion du double-clic dans le Sound Editor.
  - `src/ui.rs` (`draw_editor_frequency_row`) : propagation du `default` pour le slider de fréquence (Hz) des bass drums.
  - `src/ui.rs` : `reset_value` renseigné sur tous les `LocalParamSlider` :
    - menus P-lock (volume, champs standard, params spéciaux) → valeur globale du slot ou `def.default` ;
    - menus Morph / fusion → valeur globale du slot ou `def.default` ;
    - menu Seq P-lock (probabilité = 1.0, stutter = 1.0) ;
    - config Default Analog reste à 0.5.

### À tester dans Studio One (build 20260716-150443)
1. Double-cliquer sur le slider Master → doit afficher 0 dB (valeur 1.0, pas 0.8 / -1.9 dB).
2. Double-cliquer sur les sliders d'en-tête (BPM, Swing, Pattern Length) → retour à la valeur par défaut du paramètre.
3. Ouvrir le Sound Editor (sélectionner une lane) et double-cliquer sur Volume, Decay, Freq, Filter, etc. → retour à la valeur par défaut de l'instrument.
4. Clic droit sur une step pour ouvrir le menu P-lock : double-clic sur les sliders → retour à la valeur globale du slot (ou valeur par défaut pour les spéciaux).
5. Clic droit sur une step fusionnée pour ouvrir le menu Morph : double-clic sur les sliders → retour à la valeur globale du slot.
6. Clic droit sur une step pour Seq P-lock : double-clic sur Probability → 100%, Stutter → 1x.
7. Régression : drag et click normal sur tous ces sliders fonctionnent toujours.

---

## 2026-07-16 — Sliders : double-clic pour reset à la valeur par défaut (build 20260716-144917)

**Build:** `20260716-144917`
**Validation:** `cargo test` OK (173 lib + 1 midi_drag_helper + 102 test_standalone), `build.ps1 -Install` OK (Studio One fermé par l'utilisateur)

### Changements
- **Double-clic sur les sliders d'en-tête pour revenir à la valeur par défaut.**
  - `src/ui.rs` (`header_param_slider`) : gestion de `response.double_clicked()` pour définir le paramètre à `param.default_normalized_value()` via `setter.set_parameter_normalized()`.
  - Le reset est encadré par `begin_set_parameter` / `end_set_parameter` pour que l'historique DAW (undo) et l'automation capturent le changement.

### À tester dans Studio One (build 20260716-144917)
1. Double-cliquer sur un slider d'en-tête (Master Volume, Swing, Pattern Length, etc.) → la valeur doit revenir à son défaut DAW.
2. Vérifier que le déplacement visuel du slider suit immédiatement le retour à la valeur par défaut (pas de décalage d'un frame).
3. Vérifier que le drag/click normal fonctionne toujours comme avant.
4. Régression : vérifier que l'automation et l'undo du DAW capturent bien le changement de valeur (le paramètre doit apparaître comme modifié dans l'historique d'édition).

---

## 2026-07-16 — Pattern Bank : retrait indicateur [P:S] + alignement boutons (build 20260716-144332)

**Build:** `20260716-144332`
**Validation:** `cargo test` OK (173 lib + 1 midi_drag_helper + 102 test_standalone), `build.ps1 -Install` OK (Studio One fermé par l'utilisateur)

### Changements
- **Suppression de l'indicateur de debug `[P:X S:X]` dans la barre Pattern Bank.**
  - `src/ui.rs` (`draw_pattern_bank`) : retrait du comptage et de l'affichage des p-locks sound/séquenceur.
- **Alignement vertical des boutons de la barre Pattern Bank.**
  - `src/ui.rs` (`draw_pattern_bank`) : hauteurs uniformisées à 26 px pour `Save` (était 22 px) et les slots `P1-P8` (étaient 22 px), afin d'être alignés avec les chips `Export`/`Drag` et le bouton `Clr`.

### À tester dans Studio One (build 20260716-144332)
1. Ouvrir la barre Pattern Bank (en haut de l'interface) → vérifier que l'indicateur `[P:X S:X]` à côté du label *Patterns* a disparu.
2. Vérifier que les boutons `Export`, `Drag`, `Save`, `P1-P8`, `Clr` sont tous alignés sur la même ligne de base (même hauteur), sans bouton décalé.
3. Vérifier que les interactions (clic sur P1-P8 pour charger/sauvegarder, mode Save, mode Clear) fonctionnent toujours normalement.
4. Régression : vérifier que le reste de l'interface (grille, Sound Editor, Song Editor) n'a pas été décalé.

---

## 2026-07-16 — Micro-animations UI : hover 0.14s + glow de playback (build 20260716-142342)

**Build:** `20260716-142342`
**Validation:** `cargo test` OK (173 lib + 1 midi_drag_helper + 102 test_standalone), `build.ps1 -Install` OK (Studio One fermé par l'utilisateur)

### Changements
- **Implémentation de [100q] (points 1 et 2).**
  - `src/ui/theme.rs` : ajout de `lerp_color()` pour interpoler les couleurs de manière fluide.
  - `src/ui/widgets.rs` : ajout de `hover_t()` (animation de hover 0.14s via `ctx.animate_value_with_time`).
  - `src/ui/widgets.rs` : animations de hover sur :
    - `ToggleLED` (bordure + couleur du texte) ;
    - `styled_select` (bordure du champ) ;
    - options du dropdown de `styled_select` (fond + texte) ;
    - LED segmented control (couleur du texte).
  - `src/ui.rs` (`draw_step_cell_v2`) : les cellules de la grille lissent leur bordure bleue au hover au lieu de basculer instantanément.
  - `src/ui.rs` (`draw_step_cell_v2`) : l'anneau blanc de la tête de lecture pulse doucement (alpha 120→200) pour marquer visuellement la colonne en cours de lecture.

### À tester dans Studio One (build 20260716-142342)
1. Passer la souris sur une cellule de la grille inactive → la bordure doit s'illuminer en bleu progressivement (~0.14s), pas instantanément.
2. Passer la souris sur un bouton LED (`Solo`, `Mute`, `Link`, etc.) → le texte et la bordure doivent lisser leur intensité.
3. Passer la souris sur un `styled_select` (choix de style, etc.) → la bordure doit s'activer en douceur ; ouvrir le dropdown et survoler les options → fond + texte lissent également.
4. Lancer la lecture → l'anneau blanc autour de la cellule courante doit pulser (respiration subtile) ; la bordure ne doit pas baver sur les cellules voisines (pas de halo translucide expansif).
5. Régression : vérifier que l'affichage des états actifs (cellules bleues, p-locks orange/rouge, fusion bleue) reste correct et lisible.

---

## 2026-07-16 — Tests moteurs audio allégés (build 20260716-114252)

**Build:** `20260716-114252`
**Validation:** `cargo test` OK (173 lib + 1 midi_drag_helper + 102 test_standalone)

### Changements
- **Implémentation d'un mode test allégé pour [100r].**
  - `src/synthesis/mod.rs` : ajout de deux tests sur le `DrumSynthesizer` complet :
    - `all_voices_render_finite_non_silent_output` — chaque voix (Kick, Snare, HiHat, OpenHH, Tom1/2/3, Clap, Ride, Cymbal, Snare606, B8, Perc1) est déclenchée avec ses réglages par défaut et rend ~4,5 ms ; on vérifie que le signal est fini et non silencieux.
    - `all_voices_stay_finite_on_retrigger` — chaque voix est déclenchée deux fois de suite pour s'assurer qu'aucune valeur `NaN`/`Inf` n'apparaît sur retrigger.
  - `src/pattern_bank.rs` : `SongSequence` devient `Copy + PartialEq` (requis pour les tests précédents et le contrôleur SPSC).
  - Surcoût de `cargo test` négligeable (~quelques ms), pas d'impact sur `build.ps1 -Install`.

### À tester dans Studio One (build 20260716-115XXX)
1. Jouer un pattern avec plusieurs voix (BD, SD, HH, Tom, etc.) → vérifier que tout sonne.
2. Déclencher des répétitions rapides sur une voix → pas de clic/disparition anormale.
3. Charger un ancien projet → le son doit être identique à la build précédente.

---

## 2026-07-16 — Song mode lock-free + SPSC UI→audio (build 20260716-111228)

**Build:** `20260716-111228`
**Validation:** `cargo test` OK (171 lib + 1 midi_drag_helper + 100 test_standalone), `build.ps1 -Install` OK (Studio One fermé par l'utilisateur)

### Changements
- **Implémentation d'un contrôleur lock-free pour le song mode (AUDIT-1 option).**
  - `src/atomic_song.rs` : nouveau module `SongStateController` basé sur une file SPSC `crossbeam::queue::ArrayQueue<SongSequence>`.
  - `Cargo.toml` : ajout de `crossbeam = "0.8"`.
  - `DrumFlashParams` : champ `song_controller` (non persisté, runtime uniquement).
  - `DrumFlashVst::initialize()` : synchronise le contrôleur avec `PatternBank.song` à l'init et après restauration d'état DAW.
  - `src/ui.rs` (`draw_song_editor`) : publie un snapshot `SongSequence` à chaque modification UI, sans redondance grâce à `last_published_song`.
  - `src/lib.rs` (`process`) : le thread audio consomme le dernier snapshot au début de chaque bloc ; le mode song n'utilise plus `try_lock` sur `PatternBank` pour lire la séquence.
- **`SongSequence` devient `Copy + PartialEq`** pour permettre le passage par valeur dans la file SPSC et la détection de changement côté UI.

### À tester dans Studio One (build 20260716-111228)
1. Créer une song (onglet `Song`) avec plusieurs blocs P1-P8 et des répétitions différentes → lancer la lecture → vérifier que les patterns changent aux bons endroits.
2. Modifier un bloc ou un repeat en cours de lecture → le changement doit s'appliquer au prochain cycle/transition.
3. Sauvegarder/recharger le projet Studio One → la song est restaurée et le mode song continue de fonctionner.
4. Vérifier que les opérations Save/Load de Pattern Bank (P1-P8) fonctionnent toujours.
5. Régression : vérifier qu'il n'y a pas de goutte audio ou de retard de changement de pattern en mode song.

---

## 2026-07-16 — Retour au NoteOff même échantillon + test MIDI déterministe (build 20260716-101309)

**Build:** `20260716-101309`
**Validation:** `cargo test` OK (168 lib + 1 midi_drag_helper + 100 test_standalone), `build.ps1 -Install` OK (Studio One fermé par l'utilisateur)

### Changements
- **Retour au NoteOn/NoteOff au même offset d'échantillon.**
  - Le build 20260716-093036 retardait le `NoteOff` d'un échantillon pour éviter les notes de longueur nulle. Studio One ne reçoit toujours pas de MIDI, donc on revient au comportement précédent pour vérifier si le retard était la cause.
  - `src/lib.rs` : `NoteOff` est à nouveau envoyé au même `timing` que `NoteOn` en mode séquenceur interne et en mode `Ext MIDI`. Suppression du paramètre `buffer_samples` de `fire_voice_trigger`.
- **Correction du test `resolve_midi_output_uses_slot_note_and_global_channel`.**
  - Le test utilisait le channel global chargé depuis `config.json` (non déterministe). Il force maintenant le channel à 10 avant les assertions.

### À tester dans Studio One (build 20260716-101309)
1. Insérer Flash Drum sur une piste instrument → lancer le séquenceur interne → vérifier si la sortie MIDI est reçue par un instrument cible (channel global, par défaut 10).
2. Mode `Ext MIDI` : envoyer une note MIDI sur l'entrée du plugin → elle doit être retransmise sur la sortie MIDI (channel global).
3. Vérifier que le mode `Internal` + `MIDI Pat` ON fonctionne toujours (chargement des patterns C3-G3).
4. Régression : s'assurer qu'il n'y a pas de clic audio supplémentaire sur les déclenchements rapides (suite au retour au NoteOff même échantillon).

---

## 2026-07-16 — Contrôle du channel MIDI global (build 20260716-094626)

**Build:** `20260716-094626`
**Validation:** `cargo test` OK (168 lib + 1 midi_drag_helper + 100 test_standalone), `build.ps1 -Install` OK (Studio One fermé par l'utilisateur)

### Changements
- **Le channel MIDI global est maintenant visible et modifiable.**
  - `src/config.rs` : ajout de `global_midi_channel` (1-16, défaut 10) dans `GlobalConfig`.
  - `src/lib.rs` : à la création d'une nouvelle instance, le channel global est initialisé depuis la config utilisateur.
  - `src/ui.rs` : section `MIDI` dans l'onglet `Track` affichant `Channel` (global) + `Note` (par slot).
  - `src/ui.rs` : le popup `Settings` contient un champ `Global MIDI Channel` (1-16) qui met à jour la config et le projet courant immédiatement.

### À tester dans Studio One (build 20260716-094626)
1. Ouvrir le plugin → dans l'onglet `Track` le champ `Channel` doit afficher `10`.
2. Ouvrir `Settings` (header) → changer `Global MIDI Channel` à `11` → fermer.
3. Dans l'onglet `Track` le champ `Channel` doit maintenant afficher `11`.
4. Routage MIDI out vers un autre instrument réglé sur channel 11 → les déclenchements du séquenceur interne doivent être reçus.
5. Sauvegarder/recharger le projet Studio One → le channel choisi dans `Settings` doit être restauré.
6. Régression : le changement de `Note` par slot reste indépendant du channel global.

---

## 2026-07-16 — MIDI Pat géré + MIDI out real-time corrigé (build 20260716-093036)

**Build:** `20260716-093036`
**Validation:** `cargo test` OK (168 lib + 1 midi_drag_helper + 100 test_standalone), `build.ps1 -Install` OK (Studio One fermé par l'utilisateur)

### Changements
- **Le switch `MIDI Pat` ne s'active / ne s'affiche actif que lorsque le séquenceur interne est sélectionné.**
  - `src/ui/widgets.rs` : `ToggleLED` prend désormais un état `enabled` (grisé + non cliquable quand désactivé).
  - `src/ui.rs` : quand on passe `Seq` de `Internal` à `Ext MIDI`, `MIDI Pat` est automatiquement coupé ; le bouton est grisé et inactif en mode `Ext MIDI`.
- **Correction du MIDI out temps réel.**
  - Le MIDI out était bien implémenté, mais `NoteOn` et `NoteOff` étaient envoyés au même offset d'échantillon. Certains hôtes/samplers ignorent ces notes de longueur nulle.
  - `src/lib.rs` : `NoteOff` est maintenant envoyé un échantillon plus tard (dans la limite du buffer), en mode séquenceur interne comme en mode `Ext MIDI`.

### À tester dans Studio One (build 20260716-093036)
1. Mode `Internal` + `MIDI Pat` ON → envoyer une note C3..G3 (60-67) sur l'entrée MIDI du plugin → le pattern P1-P8 correspondant doit se charger.
2. Passer `Seq` en `Ext MIDI` → `MIDI Pat` doit s'éteindre et devenir grisé/cliquable.
3. Revenir en `Internal` → `MIDI Pat` redevient cliquable (il reste OFF, il faut le réactiver).
4. Routage MIDI out : insérer Flash Drum sur une piste instrument, activer sa sortie MIDI vers une autre piste/instrument, et lancer le séquenceur interne → l'instrument cible doit recevoir les notes GM (Kick 36, Snare 38, etc.).
5. Mode `Ext MIDI` : envoyer une note MIDI standard sur l'entrée du plugin → elle doit être retransmise sur la sortie MIDI (channel global, par défaut 10).
6. Régression : les LEDs `Choke` et `Auto-Edit` doivent rester actives et fonctionnelles.

---

## 2026-07-16 — Menu Settings global + valeur analog par défaut (build 20260716-085356)

**Build:** `20260716-085356`
**Validation:** `cargo test` OK (168 lib + 1 midi_drag_helper + 100 test_standalone), `build.ps1 -Install` OK (Studio One fermé par l'utilisateur)

### Changements
- **[127] Menu Settings global.**
  - `src/config.rs` : nouveau module `GlobalConfig` ; persistance JSON dans `%USERPROFILE%/Documents/Flash Drum/config.json`.
  - `src/sound_settings.rs` : `SoundSettingsState::new_with_default_analog` et `reset_slot_to_defaults` prennent la valeur analog globale en paramètre ; `PersistentSoundSettings::new()` charge la config à l'initialisation.
  - `src/lib.rs` : chargement de la config dans `editor()` et passage à l'UI.
  - `src/ui.rs` : bouton `Settings` dans le header ; popup `draw_settings_popup_if_any` avec un slider `Default Analog` (0.0-1.0) ; sauvegarde automatique du fichier config à chaque mouvement du slider.
- **La valeur par défaut de `Analog` est désormais variabilisée.**
  - Nouvelle lane / changement d'instrument / preset de lanes utilisent la valeur configurée dans le menu Settings au lieu de 0.5 figé.
  - Double-clic sur le slider remet à 0.5 (valeur historique).

### À tester dans Studio One (build 20260716-085356)
1. Ouvrir le plugin → vérifier qu'un fichier `Documents/Flash Drum/config.json` est créé avec `default_analog: 0.5`.
2. Cliquer sur le bouton `Settings` dans le header → le popup s'ouvre en haut à droite.
3. Déplacer le slider `Default Analog` à 1.0 → fermer le popup ; ajouter une nouvelle lane (ex. `+ Add Module`) → l'Analog du nouvel instrument doit être 1.0.
4. Changer l'instrument d'une lane active (onglet Track → dropdown Instrument) → son Analog doit être réinitialisé à la valeur globale (1.0 si réglé à 1.0).
5. Appliquer un preset `Clear All` ou `Preset 12 Lanes` → toutes les lanes actives doivent prendre l'Analog configuré globalement.
6. Fermer/recharger le projet Studio One → la valeur `Default Analog` persiste (elle est relue depuis `config.json`).
7. Régression : avec `default_analog: 0.5`, créer une nouvelle lane Kick → l'Analog doit rester à 0.5 comme avant.
8. Régression : le bouton `Settings` ne doit pas décaler les autres éléments du header ni perturber les sliders Master/Swing.

---

## 2026-07-15 — Morphing origine vs cible (build 20260715-203803)

**Build:** `20260715-203803`
**Validation:** `cargo fmt` OK, `cargo test` OK (166 lib + 1 midi_drag_helper + 100 test_standalone), `build.ps1 -Install` OK (Studio One fermé par l'utilisateur)

### Changements
- **[125] Direction du morphing par cible dans une fusion.**
  - `src/sequencer/pattern.rs` : ajout de `MorphDirection` (`Target` / `Source`) et stockage dans les bits 56-59 du 3e u64 de chaque groupe fusion.
  - `src/lib.rs` : le moteur audio choisit le sens d'interpolation selon la direction.
    - `Target` : morph de la valeur lane/plock vers la valeur saisie (comportement existant).
    - `Source` : morph de la valeur saisie vers la valeur lane/plock.
  - `src/ui.rs` : popup Morph élargie à 350 px ; chaque ligne de paramètre cible affiche un petit bouton `Target`/`Source` à côté du slider (ou des boutons +/- pour les notes). Le slider de valeur par défaut tient désormais compte du sound plock présent sur la cellule de départ de la fusion.
- **Compatibilité ascendante :** les fusions sauvegardées avant cette build n'ont pas de bit de direction ; elles sont chargées en mode `Target` par défaut.

### À tester dans Studio One (build 20260715-203803)
1. Créer une Fusion sur une lane Kick/Snare de 2-4 steps → clic droit → Morph → ajouter un paramètre (ex. `Volume`).
2. Par défaut le bouton affiche `Target` : le morph va de la valeur lane/plock vers la valeur saisie. Lire la fusion → on doit entendre le paramètre glisser du réglage lane/plock vers la valeur cible.
3. Cliquer sur le bouton `Target` pour le passer en `Source` : le morph va maintenant de la valeur saisie vers la valeur lane/plock. Lire à nouveau → la direction doit être inversée.
4. Vérifier que les autres lignes morphables (Freq, Decay, Analog, specials continus, checkbox Stereo) affichent aussi le bouton `Target`/`Source` quand elles sont cibles.
5. Poser un sound plock sur la cellule de départ de la fusion, puis ouvrir le Morph : le slider de valeur par défaut doit refléter la valeur du plock, pas celle du lane global.
6. Sauvegarder/recharger le projet Studio One → la direction choisie (`Target` ou `Source`) doit être conservée pour chaque cible.
7. Sauvegarder/charger un pattern dans la Pattern Bank (slot P1-P8) → la direction doit être conservée.
8. Régression : une fusion existante créée avant cette build doit se charger en mode `Target` et sonner comme avant.

---

## 2026-07-15 — Generator/Song : switch positionné explicitement (build 20260715-174349)

**Build:** `20260715-174349`
**Validation:** `cargo fmt` OK, `cargo test` OK (165 lib + 1 midi_drag_helper + 99 test_standalone), `build.ps1 -Install` OK (Studio One fermé par l'utilisateur)

### Changements
- **Retour à une géométrie compacte d'origine pour le panel inférieur.**
  - Header : 42 px.
  - Séparation : 42 px.
  - Contenu : 44 px.
- **Le switch `Generator | Song` n'est plus placé par le layout automatique.**
  - Position verticale calculée explicitement dans le header.
  - Texte meta dessiné directement au painter, aligné au centre du switch.
- **Le cadre du switch reste dessiné en dernier**, pour éviter que le segment actif masque son bord.

### À tester dans Studio One (build 20260715-174349)
1. Ouvrir le panel `Generator` / `Song` → le switch doit être à la même hauteur qu'avant les essais de déplacement.
2. Le cadre du switch doit être complet, surtout en bas.
3. Le contenu Generator/Song doit être à sa position compacte habituelle et non tronqué.
4. Le texte meta doit rester aligné horizontalement avec le switch.

---

## 2026-07-15 — Generator/Song : cadre switch garanti visible (build 20260715-172822)

**Build:** `20260715-172822`
**Validation:** `cargo fmt` OK, `cargo test` OK (165 lib + 1 midi_drag_helper + 99 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Disposition du header Generator/Song revue avec des constantes explicites.**
  - Header dédié : 54 px.
  - Ligne de séparation : 58 px.
  - Contenu : démarre à 64 px.
- **Cadre du switch `Generator | Song` redessiné en dernier.**
  - Avant, le remplissage actif/hover pouvait visuellement manger le bas du cadre.
  - Le contour externe est maintenant rendu après les segments, donc il reste visible.

### À tester dans Studio One (build 20260715-172822)
1. Ouvrir le panel `Generator` / `Song` → le cadre complet du switch doit être visible, surtout le bord bas.
2. Basculer `Generator` ↔ `Song` → le cadre doit rester complet sur les deux états.
3. Vérifier que la ligne de séparation est clairement sous le switch, sans le couper.
4. Vérifier que les contrôles du panel restent lisibles et fonctionnels.

---

## 2026-07-15 — Refonte disposition du panel Generator/Song (build 20260715-172302)

**Build:** `20260715-172302`
**Validation:** `cargo fmt` OK, `cargo test` OK (165 lib + 1 midi_drag_helper + 99 test_standalone), `build.ps1 -Install` OK (Studio One fermé par l'utilisateur)

### Changements
- **Refonte complète de la disposition du panel inférieur Generator/Song.**
  - Le header est maintenant plus haut (48 px au lieu de 42 px) pour laisser respirer le switch `Generator | Song` (26 px) et éviter que son cadre ne soit tronqué.
  - La ligne de séparation descend de 42 px à 50 px, créant un espace clair entre le header et le contenu.
  - Le contenu du panel (barre de contrôles + presets / song editor) commence plus bas (54 px) mais conserve le même espacement et le même rendu visuel.
  - L'ensemble reste contenu dans la hauteur fixe du panel (210 px).

### À tester dans Studio One (build 20260715-172302)
1. Ouvrir le panel `Generator` / `Song`.
2. Vérifier que le cadre du switch `Generator | Song` n'est plus tronqué en bas.
3. Vérifier que le switch est bien centré verticalement dans le header.
4. Vérifier que la ligne de séparation est plus basse et que le texte meta (`Euclidean · Rock -> Rock` ou `N blocks · N patterns`) reste aligné avec le switch.
5. Vérifier que les contrôles du panel (combos, sliders, `GENERATE`, presets chips / song editor) restent lisibles et fonctionnels.

---

## 2026-07-15 — Baisse de la ligne de séparation Generator/Song sans toucher au contenu (build 20260715-170136)

**Build:** `20260715-170136`
**Validation:** `cargo fmt` OK, `cargo test` OK (165 lib + 1 midi_drag_helper + 99 test_standalone), `build.ps1 -Install` OK (Studio One fermé par l'utilisateur)

### Changements
- **Correction : la ligne de séparation du panel inférieur est abaissée de 7 pixels sans déplacer le contenu.**
  - La ligne passe de 42 px à 49 px depuis le haut du panel.
  - Le header et le body conservent leurs positions d'origine (42 px et 44 px) ; seul le trait de séparation descend.
  - Le switch `Generator | Song` a plus d'espace sans que les contrôles du panel soient tronqués.

### À tester dans Studio One (build 20260715-170136)
1. Ouvrir le panel `Generator` / `Song`.
2. Vérifier que le switch `Generator | Song` n'est plus coupé.
3. Vérifier que la ligne de séparation est plus basse que l'origine.
4. Vérifier que les contrôles du panel (combos, sliders, bouton `GENERATE`, presets chips) restent à la même place et ne sont pas tronqués.

---

## 2026-07-15 — Baisse de la ligne de séparation du panel Generator/Song (build 20260715-165429)

**Build:** `20260715-165429`
**Validation:** `cargo fmt` OK, `cargo test` OK (165 lib + 1 midi_drag_helper + 99 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Correction de la ligne de séparation du panel inférieur Generator/Song.**
  - Le premier ajustement (build 20260715-164724) a malencontreusement monté la ligne au lieu de la descendre. C'est corrigé : la ligne est maintenant abaissée de 7 pixels (header 42 px → 49 px, séparation à 49 px).
  - Le switch `Generator | Song` dispose maintenant de plus d'espace et ne doit plus être coupé.

### À tester dans Studio One (build 20260715-165429)
1. Ouvrir le panel `Generator` / `Song`.
2. Vérifier que le switch `Generator | Song` n'est plus coupé en haut.
3. Vérifier que la ligne de séparation est bien plus basse que sur le build précédent.
4. Régression : le contenu du panel reste lisible et fonctionnel.

---

## 2026-07-15 — Ajustement hauteur header du panel Generator/Song (build 20260715-164724)

**Build:** `20260715-164724`
**Validation:** `cargo fmt` OK, `cargo test` OK (165 lib + 1 midi_drag_helper + 99 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Hauteur du header du panel inférieur Generator/Song réduite de 42 px à 35 px.**
  - La ligne de séparation descend de 7 pixels, ce qui laisse plus de place au switch `Generator | Song` et évite qu'il soit coupé.
  - La zone de contenu du panel gagne 7 pixels de hauteur en conséquence.

### À tester dans Studio One (build 20260715-164724)
1. Ouvrir le panel inférieur `Generator` / `Song`.
2. Vérifier que le switch `Generator | Song` n'est plus coupé en haut.
3. Vérifier que la ligne de séparation est plus basse qu'avant (environ 7 pixels).
4. Régression : le contenu du panel (contrôles generator ou song editor) reste lisible et fonctionnel.

---

## 2026-07-15 — Retrait du sélecteur Mix du Sound editor + drag uniquement sur step actif (build 20260715-163647)

**Build:** `20260715-163647`
**Validation:** `cargo fmt` OK, `cargo test` OK (165 lib + 1 midi_drag_helper + 99 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Le sélecteur `Mix` est retiré de la section `Output` du Sound editor.**
  - L'assignation Main Mix est désormais gérée exclusivement dans l'onglet `Track` → section `Routing` → checkbox `Main`.
  - La section `Output` du Sound editor continue d'afficher les autres paramètres (Stereo, etc.).
- **Le drag long ne se déclenche plus sur une cellule vide.**
  - Seuls les steps déjà actifs peuvent être déplacés ; cliquer sur une cellule vide toggle juste le step.
  - Cela corrige les incohérences quand on essayait de créer un nouveau step et que le mode drag s'activait.

### À tester dans Studio One (build 20260715-163647)
1. Onglet `Sound` → section `Output` : le sélecteur `Mix` ne doit plus apparaître.
2. Onglet `Track` → section `Routing` : la checkbox `Main` doit toujours activer/désactiver le routage Main Mix.
3. Clic sur une cellule vide → le step s'active normalement (pas de drag, pas de clignotement).
4. Appui long sur une cellule active → au bout de ~0,5 s elle clignote et peut être déplacée.
5. Régression : appui long sur une cellule fusionnée ou en mode `Fusion` → aucun drag.

---

## 2026-07-15 — Drag cell long press : feedback clignotant plus visible (build 20260715-162108)

**Build:** `20260715-162108`
**Validation:** `cargo fmt` OK, `cargo test` OK (165 lib + 1 midi_drag_helper + 99 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Le feedback clignotant de la cellule source est maintenant dessiné par-dessus le hover.**
  - Le trait bleu de hover masquait le bord blanc pulsant ; le clignotement est désormais rendu en overlay après `draw_step_cell_v2`.
- **Alpha augmentée** : overlay blanc 50-100 + bord blanc 140-255 pulsant, bien visible sur toutes les couleurs de cellule.
- Délai d’appui long reste à 0,5 s. Le déplacement copie toujours step, sound plock et sequencer plock.

### À tester dans Studio One (build 20260715-162108)
1. Appui long gauche sur une cellule active (pas fusionnée) → au bout de ~0,5 s, la cellule doit clignoter fortement (bord blanc pulsant + éclaircissement).
2. Le clignotement doit rester visible même quand la souris est sur la cellule.
3. Clic bref (< 0,5 s) → pas de clignotement, juste un toggle.
4. Glisser puis relâcher → déplacement fonctionne toujours.
5. Régression : cellule fusionnée → pas de clignotement ni drag.

---

## 2026-07-15 — Drag cell long press : délai raccourci et feedback clignotant (build 20260715-161640)

**Build:** `20260715-161640`
**Validation:** `cargo fmt` OK, `cargo test` OK (165 lib + 1 midi_drag_helper + 99 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Délai d’appui long réduit de 2 s à 0,5 s.**
- **Feedback visuel quand la cellule est prête à être déplacée :** la cellule source clignote (bord blanc pulsant + overlay blanc subtil) une fois le seuil de 0,5 s atteint.
- Le déplacement copie toujours le step actif/inactif, le sound plock (Link/Snapshot) et le sequencer plock (probabilité, stutter, condition, micro-timing) du pas source vers le pas cible, puis efface le pas source.
- Garde-fous conservés : les cellules fusionnées ne déclenchent pas le drag et le mode `Fusion` désactive le drag.
- Le clic court reste inchangé.

### À tester dans Studio One (build 20260715-161640)
1. Appui long gauche sur une cellule active → au bout de ~0,5 s la cellule doit clignoter (bord blanc pulsant) et le déplacement doit être prêt.
2. Clic très bref (< 0,5 s) sur une cellule → le step doit juste toggle, sans drag.
3. Glisser la cellule clignotante à droite/gauche et relâcher → le step + plocks doivent se déplacer.
4. Régression : relâcher sans déplacer ne doit pas effacer le step d’origine.
5. Cellule fusionnée : appui long → aucun clignotement ni drag.

---

## 2026-07-15 — Drag cell long press 2s (build 20260715-154453)

**Build:** `20260715-154453`
**Validation:** `cargo fmt` OK, `cargo test` OK (165 lib + 1 midi_drag_helper + 99 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Déplacement d'une cellule par appui long (2 s) dans la grille.**
  - Maintenir le clic gauche sur une cellule non fusionnée pendant ~2 s active le mode déplacement.
  - La cellule source est entourée d'un trait blanc ; la cible actuelle est surlignée en orange avec un overlay semi-transparent.
  - Bouger la souris horizontalement déplace la cible d'un pas à gauche ou à droite (limité à la page courante, 16 pas).
  - Relâcher applique le déplacement : le step actif/inactif, le sound plock (Link/Snapshot) et le sequencer plock (probabilité, stutter, condition, micro-timing) sont copiés du pas source vers le pas cible, puis le pas source est effacé.
- **Garde-fous :** les cellules fusionnées ne déclenchent pas le drag (un clic long sur une fusion est ignoré), et le mode `Fusion` (Shift+clic) désactive aussi le drag pour ne pas entrer en conflit avec la sélection de fusion.
- Le clic normal reste inchangé : un appui bref (< 2 s) continue de toggle le step ou de sélectionner une plage de fusion.

### À tester dans Studio One (build 20260715-154453)
1. Sur une lane active, cliquer brièvement sur une cellule vide → le step doit s'activer normalement (régression : clic normal cassé).
2. Sur une lane active, ajouter un `Snapshot` ou `Link` plock sur une cellule active → maintenir le clic gauche sur cette cellule ~2 s → la cellule doit être entourée de blanc et pouvoir être glissée.
3. Glisser la cellule de 2-3 pas à droite et relâcher → le step + plock doivent apparaître à la nouvelle position ; le pas d'origine doit être vide.
4. Ajouter un sequencer plock (probabilité, stutter, condition ou micro-timing) sur une cellule active, puis déplacer cette cellule → le seq plock doit suivre à la nouvelle position.
5. Créer une fusion (Shift+clic sur deux cellules adjacentes) et maintenir le clic gauche sur une cellule de fusion ~2 s → aucun drag ne doit démarrer (le mode fusion garde la priorité).
6. Régression : un appui long suivi d'un relâchement sans déplacement ne doit pas effacer le step d'origine.

---

## 2026-07-15 — MIDI pattern switching en temps réel (build 20260715-093205)

**Build:** `20260715-093205`
**Validation:** `cargo fmt` OK, `cargo test` OK (165 lib + 1 midi_drag_helper + 99 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Nouveau paramètre `MIDI Pattern Switch` (toggle LED "MIDI Pat" dans le header).**
  - Quand il est actif et que le séquenceur est en mode `Internal` (pas `Ext MIDI`) et hors `Song Mode`, les notes MIDI entrantes C3..G3 (60-67) chargent les slots Pattern Bank P1..P8 en temps réel.
  - Le slot est mis en file d’attente pendant le bloc audio puis chargé via `load_pattern_from_slot` (déjà sécurisé pour le thread audio avec `try_lock`).
  - Si la Pattern Bank est temporairement verrouillée par l’UI, le chargement est réessayé au bloc audio suivant.
  - Le séquenceur redémarre à zéro après le chargement (`pending_song_pattern_restart`) pour commencer le nouveau pattern proprement.
- **Lecture des événements MIDI en mode Internal** : le thread audio consomme désormais les `NoteOn` entrants même en mode `Internal Sequencer`, mais seuls les notes de changement de pattern agissent ; les autres notes sont ignorées (pas de déclenchement de voix, ce qui reste réservé au mode `Ext MIDI`).

### À tester dans Studio One (build 20260715-093205)
1. Préparer 2 patterns différents en P1 et P2 (Pattern Bank), vérifier que le slot actuel est P1.
2. Mettre le séquenceur sur `Internal` et activer le toggle `MIDI Pat`.
3. Envoyer une note MIDI C3 (note 60) au plugin → le pattern P1 doit rester/jouer.
4. Envoyer une note MIDI C#3 (note 61) → le pattern doit basculer sur P2 et le séquenceur redémarrer au début du nouveau pattern.
5. Activer `Song Mode` → les notes MIDI ne doivent plus changer de pattern (Song mode garde la main).
6. Passer en `Ext MIDI` → les notes MIDI doivent déclencher les voix comme avant et ne pas changer de pattern.
7. Régression : désactiver `MIDI Pat` puis envoyer des notes C3-G3 → aucun changement de pattern.

---

## 2026-07-15 — Cellules fusionnées : couleur plock / morph visible (build 20260715-085935)

**Build:** `20260715-085935`
**Validation:** `cargo fmt` OK, `cargo test` OK (165 lib + 1 midi_drag_helper + 99 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Les cellules de début de fusion affichent désormais la couleur plock quand elles en portent une.**
  - Avant, une cellule fusionnée était toujours bleue, même si le pas de départ avait un `Link` / `Snapshot` plock.
  - `step_colors_v2` calcule d’abord la couleur de plock, puis ne l’écrase par la couleur fusion que si le pas n’a pas de plock.
- **La modulation (morph) est traitée comme un indicateur visuel de plock.**
  - Un groupe de fusion avec au moins une cible de morphing (`morph_active()`) affiche la couleur plock (`PL_LINK`) sur la cellule de départ.
  - Cela unifie le feedback visuel : plock sonore ET morph apparaissent tous deux comme une couleur spéciale sur le bloc fusion.
- Seule la cellule de début d’un groupe est dessinée visuellement ; le traitement est donc appliqué à cette cellule pour tout le bloc.

### À tester dans Studio One (build 20260715-085935)
1. Créer une fusion sur une lane active (Shift+clic sur deux cellules) → le bloc doit rester bleu.
2. Clic droit sur la cellule de début de la fusion → ajouter un `Link` ou `Snapshot` plock → le bloc entier doit prendre la couleur plock (orange/rose) au lieu de bleu.
3. Clic droit sur la cellule de début de la fusion → menu `Morph` → ajouter un paramètre morphé → le bloc doit prendre la couleur plock (orange `PL_LINK`).
4. Régression : une fusion sans plock ni morph doit rester bleue ; une cellule non fusionnée avec plock doit garder sa couleur habituelle.

---

## 2026-07-14 — Option Delete Lane : désactiver un slot (build 20260714-194847)

**Build:** `20260714-194847`
**Validation:** `cargo fmt` OK, `cargo test` OK (165 lib + 1 midi_drag_helper + 99 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Clic droit sur le titre d'une lane active → nouvelle option `Delete Lane`.**
  - Désactive le slot : la lane devient une ligne vide avec le chip `+N` et peut être réactivée plus tard via `+ Add Module`.
  - Conférence à deux clics (`Confirm Delete Lane?`) pour éviter les fausses manips.
  - Si le slot supprimé était sélectionné, la sélection bascule automatiquement sur la première lane active.
  - Les données steps/fusions/plocks du slot ne sont pas effacées, mais le slot inactif n'est ni affiché ni audible.
- Synchronisation des états de confirmation : `Clear Lane` et `Delete Lane` s'annulent l'un l'autre.

### À tester dans Studio One (build 20260714-194847)
1. Clic droit sur le titre d'une lane active (ex. Tom) → choisir `Delete Lane` puis confirmer → la lane devient une ligne vide (`+4`).
2. Cliquer sur le chip `+4` → choisir un instrument → la lane se réactive avec le nouvel instrument.
3. Supprimer la lane sélectionnée → la sélection doit basculer sur une autre lane active (pas de panneau vide / crash).
4. Régression : `Clear Lane`, `Randomize Lane`, `Copy/Paste Lane` et `Paste Grid` doivent toujours fonctionner normalement.

---

## 2026-07-14 — Menu contextuel de lane : Clear Lane + Randomize Lane (build 20260714-193958)

**Build:** `20260714-193958`
**Validation:** `cargo fmt` OK, `cargo test` OK (165 lib + 1 midi_drag_helper + 99 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Clic droit sur le titre d'une lane active → nouvelles options `Clear Lane` et `Randomize Lane`.**
  - `Clear Lane` remplace l'ancien label `Clear Grid` dans le menu du titre de lane ; il efface les steps, fusions et plocks de la lane tout en gardant l'instrument, les réglages sonores, le routing et les contrôles de lane.
  - `Randomize Lane` remplit la lane de steps aléatoires avec une densité de 30 % et efface les fusions/plocks existants. Le module et les réglages sonores restent intacts.

### À tester dans Studio One (build 20260714-193958)
1. Clic droit sur le titre d'une lane active (ex. BD) → choisir `Randomize Lane` → des steps doivent apparaître aléatoirement sur la lane ; la lecture doit produire des coups.
2. Clic droit sur le titre de la même lane → choisir `Clear Lane` puis confirmer → tous les steps, fusions et plocs de la lane doivent disparaître ; le module et les réglages sonores restent inchangés.
3. Régression : `Copy Lane` / `Paste Lane` / `Paste Grid` doivent toujours fonctionner depuis le menu contextuel du titre de lane.

---

## 2026-07-14 — Vrai fix Analog 50% par défaut (build 20260714-192351)

**Build:** `20260714-192351`
**Validation:** `cargo fmt` OK, `cargo test` OK (164 lib + 1 midi_drag_helper + 99 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Correction de la valeur par défaut de `analog` à 0.5 pour tous les instruments.**
  - Le build précédent (20260714-190609) n'avait modifié que `VoiceSettings::*`, mais les nouveaux slots ajoutés via `+ Add Module` utilisent `instrument_registry::INSTRUMENTS[].sound_settings_default`.
  - Tous les tableaux `sound_settings_default` des 13 instruments passent désormais `analog` à 0.5.
  - Ajout d'un test de régression : `default_analog_is_50_percent_for_every_instrument_kind`.

### À tester dans Studio One (build 20260714-192351)
1. Ajouter un slot via `+ Add Module` (ex. BD8) → vérifier que `Analog` démarre bien à 50 %.
2. Réinitialiser un slot existant (changer d'instrument puis revenir) → `Analog` doit revenir à 50 %.
3. Régression : charger un ancien preset → ses valeurs `analog` personnalisées sont conservées.

---

**Build:** `20260714-190609`
**Validation:** `cargo fmt` OK, `cargo test` OK (164 lib + 1 midi_drag_helper + 99 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Le paramètre `analog` est désormais à 0.5 par défaut pour tous les instruments** (`VoiceSettings::kick`, `snare`, `hihat`, `open_hihat`, `tom*`, `clap`, `ride`, `cymbal`, `snare606`, `kick808`, `perc1` et le `Default`).
  - Avant, les valeurs variaient entre 0.3 et 1.0 selon l'instrument ; les nouvelles pistes/presets ont maintenant un comportement analogique cohérent à 50 %.
- **Les paramètres d'automation `Algo` des instruments n'ayant qu'un seul algorithme sont cachés.**
  - Dans la disposition par défaut (13 voix legacy), les slots 3 (HiHat), 4 (OpenHiHat), 10 (Cymbal), 11 (Snare606), 12 (BassDrum808) et 14 (vide) n'exposent plus leur `Slot N Algo` au DAW.
  - Cela évite qu'un automate affiche `BD808 Algo` avec une seule valeur possible.

### À tester dans Studio One (build 20260714-190609)
1. Ajouter une nouvelle piste / réinitialiser un slot existant → vérifier que `Analog` démarre à 50 % dans le Sound Editor.
2. Ouvrir l'automation du slot 12 (BassDrum808) → vérifier que `Slot 12 Algo` n'apparaît plus dans la liste.
3. Ouvrir l'automation du slot 1 (Kick) → vérifier que `Slot 1 Algo` reste disponible (Kick a plusieurs algos).
4. Régression : charger un ancien preset sauvegardé avant cette build → ses réglages `analog` existants doivent être conservés ; seules les valeurs par défaut des nouveaux slots changent.

---

**Build:** `20260714-141959`
**Validation:** `cargo fmt` OK, `cargo test` OK (163 lib + 1 midi_drag_helper + 98 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Compensation automatique du gain sur la saturation.**
  - `SaturationConfig` calcule désormais un `compensation_gain` à chaque changement de type ou d'amount, basé sur une référence d'entrée de 0.5.
  - Le signal saturé est atténué pour que le niveau perçu reste proche du signal sec quand on active la saturation (mix à 100 %, output gain à 1.0).
  - Le paramètre `Output Gain` devient un réglage fin manuel plutôt qu'une obligation pour compenser.
- **Valeur par défaut de `saturation_output_gain` corrigée à 1.0** pour tous les instruments (elle était à 0.0, ce qui rendait la saturation silencieuse dès qu'on augmentait le mix).
- Toutes les voix appellent `update_compensation()` après chaque changement de saturation dans `set_settings` et `set_special_param`.

### À tester dans Studio One (build 20260714-141959)
1. Choisir un instrument (ex. Kick), régler `Sat Type` sur `SoftClip`, `Sat Amount` à 0.5, `Sat Mix` à 100 %, `Output Gain` à 1.0 → le niveau doit être proche du son sans saturation, sans explosion de volume.
2. Tester les autres types de saturation (`Valve`, `Transistor`, `HardClip`, `Tape`) → le niveau global doit rester cohérent, seul le caractère change.
3. Variation : augmenter `Sat Amount` à 1.0 → le son doit rester saturé mais le niveau perçu ne doit pas monter énormément.
4. Régression : désactiver la saturation (`Sat Type` = `None` ou `Sat Mix` = 0 %) doit restituer le son sec exact.

---

**Build:** `20260714-135251`
**Validation:** `cargo fmt` OK, `cargo test` OK (163 lib + 1 midi_drag_helper + 98 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **La création de plock (clic droit Link/Snapshot sur une cellule) est désormais autorisée uniquement en mode Pattern avec Follow OFF.**
  - En mode Song, le clic droit sur une cellule n'affiche plus le menu plock ; seule l'ouverture du menu `Plock` dans le header reste possible pour modifier un plock existant.
  - Avec Follow ON en mode Pattern, le clic droit est ignoré pour éviter que le défilement de la grille ne déplace la cellule sous le curseur pendant la manipulation.
  - Cela empêche les créations de plock sur des pas qui vont être remplacés par le défilement du séquenceur, et évite le crash rencontré sur Kick 808 en mode Song.

### À tester dans Studio One (build 20260714-135251)
1. Mettre le plugin en mode **Pattern** avec `Follow OFF` → clic droit sur une cellule → le menu `Link/Snapshot` doit apparaître et créer un plock normalement.
2. Activer `Follow ON` en mode Pattern → clic droit sur une cellule → aucun menu plock ne doit s'afficher.
3. Passer en mode **Song** → clic droit sur une cellule → aucun menu plock ne doit s'afficher ; le plugin ne doit pas crasher.
4. Régression : les plocks déjà existants restent éditables via le menu `Plock` dans le header, et les patterns sauvegardés se relisent avec leurs plocks.

---

**Build:** `20260714-122623`
**Validation:** `cargo fmt` OK, `cargo test` OK (163 lib + 1 midi_drag_helper + 98 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Garde-fous défensifs autour des plocks et de l'affichage fréquence/note.**
  - `freq_to_note` retourne 0 pour une fréquence <= 0 ou non-finie (évite `log2(0)` = -inf).
  - `note_to_freq` retourne 440 Hz pour une note non-finie.
  - `note_name` clamp la note entre 0 et 127 pour éviter un index négatif dans le tableau des noms.
  - `PlockValues::get` et `set` vérifient désormais les bornes `instrument < INSTRUMENT_COUNT`, `step < STEP_COUNT`, `field < FIELD_COUNT`.
- Ces correctifs visent à empêcher un panic/crash lors de la création d'un plock sur Kick 808 en mode Song (diagnostic en cours).

### À tester dans Studio One (build 20260714-122623)
1. Ouvrir le panneau Song → vérifier que le checkbox `Follow` est bien retiré.
2. Activer `Song Mode`, lancer la lecture, créer un plock (clic droit → Link/Snapshot) sur une cellule Kick 808 (B8) → le plugin ne doit pas crasher.
3. Si le crash persiste, noter exactement à quel moment (clic droit, choix du menu, manipulation d'un slider) et récupérer le log Event Viewer.
4. Régression : créer des plocks sur d'autres instruments (BD, SD, HH, Tom) doit toujours fonctionner.

---

# Changelog

## 2026-07-14 — Retrait du mode Follow dans l'onglet Song (build 20260714-114115)

**Build:** `20260714-114115`
**Validation:** `cargo fmt` OK, `cargo test` OK (163 lib + 1 midi_drag_helper + 98 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Retrait du toggle `Follow` du panneau Song.**
  - Le checkbox `Follow` et la logique de figement de la song ont été supprimés.
  - La song avance à nouveau normalement dès que `Song Mode` est actif.
  - Les atomiques `song_follow` et `follow_song_step_request` ont été retirés de `DrumFlashVst`.
  - L'auto-save des edits en mode Song (build 20260713-162128) reste en place.

### À tester dans Studio One (build 20260714-114115)
1. Ouvrir le panneau Song → vérifier que le checkbox `Follow` a disparu.
2. Activer `Song Mode` et lancer la lecture : la song doit avancer normalement P1 → P2 → …
3. Régression : cliquer un block de song ne doit plus figer la song ni charger un pattern différent.

---

# Changelog

## 2026-07-13 — Correction du graphique Filter Decay pour T1 (build 20260713-171854)

**Build:** `20260713-171854`
**Validation:** `cargo fmt` OK, `cargo test` OK (163 lib + 1 midi_drag_helper + 98 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Le graphique d'enveloppe de filtre utilise désormais la courbe réelle du moteur par instrument.**
  - Avant : le graphique prenait toujours `decay_curve` (courbe de l'enveloppe d'amplitude), qui ne correspondait pas à la courbe fixe utilisée par le moteur pour la `filter_env`.
  - Après : le graphique utilise la courbe spécifique à chaque voix.
- Courbes du moteur exposées via `DrumVoice::filter_env_curve()` :
  - Tom1/Tom2/Tom3 : `6.0`
  - Kick, Snare, HiHat, Snare606 : `8.0`
  - Perc1 : dynamique (`decay_curve`)
  - Les autres voix n'ont pas de `filter env`.
- Le son des instruments n'est pas modifié ; seule la visualisation est corrigée.

### À tester dans Studio One (build 20260713-171854)
1. Sélectionner la lane T1 (Tom1) → ouvrir l'onglet `Sound` → section `Filter`.
2. Vérifier que le graphique orange `Filter Decay` affiche une courbe exponentielle descendante cohérente avec le réglage `Filter Decay`.
3. Changer `Filter Decay` → la courbe doit s'étirer/raccourcir en temps réel.
4. Sélectionner Kick, Snare, HiHat ou Snare606 → le graphique doit utiliser leur courbe fixe (8.0) sans être influencé par le slider `Decay Curve` de l'enveloppe d'amplitude.
5. Sélectionner Perc1 → le graphique doit continuer de suivre le slider `Decay Curve` (courbe dynamique).
6. Régression : l'enveloppe d'amplitude (section `Envelope`) doit continuer d'utiliser `Decay Curve` comme avant.

---

# Changelog

## 2026-07-13 — Bouton Follow on/off dans le panel Song (build 20260713-170024)

**Build:** `20260713-170024`
**Validation:** `cargo fmt` OK, `cargo test` OK (163 lib + 1 midi_drag_helper + 98 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Ajout d'un toggle `Follow` dans le panneau Song.**
  - `Follow` est on par défaut : la song avance normalement et charge le pattern du block courant à chaque loop.
  - `Follow` off : la song cesse d'avancer ; cliquer sur un block de song charge immédiatement son pattern dans la grille pour édition.
- Communication UI → audio thread via un atomic `song_follow` et une requête `follow_song_step_request`.
- Quand on clique un block en Follow off, le toggle passe visuellement à off et le step demandé est traité par le thread audio au prochain bloc.
- L'auto-save des edits en mode Song (build 20260713-162128) continue de fonctionner : le pattern édité est sauvegardé dans le slot de la bank actuellement chargé.

### À tester dans Studio One (build 20260713-170024)
1. Créer une song avec plusieurs blocks (P1, P2, P3…) et lancer la lecture.
2. Désactiver `Follow` → la song doit rester sur le block courant ; la grille doit rester sur ce pattern.
3. Cliquer sur un autre block de la song → la grille doit charger ce pattern et le séquenceur doit le jouer (la song reste figée sur ce block).
4. Réactiver `Follow` → la song doit reprendre son défilement normal au prochain loop.
5. Régression : en `Follow` on, la song doit avancer normalement d'un block à l'autre.
6. Régression : en `Follow` off, éditer la grille puis réactiver `Follow` doit conserver les modifications (auto-save).

---

# Changelog

## 2026-07-13 — Song Mode joue le premier block au démarrage (build 20260713-164153)

**Build:** `20260713-164153`
**Validation:** `cargo fmt` OK, `cargo test` OK (163 lib + 1 midi_drag_helper + 98 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Le Song Mode charge maintenant le premier block dès l'activation.**
  - Avant : quand on activait `Song Mode` pendant la lecture, le séquenceur restait sur le pattern courant et attendait le prochain loop pour avancer.
  - Après : dès que `Song Mode` est activé (ou que le transport redémarre avec Song Mode déjà actif), le premier block de la song (step 0) est chargé immédiatement.
- Ajout de deux flags internes dans `DrumFlashVst` :
  - `song_mode_was_active` : détecte la transition off → on.
  - `song_needs_init` : demande le chargement du premier block au prochain bloc audio.

### À tester dans Studio One (build 20260713-164153)
1. Préparer une song avec au moins 2 blocks différents (ex. P1 = Kick, P2 = Snare).
2. Lancer la lecture sur un pattern autre que P1.
3. Activer `Song Mode` → la lecture doit immédiatement passer au premier block de la song (P1 ou le slot défini au step 0).
4. La song doit ensuite avancer normalement P1 → P2 → ... au rythme des loops.
5. Régression : désactiver / réactiver `Song Mode` doit toujours repartir du premier block.
6. Régression : l'auto-save des edits en mode Song (build 20260713-162128) doit continuer de fonctionner.

---

## 2026-07-13 — Auto-save des edits de pattern en mode Song (build 20260713-162128)

**Build:** `20260713-162128`
**Validation:** `cargo fmt` OK, `cargo test` OK (163 lib + 1 midi_drag_helper + 98 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **L'édition du pattern n'est plus "bloquée" en mode Song.**
  - Quand `Song Mode` est actif, chaque modification du pattern (toggle de step, édition/suppression de fusion, Clear Grid, Paste Lane/Grid) est automatiquement sauvegardée dans le slot de la Pattern Bank actuellement joué.
  - Avant : le pattern changeait à chaque avancée de la song, et les edits récents étaient perdus.
  - Après : les edits persistent dans le slot P1-P8 concerné ; la song les rejouera à sa prochaine boucle sur ce slot.
- Ajout d'un flag `pattern_dirty_slot` dans `EditorUIState` qui capture le slot de la bank au moment de l'édition.
- Ajout de `save_current_pattern_to_bank_slot()` pour sauvegarder le pattern courant (steps, fusions, plocks sonores, plocks séquenceur) depuis le thread UI.

### À tester dans Studio One (build 20260713-162128)
1. Activer `Song Mode`, créer une song avec plusieurs steps P1-P8, lancer la lecture.
2. Pendant que la song joue, toggle un step sur le pattern courant → le step doit rester allumé/éteint.
3. Attendre que la song revienne sur le même slot P1-P8 → le step modifié doit être rejoué (le pattern bank a bien été mis à jour).
4. Tester l'édition d'une fusion en mode Song (créer / modifier / supprimer) → la fusion doit être conservée au retour du slot.
5. Tester `Clear Grid` et `Paste Lane/Grid` sur une lane en mode Song → la modification doit persister au prochain passage du slot.
6. Régression : hors mode Song, les boutons Save/Load P1-P8 doivent fonctionner comme avant (pas d'auto-save involontaire).

---

## 2026-07-13 — Double-clic sur le nom de lane pour renommer (build 20260713-154411)

**Build:** `20260713-154411`
**Validation:** `cargo fmt` OK, `cargo test` OK (163 lib + 1 midi_drag_helper + 98 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Double-clic sur le nom d'une lane active** pour l'éditer directement.
  - Le double-clic sélectionne le slot, bascule l'onglet `Track` et donne le focus au champ `Name`.
  - Le champ est initialisé avec le nom personnalisé de la lane (ou vide si aucun nom personnalisé n'est défini).
  - La limite de 6 caractères et la persistance `track-layout-v1` restent actives.

### À tester dans Studio One (build 20260713-154411)
1. Double-cliquer sur le nom d'une lane active (ex. `BD`, `SD`) → l'onglet `Track` doit s'ouvrir et le champ `Name` doit être focusé avec le texte courant.
2. Taper un nouveau nom (max 6 caractères) → valider avec `Enter` ou cliquer ailleurs → le nom de la lane doit se mettre à jour.
3. Vérifier que la limite de 6 caractères bloque toujours au-delà.
4. Sauvegarder/recharger la song → le nom modifié par double-clic doit être conservé.
5. Régression : un clic simple sur le nom de lane doit toujours sélectionner la lane sans changer d'onglet.

---

## 2026-07-13 — Limite 6 caractères et largeur fixe pour le nom de lane (build 20260713-151535)

**Build:** `20260713-151535`
**Validation:** `cargo fmt` OK, `cargo check` OK, `cargo test` OK (163 lib + 1 midi_drag_helper + 98 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Limite de 6 caractères pour le nom de lane.**
  - `src/ui.rs` : le champ `Name` dans l'onglet `Track` utilise `.char_limit(6)`.
  - L'affichage dans l'en-tête de lane est tronqué à 6 caractères.
- **Largeur fixe et identique pour la box de nom de toutes les lanes.**
  - `name_w` passe de 34 px à 50 px pour accueillir 6 caractères en mono.
  - La box est donc la même largeur pour les lanes actives et les lanes vides (`+N`).

### À tester dans Studio One (build 20260713-151535)
1. Sélectionner une lane → `Track` → tenter de taper plus de 6 caractères dans `Name` : le champ doit bloquer à 6.
2. Vérifier que l'en-tête de lane affiche jusqu'à 6 caractères sans déborder.
3. Vérifier que toutes les boxes de nom (lanes actives et lanes vides `+N`) ont la même largeur.
4. Vérifier que la grille reste lisible (les 16 steps tiennent sur la ligne).
5. Régression : sauvegarder/recharger la song conserve le nom tronqué ou saisi.

---


## 2026-07-13 — Persiste le nom personnalisé des lanes dans track-layout-v1 (build 20260713-150831)

**Build:** `20260713-150831`
**Validation:** `cargo fmt` OK, `cargo check` OK, `cargo test` OK (163 lib + 1 midi_drag_helper + 98 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Correction : le nom personnalisé d'une lane était affiché dans le champ `Name` mais pas vraiment persisté, donc l'en-tête de lane revenait toujours au label de l'instrument.**
  - `src/track.rs` : `PersistentTrackLayout` stocke maintenant un tableau `slot_names` synchronisé avec `TrackLayoutState` dans `set()` et relu dans `map()`.
  - Avant, `map()` reconstruisait `slot.name` à partir de `kind.default_name()` — la valeur personnalisée était perdue.
  - Ajout du test `persistent_track_layout_preserves_custom_names`.

### À tester dans Studio One (build 20260713-150831)
1. Sélectionner une lane → onglet `Track` → champ `Name` doit afficher le nom actuel.
2. Taper un nom personnalisé, appuyer `Enter` : l'en-tête de lane doit immédiatement afficher le nouveau nom.
3. Changer de lane et revenir : le nom personnalisé doit être conservé.
4. Sauvegarder/recharger la song : le nom personnalisé persiste dans le projet DAW.
5. Changer d'instrument : le nom retombe au label par défaut du nouvel instrument.
6. Régression : vérifier que les menus de Studio One restent accessibles quand le plugin est visible et qu'aucun champ texte n'a le focus.

---


## 2026-07-13 — Fix validation Enter du champ Name (build 20260713-145653)

**Build:** `20260713-145653`
**Validation:** `cargo fmt` OK, `cargo check` OK, `cargo test` OK (162 lib + 1 midi_drag_helper + 97 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Correction du retour à l'ancien nom quand on appuie sur `Enter` dans le champ `Name`.**
  - `src/ui.rs` : suppression du rafraîchissement automatique du buffer de saisie quand le `TextEdit` perd le focus. La sync se fait maintenant uniquement au changement de slot sélectionné (`track_name_input_slot` est un `Option<usize>` initialisé à `None`, donc le premier affichage charge bien le nom courant).
  - Quand on change d'instrument dans le ComboBox, le buffer est explicitement mis à jour avec le nom par défaut du nouvel instrument.
  - Le nom saisi reste donc stable après `Enter` ou perte de focus.

### À tester dans Studio One (build 20260713-145653)
1. Sélectionner une lane → onglet `Track` → champ `Name` doit afficher le nom actuel.
2. Taper un nouveau nom, appuyer sur `Enter` : le champ doit conserver le nouveau nom (pas de retour à l'ancien).
3. Sélectionner une autre lane puis revenir : la nouvelle valeur doit être conservée.
4. Sauvegarder/recharger la song : le nom personnalisé persiste.
5. Changer d'instrument : le nom retombe sur le label par défaut du nouvel instrument.
6. Régression : vérifier que les menus de Studio One restent accessibles quand le plugin est visible et qu'aucun champ texte n'a le focus.

---


## 2026-07-13 — Fix saisie clavier du champ Name + diagnostics (build 20260713-144759)

**Build:** `20260713-144759`
**Validation:** `cargo fmt` OK, `cargo check` OK, `cargo test` OK (162 lib + 1 midi_drag_helper + 97 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Fix tentative pour la saisie clavier du champ `Name` dans l'onglet `Track`.**
  - `src/ui.rs` : le `TextEdit` est maintenant lié à un buffer stable dans `EditorUIState` (`track_name_input`) avec un ID explicite par slot, plutôt qu'à une `String` locale recréée à chaque frame. Cela évite que le widget perde le focus/curseur à chaque frame.
  - Le buffer est synchronisé avec `slot.name` quand on change de slot sélectionné, et rafraîchi quand le nom change par une autre voie (changement d'instrument) tant que l'utilisateur n'est pas en train d'éditer.
- **Diagnostics temporaires dans le workaround clavier Windows.**
  - `vendor/nih-plug/nih_plug_egui/src/editor.rs` : la fenêtre message est créée avec une taille 1×1 et sans `WS_EX_NOACTIVATE` (possible cause de non-réception du focus).
  - Ajout d'un log `%TEMP%\flash_drum_kbd.log` qui enregistre les appels `set_keyboard_focus`, les messages reçus par `msg_wnd_proc` et les messages traduits par `subclass_proc`. Ce log permet de diagnostiquer le chemin des événements clavier si le champ ne fonctionne toujours pas.

### À tester dans Studio One (build 20260713-144759)
1. Ouvrir le plugin, sélectionner une lane, aller dans l'onglet `Track`.
2. Cliquer dans le champ `Name` et taper un nom personnalisé → il doit s'afficher dans l'en-tête de lane.
3. Si la saisie ne fonctionne toujours pas :
   - Vérifier que le curseur apparaît bien dans le champ `Name` après le clic.
   - Vérifier si la saisie clavier fonctionne dans d'autres champs texte du plugin (double-clic sur un slider de paramètre pour entrer une valeur, ou champ `Name` dans `Dev: Preset Dumps` si visible).
   - Envoyer le fichier `%TEMP%\flash_drum_kbd.log` (il est recréé/complété à chaque ouverture du plugin).
4. Régression : vérifier que les menus de Studio One restent accessibles quand le plugin est visible et qu'aucun champ texte n'a le focus.

---


## 2026-07-13 — Rename de lane dans l'onglet Track (build 20260713-143422)

**Build:** `20260713-143422`
**Validation:** `cargo fmt` OK, `cargo check` OK (17 warnings UI restants), `cargo test` OK (162 lib + 1 midi_drag_helper + 97 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Renommage de lane dans l'onglet `Track` (sous-point de [100i]).**
  - `src/ui.rs` : ajout d'un champ `Name` éditable dans `draw_track_tab`, au-dessus du sélecteur d'instrument.
  - Le nom est stocké dans `TrackSlot::name` (déjà persistant dans `track-layout-v1`).
  - L'en-tête de lane affiche le nom personnalisé s'il est non vide ; sinon il retombe sur le label de l'instrument (BD, SD, HH…).
  - Refactor interne : le changement d'instrument utilise maintenant le même `new_state`/`changed` que le reste de l'onglet Track, au lieu d'écrire directement l'état dans le ComboBox (évite d'écraser d'éventuelles modifications Routing/MIDI/Name faites dans la même frame).

### À tester dans Studio One (build 20260713-143422)
1. Sélectionner une lane active (ex. Kick slot 1) → l'onglet `Track` affiche son nom actuel en haut.
2. Dans le champ `Name`, taper un nom personnalisé (ex. `MyKick`) → le nom doit s'afficher dans l'en-tête de lane à la place de `BD`.
3. Vider le champ `Name` → l'en-tête de lane doit revenir au label de l'instrument.
4. Changer d'instrument via le ComboBox `Instrument` → le nom retombe sur le label par défaut du nouvel instrument (comportement existant).
5. Sauvegarder/recharger la song Studio One → le nom personnalisé doit être conservé (persisté dans `track-layout-v1`).
6. Régression : vérifier que `Routing`, `MIDI Note` et `Length` fonctionnent toujours normalement après le refactor de `draw_track_tab`.

---


## 2026-07-13 — Vérification : [FIX] new tracks silent + solo par slot + UI track-based (build 20260713-142542)

**Build:** `20260713-142542`
**Validation:** `cargo fmt` OK, `cargo check` OK (17 warnings UI restants), `cargo test` OK, `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **TODO cleanup — l'item `[FIX] New tracks silent + solo shared by instrument family + all UI interactions now track-based` est confirmé déjà implémenté.**
  - `src/ui.rs` : `activate_slot()` appelle `reset_slot_to_defaults()` et `reinitialize_slot()` ; les boutons Mute/Solo lisent `params.mutes()[slot]` / `params.solos()[slot]`.
  - `src/lib.rs` : `compute_mix_gating()` traite 14 slots indépendamment ; `seq_mutes`/`seq_solos` utilisent `slot_mute_states[slot]` / `slot_solo_states[slot]`.
  - Pattern, plocks, seq-plocks, lane length, routing, MIDI note sont indexés par slot dans toute la base de code.
  - L'item a été coché dans `TODO.md`.

### À tester dans Studio One (build 20260713-142542)
1. Ajouter une nouvelle lane via `+5` (ex. une seconde Kick) : elle doit produire du son immédiatement et avoir les mêmes paramètres par défaut que le slot 1.
2. Muter/Solo la nouvelle lane : seule cette lane doit être affectée ; la lane d'origine (même famille d'instrument) doit rester audible sauf si elle est aussi en solo/mute.
3. Déplacer une lane avec la poignée de drag : ses états Mute/Solo doivent suivre la lane déplacée.
4. Régression : aucune lane ne doit rester silencieuse après création ou changement de type d'instrument.

---


## 2026-07-13 — MG-9 : MIDI note/channel par slot et canal global (build 20260713-102332)

**Build:** `20260713-102332`
**Validation:** `cargo fmt` OK, `cargo check` OK (17 warnings UI restants), `cargo test` OK (162 lib + 1 midi_drag_helper + 97 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **MG-9 — Correction de la sortie MIDI interne pour respecter le comportement "par slot".**
  - `src/lib.rs` : `fire_voice_trigger()` utilisait la note MIDI par défaut du registre d'instruments (`INSTRUMENTS[voice_idx].midi_note`) au lieu de la note configurée pour le slot.
  - Ajout de `resolve_midi_output_for_slot(slot_idx)` qui lit `track_layout.state.midi_note_for_slot(slot_idx)` et `global_midi_channel()`.
  - La sortie MIDI interne envoie désormais la note du slot et sur le canal MIDI global (10 par défaut, converti en index 9).
  - L'external MIDI thru utilise aussi le canal global pour les événements forwardés.
- **Test de régression ajouté :** `resolve_midi_output_uses_slot_note_and_global_channel` vérifie qu'une note MIDI personnalisée sur un slot et un canal global modifié sont bien utilisés.

### À tester dans Studio One (build 20260713-102332)
1. Insérer une piste instrument externe (ex. General MIDI drums) et la configurer pour recevoir le MIDI de Flash Drum sur le canal 10.
2. Sur la lane Kick (slot 1), changer la note MIDI dans l'onglet `Track` (ex. `C1` → `D1`).
3. Jouer le séquenceur : l'instrument externe doit recevoir la nouvelle note, pas le Kick par défaut.
4. Sur un autre slot, changer la note MIDI et vérifier que chaque slot envoie sa propre note.
5. Régression : si la note n'est pas modifiée, l'ancienne note par défaut de l'instrument doit toujours être envoyée.
6. Mode `Ext MIDI` : envoyer une note MIDI au plugin depuis le DAW, vérifier qu'elle est retransmise sur le canal global 10.

---


## 2026-07-13 — AUDIT-8 : nettoyage échafaudage UI mort + SAFETY native_drag (build 20260713-100057)

**Build:** `20260713-100057`
**Validation:** `cargo fmt` OK, `cargo check` OK (17 warnings UI restants), `cargo test` OK (161 lib + 1 midi_drag_helper + 97 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **AUDIT-8 — Nettoyage de l'échafaudage UI mort.**
  - Suppression du module `src/ui/design_system.rs` (Typography, Spacing, panel_frame, button, tag, panel_header — tous non câblés).
  - Suppression du widget `StyledButton` inutilisé dans `src/ui/widgets.rs`.
  - Remplacement de l'API dépréciée `egui::Ui::allocate_ui_at_rect` par `allocate_new_ui` dans `src/ui.rs` (10 occurrences).
  - Warnings réduits de 41 à 17.
- **`src/native_drag.rs` — documentation des invariants unsafe.**
  - Ajout de commentaires `// SAFETY:` sur toutes les opérations unsafe : initialisation OLE, vtables COM, gestion des références, `GlobalAlloc`/`GlobalLock`/`copy_nonoverlapping`, etc.
  - Ajout du test `build_hdrop_medium_produces_valid_global_medium` qui vérifie la structure `CF_HDROP` construite pour le drag & drop MIDI.

### À tester dans Studio One (build 20260713-100057)
1. Ouvrir le plugin : la grille, le Sound Editor, le panneau Song/Generator et le panneau inférieur doivent s'afficher normalement (replacement de `allocate_ui_at_rect` par `allocate_new_ui`).
2. Cliquer sur les onglets `Sound Editor` / `Track` : ils doivent basculer sans glitch visuel.
3. Ouvrir le Song Editor et éditer un block (sélecteur de pattern + répétitions) : les widgets doivent rester alignés.
4. Bouton `Drag MIDI` : glisser-déposer un fichier MIDI vers Studio One doit toujours créer le clip MIDI (le helper OLE est inchangé, mais le test du medium `CF_HDROP` est maintenant validé).
5. Régression : aucune disparition de panneau, aucun décalage de layout, aucun crash à l'ouverture du plugin.

---


## 2026-07-13 — AUDIT-7 : Infrastructure git (Cargo.lock, cruft binaire) (build 20260713-093252)

**Build:** `20260713-093252`
**Validation:** `git status` OK, commit `5c243e0`, `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **AUDIT-7 — Nettoyage de l'infrastructure git.**
  - `.gitignore` : suppression de la règle `Cargo.lock`, ajout de `*.pdb`, `*.zip` et exclusion des `Cargo.lock` dans `vendor/`.
  - `drum-pattern-vst/Cargo.lock` ajouté au dépôt pour des builds reproductibles.
  - Fichiers binaires/cruft retirés du suivi git (supprimés de l'index, conservés localement sauf `fix_roles.pdb`) :
    - `.claude/settings.local.json`
    - `design-pack.zip`
    - `design-pack/Flash_Drum_design_11062026.zip`
    - `docs/design/mockup_Flash_Drum.zip`
    - `drum-pattern-vst/assets/fonts/IBMPlexSans.zip`
    - `drum-pattern-vst/fix_roles.pdb`

### À tester dans Studio One (build 20260713-093252)
1. Aucun test fonctionnel requis : cette build n'affecte que le suivi git et les dépendances figées. Le plugin reste identique à la build précédente.
2. Régression : ouvrir le plugin dans Studio One, le charger, jouer un pattern, vérifier que le son est identique à la build d'hier.
3. Vérifier que le projet Studio One sauvegardé avec la build précédente se recharge correctement (pas de changement de format VST3).
---


## 2026-07-12 — AUDIT-5 : tests du routage mute/solo/mix (build 20260712-160415)

**Build:** `20260712-160415`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants), `cargo test` OK (161 lib + 97 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **AUDIT-5 — Extraire la logique de gating (effective_mutes/mix_gains) en fonction pure.**
  - `src/lib.rs` : ajout de `compute_mix_gating(mute_states, solo_states, main_mix_enabled)` qui renvoie les tableaux `effective_mutes` et `mix_gains`.
  - La boucle `process()` appelle cette fonction pure au lieu de calculer `effective_mutes` et `mix_gains` en inline ; `track_routings` reste utilisé pour le routage des sorties auxiliaires (`Out N`).
  - Aucun changement de comportement : la logique mute/solo (solo a priorité sur mute) et les gains Main Mix (1.0 si `main_on`, sinon 0.0) sont identiques.
- **Tests unitaires de régression ajoutés.**
  - `src/lib.rs` (module `tests`) : 6 cas couvrant 1 mute, 1 solo, mute+solo, plusieurs solos, aucun mute/solo, et l'interaction indépendante entre solo et Main Mix.

### À tester dans Studio One (build 20260712-160415)
1. Démarrer la lecture sur le pattern par défaut : toutes les lanes actives (BD/SD/HH/Tom) doivent être audibles sur le Main Mix.
2. Cliquer `Mute` sur la lane HiHat : seule la HiHat disparaît du Main Mix ; les autres lanes restent audibles.
3. Cliquer `Solo` sur la lane Snare : seule la Snare est audible (le mute de la HiHat est ignoré), et le solo a priorité sur n'importe quel autre mute.
4. Activer `Solo` sur deux lanes (ex. Kick + Snare) : seules ces deux lanes sont audibles.
5. Désactiver tous les solos, puis désactiver le bouton `Main` dans l'onglet `Track` d'une lane : cette lane ne doit plus alimenter le Main Mix (mais reste audible sur sa sortie aux `Out N` si elle est assignée).
6. Régression : sauvegarder/recharger le projet Studio One doit conserver l'état Mute/Solo/Main Mix de chaque lane.

---


## 2026-07-12 — Fix plock : Saturation Type, Noise Type et Click Type en dropdown (build 20260712-155421)

**Build:** `20260712-155421`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants), `cargo test` OK (155 lib + 97 test_standalone), `build.ps1 -Install` OK (Studio One fermé)

### Changements
- **Correction du menu Plock pour les special params à choix discrets.**
  - `src/ui.rs` : ajout d'un helper `plock_menu_enum_row` et traitement spécifique des labels `Saturation Type`, `Noise Type` et `Click Type` dans `draw_plock_menu`.
  - Ces paramètres sont maintenant rendus avec un dropdown nommé (`styled_select`) au lieu d'un slider numérique.
  - Les options restent les mêmes que dans le Sound Panel : Saturation Type = None / SoftClip / Valve / Transistor / HardClip / Tape ; Noise Type = White / Pink / Brown / Blue ; Click Type = Soft / Medium / Hard.
- **Aucun changement de données** : les valeurs stockées en `special[]` (0..5, 0..3, 0..2) sont identiques ; seul le rendu UI change.

### À tester dans Studio One (build 20260712-155421)
1. Sur une lane Kick, ouvrir le menu Plock d'un step, choisir `Saturation Type` : un dropdown avec les noms doit s'afficher ; sélectionner `Tape` (ou autre) doit appliquer le plock et le step doit sonner saturé.
2. Sur une lane HiHat ou Cymbal, Plock → `Noise Type` : dropdown White / Pink / Brown / Blue ; changer le type doit s'entendre.
3. Sur une lane Kick ou 808 Kick, Plock → `Click Type` : dropdown Soft / Medium / Hard ; changer doit modifier le transient du kick.
4. Sauvegarder le projet Studio One, le rouvrir : les plocks de type enum doivent être restaurés avec les bonnes valeurs (vérifier que le dropdown affiche le nom correct).
5. Régression : les autres special params continus (Amount, Mix, Shimmer, Echo, etc.) doivent toujours apparaître comme des sliders et fonctionner normalement.

---



## 2026-07-12 — AUDIT-6 : tests round-trip des réglages synthèse sur toutes les voix (build 20260712-152139)

**Build:** `20260712-152139`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants), `cargo test` OK (155 lib + 97 test_standalone), `build.ps1 -Install` OK

### Changements
- **AUDIT-6 — Généraliser les tests round-trip des settings à toutes les voix.**
  - `src/synthesis/settings/mod.rs` : ajout de la macro `settings_roundtrip_test!` qui vérifie la conversion `VoiceSettings -> Settings typé -> VoiceSettings` pour tous les champs standard et le tableau `special[32]`.
  - Application de la macro dans chaque fichier `settings/<voix>.rs` : Kick, Snare, HiHat, OpenHiHat, Tom (1/2/3), Clap, Ride, Cymbal, Snare606, Kick808, Perc1.
  - Clap est testé avec `skip_frequency` car le struct `ClapSettings` ne persiste pas le champ `frequency` (réglé à 1000 Hz par défaut lors de la conversion).
- **Correction des défauts `special[]` incohérents avec le type `u8` de `saturation_type`.**
  - `src/synthesis/mod.rs` : les valeurs par défaut de `saturation_type` (stocké en `special[]` et lu comme `u8`) étaient des flottants non-entiers (`0.5` ou `0.01`) qui étaient tronqués à `0` à l’utilisation. Corrigé pour `VoiceSettings::kick()` (`0.01 -> 0.0`), `snare()` (`0.5 -> 0.0`), `hihat()` (`0.5 -> 0.0`), `open_hihat()` (`0.5 -> 0.0`) et `ride()` (`0.5 -> 0.0`).
  - Aucun changement audible : le moteur lisait déjà ces valeurs comme `SaturationType::from(u8)`, donc le type effectif était déjà `None` (0). Cela nettoie simplement les données par défaut pour que le round-trip soit stable.

### À tester dans Studio One (build 20260712-152139)
1. Charger une session avec les réglages par défaut sur Kick, Snare, HiHat, OpenHiHat ou Ride : le son doit être identique à la build précédente (la saturation par défaut reste désactivée).
2. Sur une lane Kick/Snare/HiHat/OpenHiHat/Ride, créer un plock `Saturation Type` avec une valeur autre que `None` : après sauvegarde/recharge du projet, le plock doit être conservé et le type de saturation appliqué.
3. Vérifier que les paramètres spéciaux (Click Level, Snap, Echo, Shimmer, Resonance, etc.) survivent à une copie de lane (`Copy Lane` -> `Paste Lane`).
4. Régression à surveiller : aucune distorsion ou silence inattendu sur les voix par défaut ; les sauvegardes existantes continuent de charger normalement.

---



## 2026-07-12 — AUDIT-4 : générateur déterministe et seedé, mappé par kind (build 20260712-122939)

**Build:** `20260712-122939`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants), `cargo test` OK (143 + 85 tests), `build.ps1 -Install` OK

### Changements
- **AUDIT-4 — Rendre le générateur déterministe et robuste au layout par kind.**
  - `src/generator/mod.rs` : `GeneratorParams` reçoit un champ `seed: u64`. Le générateur instancie son propre RNG ChaCha8 à partir de cette graine et ne dépend plus d'un RNG externe.
  - `generate()` remappe les rôles musicaux selon `kind.drum_voice_index()` / `track_layout`, pas selon l'index de rangée.
  - `src/ui.rs` : le bouton `GENERATE` génère une nouvelle graine à chaque clic et la passe dans `GeneratorParams`.
- **Tests de régression ajoutés.**
  - `generate_is_deterministic_with_same_seed` : même graine → même pattern.
  - `generate_differs_with_different_seeds` : graine différente → pattern différent.
  - `generate_maps_kick_by_kind_not_row_index` : le rôle Kick est assigné au slot de kind Kick même s'il est à l'index 13.
  - `generate_no_kick_when_layout_lacks_kick` : sans Kick dans le layout, aucun slot ne reçoit le rôle Kick.

### À tester dans Studio One (build 20260712-122939)
1. Cliquer plusieurs fois sur `GENERATE` avec les mêmes Style / Density / Variation : chaque clic doit produire un pattern différent, et re-cliquer sur la graine précédente (non possible en UI, mais à vérifier audit) n'est pas requis ; l'important est que le générateur soit stable si on relance avec les mêmes réglages et le même contexte.
2. **Même graine** n'est pas exposée en UI ; valider par l'audit interne : `cargo test` génère deux patterns identiques pour une graine fixe et deux patterns différents pour une graine différente.
3. Sur un layout custom sans Kick (ex. Snare / HiHat / Clap / Ride), `GENERATE` ne doit créer aucun hit sur une position de Kick fantôme ; seuls les kinds présents reçoivent des rôles.
4. Sur un layout avec deux slots Kick, `GENERATE` avec `Variation > 0` doit produire deux patterns Kick différents (même rôle, variations distinctes).
5. Sur un layout avec le Kick déplacé à un slot autre que le premier (ex. slot 13), `GENERATE` doit toujours assigner le rôle Kick à ce slot.
6. Régression à surveiller : le bouton `GENERATE` doit rester actif, les styles doivent rester identifiables, et aucun crash ne doit survenir sur les 4 modes Probabilistic / Markov / Euclidean / Classic.

---



## 2026-07-12 — AUDIT-2 : éliminer les allocations sur le thread audio (build 20260712-113358)

**Build:** `20260712-113358`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants), `cargo test` OK (138 + 85 tests), `build.ps1 -Install` OK

### Changements
- **AUDIT-2 — Suppression des allocations sur le thread audio.**
  - `src/synthesis/mod.rs` : `reinitialize_slot()` réutilise maintenant la `Box<DrumVoiceKind>` existante du slot. Un changement d'instrument sur une lane active n'alloue plus une nouvelle voix sur le tas ; seule la création de voix sur un slot vide alloue (cas rare).
  - `src/pattern_bank.rs` : `capture()` (sauvegarde Pattern Bank) et `restore_from_buffers()` (chargement) utilisent des tableaux fixes `[FusedGroup; MAX_FUSIONS]` à la place de `Vec::with_capacity`/`push` pour les groupes Step Fusion.
  - `src/lib.rs` : les `nih_log!` appelés depuis `process()` (`save_pattern_to_slot`, `load_pattern_from_slot`, `clear_plocks`) et le `println!` de `fire_voice_trigger` sont conditionnés à `#[cfg(debug_assertions)]` ; ils disparaissent en release pour éviter toute allocation/formatage sur le thread audio.

### À tester dans Studio One (build 20260712-113358)
1. Changer l'instrument d'une lane active (onglet `Track` > `Instrument`) : le slot doit rester audible, pas de freeze, pas de drop audio (régression possible : crash si l'enum `DrumVoiceKind` est placé inline sur la pile).
2. Sauvegarder un pattern dans la Pattern Bank (`P1`…`P8`) puis le recharger : la grille, les plocks et les fusions Step Fusion doivent être restaurés tels quels.
3. Créer des fusions Step Fusion, sauvegarder le pattern, recharger : la géométrie et le nombre de pulses des fusions doivent être conservés.
4. Jouer un pattern dense avec des changements fréquents de kind/lane : aucun glitch ou saut de timing lié à une allocation sur le thread audio.
5. Régression à surveiller : le bouton `Clear Plocks` et les actions Save/Load de Pattern Bank doivent toujours fonctionner ; les logs debug ne doivent plus apparaître en build release (pas de "fire_voice_trigger" dans la console DAW).

---


## 2026-07-12 — Step Fusion : DragValue natif pour valider `Enter` sans freeze (build 20260712-110414)

**Build:** `20260712-110414`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI preexistants), `cargo test` OK (138 + 85 tests), `build.ps1 -Install` OK

### Changements
- **Correction definitive de la regression `Enter` dans Step Fusion.**
  - `src/ui.rs` : le champ `Steps` utilise maintenant un `egui::DragValue` natif (meme famille que les champs `Len` et `Repeat`) au lieu d'un `TextEdit` manuel.
  - Le `DragValue` gere `Enter` en interne : saisie, clamp 1..64, sortie du mode edition et perte de focus sont standards egui.
  - L'application detecte `response.lost_focus()` pour appliquer la valeur a la fusion et fermer l'editeur inline.
  - Suppression du `TextEdit` manuel, de `ui.input(Key::Enter)`, de `surrender_focus` et du flag `fusion_edit_close_request` qui faisaient planter Studio One.
  - Le double-clic sur une cellule fusionnee initialise le `DragValue` avec `step_count` et lui donne automatiquement le focus.

### A tester dans Studio One (build 20260712-110414)
1. Double-cliquer une cellule fusionnee : la box Fusion affiche un champ numerique `DragValue` pour `Steps`.
2. Modifier la valeur au clavier (ex. `4`, `8`, `16`) puis appuyer sur `Enter` : Studio One ne doit pas freezer, la valeur doit s'appliquer et la Fusion box doit sortir du mode edition.
3. Saisir `0` puis `Enter` : le champ doit se clamp a `1` et l'edition doit se fermer sans freeze.
4. Saisir `99` puis `Enter` : le champ doit se clamp a `64` et l'edition doit se fermer sans freeze.
5. Cliquer ailleurs ou sur le bouton `X` : la valeur actuelle du `DragValue` doit s'appliquer et l'edition doit se fermer.
6. Verifier que les boutons `Del` et `X` de la Fusion box fonctionnent toujours normalement.
7. Regression a surveiller : le champ `Steps` ne doit pas perdre la valeur saisie quand on passe d'une page a l'autre ou quand on change de lane.

---


## 2026-07-12 — Step Fusion : fix freeze Studio One après Enter (build 20260712-104124)

**Build:** `20260712-104124`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants), `cargo test` OK (138 + 85 tests), `build.ps1 -Install` OK

### Changements
- **Correction de régression du build `20260712-103414`.**
  - Symptôme rapporté : Studio One pouvait freezer après validation du champ Step Fusion `Steps` avec `Enter`.
  - `src/ui.rs` : la validation `Enter` applique toujours la valeur saisie, mais relâche d'abord explicitement le focus clavier egui (`surrender_focus`).
  - La fermeture de l'éditeur inline est décalée à la frame suivante via un flag UI transitoire non persisté, au lieu de supprimer l'état d'édition pendant que le `TextEdit` traite encore la touche.
  - Objectif : éviter le focus clavier orphelin dans le host VST3/Studio One tout en gardant le comportement demandé.

### À tester dans Studio One (build 20260712-104124)
1. Créer une Fusion sur une lane active, double-cliquer la cellule fusionnée, modifier `Steps`, puis appuyer sur `Enter` : Studio One ne doit pas freezer, la valeur doit s'appliquer et l'édition doit se fermer.
2. Répéter l'opération plusieurs fois d'affilée sur la même Fusion (`2`, `3`, `4`, `Enter`) : aucun gel, aucun blocage clavier, la Fusion box revient en mode normal à chaque fois.
3. Saisir `0`, puis `Enter` : la valeur doit être clampée à `1`, l'édition doit se fermer et Studio One doit rester réactif.
4. Saisir `99`, puis `Enter` : la valeur doit être clampée à `64`, l'édition doit se fermer et Studio One doit rester réactif.
5. Régression à surveiller : après validation, cliquer dans d'autres champs texte/menus du plugin et utiliser les raccourcis Studio One ; le clavier ne doit pas rester capturé par le champ `Steps`.

---

## 2026-07-12 — Step Fusion : Enter valide et ferme l'édition Steps (build 20260712-103414)

**Build:** `20260712-103414`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants), `cargo test` OK (138 + 85 tests), `build.ps1 -Install` OK

### Changements
- **Step Fusion : validation clavier du champ `Steps`.**
  - `src/ui.rs` : le `TextEdit` du nombre de steps détecte maintenant `Enter` quand il a le focus ou vient de le perdre.
  - La valeur saisie est parsée puis clampée comme avant dans `1..64`.
  - Si la valeur est valide, `Enter` applique le nouveau `step_count` et appelle la fermeture d'édition inline, ce qui remet la Fusion box en mode normal.
  - Aucun changement audio, pattern, plock ou persistance ; correction UI uniquement.

### À tester dans Studio One (build 20260712-103414)
1. Créer une Fusion sur une lane active, double-cliquer la cellule fusionnée, modifier `Steps`, puis appuyer sur `Enter` : la valeur doit s'appliquer et la Fusion box doit sortir du mode édition.
2. Réouvrir la même Fusion : le nombre de `Steps` doit être conservé avec la valeur validée.
3. Saisir `0`, puis `Enter` : la valeur doit être clampée à `1` et l'édition doit se fermer.
4. Saisir `99`, puis `Enter` : la valeur doit être clampée à `64` et l'édition doit se fermer.
5. Régression à surveiller : cliquer hors du champ doit toujours appliquer/fermer comme avant, et la fusion doit rester active sans désactiver la cellule.

---

## 2026-07-09 — Clear Grid confirmé remplace Clear Lane (build 20260709-182258)

**Build:** `20260709-182258`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants), `cargo test` OK (138 + 85 tests), `build.ps1 -Install` OK (Studio One fermé pour l'installation)

### Changements
- **[116] `Clear Lane` remplacé par `Clear Grid` avec confirmation.**
  - `src/ui.rs` : l'entrée destructive du menu contextuel ne supprime plus le module/lane.
  - Premier clic : le menu passe de `Clear Grid` à `Confirm Clear Grid?`.
  - Deuxième clic : efface les steps, fusions, sound plocks et seq plocks de la lane active.
  - Les données non-grid restent intactes : instrument, réglages sonores, algo, Hum/Push/Len, lock Len, routing, mute/solo/mix et note MIDI.
  - `Clear Grid` n'est disponible que sur une lane active ; les lanes vides gardent seulement `Paste Lane`.

### À tester dans Studio One (build 20260709-182258)
1. Clic droit sur une lane active → `Clear Grid` : au premier clic, rien ne doit être effacé et le menu doit demander `Confirm Clear Grid?`.
2. Cliquer `Confirm Clear Grid?` : tous les steps de cette lane doivent disparaître et la lane ne doit plus déclencher de hits.
3. Vérifier que la lane reste active avec le même instrument, les mêmes réglages Sound, le même routing, la même note MIDI, les mêmes Hum/Push/Len et les mêmes états mute/solo/mix.
4. Sur une lane avec fusions, sound plocks et seq plocks → `Clear Grid` confirmé : recréer des steps aux mêmes positions ne doit pas faire réapparaître les anciennes fusions/plocks.
5. Clic droit sur une lane vide : `Clear Grid` ne doit pas être proposé ; `Paste Lane` doit rester disponible si un clipboard existe.

---

## 2026-07-09 — Clear Lane dans le menu contextuel (build 20260709-181427)

**Build:** `20260709-181427`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants), `cargo test` OK (138 + 85 tests), `build.ps1 -Install` OK

### Changements
- **[116] Ajout de `Clear Lane` dans le menu contextuel d'une lane active.**
  - `src/ui.rs` : action rouge sous `Copy Lane` / `Paste Lane` / `Paste Grid`.
  - `Clear Lane` désactive le slot dans `track-layout-v1`, ce qui remet immédiatement la rangée en lane vide.
  - Nettoyage complet des données cachées du slot : steps, fusions, sound plocks, seq plocks.
  - Remise à l'état neutre des contrôles par lane : mute off, solo off, mix on, algo 0, Hum 0, Push 0, Len 16, lock Len off.
  - Le routing du slot est réinitialisé via `TrackSlot::inactive()` ; le clipboard de lane reste disponible pour pouvoir clear puis coller ailleurs.
  - Après clear, la sélection UI bascule vers la prochaine lane active, ou reste sur le slot vidé si aucune lane active ne reste.

### À tester dans Studio One (build 20260709-181427)
1. Clic droit sur une lane active → `Clear Lane` : la rangée doit devenir vide (`+N` / Empty) et ne plus jouer de son.
2. Sur une lane avec steps, fusions, sound plocks et seq plocks → `Clear Lane`, puis recréer une lane au même slot : aucun ancien step/plock/fusion ne doit réapparaître.
3. Mettre une lane en mute/solo, changer Hum/Push/Len et activer lock Len, puis `Clear Lane` : en recréant une lane au même slot, ces contrôles doivent être revenus à l'état neutre.
4. Faire `Copy Lane`, puis `Clear Lane` sur la source, puis `Paste Lane` ailleurs : le clipboard doit encore coller la lane copiée.
5. Effacer la dernière lane active : l'UI doit rester stable, afficher le slot vide sélectionné, sans crash ni décalage des panneaux.

---

## 2026-07-09 — Copier/coller lanes + cleanup Analog HH/OH (build 20260709-180637)

**Build:** `20260709-180637`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants), `cargo test` OK, `build.ps1 -Install` OK (Studio One fermé pour l'installation)

### Changements
- **[116] Copier/coller une lane depuis le menu contextuel de la grille.**
  - `src/ui.rs` : ajout d'un clipboard de lane en mémoire (`LaneClipboardData`) et des actions `Copy Lane`, `Paste Lane`, `Paste Grid` sur les lanes actives.
  - Les lanes vides acceptent `Paste Lane`, ce qui active le slot cible avec l'instrument du clipboard.
  - `Paste Lane` copie instrument, settings sonores complets, algo, steps, fusions, sound plocks, seq plocks, Humanize, Push/Pull, Len et lock Len.
  - `Paste Grid` remplace `Paste Params` et copie uniquement les steps on/off de la grille sur une lane active cible.
  - `Paste Grid` ne change pas l'instrument, les settings sonores, l'algo, les fusions, les plocks, Humanize, Push/Pull, Len, lock Len, routing, mute/solo/mix ni la note MIDI du slot cible.
  - Routing, Main/Out, note MIDI source personnalisée, mute/solo/mix ne sont pas copiés pour éviter les effets de bord dans une session Studio One.
- **Snapshot de settings par slot pour le clipboard.**
  - `src/sound_settings.rs` : ajout de `SoundSettings` + `get_settings_for_slot()` / `set_settings_for_slot()` pour copier les standards, specials et le mode Hz/Notes sans repasser par les anciens params legacy.
- **Correction de l'implémentation précédente dangereuse.**
  - Le collage des steps ne passe plus par une sérialisation de `Pattern` ni par un remplacement de rangée global ; il ne modifie que le bit du slot cible dans chaque step.
  - Le fichier `shared_pattern_clipboard.rs` a été retiré et `Pattern` reste non sérialisé.
- **Cleanup Analog HH/OH.**
  - Le drift timing HiHat / OpenHiHat ne crée plus de délai silencieux avant le hit (`timing_delay_samples` reste à 0 au trigger).
  - Les logs de debug `println!` ajoutés pendant l'itération Analog ont été retirés.

### À tester dans Studio One (build 20260709-180637)
1. Clic droit sur une lane active (ex. BD) → `Copy Lane`, puis clic droit sur une lane vide (`+N`) → `Paste Lane` : le slot doit s'activer avec le même instrument, le même son et la même séquence.
2. Sur une lane source avec fusions, sound plocks, seq plocks, Hum/Push/Len et Len lock, faire `Copy Lane` puis `Paste Lane` vers une autre lane : tous ces éléments doivent suivre la copie.
3. Sur une lane active cible d'un instrument différent, faire `Paste Grid` : seuls les steps on/off doivent être remplacés ; l'instrument, le son, l'algo, les fusions/plocks, Hum/Push/Len, routing, mute/solo/mix et note MIDI de la cible doivent rester inchangés.
4. Clic droit sur une lane vide : vérifier que `Paste Params` n'existe plus et que `Paste Grid` n'est pas proposé ; seul `Paste Lane` doit permettre d'activer la lane depuis le clipboard.
5. Vérifier que `Paste Lane` ne recopie pas le routing Main/Out ni mute/solo/mix depuis la source : le slot cible ne doit pas déplacer le son vers une sortie inattendue ni hériter d'un état mute/solo.
6. Sur HiHat et OpenHiHat, jouer une pattern dense avec `Analog` élevé : les hits ne doivent pas être retardés/silencieux au départ (régression possible du drift timing), et le tone doit toujours varier par hit.

---

## 2026-07-09 — Analog : tone drift sur HiHat / OpenHiHat / Clap / Ride / Cymbal (build 20260709-141013)

**Build:** `20260709-141013`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants), `cargo test` OK (138 + 85 tests), `build.ps1 -Install` OK (Studio One fermé pour l’installation)

### Changements
- **[115] Le paramètre `Analog` module désormais le tone des instruments non-tonaux.**
  - `src/synthesis/dsp.rs` : ajout de `ToneDrift` avec une profondeur configurable par instrument (0.0 = déterministe, 1.0 = maximum).
  - `src/synthesis/hihat.rs` / `open_hihat.rs` : `Analog` décale le centre du peaking filter (`Tone`) de **±25 %** par hit.
  - `src/synthesis/ride.rs` : `Analog` décale la fréquence de base des oscillateurs inharmoniques (`Frequency`) de **±7.5 %** par hit.
  - `src/synthesis/clap.rs` : `Analog` décale le highpass cutoff de **±25 %** par hit.
  - `src/synthesis/cymbal.rs` : `Analog` décale le highpass cutoff de **±25 %** avant la modulation shimmer.
  - Anti-click : pas de reset de phase, filtre ou générateur de bruit ; la dérive est échantillonnée au trigger et appliquée via les setters de fréquence existants.
  - Tests ajoutés : `tone_drift_is_deterministic_at_zero_and_varies_at_full` (dsp.rs), `test_hihat_analog_affects_tone`, `test_open_hihat_analog_affects_tone`, `test_ride_analog_affects_tone`, `test_clap_analog_affects_tone`, `test_cymbal_analog_affects_tone`.

### À tester dans Studio One (build 20260709-141013)
1. **HiHat** : monter `Analog` à fond et jouer une pattern dense → le pic métallique doit varier clairement d’un hit à l’autre (régression précédente : inaudible avec ±7.5 %).
2. **OpenHiHat** : idem, vérifier que le tone varie.
3. **Ride** : `Analog` à fond → le timbre métallique doit varier (conservé à ±7.5 %).
4. **Clap** : `Analog` à fond → le tone/la brillance doit varier par hit.
5. **Cymbal** : `Analog` à fond → le haut du spectre (cutoff) doit fluctuer.
6. **Regression** : mettre `Analog` à 0 sur ces instruments → chaque hit doit être identique en tone.
7. Vérifier que les instruments tonaux (Kick, Snare, Tom, Kick808, Perc1, Snare606) conservent leur comportement `AnalogDrift` sans régression.
8. Recharger une session sauvegardée avant cette build : le slider `Analog` reste restauré et s’applique au tone des non-tonaux.

---

**Build:** `20260709-123527`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants), `cargo test` OK (132 + 79 tests), `build.ps1 -Install` OK (Studio One fermé pour l’installation)

### Changements
- **[115] Nouvelle section `Analog` dans le Sound Editor, positionnée entre `Envelope` et `Filter`.**
  - `src/instrument_registry.rs` :
    - Ajout de `ParamFamily::Analog`.
    - Déplacement du champ `StandardField::Analog` de `ParamFamily::Output` vers `ParamFamily::Analog` dans toutes les listes standard des 13 instruments.
  - `src/ui.rs` :
    - `sound_family_sections()` affiche désormais la section `Analog` après `Envelope` et avant `Filter`.
    - Ordre final des familles : `Osc`, `Env`, `Analog`, `Filter`, `Sat`, `Output`.
  - `src/synthesis/mod.rs` / `src/synthesis/voice.rs` : `AnalogDrift` reste piloté par le paramètre `analog` pour les instruments tonaux (Kick, Snare, Tom, Kick808, Perc1, Snare606).
  - Les instruments non tonaux (HiHat, OpenHiHat, Clap, Ride, Cymbal, Zap) affichent le slider `Analog` mais ne l’appliquent pas encore en synthèse (pas d’implémentation `AnalogDrift` sur ces voix).
  - Tests de régression existants : `cargo test` OK (132 + 79 tests), `cargo check` OK, pas de nouveaux tests spécifiques ajoutés pour cette refonte.

### À tester dans Studio One (build 20260709-123527)
1. Sélectionner une lane Kick (ou Snare, Tom, Kick808, Perc1, Snare606) → onglet `Sound` : vérifier que la section `Analog` apparaît entre `Envelope` et `Filter` (et non plus dans `Output`).
2. Sur la même lane tonal, monter `Analog` à fond et jouer une pattern rapide : chaque hit doit légèrement varier en hauteur/niveau/temps (drift analogique audible).
3. Sélectionner une lane HiHat (ou OpenHiHat, Clap, Ride, Cymbal, Zap) → onglet `Sound` : vérifier que `Analog` est aussi visible entre `Envelope` et `Filter`.
4. Sur une lane non tonale, bouger le slider `Analog` : le son ne doit pas changer pour l’instant (comportement attendu, pas de drift implémenté sur ces voix).
5. Vérifier que la section `Output` n’affiche plus le slider `Analog` (il n’y reste que `Volume`, `Stereo`, `Main`, `Out`, etc.).
6. Créer un plock sur un step Kick/Snare : vérifier que `Analog` est proposé dans le menu Plock (Snapshot/Link/Morph) et que sa valeur s’applique.
7. Créer une Fusion sur une lane Kick/Snare : vérifier que `Analog` est proposé dans le menu Morph et que le morph entre la valeur globale et la cible s’entend.
8. Recharger une session sauvegardée avant cette build : le paramètre `Analog` doit être restauré à sa valeur globale et le slider doit être visible dans la nouvelle section.
9. Vérifier que les autres instruments conservent leur ordre de sections habituel et que `Env` reste juste avant `Analog`.

---

**Build:** `20260709-121611`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants), `cargo test` OK (132 + 79 tests), `build.ps1 -Install` OK (Studio One fermé pour l’installation)

### Changements
- **[114] Refonte complète du panneau sonore HiHat / OpenHiHat.**
  - `src/instrument_registry.rs` :
    - `Frequency` → `Tone` (range 100–20000 Hz).
    - `Filter` → `Cutoff` (range 100–20000 Hz).
    - Ajout des paramètres spéciaux : `Noise Type` (White/Pink/Brown/Blue), `Resonance` (0.1–10.0), `Shimmer` (0.0–1.0).
    - Suppression de l’algorithme `Bright` (`algo_count: 1`).
  - `src/synthesis/dsp.rs` : ajout de l’enum `NoiseSource` pour sélectionner le type de bruit sans allocation.
  - `src/synthesis/hihat.rs` / `open_hihat.rs` :
    - Remplacement des générateurs `WhiteNoise` fixes par `NoiseSource`.
    - Le peaking filter utilise désormais `settings.resonance` à la place de Q=2.0 fixe.
    - Ajout d’un chemin shimmer parallèle (bruit **bleu** high-pass à 8 kHz, mixé selon `Shimmer` avec un gain de 2.0).
    - Suppression de la branche `algo == 1` (Bright).
  - `src/synthesis/settings/hihat.rs` / `open_hihat.rs` : mapping des nouveaux special params `special[5]` (noise type), `special[6]` (resonance), `special[7]` (shimmer).
  - `src/synthesis/mod.rs` : mise à jour des valeurs par défaut de `VoiceSettings::hihat()` / `open_hihat()` (`resonance = 2.0`).
  - `src/synthesis/special_params.rs` : `HIHAT_ALGOS` réduit à `[Standard]`.

### À tester dans Studio One (build 20260709-121611)
1. Sélectionner la lane HiHat (HH) → onglet `Sound` : vérifier que les paramètres sont `Tone`, `Cutoff`, `Resonance`, `Noise Type`, `Shimmer` (plus d’algorithme).
2. Sélectionner la lane OpenHiHat (OH) → idem, vérifier que les labels sont identiques et qu’il n’y a pas de dropdown Algorithme.
3. Bouger `Tone` de 100 à 20000 Hz sur HH : le pic métallique doit se déplacer clairement dans les graves/aigus.
4. Bouger `Cutoff` : vérifier que l’on comprend la relation avec `Tone` (Cutoff enlève les basses, Tone pousse une bande).
5. Bouger `Resonance` de 0.1 à 10 : à 0.1 le pic doit être très large/doux, à 10 très aigu et crécelle.
6. Changer `Noise Type` (White / Pink / Brown / Blue) : le timbre doit changer de manière audible (White = standard, Brown = sombre, Blue = très aigu).
7. Monter `Shimmer` à 1.0 : un bruit d’air/souffle aigu doit apparaître clairement par-dessus le HiHat (régression à surveiller : rester inaudible).
8. Jouer une pattern avec HH et OH : vérifier qu’il n’y a plus de dropdown Algorithme et que le son est proche de l’ancien `Standard` par défaut.
9. Recharger une session sauvegardée avec des réglages HiHat : les valeurs doivent être restaurées (champs persistants inchangés, seuls les labels et les ranges ont changé).
10. Vérifier que les autres instruments (Kick, Snare, Tom, etc.) conservent encore leurs labels et ranges d’origine.

---

## 2026-07-09 — Clarification HiHat : `Frequency` renommé en `Tone` (build 20260709-101109)

**Build:** `20260709-101109`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants), `cargo test` OK (132 + 79 tests), `build.ps1 -Install` OK (Studio One fermé pour l’installation)

### Changements
- **[114] Clarification du paramètre HiHat / OpenHiHat.**
  - `src/instrument_registry.rs` : création de `HIHAT_STD` et `OPENHIHAT_STD` où le label de `StandardField::Freq` est "Tone" au lieu de "Frequency".
  - Les instruments HiHat et OpenHiHat utilisent désormais ces listes dédiées ; le champ persistant reste `frequency` (pas de migration nécessaire, seul le label UI change).
  - `src/synthesis/hihat.rs` : documentation du rôle de `frequency` (centre du filtre peaking, Q=2, gain=6 dB) et des deux algorithmes (`Standard` vs `Bright`).
  - `src/synthesis/open_hihat.rs` : documentation similaire du peaking filter et du partage des algos avec le HiHat fermé.
  - `src/synthesis/special_params.rs` : commentaire structuré sur les algos HiHat (Standard/Bright).

### À tester dans Studio One (build 20260709-101109)
1. Sélectionner la lane HiHat (HH) → onglet `Sound` : le premier paramètre doit s’appeler `Tone` et non plus `Frequency`.
2. Sélectionner la lane OpenHiHat (OH) → onglet `Sound` : idem, le premier paramètre doit être `Tone`.
3. Jouer une pattern avec des notes HH/OH et bouger le knob `Tone` : on doit entendre clairement le pic métallique se déplacer dans les aigus/graves (pas de silence ou d’effet absent).
4. Passer l’algorithme de `Standard` à `Bright` sur HH ou OH : le son doit devenir plus brillant/saturation légèrement accentuée (régression à surveiller : son identique entre les deux algos).
5. Vérifier que les autres instruments (Kick, Snare, Tom, etc.) conservent encore leur label `Frequency`.
6. Recharger une session sauvegardée avec des réglages HiHat : les valeurs `Tone` doivent être restaurées (le champ persistant est inchangé).

---

## 2026-07-08 — Song Editor : finition, blocks vides assombris, Clear All avec confirmation (build 20260708-185335)

**Build:** `20260708-185335`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants), `cargo test` OK (132 + 79 tests), `build.ps1 -Install` OK (Studio One fermé pour l’installation)

### Changements
- **[112] Polish du Song Editor.**
  - `src/ui.rs` : les widgets pattern/répétition sont maintenant encadrés avec une marge de 2 px à l’intérieur du block, ce qui évite que les bordures débordent quand le block est sélectionné.
  - `src/ui.rs` : les blocks sans pattern ont un fond assombri (`Color32::from_rgb(18, 18, 24)`).
  - `src/ui.rs` : suppression du bouton `Reset`.
  - `src/ui.rs` : `Clear All` demande maintenant une confirmation via un état `song_clear_confirm` (le bouton devient rouge "Confirm?" après le premier clic).
  - Menu contextuel `Copy / Paste / Duplicate / Clear` conservé.

### À tester dans Studio One (build 20260708-185335)
1. Ouvrir l’onglet `Song` → vérifier que les widgets ne débordent plus du contour du block sélectionné.
2. Vérifier que les blocks vides sont plus sombres que les blocks occupés.
3. Confirmer que le bouton `Reset` a disparu.
4. Cliquer sur `Clear All` → le bouton doit devenir rouge `Confirm?` ; cliquer une seconde fois vide la song.
5. Clic droit sur un block → `Copy / Paste / Duplicate / Clear` fonctionnent toujours.
6. Éditer pattern/répétition directement dans les blocks et lire la song pour vérifier le comportement de boucle.

---

## 2026-07-08 — Song Editor : panneau agrandi, édition directe dans les blocks (build 20260708-183824)

**Build:** `20260708-183824`
**Validation:** `cargo fmt` OK, `cargo check` OK (41 warnings UI préexistants + 2 nouveaux `allocate_ui_at_rect` dans la grille), `cargo test` OK (132 + 79 tests), `build.ps1 -Install` OK

### Changements
- **[112] Suite de la refonte du Song Editor.**
  - `src/ui.rs` : hauteur du panneau Song/Generator passée de 180 px à 210 px.
  - Suppression de la rangée d’inspection `Step X` / dropdown Pattern / `Rpt`.
  - Chaque block est maintenant éditable directement : partie supérieure = `ComboBox` de pattern, partie inférieure = `DragValue` de répétition (`xN`).
  - La sélection du block (stroke bleue) est conservée ; le clic sur le fond d’un block le sélectionne.
  - Menu contextuel (clic droit) conservé : `Copy / Paste / Duplicate / Clear`.

### À tester dans Studio One (build 20260708-183824)
1. Ouvrir l’onglet `Song` → vérifier que le panneau est plus haut (210 px) et que les 16 blocks ne sont plus tronqués.
2. Cliquer sur la partie supérieure d’un block → un dropdown permet de choisir le pattern (`P1`–`P8` ou vide).
3. Cliquer/glisser sur la partie inférieure d’un block → régler le nombre de répétitions (`x1`–`x64`).
4. Vérifier qu’il n’y a plus de rangée `Step X` / `Rpt` sous l’en-tête Song.
5. Vérifier que la lecture en mode Song boucle correctement sur les blocks remplis et revient au début sur un block vide.
6. Clic droit sur un block → `Copy / Paste / Duplicate / Clear` fonctionnent.

---

## 2026-07-08 — Song Editor : 16 blocks fixes, mode Song via checkbox, loop implicite (build 20260708-182802)

**Build:** `20260708-182802`
**Validation:** `cargo fmt` OK, `cargo check` OK (39 warnings UI préexistants), `cargo test` OK (132 + 79 tests), `build.ps1 -Install` OK (Studio One fermé pour l’installation)

### Changements
- **[112] Refonte du Song Editor.**
  - `src/pattern_bank.rs` : ajout de `SONG_BLOCKS = 16` et `SongSequence::default().length = 16`.
  - `src/lib.rs` : la longueur du mode song est plafonnée à 16 blocks ; quand le prochain block est vide, la song revient au début ; en fin de song, elle boucle toujours ; le paramètre `loop_enabled` n’est plus utilisé.
  - `src/ui.rs` : l’onglet `Generator | Song` ne fait plus que changer la vue (`bottom_panel_tab` dans `EditorUIState` sérialisé).
  - `src/ui.rs` : ajout d’une checkbox `Song Mode` dans le panneau Song pour activer/désactiver le mode song.
  - Suppression du bouton `Loop` et du paramètre `Len` du mode song.
  - Grille désormais une seule rangée de 16 blocks, chaque cellule affiche le pattern en haut (`P1`) et le nombre de répétitions en bas (`x4`), ou `--` si vide.
  - `Clear All` et `Duplicate` ne touchent plus que les 16 premiers blocks.

### À tester dans Studio One (build 20260708-182802)
1. Ouvrir l’onglet `Song` → vérifier que la grille affiche 16 blocks en une ligne.
2. Cocher `Song Mode` → la lecture doit suivre la song (et non plus le pattern courant).
3. Décocher `Song Mode` → la lecture revient au pattern classique.
4. Remplir les blocks 1-3 avec des patterns, régler leurs répétitions, puis laisser le block 4 vide → en lecture, la song doit boucler sur les 3 premiers blocks et ne jamais avancer au-delà.
5. Remplir le block 16 → en fin de song, elle doit repartir au block 1 automatiquement.
6. Vérifier que le bouton `Loop` a disparu et que `Len` n’est plus présent dans le panneau Song.
7. Clic droit sur un block → `Copy / Paste / Duplicate / Clear` doivent fonctionner sans dépasser 16 blocks.
8. Passer à l’onglet `Generator` puis revenir à `Song` : le panneau doit revenir à la vue Song, et la checkbox doit refléter l’état réel du mode song.

---

## 2026-07-08 — Song Editor : fixes UI dropdown / repeat / couleur / hauteur de ligne (build 20260708-171322)

**Build:** `20260708-171322`
**Validation:** `cargo fmt` OK, `cargo check` OK (39 warnings UI préexistants), `cargo test` OK (132 + 79 tests), `build.ps1 -Install` OK

### Changements
- **[112] Fix UI Song Editor.**
  - `src/ui.rs` : le dropdown de pattern de l’inspecteur utilisait `ui.selectable_label(...).clicked()` et mettait à jour le bank à l’intérieur du `show_ui` ; il est remplacé par `ui.selectable_value(&mut slot, ...)` avec mise à jour du bank après fermeture du popup.
  - Le nombre de répétitions est maintenant affiché dans la cellule de la grille sous la forme `P1x4` (uniquement si `repeat > 1`).
  - Le step courant utilise le bleu `BLUE` à la place du rouge vif, avec le texte en `INK` (blanc) pour rester lisible.
  - Les cellules sont dimensionnées avec `ui.add_sized(Vec2::new(cell_w, cell_h), btn)` et `cell_h` passe à 18 px, ce qui empêche les rangées 2-3-4 d’être absorbées/coupées par la mise en page.
  - Le contexte du bouton de cellule propose `Copy / Paste / Duplicate / Clear`.

### À tester dans Studio One (build 20260708-171322)
1. Ouvrir l’onglet `Song`, sélectionner une step, ouvrir le dropdown `Pattern` et choisir un pattern occupé → la case de la grille affiche `P1` et le step joue ce pattern.
2. Régler `Rpt` à 3 ou plus → la grille affiche `P1x3`, et la lecture répète le pattern 3 fois.
3. Lancer la lecture en mode Song et regarder le step courant → le fond est bleu et le texte blanc reste lisible (pas de rouge).
4. Remplir des steps au-delà de la 16e (rangées 2, 3, 4) → toutes les cases 4×16 sont visibles et alignées.
5. Faire un clic droit sur une cellule → `Copy / Paste / Duplicate / Clear` fonctionnent comme avant.

---

## 2026-07-08 — Song Editor : répétitions par step, inspecteur et workflow retravaillé (build 20260708-164626)

**Build:** `20260708-164626`
**Validation:** `cargo fmt` OK, `cargo check` OK (39 warnings UI préexistants), `cargo test` OK (132 + 79 tests), `build.ps1 -Install` OK

### Changements
- **[112] Rework du Song Editor.**
  - `src/pattern_bank.rs` : ajout du champ `repeats: [u8; 64]` dans `SongSequence` avec `#[serde(default)]` pour compatibilité `pattern-bank-v1`.
  - `src/lib.rs` : le moteur audio reste sur le step courant pendant `repeats` boucles de pattern avant d’avancer ; ajout de `song_repeat_counter` et `last_song_position` pour détecter les resets UI.
  - `src/ui.rs` : panneau Song/Generator passé de 144 px à 180 px ; nouvelle rangée d’inspection avec dropdown `P1-P8` / vide, compteur de répétitions, `Copy` / `Paste` / `Dup` / `Clear`.
  - La grille 4×16 reste : clic gauche sélectionne la step, clic droit menu `Copy / Paste / Duplicate / Clear`.
  - Boutons globaux `Reset` (remet la song à 0) et `Clear All`.
  - Suppression du toggle `Song Enabled` redondant ; le suivi du playhead song fonctionne dès que l’onglet `Song` est actif.
  - Reset automatique de `song_position` quand on quitte le mode Song ou quand le transport s’arrête.
- **Tests ajoutés.**
  - `song_sequence_repeat_clamps_and_defaults`, `pattern_bank_legacy_load_without_repeats_defaults_to_one`, `pattern_bank_persistence_roundtrips_song` mis à jour avec les répétitions.

### À tester dans Studio One (build 20260708-164626)
1. Basculer sur l’onglet `Song` → vérifier que le panneau est plus haut (180 px) et affiche la rangée d’inspection au-dessus de la grille.
2. Sélectionner une step, choisir un pattern dans le dropdown, régler `Rpt` à 4 → lire la song : le pattern doit boucler 4 fois avant de passer à la step suivante.
3. Remplir plusieurs steps avec des répétitions différentes, activer `Loop` et lire → la chaîne avance au bon rythme.
4. Cliquer `Reset` pendant la lecture → la song repart de la step 1 (la position audio se reset au prochain process).
5. Passer de l’onglet `Song` à `Generator` puis revenir à `Song` → la position de lecture doit être remise à 0.
6. Faire un clic droit sur une cellule de la grille → le menu doit proposer `Copy / Paste / Duplicate / Clear`.
7. Copier une step, coller sur une autre → le pattern et le nombre de répétitions doivent être transposés.
8. Vérifier que les songs sauvegardées avant cette build se chargent toujours (test de compatibilité `pattern-bank-v1`).
9. Vérifier que les répétitions sont conservées après sauvegarde/recharge du projet Studio One.

---

## 2026-07-08 — Grille : alignement précis de l’indicateur de drop (build 20260708-162542)

**Build:** `20260708-162542`
**Validation:** `cargo fmt` OK, `cargo check` OK (37 warnings UI préexistants), `cargo test` OK (130 + 79 tests), `build.ps1 -Install` OK

### Changements
- **[108] Correction du décalage du trait de drop.**
  - Le trait est maintenant dessiné à la limite exacte de la lane cible : en haut de la ligne visée, ou en bas de la dernière ligne pour un drop après la fin.
  - `compute_reorder_gap()` retourne un index de gap `0..14` ; `handle_lane_reorder_drop()` le clampe à l’index slot valide `0..13`.
  - `draw_lane_reorder_indicator()` utilise le haut de la ligne cible au lieu du milieu de l’intervalle, corrigeant notamment le positionnement à la fin de la grille.

### À tester dans Studio One (build 20260708-162542)
1. Glisser une lane vers le haut de la grille → le trait doit apparaître exactement au-dessus de la première ligne quand le curseur est dans la moitié supérieure de cette ligne.
2. Glisser une lane vers le bas de la grille → le trait doit descendre au bas de la dernière ligne quand on dépasse son centre, et non rester coincé au milieu de l’intervalle précédent.
3. Déplacer le curseur lentement entre deux lanes → le trait doit basculer nettement au bord supérieur de la lane cible, au même emplacement où la lane sera insérée.
4. Relâcher quand le trait est sur le bord supérieur d’une lane → la lane doit être insérée juste avant cette ligne.
5. Vérifier que les données (steps, plocks, volume, routing, etc.) suivent toujours la lane déplacée.

---

## 2026-07-08 — Grille : feedback visuel de drop pour le drag-reorder (build 20260708-161106)

**Build:** `20260708-161106`
**Validation:** `cargo fmt` OK, `cargo check` OK (37 warnings UI préexistants), `cargo test` OK (130 + 79 tests), `build.ps1 -Install` OK

### Changements
- **[108] Trait indicateur de position de drop pendant le drag-reorder des lanes.**
  - `src/ui.rs` : ajout de `compute_reorder_gap()` et `draw_lane_reorder_indicator()`.
  - Le trait bleu est dessiné dans l’intervalle entre deux lanes (ou au-dessus/en-dessous des extrémités) pendant qu’une poignée est traînée.
  - La position de drop est calculée à partir du centre vertical de chaque lane : le pointeur au-dessus du centre d’une lane déplace la ligne juste au-dessus, en dessous juste au-dessous.
  - `handle_lane_reorder_drop()` utilise maintenant cette logique gap-based pour déterminer l’index cible.

### À tester dans Studio One (build 20260708-161106)
1. Sur le layout 4 lanes, cliquer-glisser la poignée d’une lane → un trait bleu doit apparaître entre les lanes au fur et à mesure du déplacement du curseur.
2. Déplacer le curseur lentement d’une lane à l’autre → le trait doit basculer de manière nette au milieu de l’intervalle entre deux lanes.
3. Relâcher la poignée quand le trait est entre deux lanes → la lane doit être insérée à l’emplacement indiqué par le trait, pas sur la lane survolée.
4. Vérifier que les données (steps, plocks, volume, routing, etc.) suivent toujours la lane déplacée.

---

## 2026-07-07 — Grille : boutons de longueur stables quand `Len < 10` (build 20260707-174844)

**Build:** `20260707-174844`
**Validation:** `cargo fmt` OK, `cargo check` OK (37 warnings UI préexistants), `cargo test` OK (127 + 76 tests), `build.ps1 -Install` OK

### Changements
- **[107] Correction du déplacement des boutons `16/32/48/64` quand l'indicateur `Len` passe à un chiffre.**
  - `src/ui.rs` : l'indicateur `N steps` est maintenant dessiné dans un rectangle alloué en taille exacte.
  - Le nombre est formaté sur 2 digits (` 9`, `10`) et le texte `steps` garde une position fixe.
  - Les boutons de longueur ne dépendent plus de la largeur réelle du texte `9 steps` vs `10 steps`.

### À tester dans Studio One (build 20260707-174844)
1. Descendre `Len` global de `10` à `9` → les boutons `16`, `32`, `48`, `64` ne doivent plus bouger horizontalement.
2. Remonter `Len` de `9` à `10` → les boutons doivent rester exactement en place.
3. Tester `Len` `1`, `8`, `9`, `10`, `16` → l'indicateur change, mais le groupe de boutons reste stable.
4. Vérifier que les boutons `16`, `32`, `48`, `64` et `x2` restent cliquables et fonctionnels.

---

## 2026-07-07 — Grille : largeur stable quand `Len < 10` (build 20260707-174302)

**Build:** `20260707-174302`
**Validation:** `cargo fmt` OK, `cargo check` OK (37 warnings UI préexistants), `cargo test` OK (127 + 76 tests), `build.ps1 -Install` OK

### Changements
- **[107] Correction du décalage UI quand `Len` global passe sous 10.**
  - `src/ui.rs` : la zone complète `Len` de la page-bar réserve désormais une largeur fixe.
  - L'affichage `N steps` a aussi une sous-zone fixe, donc `9 steps` ne réduit plus le bloc par rapport à `10 steps`.
  - La grille conserve la même largeur et le reste de l'interface ne doit plus bouger lors du passage `10 -> 9` ou `9 -> 10`.

### À tester dans Studio One (build 20260707-174302)
1. Dans la page-bar, descendre `Len` global de `10` à `9` avec le slider → le bloc grille ne doit pas rétrécir et aucun panneau ne doit se décaler.
2. Remonter `Len` global de `9` à `10` → aucune expansion/saut horizontal ne doit apparaître.
3. Tester aussi `Len` `1`, `8`, `16`, `32` → la page-bar doit rester stable et les cellules hors longueur doivent toujours s'afficher correctement.
4. Vérifier que les boutons `16`, `32`, `48`, `64` et `x2` fonctionnent toujours.

---

## 2026-07-07 — Grille : contraste inactif renforcé + lanes non activées (build 20260707-173031)

**Build:** `20260707-173031`
**Validation:** `cargo fmt` OK, `cargo check` OK (37 warnings UI préexistants), `cargo test` OK (127 + 76 tests), `build.ps1 -Install` OK

### Changements
- **[107] Contraste renforcé entre cellules actives et inactives.**
  - `src/ui.rs` : l'état disabled utilise maintenant un fond beaucoup plus sombre (`10,10,14`) et une bordure pointillée noire plus épaisse.
  - Les cellules hors longueur et les cellules des lanes non activées partagent désormais le même rendu inactif.
  - Aucun changement audio ou interaction : hors longueur / lanes vides restent non cliquables et non jouées.

### À tester dans Studio One (build 20260707-173031)
1. Mettre `Len` global à `16`, aller page 2 → les cellules hors longueur doivent fortement contraster avec les cellules actives de la page 1.
2. Sur une lane active, régler `Length` individuel à `8` dans `Track` → les steps 9-16 doivent être clairement inactifs, avec fond très sombre + pointillés épais.
3. Regarder une lane non activée (`+N`) → ses cellules doivent avoir le même design inactif que les cellules hors longueur.
4. Cliquer sur une cellule inactivée ou hors longueur → elle ne doit pas s'activer ni déclencher de note.

---

## 2026-07-07 — Grille : pointillés hors longueur plus visibles (build 20260707-171944)

**Build:** `20260707-171944`
**Validation:** `cargo fmt` OK, `cargo check` OK (37 warnings UI préexistants), `cargo test` OK (127 + 76 tests), `build.ps1 -Install` OK

### Changements
- **[107] Retouche visuelle des cellules hors longueur.**
  - `src/ui.rs` : les segments du contour pointillé passent à 5 px avec un trait 2 px.
  - La couleur du pointillé est assombrie pour rendre l'état hors longueur beaucoup plus évident.
  - Aucun changement de logique : ces cellules restent non cliquables et non jouées.

### À tester dans Studio One (build 20260707-171944)
1. Mettre `Len` global à `16`, aller page 2 → les pointillés hors longueur doivent être nettement plus gros et plus sombres qu'avant.
2. Sur une lane active, régler `Length` individuel à `8` dans `Track` → les steps 9-16 doivent être immédiatement identifiables comme hors longueur.
3. Cliquer sur une cellule pointillée hors longueur → elle ne doit pas s'activer ni déclencher de note.
4. Vérifier une lane vide (`+N`) → elle doit rester grisée normalement, sans pointillés sombres.

---

## 2026-07-07 — Grille : cellules hors longueur en pointillé (build 20260707-165324)

**Build:** `20260707-165324`
**Validation:** `cargo fmt` OK, `cargo check` OK (37 warnings UI préexistants), `cargo test` OK (127 + 76 tests), `build.ps1 -Install` OK

### Changements
- **[107] Les cellules hors longueur sont maintenant visuellement distinctes.**
  - `src/ui.rs` : `draw_step_cell_v2()` accepte un état `dashed_border` séparé de `enabled`.
  - Les cellules dont `global_step >= lane_length` conservent le fond désactivé existant mais remplacent la bordure continue par un contour segmenté.
  - Les cellules de slots vides restent avec leur rendu désactivé classique, sans pointillés.

### À tester dans Studio One (build 20260707-165324)
1. Mettre `Len` global à `16`, aller page 2 → les cellules visibles hors longueur doivent être grisées avec bordure pointillée, et ne doivent pas être cliquables.
2. Sur une lane active, régler `Length` individuel à `8` dans `Track` → les steps 9-16 de cette lane doivent apparaître pointillés, sans toucher aux autres lanes.
3. Remettre la lane en `Follow pattern length` → les pointillés doivent suivre à nouveau le `Len` global.
4. Vérifier une lane vide (`+N`) → elle doit rester grisée normalement, sans nouvelle bordure pointillée.

---

## 2026-07-07 — Track tab : retrait Humanize / Push-Pull (build 20260707-164821)

**Build:** `20260707-164821`
**Validation:** `cargo fmt` OK, `cargo check` OK (37 warnings UI préexistants), `cargo test` OK (127 + 76 tests), `build.ps1 -Install` OK

### Changements
- **[106] `Humanize` et `Push/Pull` retirés de l'onglet `Track`.**
  - `src/ui.rs` : suppression des deux lignes de sliders dans la section `Sequencing` du Track tab.
  - Les contrôles restent présents dans la grille, conformément au retour utilisateur.
  - Le tooltip de l'onglet `Track` est ajusté : `Instrument type, MIDI note, routing, length`.

### À tester dans Studio One (build 20260707-164821)
1. Ouvrir l'onglet `Track` sur une lane active → les lignes `Humanize` et `Push/Pull` ne doivent plus apparaître.
2. Vérifier que `Length` est toujours présent et modifiable dans `Track`.
3. Vérifier sur la grille que les mini-sliders `Hum` et `Push` sont toujours visibles et fonctionnels.
4. Modifier `Hum`/`Push` depuis la grille puis lancer la lecture → le comportement audio doit rester identique à avant.

---

## 2026-07-07 — Generator : HiHats différenciés par style (build 20260707-163927)

**Build:** `20260707-163927`
**Validation:** `cargo fmt` OK, `cargo test` OK (127 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[113] Les rôles HiHat ne sont plus quasi identiques entre styles.**
  - `src/generator/styles.rs` : chaque style a maintenant une signature HiHat dédiée.
  - Rock : 8ths + ghost 16ths légers.
  - Funk : offbeat 8ths + ghost notes.
  - Techno / Metal / Disco : 16ths droits.
  - Hip-Hop : sparse/swung.
  - Jazz : 8ths + skip-beat accents.
  - Latin : pattern syncopé type clave.
  - Trap : 8ths avec rolls 16ths très probables.
  - Reggae : one-drop sparse.
- **Test de régression ajouté.**
  - `src/generator/mod.rs` : `hihat_roles_are_style_specific` vérifie qu’au moins 8 signatures HiHat distinctes existent et verrouille des anchors représentatifs (`Funk`, `Latin`, `Reggae`).

### À tester dans Studio One (build 20260707-163927)
1. Layout 4 lanes `Kick / Snare / HiHat / Tom` → `GENERATE` en `Rock` → HiHat majoritairement en 8ths, avec peu de 16ths.
2. Même layout → `Funk` → HiHat sur les offbeats/contretemps, différent du Rock.
3. `Techno` puis `Disco` → HiHat très droit en 16ths, régulier.
4. `Hip-Hop` → HiHat plus sparse/swung, pas le même tapis 8ths que Rock.
5. `Latin` → HiHat syncopé type clave, accents irréguliers.
6. `Trap` → HiHat plus dense, avec beaucoup de 16ths/roll feel.
7. `Reggae` → HiHat sparse one-drop, surtout steps 2/6/10/14.

---

## 2026-07-07 — Générateur : mapping par `track_layout` et variations sur duplicates (build 20260707-161620)

**Build:** `20260707-161620`
**Validation:** `cargo test` OK (126 + 76 tests), `cargo check` OK (40 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[MG-10] Le générateur s’adapte désormais au `track_layout` courant.**
  - `src/generator/mod.rs` : `generate()` accepte un `AtomicTrackLayout` et appelle `remap_roles_to_slots()` après la génération des rôles legacy.
  - Les rôles musicaux (Kick, Snare, HiHat, OpenHH, Tom, Clap, Ride, Cymbal, Snare606, 808 Kick, Perc1) sont assignés aux slots actifs selon leur `TrackInstrumentKind::drum_voice_index()`, et non plus selon leur index de rangée.
  - Les slots vides/inactifs restent silencieux après `GENERATE`.
- **Gestion des duplicates.**
  - Jusqu’à 3 slots `Tom` répartissent naturellement les rôles legacy `Tom1/Tom2/Tom3`.
  - Pour toute autre duplication de kind (ex. deux Kicks) ou un 4e Tom, une variation déterministe est appliquée (shift de phase + éclaircissement/ajouts proportionnels au paramètre `Variation`).
- **Tests de régression ajoutés dans `src/generator/mod.rs`.**
  - `generate_maps_kick_to_kick_slot_not_opens_hh` : en layout 4 lanes par défaut, le slot 3 (Tom) ne reçoit plus le rôle OpenHH.
  - `generate_uses_distinct_tom_roles_for_multiple_toms` : 3 slots Tom produisent des patterns différents.
  - `generate_varies_duplicate_kick_slots` : deux slots Kick avec `Variation=1.0` ne sont pas identiques.
  - `generate_leaves_empty_slots_silent` : les slots inactifs restent vides.

### À tester dans Studio One (build 20260707-161620)
1. **Layout 4 lanes par défaut** (Kick/Snare/HiHat/Tom) → cliquer `GENERATE` (style Rock, density 0.8) → le Tom (lane 4) doit jouer uniquement en fin de mesure (steps 14-15), pas les offbeats d’OpenHH.
2. **Layout legacy 13 voix** → `GENERATE` → chaque instrument reçoit son rôle attendu (Kick sur 1/3, Snare sur 2/4, HiHat en 8e, OpenHH sur offbeats, Toms en fill, etc.).
3. **Deux slots Kick** → `GENERATE` avec `Variation > 0` → les deux lanes Kick ont des patterns différents (pas de copie conforme).
4. **Trois slots Tom** → `GENERATE` → les 3 lanes Tom ont des fills distincts (Tom1/Tom2/Tom3).
5. **Slot vide** → après `GENERATE`, il reste vide (pas de notes parasites).
6. **Tester les 4 modes de générateur** (Probabilistic, Markov, Euclidean, Classic) sur le layout 4 lanes : le Kick/Snare/HiHat/Tom doivent tous recevoir un pattern cohérent avec le style choisi.

---

## 2026-07-07 — Morphing : correction généralisée à tous les instruments (build 20260707-155108)

**Build:** `20260707-155108`
**Validation:** `cargo test` OK (122 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[118] Généralisation du fix morph à tous les instruments.**
  - `src/ui.rs` : popup Morph élargi de 284 px à 350 px et sliders réduits de 104 px à 96 px, afin d’éviter que les longs labels (`Saturation Output Gain`, `bassdrum808_saturation_amount`, etc.) ne poussent le slider hors du cadre et ne perdent l’interaction au relâchement.
  - Clamp systématique des valeurs morph affichées/stockées à `[min, max]` pour Volume, les champs standard sliders et les paramètres spéciaux continus.
- **Cohérence `morphable_fields()` avec le menu Morph.**
  - `src/instrument_registry.rs` : les champs standard de type checkbox (ex. `Stereo`) sont désormais inclus dans `morphable_fields()` avec `min=0.0, max=1.0`, puisque le menu Morph les permet déjà. Évite que la Fusion box affiche `?` à la place de `Stereo` quand celui-ci est une cible de morph.
- **Test de régression ajouté.**
  - `src/lib.rs` : `morphable_fields_include_checkbox_standard_fields` vérifie que chaque champ standard de chaque instrument est présent dans `morphable_fields()` et que les checkbox ont les bonnes bornes.

### À tester dans Studio One (build 20260707-155108)
1. **Tom** : Fusion → Morph → `Saturation Amount` / `Saturation Mix` : valeur fixée au relâchement, `×` visible, ré-ouverture conservée.
2. **Kick / Snare / 808 Kick** : Fusion → Morph → `Saturation Amount`, `Saturation Mix`, et si visible `Saturation Output Gain` (selon l’instrument) : même comportement stable.
3. **HiHat / OpenHiHat / Ride / Cymbal / Snare606 / Perc1** : vérifier que les paramètres continus spéciaux (shimmer, saturation, width, etc.) peuvent être morphés sans retour à la valeur de base.
4. **Kick / Snare606 / 808 Kick / Perc1** : Fusion → Morph → `Stereo` (checkbox) : cocher/décocher, fermer le menu, ré-ouvrir → l’état cible est conservé et la Fusion box affiche `M: Stereo` au lieu de `M: ?`.
5. **Tous instruments** : vérifier visuellement qu’aucun slider du menu Morph ne dépasse de la fenêtre, même après avoir défini une cible (apparition du `×`).

---

## 2026-07-07 — Morphing Tom : popup plus large + clamp morph (build 20260707-153553)

**Build:** `20260707-153553`
**Validation:** `cargo test` OK (121 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK après fermeture de Studio One

### Changements
- **[118] Correction du retour à la valeur de base pour `Saturation Amount` et `Saturation Mix` dans le menu Morph.**
  - `src/ui.rs` : largeur max du popup `plock_menu_frame` augmentée de 284 px à 320 px, car les longs labels `Saturation Amount` / `Saturation Mix` poussaient le slider hors du cadre et faisaient perdre l’interaction au relâchement.
  - Clamp systématique de la valeur morph affichée et stockée à `[min, max]` pour Volume, les champs standard et les specials continus.
- **TODO.md** : ajout de l’item [118] et marquage comme corrigé.

### À tester dans Studio One (build 20260707-153553)
1. Sur une lane Tom, créer une Fusion de plusieurs steps (ex. F 1-4) → ouvrir son menu Morph.
2. Régler `Saturation Amount` sur une valeur autre que 0, relâcher le slider → la valeur reste affichée et le petit `×` apparaît (cible enregistrée).
3. Régler `Saturation Mix` sur une valeur autre que 1.0, relâcher → idem, pas de retour à 1.0.
4. Fermer/ré-ouvrir le menu Morph de la même Fusion → les deux valeurs cibles sont conservées.
5. Lancer la lecture → le morph entre la valeur globale et la cible doit s’entendre sur les pulses de la fusion.
6. Vérifier que les autres paramètres continus du menu Morph (Volume, Freq, Decay, etc.) conservent aussi leur cible au relâchement.

---

## 2026-07-07 — Morphing Tom : params discrets exclus + conflit Attack/saturation évités (build 20260707-151057)

**Build:** `20260707-151057`
**Validation:** `cargo test` OK (121 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **Menu Morph : les paramètres spéciaux discrets ne sont plus proposés.**
  - `src/ui.rs` : le menu morph exclut désormais les specials `!continuous` (ex. `Saturation Type`, `Saturation Pre-Filter`).
  - Ces paramètres étant indexés (pas interpolables), les afficher comme des sliders faisait revenir leur valeur à la base au relâchement.
- **Évite les conflits entre champs standard et spéciaux dans la liste morphable.**
  - `src/ui.rs` + `src/instrument_registry.rs` : un special dont l'index de champ plock entre en collision avec un champ standard (cas connu : `Attack` utilise le champ 18, qui est aussi `SPECIAL_FIELD_START + 4` pour `Saturation Output Gain` / `Saturation Pre-Filter` / `Saturation Type` selon l'instrument) est ignoré dans le menu morph.
  - Cela empêche deux sliders de partager le même champ et de s'écraser mutuellement.
- **Lecture fraîche de l'état morph à chaque ligne.**
  - Remplacement de la closure `morph_state` qui capturait un `group` copié au début du menu par une fonction `fusion_morph_state` qui relit `new_fusions[fusion_index]` à chaque appel. Évite que la valeur affichée ne reprenne un état obsolète.
- **Tests de régression ajoutés.**
  - `src/lib.rs` : vérification que `morphable_fields()` n'a pas d'indices en double, n'inclut pas de params discrets, et n'overlappe pas les champs standard.

---

## 2026-07-07 — Fusion box : centrage vertical du contenu d’édition (build 20260707-142118)

**Build:** `20260707-142118`
**Validation:** `cargo test` OK (118 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[104] Centrage vertical du contenu de la Fusion box en mode édition.**
  - `src/ui.rs` : le layout interne passe explicitement à `left_to_right(Align::Center)`.
  - Les boutons `Del` et `×` sont maintenant alloués dans des emplacements de 18 px de hauteur pour être alignés avec le TextEdit.
  - Objectif : tous les éléments de la ligne d’édition (`F x-y`, `Steps:`, champ, `M: …`, `Del`, `×`) sont sur la même ligne de base / centrés verticalement dans le bloc.

---

## 2026-07-07 — Fusion box : marge interne réduite de 4 px à 3 px (build 20260707-140953)

**Build:** `20260707-140953`
**Validation:** `cargo test` OK (118 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[104] Marge interne de la Fusion box réduite de 4 px à 3 px.**
  - `src/ui.rs` : `inner_margin(4.0)` → `inner_margin(3.0)` et `inner_size` calculé avec `box_size - 6.0` au lieu de `box_size - 8.0`.
  - Objectif : laisser 1 px de plus de chaque côté pour le trait extérieur du cadre, afin que le contenu en mode édition (TextEdit + petits boutons) ne déborde pas visuellement et ne fasse plus sauter la ligne de 1–2 px.

---

## 2026-07-07 — Fusion box : hauteur de ligne verrouillée à 28 px (build 20260707-140333)

**Build:** `20260707-140333`
**Validation:** `cargo test` OK (118 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[104] La ligne `P-Lock Mode / Fusion` est maintenant clampée à exactement 28 px de haut.**
  - `src/ui.rs` : `ui.set_height(28.0)` remplacé par `ui.set_min_size(..., 28.0)` + `ui.set_max_size(..., 28.0)` sur le `horizontal` parent.
  - Conséquence : le passage idle ↔ édition d’une fusion ne peut plus faire pousser/rétrécir la ligne, même si les widgets internes (TextEdit, petits boutons) ont des tailles naturelles différentes.

---

## 2026-07-07 — Validation installée : Fusion box, Plock Frequency, preset Tom (build 20260707-135525)

**Build:** `20260707-135525`
**Validation:** `cargo test` OK (118 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[104][105][111] Build final installé après fermeture de Studio One.**
  - Aucun changement de code supplémentaire ; ce build consolide les corrections déjà documentées dans les builds `20260707-125743`, `20260707-113932` et `20260707-120216`.
  - Installation réussie dans `C:\Program Files\Common Files\VST3\drum-pattern-vst.vst3`.

---

## 2026-07-07 — Fusion box : allocation de taille exacte, plus de saut d’interface (build 20260707-125743)

**Build:** `20260707-125743`
**Validation:** `cargo test` OK (118 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[104] La Fusion box utilise maintenant `allocate_exact_size` (380×28 px).**
  - `src/ui.rs` : le rectangle externe est alloué avec une taille fixe, indépendamment du contenu idle ou édition.
  - Le contenu est dessiné dans ce rectangle via `allocate_ui_at_rect`, avec une taille interne min/max verrouillée.
  - Conséquence : la hauteur de la ligne P-Lock Mode/Fusion reste identique ; la Pattern Bank et le Bottom Panel ne bougent plus quand on entre/sort de l’édition d’une Fusion.

---

## 2026-07-07 — Fusion box : suppression du micro-saut idle/édition (build 20260707-124135)

**Build:** `20260707-124135`
**Validation:** `cargo test` OK (118 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[104] Fix du décalage de 2-3 px entre idle et édition de la Fusion box.**
  - `src/ui.rs` : la zone interne de la Fusion box est maintenant contrainte à une taille max égale à sa taille min (20 px de contenu), empêchant les boutons d’agrandir la hauteur en mode édition.
  - Boutons `Del` et `×` passés en `small_button` pour tenir dans l’espace fixe.

---

## 2026-07-07 — Placement Fusion box : même ligne que P-Lock Mode (build 20260707-121720)

**Build:** `20260707-121720`
**Validation:** `cargo test` OK (118 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[104] Fusion box sur la même ligne que le sélecteur P-Lock Mode.**
  - `src/ui.rs` : la Fusion box (380 px) est dessinée à droite de la barre `P-Lock Mode | Sound/Sequencer` au lieu d’occuper une ligne dédiée sous la grille.
  - Réduction de la largeur de la box de 720 px à 380 px ; labels et boutons compactés (`F x-y`, `M: ...`, `Del`, `×`).
  - Conséquence : la Pattern Bank et le Bottom Panel ne sont plus décalés vers le bas par la Fusion box.

---

## 2026-07-07 — Ajustement Tom : fréquence par défaut à 196 Hz (build 20260707-120216)

**Build:** `20260707-120216`
**Validation:** `cargo test` OK (118 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[111] Fréquence du Tom par défaut fixée à 196 Hz.**
  - `instrument_registry.rs` : Tom1 (lane `Tom` par défaut) passe de 150 Hz à **196 Hz** ; Tom2 reste à 150 Hz, Tom3 à 100 Hz.
  - `synthesis/mod.rs` : `VoiceSettings::tom1()` aligné sur 196 Hz.

---

## 2026-07-07 — Preset Tom retravaillé (build 20260707-115036)

**Build:** `20260707-115036`
**Validation:** `cargo test` OK (118 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **[111] Preset Tom plus musical/utilisable dès création de lane.**
  - `instrument_registry.rs` : ajustement des `sound_settings_default` des 3 voix Tom et du défaut du paramètre spécial *Stick Attack* (0.5 → 0.3).
  - `synthesis/mod.rs` : alignement de `VoiceSettings::tom1/2/3()` sur les nouveaux défauts du registre.
  - Nouvelle famille Tom :
    - **Tom1** (utilisé par la lane `Tom` par défaut) : 150 Hz, decay 0.35 s, volume 0.7, filter 600 Hz, release 0.25 s.
    - **Tom2** : 200 Hz, decay 0.30 s, volume 0.7, filter 650 Hz, release 0.20 s.
    - **Tom3** : 100 Hz, decay 0.45 s, volume 0.7, filter 500 Hz, release 0.35 s.
  - Objectif : moins aigu et plus audible que l’ancien défaut Tom1 à 300 Hz / volume 0.5.

---

## 2026-07-07 — Plock sound : vérification Frequency > 0 + tests de régression (build 20260707-113932)

**Build:** `20260707-113932`
**Validation:** `cargo test` OK (118 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **Investigation du retour [105] : `Frequency` à 0 par défaut dans le menu plock sound.**
  - Le code actuel (`ST-7` + `reset_slot_to_defaults`) initialise correctement la fréquence globale de chaque slot depuis les défauts du registre ; le menu plock sound affiche déjà la valeur globale courante quand aucun override n’est actif.
  - Aucun instrument du registre n’a de fréquence par défaut à 0 (Kick 60, Snare 220, HiHat 1000, OpenHH 300, Tom 120/200/120, Clap 1000, Ride 3000, Cymbal 5000, Snare606 220, BassDrum808 50, Perc1 800).
- **Ajout de tests de régression dans `sound_settings.rs`.**
  - `default_frequency_is_nonzero_for_every_instrument_kind` : pour chaque kind, un slot actif obtient la fréquence par défaut attendue et elle est strictement positive.
  - `duplicate_slots_keep_nonzero_default_frequency` : deux slots B8 ont chacun la fréquence par défaut 50 Hz et sont indépendants.

---

## 2026-07-07 — Ajustement visuel : flash `T` en AMBER (build 20260707-111442)

**Build:** `20260707-111442`
**Validation:** `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **Le flash visuel du bouton `T` passe du bleu à l’ambre.**
  - L’indicateur d’activité MIDI externe utilise maintenant `AMBER` avec du texte noir, plus harmonieux avec les pastilles rouge (`M`) et verte (`S`).

---

## 2026-07-07 — Ext MIDI : playhead gelée, flash T, swing exporté (build 20260707-103907)

**Build:** `20260707-103907`
**Validation:** `cargo test` OK (116 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **En mode Ext MIDI, la tête de lecture interne est masquée.**
  - Quand `Seq` est sur `Ext MIDI`, `current_step`/`current_steps` sont stockés avec une valeur hors plage (`u32::MAX`) au lieu de suivre le transport hôte.
  - Aucune cellule de grille n’est donc surlignée comme "en cours" ; le plugin se contente de répondre aux notes MIDI entrantes.
- **Le bouton `T` (Test) de chaque lane clignote quand la lane est déclenchée par MIDI externe.**
  - Le thread audio lève un drapeau atomique par slot à la réception d’un `NoteOn` correspondant.
  - L’UI lit ce drapeau et allume le `T` en bleu pendant ~100 ms.
- **L’export MIDI (fichier + drag) applique maintenant le swing/groove.**
  - `midi_export.rs` reçoit `swing` et `groove_type` ; les steps impairs sont décalés selon l’algorithme actif (`Swing16`, `Shuffle`, `MPC`).
  - Test ajouté : un step 1 avec Swing16 +50 % est exporté à 160 ticks au lieu de 120.

---

## 2026-07-07 — Restauration du drag & drop MIDI + export 14 slots (build 20260707-094444)

**Build:** `20260707-094444`
**Validation:** `cargo test` OK (115 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **Restaure le drag & drop MIDI dans Studio One.**
  - Le bouton `Drag` est de retour dans la barre Patterns (à gauche, à côté de `Export`).
  - Le helper Windows (`drum-pattern-midi-drag-helper.exe`) est lancé au clic ; il faut ensuite cliquer-glisser la petite fenêtre `Flash Drum MIDI Drag` vers Studio One.
- **Corrige l’export MIDI pour les 14 slots et les notes personnalisées.**
  - `midi_export.rs` itère sur `0..MAX_TRACKS` au lieu des 13 voix legacy.
  - Il lit `track_layout.midi_note_for_slot(slot)` pour chaque slot actif, donc une note MIDI modifiée dans l’onglet `Track` est respectée.
  - Test ajouté : le 14e slot avec une note personnalisée est bien exporté.

---

## 2026-07-06 — UI routing : `No Aux` remplace `Main` dans la liste `Out` (build 20260706-192033)

**Build:** `20260706-192033`
**Validation:** `cargo test` OK (114 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- Dans l'onglet `Track`, la liste `Out` n'affiche plus `Main`.
- L'état sans sortie auxiliaire dédiée est maintenant libellé `No Aux`.
- Le switch `Main` reste le seul contrôle pour envoyer ou retirer la lane du Main Mix.

---

## 2026-07-06 — Fix init synth layout : slot Tom réactivé en OpenHH (build 20260706-190624)

**Build:** `20260706-190624`
**Validation:** `cargo test` OK (114 + 76 tests), `cargo check` OK (37 warnings UI préexistants), `cargo test mapped_aux_output_idx` dans `vendor/nih-plug` OK (3 tests), `build.ps1 -Install` OK

### Changements
- **Corrige une cause interne du Tom qui pouvait sonner comme un HH après réactivation/routing.**
  - `DrumFlashVst::initialize()` initialisait encore le synthé avec le layout legacy 13 voix (`slot 3 = OpenHH`).
  - Si `last_slot_kinds` indiquait déjà le layout modulaire (`slot 3 = Tom`), le process ne réinitialisait pas ce slot, et la lane Tom pouvait garder physiquement une voix OpenHH.
  - Le synthé est maintenant initialisé directement avec le `track-layout` courant et `last_slot_kinds` est aligné sur ce layout.
- Test ajouté : le layout modulaire par défaut initialise bien le slot 4 en `Tom`, pas en `OpenHiHat`.

---

## 2026-07-06 — Fix VST3 sparse aux outputs Studio One (build 20260706-185857)

**Build:** `20260706-185857`
**Validation:** `cargo test` OK (113 + 75 tests), `cargo check` OK (37 warnings UI préexistants), `cargo test mapped_aux_output_idx` dans `vendor/nih-plug` OK (3 tests), `build.ps1 -Install` OK

### Changements
- **Corrige le bug profond où une lane routée vers `Out 2` pouvait sortir comme une autre lane selon les sorties activées dans Studio One.**
  - Le wrapper VST3 vendored mémorise maintenant les bus audio activés via `activateBus()`.
  - Pendant `process()`, les buffers auxiliaires compactés fournis par l'hôte sont remappés vers leurs vrais indices `Out N` au lieu d'être supposés en préfixe `Out 1..N`.
  - Cas ciblé : Studio One active une sortie sparse (`Main + Out 2` sans `Out 1`) ; le premier buffer aux reçu est maintenant mappé vers `Out 2`, pas `Out 1`.
- La validation défensive des buffers aux utilise le même mapping sparse.
- Tests vendor ajoutés : fallback préfixe sans info d'activation, `Main + Out 2`, et sorties sparse multiples.

---

## 2026-07-06 — Fix routing : sortie auxiliaire exclusive par lane (build 20260706-175157)

**Build:** `20260706-175157`
**Validation:** `cargo test` OK (113 + 75 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **Corrige le cas où un Tom routé vers `Out 2` pouvait sembler sonner comme un HH.**
  - L'assignation `Track > Out` est maintenant exclusive : si une lane prend `Out N`, toute autre lane déjà routée vers ce même `Out N` repasse en sortie auxiliaire `Main`/aucune aux dédiée.
  - Objectif : éviter qu'un ancien HH ou autre slot reste caché sur le même bus auxiliaire et masque la lane qu'on vient d'assigner.
- Tests ajoutés : exclusivité d'un `Out N` entre slots, et non-régression quand une lane repasse sur `Main`.

---

## 2026-07-06 — Renommage des sorties DAW en `Out 1..14` (build 20260706-173427)

**Build:** `20260706-173427`
**Validation:** `cargo test` OK (111 + 73 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- Les noms de ports auxiliaires exposés au DAW sont maintenant génériques : `Out 1` à `Out 14`.
- Suppression des anciens noms de bus hérités (`Kick`, `Snare`, `Hi-Hat`, `Open HH`, etc.) dans `OUTPUT_PORT_NAMES`.
- Le routing audio reste celui de la build précédente : chaque slot est envoyé vers la sortie choisie dans `Track > Out`.

---

## 2026-07-06 — Fix routing Track : sorties auxiliaires par slot (build 20260706-172704)

**Build:** `20260706-172704`
**Validation:** `cargo test` OK (111 + 73 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **Corrige la régression où changer `Track > Out` donnait l'impression de changer le son/instrument.**
  - Le moteur audio lit maintenant le routing `TrackRouting` par slot au lieu d'envoyer implicitement `slot N -> Out N`.
  - `Main` suit la case `Main` du slot ; `Out N` suit le sélecteur `Out` du même slot.
  - Plusieurs lanes routées vers le même `Out` sont additionnées au lieu de s'écraser.
- Le helper d'écriture aux reste défensif : bus inactifs, mono, incomplets ou trop courts sont ignorés sans panic ni écriture invalide.
- Tests ajoutés/ajustés : écriture aux inactive/incomplète ignorée, sortie stéréo valide accumulée.

---

## 2026-07-06 — Fix multi-out : écriture aux défensive pendant activation/désactivation DAW (build 20260706-141836)

**Build:** `20260706-141836`
**Validation:** `cargo test` OK (111 + 73 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **Corrige le P0 [117] : distorsion lors de l'activation/désactivation d'une sortie dans le DAW.**
  - L'écriture des sorties auxiliaires ne suppose plus que chaque bus fourni par l'hôte contient toujours 2 canaux non vides.
  - Les bus aux inactifs, mono, incomplets ou transitoirement vides sont ignorés au lieu d'être indexés par `channels[0][sample_idx]` / `channels[1][sample_idx]`.
  - Objectif : éviter les écritures dans des buffers invalides/stale pendant les changements d'activation de sorties Studio One.
- Tests ajoutés : sorties aux inactives/incomplètes ignorées sans panic, sortie stéréo valide écrite correctement.

---

## 2026-07-05 — Fix song-mode : reset step 0 après pattern de longueur différente (build 20260705-150850)

**Build:** `20260705-150850`
**Validation:** `cargo test` OK (109 + 73 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **Corrige le song-mode quand les patterns n'ont pas tous la même longueur.**
  - `load_pattern_from_slot()` applique maintenant la longueur chargée côté audio immédiatement, sans attendre que l'UI applique `pending_pattern_length`.
  - Après une transition song réussie, le séquenceur est redémarré à step 0 au bloc suivant avec les longueurs de lanes recalculées.
  - La resynchro continue à la timeline absolue du DAW est désactivée pendant le song-mode, pour éviter qu'elle recale la tête au milieu du nouveau pattern.
- Tests ajoutés : longueur audio immédiate après load et redémarrage step 0 après changement de longueur.

---

## 2026-07-05 — AUDIT-1 : PatternBank non bloquant sur thread audio (build 20260705-132937)

**Build:** `20260705-132937`
**Validation:** `cargo test` OK (107 + 72 tests), `cargo check` OK (37 warnings UI préexistants), `build.ps1 -Install` OK

### Changements
- **Thread audio : les accès `PatternBank` ne bloquent plus.**
  - Les chemins save/load pattern et song-mode utilisent `try_lock()` au lieu de `lock()` dans `process()`.
  - Si l'UI détient temporairement le lock, la demande save/load est conservée et retentée au bloc audio suivant.
  - En song-mode, un wrap de pattern n'est consommé que lorsque la lecture du bank et le chargement du slot ont réellement réussi ; en cas de contention, le changement est retenté sans bloquer le callback audio.
- Test ajouté : `pattern_bank_actions_return_busy_instead_of_blocking_when_locked` vérifie que save/load retournent `Busy` quand la banque est déjà verrouillée.
- TODO audit mis à jour : [AUDIT-1] `try_lock()` + report save/load/song cochés ; la phase optionnelle double-buffer/SPSC reste ouverte.

---

## 2026-07-05 — ST-7 : instances par slot complètes + onglets Sound/Track + picker instrument (build 20260705-122315)

**Build:** `20260705-122315`
**Validation:** `cargo test` OK (106 + 72 tests, dont 3 nouveaux tests persistance/migration), `build.ps1 -Install` OK

### Changements
- **ST-7 — Special params par slot (fix "le Click Type de la lane 5 change celui de la lane 1").**
  - `special[32]` + mode Hz/Notes stockés PAR SLOT dans `SoundSettingsState`, seedés depuis les défauts du registre.
  - Persistance : `sound-settings-v2` format v3 (46 floats/slot) ; les anciennes sessions sont migrées automatiquement depuis les params par voix (`needs_param_seed`, seed one-shot RT-safe dans `process()`).
  - Moteur : `voice_settings_for(slot, voice, …)` lit specials + algo par slot ; UI Sound Panel, menus plock/morph, Snapshot et morphing rebranchés sur les atomics par slot.
  - Ranges algo unifiés ("Slot N Algo", `max_algo_index()`) : un Kick sur n'importe quel slot peut changer d'algo ; fixe aussi les ranges 0..0 crashogènes (`algo_cymbal`, `algo_s13`).
  - ⚠️ Les special params ne sont plus automatisables par le DAW (ils restent plockables par step) ; les params legacy servent uniquement de source de migration.
- **Onglets refaits : `Sound Editor` | `Track` (retour utilisateur).**
  - Les boutons par instrument disparaissent — la lane éditée se choisit en cliquant dans la grille ; l'en-tête affiche toujours "Slot N - nom".
  - Onglet Track complet : type d'instrument, note MIDI, routing Main/Out, **Humanize, Push/Pull, Length** (mêmes params que les mini-sliders de lane) ; message dédié si le slot est vide.
- **Choix de l'instrument à la création (retour utilisateur).**
  - Cliquer la pastille `+N` d'une lane vide ouvre un menu avec les 11 instruments ; le slot est créé avec le kind choisi (plus de Kick imposé).
- **Fix : le lock de longueur de lane était indexé par voix côté UI** (`draw_track_length_control`) alors que l'audio le lit par slot — aligné sur le slot.
- Docs : `AGENTS.md` (nouvelle section "Per-slot instances"), `CLAUDE.md` (invariant mis à jour), `ADDING_AN_INSTRUMENT.md` (étapes params specials marquées obsolètes).

---

## 2026-07-04 — Défaut 4 lanes + grille à hauteur fixe 14 rangées (build 20260704-195335)

**Build:** `20260704-195335`
**Validation:** `cargo test` OK (103 + 72 tests — un test de migration supprimé volontairement), `build.ps1 -Install` OK

### Changements
- **Nouveau défaut : 4 lanes (BD/SD/HH/Tom)** — décision produit 2026-07-04.
  - `TrackLayoutState::default_layout()` retourne le template modulaire 4 slots au lieu des 13 voix legacy.
  - Suppression de la migration `is_buggy_four_track_template` (elle re-transformait tout layout 4 lanes exact en 13 lanes au rechargement — incompatible avec le nouveau défaut).
  - ⚠️ Compat : les songs sauvegardées AVANT l'existence de `track-layout-v1` s'ouvriront avec 4 lanes (leurs patterns des autres instruments restent stockés mais inactifs). Les songs avec un layout sauvegardé conservent leur layout.
- **Grille à hauteur constante : 14 rangées toujours rendues** (règle UI : aucune ligne conditionnelle qui décale les zones).
  - Les slots inactifs sont rendus comme lanes vides stylées ; la pastille `+N` de chaque lane vide active CE slot (curseur main + tooltip).
  - Suppression de la rangée `+ Add Module` (elle apparaissait/disparaissait et décalait les panneaux du bas).
  - Les panneaux sous la grille (P-Lock mode, patterns, generator/song) ne bougent plus jamais.
- Nettoyage : `visible_lane_count()` et `draw_add_module_row_v2()` supprimés ; activation factorisée dans `activate_slot(slot_idx)`.
- **Limitation connue rendue plus visible par le défaut 4 lanes :** les générateurs de patterns supposent encore les rôles legacy par rangée (rangée 4 = OpenHH, etc.) alors que la lane 4 du template est un Tom — voir [MG-10].

---

## 2026-07-04 — La pastille `+N` de la lane vide active le slot (build 20260704-174006)

**Build:** `20260704-174006`
**Validation:** `cargo test` OK (104 + 73 tests), `build.ps1 -Install` OK

### Changements
- **Corrige "quand je clique sur +14 rien ne se passe" (rapporté par test S1).**
  - La lane vide affichait une pastille `+14` qui s'illuminait au survol mais n'était pas cliquable (`Sense::hover` seulement) — le seul bouton actif était la rangée `+ Add Module` en dessous.
  - La pastille `+N` est maintenant cliquable (curseur main, tooltip "Activate this slot") et déclenche la même activation que `+ Add Module`.
  - Logique d'activation factorisée dans `activate_next_free_slot()` (layout + reset des settings du slot + sélection).

---

## 2026-07-04 — Fix trigger : settings et plocks appliqués par slot (build 20260704-173043)

**Build:** `20260704-173043`
**Validation:** `cargo test` OK (104 + 73 tests), `build.ps1 -Install` OK

### Changements
- **Corrige "la freq de la lane 1 change celle de la lane 14" (rapporté par test S1).**
  - `voice_settings_at_step()` lisait les settings standards (`sound_settings_state.instruments[...]`) et les plocks (`plock_state.get_settings(...)`) avec l'index de **voix** alors que ces stockages sont par **slot**.
  - À chaque trigger, un slot dupliqué (ex : 2e Kick) se voyait réappliquer les settings ET les plocks du premier slot du même kind, écrasant le push par slot correct fait en début de bloc.
  - Fix : `voice_settings_at_step(slot_idx, voice_idx, step)` — settings et plocks par slot, schéma/special params par voix. Trois appelants corrigés (séquenceur, MIDI thru, test triggers).
- **Limitation restante (ST-7, connue) :** les special params (Click, Saturation, mode Hz/Notes) restent des paramètres nih-plug par voix legacy — physiquement partagés entre deux slots du même kind, dans l'UI comme dans le moteur.

---

## 2026-07-04 — Stabilisation modular grid 14 slots (build 20260704-165252)

**Build:** `20260704-165252`
**Validation:** `cargo check` OK, `cargo test` OK (104 + 73 tests), `build.ps1 -Install` OK

### Changements
- **Corrige le crash Studio One à l'ajout de la 14e piste (ST-1).**
  - `EditorUIState.fusion_selection_start` était encore taillé à 13 (`DrumVoice::COUNT`) mais indexé par slot (0..14) dans la boucle de grille → index out of bounds dès le rendu de la lane 14. Passé à `MAX_TRACKS`.
- **Corrige deux crashs latents des menus plock sur la lane 14 (ST-2).**
  - `INSTRUMENTS[slot_idx]` (13 entrées) dans les menus Plock / Morph / Seq Plock, et `DrumVoice::from_index(slot).expect(...)` dans le dropdown Algo.
  - Les menus résolvent maintenant le schéma via l'index de voix dérivé du kind du slot (`schema_voice_idx`), le stockage plock reste indexé par slot.
- **Corrige le son défectueux d'un slot ajouté (ST-3).**
  - `SoundSettingsState::reset_slot_to_defaults()` n'était jamais appelé : un slot activé via `+ Add Module` gardait les réglages d'init de la voix legacy de même index (ex : un Kick au slot 5 jouait avec des réglages de Tom).
  - Reset aux défauts du kind à l'activation et au changement d'instrument dans l'onglet TRK.
- **Sépare index de slot et index de voix dans le Sound Editor (ST-4).**
  - `selected_instrument` est désormais un index de slot (0..14) ; le schéma de paramètres (registre, special params, filter label, checks Kick/B8, liste d'algos) est dérivé du kind du slot.
  - Changer le type dans l'onglet TRK ne fait plus sauter la sélection sur un autre slot (= le "impossible de choisir le type" du test S1).
  - Les onglets du Sound Editor listent les slots actifs (labels par kind, tooltip avec numéro de slot) au lieu des 13 voix fixes.
- **Aligne la longueur de lane UI sur le moteur audio.**
  - `effective_lane_length_for_ui` utilise l'index de slot (comme `raw_lengths` / `lane_length_locks` côté audio) au lieu de l'index de voix.
- Reste à valider dans Studio One (ST-6) : ajout jusqu'à 14 pistes, clic droit lane 14, changement de type via TRK, son correct, Out 14 audible. Le layout "4 lanes par défaut" observé reste à éclaircir (ST-5).

---

## 2026-07-02 — MG-7a.2: activate 14th track slot + Track tab (build 20260702-215053)

**Build:** `20260702-215053`
**Validation:** `cargo check` OK, `cargo test` OK (104 + 73 tests), `build.ps1 -Install` OK

### Changements
- **Active le bouton `+ Add Module` : le slot 14 devient une piste fonctionnelle.**
  - Le bouton active le premier slot inactif avec l'instrument par défaut Kick (réassignable via l'onglet Track).
  - Le séquenceur, le moteur audio et les sorties auxiliaires itèrent maintenant sur `MAX_TRACKS = 14` slots.
  - `AUX_OUT_COUNT` passe à 14 ; le bus 14 est nommé `Out 14`.
  - Le `DrumSynthesizer` réinitialise automatiquement un slot dont le `kind` change dans `track-layout-v1`.
- **Ajoute un onglet `TRK` (Track) dans le Sound Editor.**
  - Affiche le slot sélectionné et permet de changer son instrument (Kick, Snare, HiHat, ...).
  - Permet de réguler le routing Main / Out et la note MIDI du slot.
- **Persistance et migration.**
  - Le champ de pattern DAW passe de `pattern-v4` à `pattern-v5` (14 rangées d'instruments).
  - Migration automatique depuis `pattern-v4` (13 instruments) et `pattern-v3` (13 instruments + fusion legacy).
  - `track-layout-v1` reste le champ de persistance de la disposition ; l'état par défaut reste la migration legacy 13 voix.
- **Rupture de compatibilité volontaire pour les projets Studio One existants.**
  - Le nombre de sorties stéréo auxiliaires change (13 → 14) : les projets sauvegardés devront réaffecter leurs bus aux.
  - L'identité VST3 (`DrumFlashPlugin1`) est volontairement conservée pour ne pas casser l'insert du plugin lui-même.

---

## 2026-07-01 — Fix plugin fixed height after empty modular lane (build 20260701-230011)

**Build:** `20260701-230011`
**Validation:** `cargo check` OK, `cargo test` OK (103 + 73 tests), `build.ps1 -Install` OK

### Changements
- Augmente la taille fixe de l'éditeur VST de `1480x800` à `1480x900`.
- Corrige le bas de l'interface masqué après l'ajout visuel du slot 14 vide et de la rangée `+ Add Module`.
- Aucun changement audio, VST3, routing ou persistance DAW.

---

## 2026-07-01 — Modular UI checkpoint 5: visual empty slot and Add Module placeholder (build 20260701-205643)

**Build:** `20260701-205643`
**Validation:** `cargo check` OK, `cargo test` OK (103 + 73 tests), `build.ps1 -Install` OK

### Changements
- **Ajoute un checkpoint visuel sûr pour MG-7a.**
  - Affiche le slot 14 comme lane vide stylée sous les 13 lanes legacy actives.
  - Ajoute une rangée `+ Add Module` sous les lanes.
  - Le bouton reste volontairement visuel/inactif dans ce checkpoint : aucune activation de piste, aucune mutation de `track-layout-v1`, aucun changement audio, VST3 ou DAW state.
- Prépare le prochain checkpoint qui pourra activer l'ajout de module de façon contrôlée.

---

## 2026-07-01 — Fix individual lane length beyond global length (build 20260701-201011)

**Build:** `20260701-201011`
**Validation:** `cargo check` OK, `cargo test` OK (103 + 73 tests), `build.ps1 -Install` OK, validation Studio One OK

### Changements
- **Corrige le comportement de `Len` individuel sur les lanes.**
  - Une lane lockée utilise maintenant sa propre longueur brute `1..64`, même si elle dépasse la longueur globale du pattern.
  - Le séquenceur accepte les longueurs par piste jusqu'à 64 au lieu de les re-clamper sur la longueur globale.
  - La grille UI et le playhead par lane utilisent la longueur effective de la lane, ce qui rend les pas au-delà de la longueur globale visibles/editables pour une lane lockée.
- Aucun changement de topologie VST3, de bus audio ou d'identité plugin.

---

## 2026-07-01 — Modular UI checkpoint 4: extracted slot-aware lane renderer (build 20260701-183243)

**Build:** `20260701-183243`
**Validation:** `cargo check` OK, `cargo test` OK (73 tests), `build.ps1 -Install` OK

### Changements
- **Refactor structurel sans changement visible.**
  - Extrait le rendu d'une lane dans `draw_legacy_slot_lane_v2(...)`.
  - La fonction reçoit explicitement `slot_idx` et `voice_idx`, ce qui prépare l'affichage de slots actifs/inactifs sans mélanger index de slot et index de voix.
  - Corrige au passage la condition d'édition fusion pour comparer contre `slot_idx`.
- Aucun changement de topologie VST3, de bus audio, de pattern, de plocks ou de persistance DAW.

---

## 2026-07-01 — Modular UI checkpoint 3: slot-to-voice bridge in grid loop (build 20260701-175321)

**Build:** `20260701-175321`
**Validation:** `cargo check` OK, `cargo test` OK (73 tests), `build.ps1 -Install` OK

### Changements
- **Prépare la grille aux vraies lanes modulaires sans changement visible.**
  - Ajoute les helpers `visible_legacy_lane_count()` et `legacy_voice_idx_for_slot()`.
  - La boucle de grille itère maintenant sur `slot_idx`, puis dérive le `voice_idx` legacy (`slot_idx == voice_idx` tant que l'UI reste en 13 lanes fixes).
  - Pattern, plocks, fusions et sélection utilisent progressivement `slot_idx`; labels et paramètres automatisables restent indexés par `voice_idx` legacy.
- Aucun changement de topologie VST3, de bus audio, de pattern, de plocks ou de persistance DAW.

---

## 2026-07-01 — Fix silent lanes after modular layout checkpoint (build 20260701-174700)

**Build:** `20260701-174700`
**Validation:** `cargo check` OK, `cargo test` OK (73 tests), `build.ps1 -Install` OK

### Changements
- **Corrige les lanes silencieuses à partir de la 5e lane.**
  - Cause : le layout modulaire par défaut n'activait que 4 slots (`BD/SD/HH/Tom`) alors que l'UI affiche encore les 13 lanes fixes.
  - `TrackLayoutState::default_layout()` revient temporairement au layout legacy 13 voix tant que l'UI modulaire complète n'est pas prête.
  - Les états `track-layout-v1` déjà sauvegardés avec le template 4 slots buggué sont automatiquement migrés vers `from_legacy_13()` au chargement.
- Ajoute un test de compat pour détecter le template 4 slots buggué.
- Aucun changement de topologie VST3 ou de bus audio.

---

## 2026-07-01 — Modular UI checkpoint 2: grid interactions select track slot (build 20260701-173832)

**Build:** `20260701-173832`
**Validation:** `cargo check` OK, `cargo test` OK (72 tests), `build.ps1 -Install` OK

### Changements
- **Étend la sélection `selected_track_slot` aux interactions de grille/lane restantes, toujours sur les 13 lanes fixes.**
  - Lane volume, Humanize, Push/Pull et Length sélectionnent désormais le slot concerné quand l'utilisateur interagit.
  - Double-clic fusion, shift-clic fusion et clic droit p-lock sélectionnent aussi le slot concerné.
  - Les actions déjà gouvernées par Auto-Edit conservent leur comportement existant, mais passent par `select_legacy_track()`.
- Aucun changement de topologie VST3, de bus audio, de pattern, de plocks ou de persistance DAW.

---

## 2026-07-01 — Modular UI checkpoint 1: non-breaking selected track alias (build 20260701-172602)

**Build:** `20260701-172602`
**Validation:** `cargo check` OK, `cargo test` OK (72 tests), `build.ps1 -Install` OK

### Changements
- **Reprise prudente de la grille modulaire après rollback Studio One.**
  - Ajoute `EditorUIState::selected_track_slot` avec `#[serde(default)]` pour ne pas casser l'état d'éditeur existant.
  - Synchronise `selected_track_slot` avec `selected_instrument` sur les 13 lanes fixes actuelles.
  - Remplace les chemins de sélection UI par un helper central `select_legacy_track()`.
- Aucun changement de topologie VST3, de bus audio, de pattern, de plocks ou de persistance DAW.
- Sécurise le loader debug de preset dumps contre un index instrument hors bornes.

---

## 2026-07-01 — Rollback to last stable pre-crash code path (build 20260701-171707)

**Build:** `20260701-171707`
**Validation:** `cargo check` OK, `cargo test` OK (72 tests), `build.ps1 -Install` OK

### Changements
- **Rollback des changements non commités de la grille modulaire qui faisaient encore crasher Studio One au lancement.**
  - Retour au code du commit stable `edb1ef8` pour les sources Rust du plugin.
  - Conserve les fondations déjà commités : modèle track 14 slots, `track-layout-v1`, audio interne 14 slots avec topologie VST3 compatible 13 sorties auxiliaires.
  - Retire les changements UI/interaction non stabilisés : `+ Add module`, empty lanes stylées, sélection canonique `selected_track_slot`, onglets Sound/Track, menus plock slot/voice-aware, solos par slot.
- `TODO.md` rouvre les tâches modular-grid UI/interaction et le fix “new tracks silent / solo / interactions track-based”.
- Les entrées de build `20260701-162641`, `20260701-163806`, `20260701-164653`, `20260701-170135` et `20260701-170950` sont à considérer comme **supplantées par ce rollback** pour la validation Studio One.

---

## 2026-07-01 — Restore Studio One bus compatibility after modular-grid crash (build 20260701-170950)

**Build:** `20260701-170950`
**Validation:** `cargo check` OK, `cargo test` OK (72 tests), `build.ps1 -Install` OK

### Changements
- **Restaure la compatibilité de topologie VST3 avec les anciennes instances Studio One.**
  - Le plugin garde 14 slots internes pour la grille modulaire.
  - Les sorties auxiliaires VST3 exposées repassent à 13 bus stéréo, comme l'identité VST3 existante.
  - Changer 13 → 14 bus avec le même `VST3_CLASS_ID` était probablement la cause du crash au restore Studio One.
- Ajoute des garde-fous sur les derniers accès `slot_idx`/`voice_idx` dangereux dans `voice_settings_for` et `voice_settings_at_step`.
- Le menu Track / Aux Out n'expose plus `Out 14` tant que l'identité VST3 reste celle de la ligne compatible.

---

## 2026-07-01 — Fix Studio One startup crash: slot/voice index confusion in `voice_settings_at_step` (build 20260701-170135)

**Build:** `20260701-170135`
**Validation:** `cargo check` OK, `cargo test` OK (72 tests), `build.ps1 -Install` OK

### Changements
- **Corrige une confusion d'index slot/voice qui provoquait un panic dans l'audio thread.**
  - `voice_settings_at_step` prend maintenant explicitement `slot_idx` *et* `voice_idx`.
  - `sound_settings.instruments[slot_idx]` est lu par slot (14 slots), tandis que `voice_settings_for(voice_idx, ...)` et `INSTRUMENTS[voice_idx]` restent indexés par `DrumVoice` (13 voix).
  - Avant ce fix, un slot d'index 13 actif passait `slot_idx = 13` à `voice_settings_for`, causant un accès hors limites sur `INSTRUMENTS[13]` (taille 13) et un crash `EXCEPTION_STACK_BUFFER_OVERRUN` à travers l'ABI VST3.
- Mise à jour des trois appelants dans `process()` pour transmettre les deux indices correctement.

---

## 2026-07-01 — Fix Studio One startup crash: TrackLayoutState + plock popup compat (build 20260701-164653)

**Build:** `20260701-164653`
**Validation:** `cargo check` OK, `cargo test` OK (72 tests), `build.ps1 -Install` OK, Studio One 7 launches without crash

### Changements
- **Corrige le crash au lancement de Studio One.**
  - `TrackLayoutState` implémente maintenant `Deserialize` manuellement et accepte un `Vec<TrackSlot>` de n'importe quelle taille, remplissant/tronquant à `MAX_TRACKS = 14` slots.
  - Cela répare la désérialisation de l'état DAW qui contenait encore 13 slots (ancien format).
  - `PlockPopup.slot_idx` et `SinglePlockClipboard.slot_idx` acceptent l'alias serde `instrument` pour la compatibilité avec l'état de l'éditeur sauvegardé avant le renommage.

---

## 2026-07-01 — Fix Studio One startup crash (build 20260701-163806)

**Build:** `20260701-163806`
**Validation:** `cargo check` OK, `cargo test` OK (72 tests), `build.ps1 -Install` OK, Studio One 7 launches without crash

### Changements
- **Corrige le crash au lancement de Studio One.**
  - `EditorUIState::fusion_selection_start` passe d'un tableau de taille fixe (`[Option<usize>; MAX_TRACKS]`) à un `Vec<Option<usize>>` initialisé à 14 entrées.
  - Cela évite l'erreur de désérialisation serde lorsque l'état précédent de l'éditeur contenait un tableau de 13 éléments (ancien format).

---

## 2026-07-01 — All UI interactions are track-based (build 20260701-162641)

**Build:** `20260701-162641`
**Validation:** `cargo check` OK, `cargo test` OK (72 tests), `build.ps1 -Install` OK

### Changements
- **Toutes les interactions de la grille sont maintenant associées au track slot, pas au type d'instrument.**
  - Clic sur le nom d'une lane, Mute, Solo, Test, clic sur une step : ciblent le `slot_idx` sélectionné.
  - `selected_track_slot` est la sélection canonique ; `selected_instrument` n'est que le type de l'instrument du slot.
- **Sound Editor édite le son du slot actif.**
  - `sound_settings.instruments[slot_idx]` est lu/écrit au lieu de `[selected_instrument]`.
- **Plock / fusion / seq-plock menus sont slot/voice-aware.**
  - `PlockPopup` stocke `slot_idx` ; `voice_idx` est dérivé du `track_layout` pour les métadonnées instrument.
  - `draw_plock_menu`, `draw_fusion_morph_menu`, `draw_sequencer_plock_menu` prennent séparément `slot_idx` et `voice_idx`.
  - `SinglePlockClipboard` stocke `slot_idx`.
- **Onglet Track restauré.**
  - Sélecteur d'instrument (`TrackInstrumentKind`) pour le slot actif.
  - Routing `Main` + `Aux Out` (`Out 1`..`Out 14`) par slot.
  - Réglage de la note MIDI par slot.
- **`LaneLengthLocks` passe à 14 bits (`AtomicU32`).**
  - Persistance `u32` à la place de `u16`.
- **Grille UI restaurée après revert accidentel.**
  - Itération sur `MAX_TRACKS = 14` slots, lanes vides stylisées, bouton `+ Add module` sous les lanes.

---

## 2026-07-01 — Solo per slot (build 20260701-155824)

**Build:** `20260701-155824`
**Validation:** `cargo check` OK, `cargo test` OK (103 tests), `build.ps1 -Install` OK

### Changements
- **Solo devient un paramètre par slot.**
  - Ajout de 14 params `solo_s00`..`solo_s13` dans `DrumFlashParams`.
  - `slot_solos()` expose les 14 params.
  - `seq_mutes` utilise `slot_solo_states[slot]` au lieu de `solo_states[voice_idx]`.
  - Le tag `S` de chaque lane contrôle le solo de ce slot uniquement.

---

## 2026-07-01 — Fix audio thread: triggers now per-slot (build 20260701-154857)

**Build:** `20260701-154857`
**Validation:** `cargo check` OK, `cargo test` OK (103 tests), `build.ps1 -Install` OK

### Changements
- **Corrigé le thread audio qui traitait les triggers par `DrumVoice` au lieu de par slot.**
  - La boucle interne itère maintenant `(slot_idx, trigger)` et déclenche uniquement ce slot.
  - Cela répare le silence sur les nouveaux tracks et la double activation des lanes de même instrument.
- **Hihat choke** et **stutter/fusion scheduling** mis à jour pour utiliser `slot_idx`.

---

## 2026-07-01 — Modular grid: pattern per slot + instrument selector (build 20260701-153855)

**Build:** `20260701-153855`
**Validation:** `cargo check` OK, `cargo test` OK (103 tests), `build.ps1 -Install` OK

### Changements
- **[MG-6] Pattern bank indexée par slot (14 pistes indépendantes).**
  - `Pattern`, `SharedPattern`, `PlockState`, `SequencerPlockState` passent de 13 voix legacy à 14 slots.
  - Le séquenceur émet des triggers par slot (`MAX_TRACKS = 14`) au lieu de `DrumVoice::COUNT`.
  - La grille UI utilise `slot_idx` pour lire/écrire les cellules, fusions, plocks et seq-plocks.
  - Les paramètres de piste (mute/solo/length/push/humanize) sont mappés slot → voix legacy.
- **Sélecteur d'instrument dans l'onglet Track.**
  - `draw_track_tab` propose un ComboBox pour changer `TrackInstrumentKind` (Kick, Snare, ...).
  - Le changement met à jour le layout, reset les `sound_settings` du slot au defaults de l'instrument, et bump la version du synthétiseur.
- **Migration de persistance `pattern-v4` (13 rows) → `pattern-v5` (14 slots).**
  - Ajout de `PatternStateV4` avec `LEGACY_INSTRUMENT_COUNT = 13` et `expand()`.
  - `filter_state` migre `pattern-v4`, `pattern-v3`, `pattern-v2`, `pattern-v1` et `st01..st16` vers `pattern-v5`.
- **Générateurs adaptés à 14 slots.**
  - `euclidean_params`, rotations et templates de style ont une entrée FX supplémentaire.
- **Export MIDI itère sur les 14 slots.**

---

## 2026-07-01 — UI: lanes vides stylisées + bouton +Add module sous les lanes (build 20260701-151829)

**Build:** `20260701-151829`
**Validation:** `cargo check` OK, `cargo test` OK (103 tests), `build.ps1 -Install` OK

### Changements
- **Bouton `+ Add module` déplacé sous les lanes.**
  - Suppression du bouton `+ Add Track` du header.
  - Ajout d'une rangée `+ Add module` en bas de la grille, pleine largeur.
- **Lanes vides stylisées (hauteur de grille fixe).**
  - `draw_empty_lane` dessine les 14 emplacements avec un style "placeholder" : bordures dashed, tags M/S/T grisés, cellules grisées, sliders muets.
  - La grille conserve toujours 14 rangées, quels que soient les pistes actives.

---

## 2026-07-01 — UI: grid modulaire + onglets Sound/Track (build 20260701-144428)

**Build:** `20260701-144428`
**Validation:** `cargo check` OK, `cargo test` OK (103 tests), `build.ps1 -Install` OK

### Changements
- **Grid modulaire (MG-7).**
  - `draw_grid_v2` itère sur les slots actifs du layout au lieu des 13 voix fixes.
  - Seules les pistes actives sont affichées (par défaut BD/SD/HH/Tom).
  - Sélection de piste via `selected_track_slot` ; `selected_instrument` reste synchronisé avec le `drum_voice_index` legacy.
- **Onglets Sound / Track dans le Sound Editor (MG-8).**
  - `SoundEditorTab` : `Sound` (panneau de synthèse actuel) / `Track` (contrôles de piste).
  - Onglet `Track` : nom de piste, type d'instrument, routing Main/Out 1..14, note MIDI.
- **Bouton `+ Add Track`.**
  - Active le premier slot inactif avec un Kick par défaut.
  - Met à jour `SoundSettingsState` avec les valeurs par défaut de l'instrument.
  - Bumper la version du layout pour forcer la réinitialisation du synthétiseur dans le thread audio.
- **Réinitialisation du synthétiseur sur changement de layout.**
  - `process()` surveille `track_layout.state.version` et réinitialise `DrumSynthesizer` si elle change.

---


## 2026-07-01 — Audio: moteur 14 slots + routing modulaire (build 20260701-093806)

**Build:** `20260701-093806`
**Validation:** `cargo check` OK, `cargo test` OK (103 tests), `build.ps1 -Install` OK

### Changements
- **Le moteur audio itère désormais sur les 14 slots actifs du layout.**
  - `process()` snapshot le `AtomicTrackLayout` au début de chaque buffer.
  - Les triggers du séquenceur (13 familles legacy) sont routés vers chaque slot actif de la famille correspondante.
  - Le mixage Main, les sorties auxiliaires et les événements MIDI sont émis par slot.
- **Routing par piste fonctionnel.**
  - `main_on` contrôle l'envoi dans le Main Mix.
  - `out_select` (`Main` / `Out 1..14`) route le signal vers la sortie auxiliaire choisie.
  - `AUX_OUT_COUNT` passe de 13 à 14 ; les noms de sorties deviennent génériques (`Out 1` .. `Out 14`).
- **Hi-hat choke adapté au modèle modulaire.**
  - Un trigger HiHat reset toutes les pistes OpenHiHat actives, quel que soit leur slot.
- **`initialize()` utilise le layout actif.**
  - Le synthétiseur est initialisé avec `TrackLayoutState::default_layout()` (BD/SD/HH/Tom) au lieu du legacy 13 voix.
  - Ajout de `AtomicTrackLayout::snapshot()` pour capturer le layout sans verrou.

---


## 2026-06-30 — Fix: crash au lancement du transport dans Studio One (build 20260630-201216)

**Build:** `20260630-201216`
**Validation:** `cargo check` OK, `cargo test` OK (103 tests), `build.ps1 -Install` OK

### Changements
- **Correction d'un crash immédiat au lancement de la lecture dans Studio One.**
  - `mix_gains` était encore dimensionné à 13 voix (`DrumVoice::COUNT`) alors que `voice_outputs` était passé à 14 slots (`MAX_TRACKS`).
  - L'index 13 provoquait un `index out of bounds` dans le mixage Main, qui tuait le plugin dès le premier échantillon.
  - `mix_gains` est maintenant un tableau de `MAX_TRACKS` ; les slots 0-12 suivent les paramètres `mix_*` existants, le slot 13 est silencieux par défaut.

---

## 2026-06-30 — Architecture: fondations du grid modulaire 14 slots (build 20260630-181506)

**Build:** `20260630-181506`
**Validation:** `cargo check` OK, `cargo test` OK (103 tests), `build.ps1 -Install` OK

### Changements
- **Nouveau modèle de tracks modulaires (`src/track.rs`).**
  - 14 slots internes fixes (`MAX_TRACKS = 14`), seuls les slots actifs sont visibles dans l'UI.
  - 11 types d'instruments : Kick, Snare, HiHat, OpenHiHat, Tom, Clap, Ride, Cymbal, Snare606, BassDrum808, Perc1.
  - `TrackLayoutState` persiste dans un nouveau champ DAW `track-layout-v1`.
  - Migration legacy 13 voix → 14 slots via `TrackLayoutState::from_legacy_13()`.
- **Adaptation de `SoundSettingsState` à 14 slots.**
  - Persistance `sound-settings-v2` compatible avec les anciens formats 12 et 13 champs.
  - Initialisation des slots selon le layout actif.
- **Adaptation de `DrumSynthesizer` à 14 instances indépendantes.**
  - `voices` allouées sur le heap (`Box<[Option<Box<DrumVoiceKind>>; MAX_TRACKS]>`) pour éviter le stack overflow.
  - `initialize_with_layout()` crée les voices selon le `TrackLayoutState`.
  - API passée de `DrumVoice` index à `slot_idx`.
- **Adaptation de `lib.rs` et `ui.rs` pour `MAX_TRACKS`.**
  - Tableaux `current_steps`, `voice_test_triggers`, `voice_outputs` passés à 14 slots.
  - Comportement actuel inchangé : le layout par défaut est le legacy 13 voix.

---

## 2026-06-30 — UX: bouton fermeture simplifié avec accent au clic (build 20260630-155543)

**Build:** `20260630-155543`
**Validation:** `cargo check` OK, `cargo test` OK (100 tests), `build.ps1 -Install` OK

### Changements
- **Refonte du bouton `×` de fermeture.**
  - Plus de fond ni de bordure au repos : juste une croix discrète en `INK3`.
  - Plus d’état hover inutile.
  - **Au clic maintenu : fond plein avec la couleur d’accent du menu** (orange pour Plock/Fusion, violet pour Seq Plock, etc.) et croix en `INK` blanc.
  - Le feedback est donc binaire et très visible : rien au repos, couleur d’accent sous le doigt.

---

## 2026-06-30 — Fix: création de fusions cassée + migration pattern-v4 (build 20260630-145727)

**Build:** `20260630-145727`
**Validation:** `cargo check` OK, `cargo test` OK (100 tests), `build.ps1 -Install` OK

### Changements
- **Correction de la régression qui empêchait de créer des cellules fusionnées.**
  - La détection d’ancien format dans `unpack_fusion` était trop large : une fusion sans morphing (champ `field` par défaut = 255) positionnait le bit 24, ce qui faisait croire à l’ancien format.
  - Résultat : `is_valid` échouait sur les données décodées comme anciennes, et la fusion disparaissait.
  - `unpack_fusion` ne décode maintenant que le nouveau layout ; l’ancien format est migré au niveau de l’état DAW.
- **Passage du champ de persistance de `pattern-v3` à `pattern-v4`.**
  - `filter_state` migre automatiquement `pattern-v3` vers `pattern-v4` en préservant la géométrie des fusions existantes (les données de morphing corrompues sont ignorées).
  - Les migrations `pattern-v2`, `pattern-v1` et legacy `st01..st16` pointent maintenant vers `pattern-v4`.
- **Tests ajoutés :**
  - round-trip `SharedPattern` avec et sans morphing ;
  - migration `pattern-v3` → `pattern-v4` avec conservation de la géométrie.

---

## 2026-06-30 — Fix: corruption des valeurs de morphing dans les fusions (build 20260630-144304)

**Build:** `20260630-144304`
**Validation:** `cargo check` OK, `cargo test` OK (97 tests), `build.ps1 -Install` OK

### Changements
- **Correction d’un bug critique d’encodage binaire des fusions.**
  - Dans l’ancien layout 3×`u64`, `end_value` du premier target était shifté de 40 bits, ce qui ne laissait que 24 bits dans le `u64` — les 8 bits de poids fort du `f32` étaient perdus.
  - Conséquence : une valeur comme `Frequency = 300.0` devenait un nombre dénormal proche de zéro après sauvegarde/recharge, d’où le "reset à zéro" constaté.
  - Le 3ème target subissait une troncature similaire, ce qui expliquait les comportements erratiques avec plusieurs cibles.
- **Nouveau layout binaire compact sur 3×`u64`.**
  - Stocke correctement la géométrie de la fusion + 4 cibles de morphing (`field` 8 bits + `end_value` 32 bits chacune).
  - Bit de validité déplacé pour éviter toute ambiguïté avec l’ancien format.
- **Migration automatique des anciennes fusions.**
  - Les fusions encodées avec l’ancien format sont reconnues : la géométrie des cellules fusionnées est conservée, mais les données de morphing corrompues sont ignorées (morphing désactivé sur ces groupes).
- **Tests unitaires ajoutés** pour valider le round-trip 1 à 4 cibles et la migration depuis l’ancien format.

---

## 2026-06-30 — Feature: morphing multi-cibles parallèles sur les fusions (build 20260630-123315)

**Build:** `20260630-123315`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Les fusions supportent maintenant jusqu'à 4 cibles de morphing en parallèle.**
  - Le menu contextuel de clic droit sur une cellule fusionnée permet d'ajouter, modifier et supprimer plusieurs cibles de morphing.
  - Chaque cible possède un paramètre (`morph_field`) et une valeur de fin (`morph_end_value`).
  - Les cibles actives sont appliquées simultanément lors de chaque pulse de la fusion.
  - L'interpolation reste linéaire de la valeur courante (globale ou plock) vers la valeur de fin sur la durée de la fusion.
- **Modèle de données refactoré.**
  - `FusedGroup` et `TriggerResult` utilisent un tableau fixe `[MorphTarget; 4]` piloté par `morph_count`.
  - `SharedPattern` stocke les fusions dans 3 slots `AtomicU64` par groupe (`FUSION_SLOT_COUNT = 3`) pour encoder 4 cibles.
- **Persistance DAW et pattern bank mises à jour.**
  - Format `pattern-v3` inchangé au niveau du champ, mais la taille des données fusion augmente (`INSTRUMENT_COUNT * MAX_FUSIONS * FUSION_SLOT_COUNT * 8`).
  - Migration automatique des anciennes fusions mono-cible (`unpack_fusion_legacy`) vers le format multi-cibles.
  - La pattern bank sauvegarde et restaure correctement les fusions multi-cibles.
- **L'UI du menu contextuel reflète les cibles multiples** avec un bouton **Add Morph Target** jusqu'à 4 cibles maximum.

---

## 2026-06-30 — Feature: morphing accessible depuis le menu contextuel des fusions (build 20260630-120230)

**Build:** `20260630-120230`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Le morphing des cellules fusionnées est maintenant accessible par clic droit, avec une présentation identique au menu p-lock.**
  - Clic droit sur une cellule fusionnée : menu contextuel avec **Morphing**, **Edit Fusion Steps**, **Delete Fusion**.
  - Sélection de **Morphing** : affichage des paramètres continus sous forme de lignes avec slider + valeur (Volume, Frequency, Decay, Filter, Attack, Release, curves, Analog, Stereo, special params continus).
  - Le paramètre actuellement morphé est surligné (label en couleur d'accent).
  - Déplacement d'un slider : définit immédiatement `morph_field` et `morph_end_value`.
  - Bouton **Disable Morphing** pour désactiver (`morph_field = 255`).
  - Toggle **Display Notes/Hz** conservé pour les bass drums (Kick / BassDrum808).
- **Le menu p-lock de la cellule source reste disponible** en dessous des actions fusion.

---

## 2026-06-29 — Feature: morphing par pulse sur les cellules fusionnées (build 20260629-160624)

**Build:** `20260629-160624`
**Validation:** `cargo check` OK, `cargo test` OK (153 tests), `build.ps1 -Install` OK

### Changements
- **Morphing par pulse sur les fusions (Step Fusion).**
  - Dans la boîte d'édition d'une fusion, un select **Morph** permet de choisir un paramètre continu (Volume, Frequency, Decay, Filter, Attack, Release, curves, Analog, Stereo, et les special params continus comme saturation amount/mix/output gain).
  - Un slider **End** définit la valeur cible à atteindre au dernier pulse.
  - L'interpolation est linéaire de la valeur actuelle (globale ou plock) vers la valeur de fin, appliquée à chaque pulse.
  - Les paramètres discrets (type d'algo, type de saturation, pre-filter, mode stéréo…) ne sont pas proposés.
- **Stockage des fusions étendu à `u64`.**
  - `FusedGroup` contient maintenant `morph_field` et `morph_end_value`.
  - `SharedPattern.fusions` passe de `AtomicU32` à `AtomicU64`.
- **Persistance DAW des fusions implémentée.**
  - Nouveau champ `pattern-v3` qui persiste les step masks + les fused groups.
  - Migration automatique depuis `pattern-v2` (masks uniquement) et `pattern-v1` / legacy `st01..st16`.
  - La pattern bank sauvegarde et restaure aussi les fusions (`fusion_bytes`).

---

## 2026-06-24 — Fix: dropdown Algo dynamique dans le menu p-lock (build 20260624-171823)

**Build:** `20260624-171823`
**Validation:** `cargo check` OK, `cargo test` OK (153 tests), `build.ps1 -Install` OK

### Changements
- **Le slider Algo du menu p-lock était fixe 0→3 et affichait un chiffre.**
  - Il est remplacé par un dropdown qui liste seulement les algorithmes disponibles pour l'instrument courant.
  - Le nom de l'algorithme est affiché (ex: `Sine`, `Square`, `FM`) au lieu de son index.
- **La ligne Algo est masquée quand l'instrument n'a qu'un seul algorithme.**
  - Concerné : Cymbal, Snare606, BassDrum808.
- **Les valeurs de plock existantes hors plage sont clampées** vers l'index valide le plus proche au moment de l'affichage.

---

## 2026-06-23 — Fix: suppression du slider Frequency inactif sur le Clap (build 20260623-163320)

**Build:** `20260623-163320`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK (après fermeture de Studio One)

### Changements
- **Le slider Frequency (onglet OSC) du Clap ne faisait rien** car le synthé Clap n'utilise que `filter_freq` (le filtre passe-bande HP/LP).
- Le Clap utilise maintenant `NO_FREQ_STD` (comme le Cymbal) : plus de slider Frequency inutile.
- `ClapSettings` n'expose plus `frequency` pour éviter toute confusion.
- Vérification des autres instruments non tonaux :
  - HiHat / OpenHiHat : Frequency contrôle le peaking filter → utilisé.
  - Ride : Frequency contrôle les oscillateurs inharmoniques → utilisé.
  - Cymbal : n'avait déjà pas de slider Frequency → cohérent.
  - Seul le Clap avait ce problème.

---

## 2026-06-23 — Feature: saturation ajoutée à HiHat, OpenHiHat, Clap, Ride, Cymbal (build 20260623-153112)

**Build:** `20260623-153112`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Saturation complète pour les 5 instruments qui n'en avaient pas.**
  - HiHat, OpenHiHat, Ride : 5 paramètres saturation (type, amount, mix, output gain, pre-filter) en `special[0..4]`.
  - Clap : echo reste en `special[0]`, saturation en `special[1..5]`.
  - Cymbal : shimmer/noise restent en `special[0..2]`, saturation en `special[3..7]`.
  - `DrumFlashParams` expose 25 nouveaux `FloatParam` (5 × 5 instruments).
  - Chaque voix DSP initialise un `SaturationConfig`, l'applique sur le signal de sortie, et réagit aux changements via `set_special_param`.

---

## 2026-06-23 — Fix: resync du séquenceur quand `pattern_length` change (build 20260623-151154)

**Build:** `20260623-151154`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Le séquenceur est resynchronisé avec le transport hôte dès que `pattern_length` change.**
  - Avant, changer `Len` pendant la lecture (ex: 16 → 48) ne mettait pas à jour `loop_count` ni `beat_position` par rapport à la nouvelle longueur.
  - Cela pouvait créer un décalage permanent entre la page affichée et la page réellement lue, surtout avec des conditions de step dépendant du loop count.
  - `process()` détecte maintenant le changement de `master_length` et appelle `sync_to_host(position_beats)` pour recaler le séquenceur.

---

## 2026-06-23 — Fix: paste de page étend automatiquement la longueur du pattern (build 20260623-145953)

**Build:** `20260623-145953`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK (après fermeture de Studio One)

### Changements
- **Coller une page au-delà de la longueur actuelle étend automatiquement `pattern_length`.**
  - Avant, coller sur la page 2, 3 ou 4 avec `Len = 16` copiait bien les notes mais elles n'étaient pas jouées, ce qui donnait l'impression que l'ordre des pages ne se lisait pas.
  - Maintenant, après un `Paste Page`, si la page cible dépasse `pattern_length`, le paramètre `Len` est augmenté au multiple de 16 nécessaire (jusqu'à 64).
  - Cela concerne aussi le menu page Copy → Paste, pas seulement les presets/générateurs.

---

## 2026-06-23 — UX: confirmations page en lignes verticales (build 20260623-143214)

**Build:** `20260623-143214`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Layout des confirmations Paste/Clear revu.**
  - Au lieu de boutons Yes/No collés côte à côte, les confirmations affichent une ligne d'info puis deux boutons pleine largeur empilés : "Yes, overwrite" / "No, cancel" et "Yes, clear" / "No, cancel".
  - Le label d'info n'est plus un faux bouton inactif.

---

## 2026-06-23 — UX: menu page se ferme sur Copy (build 20260623-142847)

**Build:** `20260623-142847`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Clic sur "Copy" dans le menu page ferme immédiatement le popup.**

---

## 2026-06-23 — UX: menu page plus compact (build 20260623-142211)

**Build:** `20260623-142211`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Menu page réduit d'environ moitié.**
  - Nouveau `page_menu_frame` : min 130 px / max 150 px (vs 260/284 px pour les plocks).
  - Labels raccourcis : Copy / Paste / Clear, puis "Overwrite?" / "Clear?" + Yes / No en confirmation.
  - Header sans sous-titre "Step N".

---

## 2026-06-23 — Fix: synchronisation fusions lors du chargement de pattern (build 20260623-144724)

**Build:** `20260623-144724`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **`load_pattern_for_ui` et `load_pattern_for_ui_with_length` copient maintenant aussi les fusions.**
  - Avant, seuls les step masks étaient copiés ; les fusions de l'ancien pattern persistaient dans `SharedPattern`.
  - Le séquenceur audio pouvait donc jouer des fusions fantômes qui n'étaient plus visibles sur le grid après un preset / génération / clear.
  - Les fusions du `Pattern` source sont maintenant écrites dans `SharedPattern` pour chaque instrument.

---

## 2026-06-23 — UX: menu page restylé comme les menus p-lock (build 20260623-141809)

**Build:** `20260623-141809`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK (après fermeture de Studio One)

### Changements
- **Le menu contextuel des pages reprend le style `.plk` des menus p-lock.**
  - `plock_menu_frame` + `plock_menu_header` + `plock_menu_action_row` pleine largeur.
  - Barre d'accent bleue en haut, fond `P_ACTIVE`, bordure `LINE2`, radius 9.
  - Copy Page (bleu), Paste Page (orange `PL_LINK` si dispo, sinon grisé), Clear Page (rouge).
  - Les confirmations Paste/Clear apparaissent comme des lignes d'action dans le même menu.

---

## 2026-06-23 — UX: menu Copy/Paste/Clear sur les boutons de page (build 20260623-124600)

**Build:** `20260623-124600`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **[100y] Menu contextuel sur les boutons de page (1-4).**
  - Clic droit sur un numéro de page : ouvre un menu avec Copy Page / Paste Page / Clear Page.
  - Les fonctions Copy/Paste/Clear du pattern (P1-P8) restent intactes.
  - Copy Page : copie les triggers, les sound plocks et les fusions de la page dans `EditorUIState.page_clipboard`.
  - Paste Page : demande confirmation avant d'écraser la page cible.
  - Clear Page : demande confirmation avant de vider la page (triggers + plocks + fusions).
  - Popup maison `egui::Area` avec le style `.plk` (fond `P_ACTIVE`, bordure `LINE2`, radius 9).

---

## 2026-06-23 — UX: focus auto sur le champ step-count en édition fusion (build 20260623-122425)

**Build:** `20260623-122425`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Double-clic sur une fusion : le champ "Steps" reçoit le focus et son texte est sélectionné.**
  - Remplacement du `DragValue` par un `TextEdit` singleline pour permettre la sélection complète.
  - `EditorUIState.fusion_edit_focus_request` déclenché à l'ouverture de l'édition.
  - Focus + sélection `CCursorRange` de 0 à len appliqués sur le `TextEditOutput`.
- La valeur est parsée et clampée 1..64 à la perte de focus ou au changement.

---

## 2026-06-23 — UX: sortie auto du mode édition fusion (build 20260623-120806)

**Build:** `20260623-120806`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **[91] Sortie automatique du mode edit quand on clique en dehors de la cellule fusionnée.**
  - Lors d'un clic sur une autre cellule, si le clic ne porte pas sur le groupe fusionné en cours d'édition, l'édition est terminée avant de traiter le toggle.
  - Conserve le comportement si on reclique sur le même groupe fusionné (l'édition reste active).

---

## 2026-06-23 — Réinstallation du VST3 (build 20260623-113150)

**Build:** `20260623-113150`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Réinstallation du bundle VST3 après suppression.**
  - Aucun changement de code ; rebuild + install du dernier état source.
  - Bundle déployé dans `C:\Program Files\Common Files\VST3\drum-pattern-vst.vst3`.

---

## 2026-06-16 — Redesign UI: fusion couleur d'édition + texte centré (build 20260616-211439)

**Build:** `20260616-211439`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Fusion : la cellule de départ reprend sa couleur normale en sortie d'édition.**
  - `is_fusion_start` n'applique plus la couleur bleue foncée quand `is_editing` est actif ; seul le mode édition clignote.
  - Après fermeture de la boîte d'édition, le bloc fusionné redevient bleu standard.
- **Le nombre de triggers (`step_count`) est centré dans le bloc fusionné entier**, plus seulement dans la première cellule.

---

## 2026-06-16 — Redesign UI: rendu continu des cellules fusionnées (build 20260616-210639)

**Build:** `20260616-210639`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Les cellules fusionnées (Step Fusion) sont à nouveau rendues comme un bloc continu.**
  - `draw_step_cell_v2` étend le rectangle de la cellule de départ pour recouvrir l'ensemble du groupe fusionné, gaps compris.
  - Les cellules internes restent transparentes : pas de bordure ni de fond qui cassent le bloc.
  - L'indicateur "pulses" (`step_count`) est affiché sur la cellule de départ.
- **Le mode édition d'une fusion fait de nouveau clignoter l'ensemble du bloc.**
  - `is_editing` est recalculé depuis `state.fusion_editing` dans `draw_grid_v2`.
  - Toutes les cellules du groupe en édition pulsent en bleu de manière synchronisée.
- **Playhead sur une fusion restreint à la cellule exacte du curseur.**
  - `is_current` ne met plus l'anneau playhead sur toutes les cellules du groupe, seulement sur la cellule active.

---

## 2026-06-16 — Redesign UI: suppression undo par paramètre dans menus p-lock (build 20260616-203617)

**Build:** `20260616-203617`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Bouton "↺" (undo) retiré de chaque rangée de paramètre dans les menus p-lock.**
  - Il décalait les sliders et n'apportait pas assez de valeur par rapport aux actions globales `Clear Plock` / `Copy Plock`.
  - `plock_menu_row` passe de 7 à 6 arguments (suppression du callback `on_undo`).
  - Tous les appelants mis à jour : Volume, Display, Freq notes, standard params, Algo, specials, Probability, Stutter.

---

## 2026-06-15 — Redesign UI: menus p-lock bordure + cellule d'édition clignotante (build 20260615-165139)

**Build:** `20260615-165139`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Bordure fine claire (`LINE2`) ajoutée autour du menu p-lock.**
  - Stroke 1 px sur le `plock_menu_frame`, cohérent avec les autres panneaux.
- **Cellule en cours d'édition clignote.**
  - Quand un menu p-lock est ouvert, la step source pulse en bleu (fond interpolé + bordure `BLUE` 1.5 px).
  - Utilise `ctx.input(|i| i.time)` pour un clignotement sinusoïdal à 4 Hz.
  - `step_colors_v2` reçoit un paramètre `is_editing` ; `draw_grid_v2` passe l'état du popup.

---

## 2026-06-15 — Redesign UI: menus p-lock popup maison (build 20260615-160242)

**Build:** `20260615-160242`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Remplacement du `context_menu` egui par un popup maison.**
  - Le cadre noir venait de `Frame::menu` de egui, non contrôlable par `window_stroke`.
  - Le clic droit sur une step ouvre maintenant un `egui::Area` personnalisé avec notre propre `Frame::NONE` rempli `P_ACTIVE` r9.
  - Plus de bordure, plus d'ombre parasite.
  - État `plock_popup` dans `EditorUIState` avec fermeture au clic à l'extérieur.
  - Fusion "Edit/Delete" toujours disponible en mode Sound ; Sequencer inchangé.

---

## 2026-06-15 — Redesign UI: menus clic-droit p-lock reskinés (build 20260615-122058)

**Build:** `20260615-122058`
**Validation:** `cargo check` OK, `cargo test` OK (91 + 62 tests), `build.ps1 -Install` OK

### Changements
- **[100x] Menus clic-droit p-lock reskinés.**
  - Menu Sound (`draw_plock_menu`) utilise le frame `.plk` (fond `P_ACTIVE`, radius 9, barre d'accent orange `PL_LINK`, ombre).
  - Menu Sequencer (`draw_sequencer_plock_menu`) réécrit avec le même style, accent violet `SEQPL`.
  - Header "Seq Plock {instrument}" + "Step N", indicateur Mode Active/Inactive.
  - Probability et Stutter en rangées avec slider `LocalParamSlider` et valeur en ligne.
  - Grille Condition en 3 colonnes avec boutons stylisés (accent sélectionné).
  - Actions "Create Seq Plock" / "Clear Seq Plock" stylisées comme les actions Sound.
- Uniformisation des helpers `plock_menu_frame`, `plock_menu_header`, `plock_menu_row`, `plock_menu_action_row` partagés entre Sound et Sequencer.

---

## 2026-06-14 — Redesign UI: bloc Generator réorganisé (build 20260614-205742)

**Build:** `20260614-205742`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Bloc Generator refondu en 2 rangées alignées** (avant : 3 rangées tassées + labels décalés d'un cran, avec un label « B » orphelin sans combo).
  - Rangée 1 (moteur) : combo algorithme · **A**/**B** = styles à morpher · sliders pilule **Mix/Dens/Var** (slider du design system, fini les `ParamSlider` bruts) · **GENERATE** poussé à droite.
  - Rangée 2 (raccourcis) : Presets Rock/Funk/Disco + ⟳ Random.
- Labels corrigés (A = style primaire, B = style secondaire) ; selects `.selbox` + contrôles h26, cohérents avec header/éditeur. Import `ParamSlider` retiré (plus utilisé).
- **Sliders pilule** : la poignée Ø11 réserve désormais son rayon à chaque extrémité (`header_param_slider`) — plus de troncature à 0 %/100 % (corrige aussi le slider Len de la page-bar).
- **Labels complets** : « Mix · Densité · Variation » (largeurs ajustées pour garder des pistes lisibles).

---

## 2026-06-14 — Redesign UI: panneau Generator en 3 lignes propre (build 20260614-102408)

**Build:** `20260614-102408`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK après fermeture de Studio One

### Changements
- Panneau Generator retravaillé en **3 lignes** suite au feedback utilisateur.
  - Ligne 1 : `Generator` / `Type` / `A` / `B` avec les combos.
  - Ligne 2 : sliders `Mix` / `Density` / `Variation` avec labels alignés et noms complets.
  - Ligne 3 : bouton `GENERATE` à gauche, texte centré manuellement.
- Hauteur du bottom panel augmentée à `190 px` pour accueillir les 3 lignes.

## 2026-06-14 — Bouton GENERATE à la ligne + centré (build 20260614-095451)

**Build:** `20260614-095451`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Bouton `GENERATE` remis sur une ligne dédiée sous les contrôles Generator.
- Hauteur du bottom panel augmentée de 132 px à 168 px pour accueillir les deux lignes.
- Centrage manuel du texte `GENERATE` dans le bouton via `ui.painter().galley()`.

## 2026-06-14 — Fix bouton GENERATE invisible (build 20260614-092628)

**Build:** `20260614-092628`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Correction du bouton `GENERATE` invisible dans le panneau Generator.
  - Retour à une seule ligne horizontale pour éviter que le layout vertical ne dépasse la hauteur allouée au panel.
  - Le bouton est poussé à droite avec un `add_space` calculé après les sliders.
  - Réduction légère des largeurs de combos/sliders pour tenir dans la ligne.

## 2026-06-13 — Redesign UI: panneau Generator en deux lignes (build 20260613-210831)

**Build:** `20260613-210831`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK après fermeture de Studio One

### Changements
- Réécriture propre du layout du panneau Generator en deux lignes fixes.
  - Ligne 1 : `Generator` / `Type` / `A` / `B` + sliders `Mix` / `Dens` / `Var` sur une seule ligne horizontale.
  - Ligne 2 : bouton `GENERATE` déplacé en dessous, aligné à droite.
  - Espacements et largeurs de combos/sliders constants via constantes locales.

## 2026-06-13 — Redesign UI: alignement panneau Generator v2 (build 20260613-203430)

**Build:** `20260613-203430`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK après fermeture de Studio One

### Changements
- Correction du layout panneau Generator après screenshot utilisateur.
  - Suppression du sous-layout `right_to_left` qui compressait le bouton `GENERATE`.
  - Le bouton `GENERATE` est poussé à droite via `ui.add_space()` calculé dans le `horizontal` parent.
  - Le bloc de sliders (morph A/B + Mix/Dens/Var) est centré dans l'espace restant.
  - Largeur des combos A/B harmonisée à 92 px.

## 2026-06-13 — Redesign UI: alignement panneau Generator (build 20260613-193615)

**Build:** `20260613-193615`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK après fermeture de Studio One

### Changements
- Réalignement des sliders `Mix`, `Dens`, `Var` et du bouton `GENERATE` dans le panneau Generator.
  - Le bloc de paramètres est désormais centré dans l'espace disponible entre les combos A/B et le bouton.
  - Le bouton `GENERATE` est ancré à droite et sa largeur passe à 96 px pour matcher la maquette.
  - Espacements harmonisés via des constantes locales.

## 2026-06-13 — Correction régression Push/Pull (build 20260613-105028)

**Build:** `20260613-105028`
**Validation:** `cargo test` OK (91 lib + 62 standalone), `build.ps1 -Install` OK

### Changements
- `Sequencer::sync_to_host` recalcule `step_counter` depuis la timeline *shifted* (position hôte moins le décalage Push/Pull) au lieu de la timeline master.
  - Évite le décalage de phase qui apparaissait après un seek/loop quand une piste avait du Push/Pull.
  - Garde la polyrythmie et les conditions de step stables après resync.
- UI grille : la playhead reste sur `current_step` global et ne bouge plus quand on module Push/Pull.
  - Push/Pull décale uniquement le timing audio ; la grille visuelle reste alignée sur le transport hôte.
- Tests ajoutés/corrigés :
  - `test_push_pull_sync_to_host_preserves_phase` valide la stabilité après `sync_to_host` avec +30 ms.
  - `test_track_push_pull_stability` corrigé : applique réellement `push_pull_ms` au lieu de passer la valeur comme `swing`.

### Point d'attention résolu
- `[101]` Régression Push/Pull : le décalage audio ne doit plus devenir énorme après lecture/seek ; le reset double-clic à `0 ms` ramène bien à un comportement neutre. La tête de lecture visuelle reste stable quand on module Push/Pull.

---

## 2026-06-12 — Redesign UI: playhead indépendante du Push (build 20260612-210534)

**Build:** `20260612-210534`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- La playhead visuelle de la grille utilise désormais `current_step` global non décalé.
- Les valeurs `Push/Pull` continuent de décaler les déclenchements audio, mais ne déplacent plus l'anneau de lecture dans l'UI.
- Les `current_steps` par piste restent produits côté moteur pour la logique interne, mais ne pilotent plus l'affichage de la tête de lecture.

### Point d'attention
- Retour utilisateur fin de session : le comportement Push/Pull est devenu incorrect (décalage énorme, difficile à annuler). Reprise prioritaire consignée dans `TODO.md` sous `[101]`.

---

## 2026-06-12 — Redesign UI: double-clic reset Hum/Push (build 20260612-205837)

**Build:** `20260612-205837`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK après fermeture de Studio One

### Changements
- Double-clic sur les mini sliders `Hum` et `Push` : reset à la valeur par défaut du paramètre (`0%` / `0 ms`).
- Le tooltip custom affiche immédiatement la valeur resetée après double-clic.

---

## 2026-06-12 — Redesign UI: tooltip custom Hum/Push (build 20260612-205255)

**Build:** `20260612-205255`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Remplacement de `Response::on_hover_text()` par une bulle custom `Foreground` pour les mini sliders `Hum` et `Push`.
- La bulle est ancrée au-dessus du slider et reste visible au hover comme pendant le drag.
- Valeurs affichées : `Humanize: xx%` et `Push/Pull: +x ms`.

---

## 2026-06-12 — Redesign UI: tooltip Hum/Push corrigé (build 20260612-174601)

**Build:** `20260612-174601`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Correction du tooltip des mini sliders `Hum` / `Push` : suppression du tooltip vide qui masquait la valeur.
- `Push/Pull` affiche désormais explicitement l'unité `ms` dans le hover.

---

## 2026-06-12 — Redesign UI: valeurs Hum/Push en tooltip (build 20260612-173557)

**Build:** `20260612-173557`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK après fermeture de Studio One

### Changements
- Suppression du texte incrusté dans les mini sliders `Hum` et `Push`.
- Le hover affiche maintenant la valeur utile : `Humanize: xx%` ou `Push/Pull: +x`.
- Le tooltip générique seul (`Humanize`, `Push/Pull`) a été remplacé par la valeur formatée.

---

## 2026-06-12 — Redesign UI: Hum/Push + switch p-lock (build 20260612-164416)

**Build:** `20260612-164416`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Les colonnes `Hum` et `Push` affichent à nouveau leurs valeurs directement dans les mini sliders (`%` pour Hum, valeur signée pour Push).
- Sliders `Hum` et `Push` harmonisés sur la même couleur bleue.
- Remplacement du switch `P-Lock Mode` par un contrôle custom fiable et coordonné : `Sound` orange / `Sequencer` violet, hauteur 26 px, rayon 6, bordure `LINE2`.
- Suppression de helpers UI devenus morts après le recâblage (`segmented_control`, ancien mini slider param sans valeur).

---

## 2026-06-12 — Redesign UI: sliders constants + Note/Freq (build 20260612-162103)

**Build:** `20260612-162103`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Largeur de colonne paramètres fixée à `340 px` pour toutes les sections du Sound Editor : les sliders gardent désormais la même longueur avec ou sans graphe ENV/Filter.
- Extraction du rendu de piste slider pour partager exactement les mêmes dimensions entre les rangées.
- Remplacement de la checkbox `Notes` des bass drums par un mini sélecteur segmenté `Hz | Note` intégré à la rangée Frequency.
- Mode Note : contrôles `-` / note mono / `+` alignés dans la rangée, sans titre ni checkbox parasite.

---

## 2026-06-12 — Redesign UI: labels ADSR dans le graphe (build 20260612-151646)

**Build:** `20260612-151646`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Réintroduction des labels `A`, `D`, `S`, `R` directement dans le cadre du graphe ADSR.
- Les labels restent discrets en IBM Plex Mono Medium gris clair et sont clampés pour ne pas sortir du graphe.
- La légende externe sous les contrôles d'enveloppe reste supprimée.

---

## 2026-06-12 — Redesign UI: enveloppe ADSR sans légendes (build 20260612-150813)

**Build:** `20260612-150813`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK après fermeture de Studio One

### Changements
- Suppression des légendes A/H/D/R affichées sous les paramètres d'enveloppe dans le Sound Editor.
- Refonte du graphe d'amplitude en lecture ADSR simplifiée conforme à la maquette : attaque ambre, decay bleu, release violet.
- Ajout des 5 lignes verticales de grille `white_a(13)` dans le cadre du graphe.
- Suppression des lettres A/D/R/H dans le canvas : le graphe ne garde que les courbes et la grille.

---

## 2026-06-12 — Redesign UI: Select stylé maquette (build 20260612-145130)

**Build:** `20260612-145130`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Remplacement des `ComboBox` egui restants par un widget Select custom aligné sur `.selbox` : hauteur 26 px, fond `PANEL2`, bordure `LINE2`, hover bleu, texte courant en IBM Plex Mono Medium.
- Application aux selects Sound Editor : Saturation Type, Noise Type, Click Type et Algorithm.
- Application aux selects header/bas de page : Groove, Generator type, Style A et Style B.
- Menu déroulant custom : fond `P_ACTIVE`, bordure `LINE2`, options en IBM Plex Sans Medium, hover bleu + texte blanc.

---

## 2026-06-12 — Redesign UI: Sound Editor réorganisé + finitions (build 20260612-142330)

**Build:** `20260612-142330`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements (suite au retour : sliders trop longs, intitulés non alignés, graphes serrés)
- **Colonne de paramètres à largeur contrainte** par section : les sliders flexent dans cette colonne (uniformes, plus courts) au lieu de s'étirer sur toute la largeur.
- **Graphes d'enveloppe** : la colonne de params réserve désormais ~196 px + un gap de 16 px pour le graphe ENV/Filter → il n'est plus serré contre la droite ; cadre redessiné (fond #0c0c11, rayon 7, ~104 px de haut, remplit la largeur dispo).
- **Intitulés alignés** : tous les labels sur la même colonne de 138 px (Algorithm et Mix utilisaient avant un label nu non aligné).
- **Titres de section** : noms complets (Oscillator / Envelope / Filter / Saturation / Output) en sans 600 INK3 au lieu d'abréviations mono MAJUSCULES.
- **Mix** : ToggleSwitch aligné à droite (au lieu d'une checkbox egui brute).
- **Intitulés alignés à gauche** : colonne label 138 px rendue en `left_to_right` (avant centrés/flottants via `add_sized`).
- **Slider Volume** ramené à la largeur des sections (340 px) — fin de l'incohérence.
- **Sections vides masquées** : une famille sans paramètre pour l'instrument (ex. Saturation sur l'OpenHiHat) n'affiche plus de titre orphelin.

---

## 2026-06-12 — Redesign UI: Sound Editor (sliders / switches / en-tête) (build 20260612-114809)

**Build:** `20260612-114809`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Slider rows de l'éditeur** : piste fine à largeur **fixe (190 px)** sans bordure (aplat PANEL2 + fill bleu + poignée au survol), valeur mono à droite ; label sans 500. (Une piste *flex* a été abandonnée : elle consommait l'espace horizontal réservé au graphe d'enveloppe inline → sliders trop longs + graphes disparus.)
- **Padding du Sound Editor** : contenu du scroll encadré (14 px gauche/droite, 6 px haut) — les labels ne touchent plus le bord gauche.
- **Switch rows** : le ToggleSwitch est poussé au bord droit (space-between) ; label sans 500.
- **En-tête éditeur** : titre « Sound Editor » en blanc/bold ; nom d'instrument en mono ; bouton « Engine ▾ » inerte retiré (réservé à la future phase modulaire).

### À suivre (éditeur)
- Modèle de section (filet DIVIDER au lieu de `separator`, espacements), combos → Select stylé, ADSR inline réécrit (modèle 3 segments), toggle Notes en pilule.

---

## 2026-06-12 — Redesign UI: grille séquenceur + page-bar (build 20260612-104952)

**Build:** `20260612-104952`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- **Grille séquenceur** : cellules pleines vives + bordure nette par état (hit bleu, link orange, snapshot rouge, seq violet) — pas de glow externe (il bavait sur les pas adjacents, egui n'a pas de flou) ; playhead = anneau blanc inset dessiné par-dessus (conserve la bordure d'état) ; tags M/S/T avec texte lisible (T blanc sur bleu, M sur ambre, S sur vert) ; noms de lane sans bordure au repos (contour bleu si sélectionné), police mono 600 ; poignée de drag en matrice de points 2×3 ; en-têtes de colonnes M/S/T ; bordure des cellules fusion-mid en `BLUE_DIM` (50% bleu).
- **Page/Length bar** : slider Len en piste fine custom (`header_param_slider` bare-track) ; bouton Follow ON en bleu plein + texte blanc ; LED rouge sous la page en lecture, z-order corrigé (halo puis point) ; lecture « {n} steps » en deux runs (nombre mono 12 + unité sans 9.5).
- **`header_param_slider`** étendu (label/valeur optionnels) et réutilisé pour Master/Swing/Len.

---

## 2026-06-12 — Redesign UI: nettoyage migration + fondations design system (build 20260612-102825)

**Build:** `20260612-102825`
**Validation:** `cargo check` OK, `cargo test --no-run` OK, `build.ps1 -Install` OK

### Changements
- **Nettoyage migration** : suppression de ~1300 lignes de code mort (anciens `draw_grid`, `draw_top_bar`, `draw_song_bar`, `draw_generator_panel`, helpers volume-dB, `bool_checkbox`, `draw_bool_toggle`) + suppression des modules morts `src/ui/schema.rs` et `src/ui/engine_registry.rs`. Un seul chemin de rendu (`*_v2`) reste actif. Helpers du menu page Copy/Paste/Clear conservés sous `#[allow(dead_code)]` pour recâblage ultérieur.
- **Polices multi-graisses** : ajout des faces IBM Plex Sans Medium/SemiBold/Bold + Mono Medium/SemiBold dans `assets/fonts/`. `install_egui_fonts` enregistre des familles nommées par graisse (`sans_med/sb/bold`, `mono_med/sb`) → fin du faux-gras `.strong()`.
- **Visuals globales** : coins r6, bordures hairline (LINE/LINE2), hover bleu, sans expansion sur les widgets egui par défaut.
- **Header refait à la maquette** : transport ▶■● et toggle Song retirés ; sliders Master/Swing en pilule fine (fill bleu, poignée au survol, valeur mono à droite) ; Groove ; segmented Seq Internal/Ext MIDI avec LED ; Choke/Auto-Edit en pilules LED ; séparateurs 1px LINE.

### À suivre
- Propagation du langage visuel aux zones restantes : grille séquenceur, éditeur, page-bar, menus p-lock (284px), patterns/generator.

---

## 2026-06-12 — Redesign UI IBM Plex fonts (build 20260612-090421)

**Build:** `20260612-090421`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Ajout des assets `IBMPlexSans-Regular.ttf` et `IBMPlexMono-Regular.ttf` dans `drum-pattern-vst/assets/fonts/`.
- Chargement des polices via `egui::FontDefinitions` au demarrage de l'editeur.
- IBM Plex Sans devient la police proportionnelle prioritaire et IBM Plex Mono la police monospace prioritaire.

---

## 2026-06-11 — Redesign UI Sound Editor controls (build 20260611-201611)

**Build:** `20260611-201611`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Remplacement des sliders natifs visibles dans le Sound Editor par des rows custom : label fixe, piste arrondie, fill bleu, valeur mono à droite.
- Application du nouveau rendu aux paramètres standards et aux paramètres spéciaux, notamment Saturation Amount/Mix/Output Gain.
- Ajout de switches custom pour les booléens d'éditeur.
- Réduction de la hauteur du panneau Generator/Song de `136px` à `116px` pour limiter le vide en bas.

---

## 2026-06-11 — Redesign UI corrections clipping/pagebar (build 20260611-194657)

**Build:** `20260611-194657`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Ajout d'un clipping explicite sur la colonne gauche pour empêcher les panneaux de peindre dans le Sound Editor.
- Repeint du fond de colonne droite après la colonne gauche pour supprimer les traces de débordement.
- Remplacement du panneau Generator/Song par un panneau à rectangle fixe, header/body clippés.
- Correction de la pagebar : suppression du layout `right_to_left` qui décalait `Len`, ordre normal `Len · slider · steps · 16/32/48/64 · x2`.
- Generator compacté en deux lignes : presets puis contrôles, combobox plus étroits, sliders sans valeur inline.

---

## 2026-06-11 — Redesign UI reprise structurelle (build 20260611-184532)

**Build:** `20260611-184532`
**Validation:** `cargo check` OK, `build.ps1 -Install` OK

### Changements
- Reprise du layout principal à partir du design pack `Flash_Drum_design_11062026` : body fixe en deux colonnes, gauche flexible et droite `568px`, bordure verticale `LINE`, padding gauche `14px`.
- Remplacement du rendu de grille basé sur `egui::Grid` par un séquenceur custom : lanes `24px`, tags M/S/T `17px`, steps `21px`, cellules sans texte `X`/`.` et couleurs p-lock/playhead conformes au design system.
- Page bar séparée au-dessus du séquenceur : pages 1-4, LED rouge de lecture, Follow, Len, presets `16/32/48/64`, `x2`.
- Header principal : remplacement du simple toggle `Seq` par un segment `Internal | Ext MIDI` branché sur `use_internal_sequencer`.
- Sound Editor : header et barre d'onglets avec zones fixes, onglets instruments sur une seule ligne au lieu de deux colonnes de tabs.
- Bottom panel Generator/Song : panneau encadré avec header séparé et contenu en dessous.

---

## 2026-06-11 — Redesign UI Phase 2a — Layout 2 colonnes (build 20260611-102814)

**Build:** `20260611-102814`
**Commits:** UI : refactor layout principal en 2 colonnes selon design pack v2

### Changements
- **Layout 2 colonnes** (design pack §1, LAYOUT.md) :
  - Colonne gauche : **912px**, padding 14h/11v, gap vertical 4px, bordure droite 1px `LINE`
  - Colonne droite : **568px**, fond `PANEL` (20,20,25), bordure gauche 1px `LINE`
  - Grid compact : spacing 2px, step buttons 18px, lignes 20px
  - Suppression des `ui.separator()` entre sections (remplacés par le gap)
  - Positionnement : `allocate_new_ui` avec rectangles fixes
- **Header** (design pack §2) :
  - Transport ▶ ■ ● ajoutés (green/red actif)
  - Brand + version + séparateurs DIVIDER
- **Page bar** (design pack §3) :
  - Boutons 1-4 : 26×20px, radius 6px, label mono 10.5px
  - Follow toggle : style coordonné (P_ACTIVE/PANEL2)
  - Len slider compact (50px)
  - Presets 16/32/48/64 : 26×20px boutons
  - ×2 : 26×20px bouton
- **P-lock mode bar** :
  - Label "P-Lock Mode" INK3 10.5px
  - Segmented Sound/Sequencer avec couleurs PL_LINK/SEQPL
  - Fusion box sur même ligne, 380px
- **Pattern Bank** :
  - Save button : 44×22px
  - Slots P1-P8 : 30×22px
  - Export MIDI / Drag MIDI : boutons stylisés PANEL2/LINE2
- **Bottom Panel** (design pack §6) :
  - Toggle unifié Generator | Song (segmented)
  - Partage le même espace
- **Sound Editor** (design pack §6) :
  - Header : "Sound Editor" + nom instrument + Engine selector placeholder
  - Onglets instruments : grille responsive (7 colonnes)
  - Volume en tête sans titre de section
  - Titres de section : UPPERCASE mono 10px INK3
  - Suppression des `ui.group` avec bordures
- **Couleurs des steps** (design pack §6) :
  - off pair/impair : #1b1b22 / #23232c
  - on plock link : PL_LINK (255,140,0)
  - on plock snapshot : PL_SNAP (220,50,50)
  - off plock link : PL_LINK_DIM (180,100,0)
  - off plock snapshot : PL_SNAP_DIM (160,30,30)
  - Playhead : #30303c
- **Réduction globale** :
  - Tous les boutons et contrôles réduits pour tenir dans 800px
  - Gaps entre sections : 4px
  - Élimination du débordement bas
  - Colonne gauche : **912px**, padding 14h/11v, gap vertical 10px, bordure droite 1px `LINE`
  - Colonne droite : **568px**, fond `PANEL` (20,20,25), bordure gauche 1px `LINE`
  - Ordre colonne gauche : séquenceur (page-bar + grille + plock mode) → pattern bank → generator/song
  - Suppression des `ui.separator()` entre sections (remplacés par le gap de 10px)
- **Positionnement** : utilisation de `allocate_new_ui` avec rectangles fixes pour un layout pixel-perfect

---

## 2026-06-11 — Réception du design pack complet (designer)

**Livrable** : `design-pack/Flash_Drum_design_11062026/flash-drum-source/`

### Contenu du design
- **`DESIGN-SYSTEM.md`** — Tokens visuels (palette IBM Plex, typo, widgets, ADSR, états p-lock)
- **`LAYOUT.md`** — Architecture (lanes modulaires, moteurs, layout 2 colonnes, séquenceur, éditeur)
- **`assets/fd-data.js`** — Schémas de paramètres par moteur (synth/sample/midi)
- **`index.html`** — Maquette interactive fonctionnelle

### Architecture proposée (à implémenter)
- **Lanes modulaires** : 4 au départ (BD/SD/HH/TOM), ajoutables jusqu'à 14, réordonnables
- **Registre de moteurs** : Synth (7 types), Sample, Sample FX, MIDI Out
- **Éditeur dynamique** : contenu selon le moteur assigné, aucun paramètre codé en dur
- **Header** : Transport (▶/■/●) + source MIDI (Internal/Ext) + toggles LED
- **Sound Editor** : Sections dynamiques (OSC/ENV/FILTER/SAT/OUTPUT)
- **Generator/Song** : Panneau partagé avec toggle segmented

### Plan d'implémentation
Voir `TODO.md` — section **[100] Redesign UI complet** (phases 1-5)

---

## 2026-06-10 — Redesign UI Phase 1d — Page buttons + glow LED (build 20260610-203051)

**Build:** `20260610-203051`
**Commits:** UI : stylisation des boutons de page (1-4) avec tokens theme + glow sur LED de lecture

### Changements
- **Boutons de page** (1-4) :
  - Actif : fond `BLUE` + bordure `BLUE`
  - Inactif : fond `PANEL2` + bordure `LINE2`
- **LED de lecture** : glow `RED` semi-transparent autour du point central

---

## 2026-06-10 — Redesign UI Phase 1c — Style global sombre (build 20260610-202742)

**Build:** `20260610-202742`
**Commits:** UI : style global sombre via `egui::Visuals`, fond BG, widgets PANEL2/P_HOVER/P_ACTIVE/BLUE

### Changements
- **Style global** : configuration `egui::Visuals::dark()` personnalisée dans le callback d'init :
  - `panel_fill` = `window_fill` = `extreme_bg_color` = `BG` (10,10,15)
  - `widgets.inactive.bg_fill` = `PANEL2` (28,28,36)
  - `widgets.hovered.bg_fill` = `P_HOVER` (36,36,48)
  - `widgets.active.bg_fill` = `P_ACTIVE` (42,42,56)
  - `selection.bg_fill` = `BLUE` (74,158,255)
  - `window_stroke` = `LINE` (42,42,53)

---

## 2026-06-10 — Redesign UI Phase 1b — Header style + widgets (build 20260610-202506)

**Build:** `20260610-202506`
**Commits:** UI : header redesign avec fond PANEL, bordure LINE, séparateurs verticaux, padding 14px

### Changements
- **Header redesign** :
  - Fond `PANEL` (20,20,25) sur toute la largeur
  - Bordure basse `LINE` (42,42,53) 1px
  - Hauteur fixe `HEADER_H` = 44px
  - Padding horizontal 14px
  - Séparateurs verticaux `DIVIDER` entre les groupes (Brand / Sliders / Toggles)
  - Typographie : `INK` pour le brand, `FAINT` pour le build ID

---

## 2026-06-10 — Redesign UI Phase 1a — Fondations (build 20260610-202115)

**Build:** `20260610-202115`
**Commits:** UI : création des widgets custom (ToggleLED, ToggleSwitch, StyledButton, SegmentedControl) + intégration dans header et plock mode

### Changements
- **`src/ui/theme.rs`** — Tokens design (palette IBM Plex, rayons, gaps, strokes, helpers)
- **`src/ui/widgets.rs`** — Widgets custom :
  - `ToggleSwitch` : 34×18 r10, pastille coulissante
  - `ToggleLED` : pilule h26 r7 avec LED Ø7 et glow
  - `StyledButton` : bouton coordonné h26 r6
  - `SegmentedControl` : toggle groupé (Sound/Sequencer) avec retour d'index
- **`src/ui/engine_registry.rs`** — Registre des moteurs (Synth/Sample/MIDI Out) + groupes de paramètres
- **`src/ui.rs`** :
  - Intégration `ToggleLED` dans le header (Seq, Choke, Auto-Edit, Song)
  - Intégration `SegmentedControl` pour le mode Plock (Sound/Sequencer)
  - Fix imports `ParamSlider` direct depuis `nih_plug_egui::widgets`
- Fix `rect_stroke` 4 arguments (StrokeKind) pour egui 0.31.1

---

## 2026-06-10 — Fix boutons Export MIDI + Drag toujours visibles (build 20260610-085721)

**Build:** `20260610-085721`
**Commits:** UI : déplacement des boutons Export MIDI et Drag vers la Pattern Bank Bar

### Changes
- **[28] Drag & Drop MIDI** : les boutons **Export MIDI** et **Drag** ont été déplacés de la barre des presets (`draw_preset_bar`) vers la **Pattern Bank Bar** (`draw_pattern_bank`).
- **[28] Fix** : ces boutons étaient cachés quand le mode **Song** était activé, car le panel generator (qui contient la barre des presets) est remplacé par le Song Editor en mode Song. Maintenant ils sont toujours visibles.

### Validation
- `cargo test` : 90 tests lib + 61 tests standalone OK
- VST3 installé (copie manuelle car fichier verrouillé)

---

## 2026-06-10 — Plock Volume en haut du menu + TODO mise à jour (build 20260610-082223)

**Build:** `20260610-082223`
**Commits:** UI : Volume en premier dans le menu contextuel des plocks

### Changes
- **[75] Plock UI** : le slider `Volume` est maintenant affiché en haut du menu contextuel des plocks, juste après l'indicateur de mode (Link/Snapshot/Mixed).
- **[75] Plock UI** : le slider `Volume` n'est plus rendu dans la liste data-driven standard pour éviter le doublon.
- **TODO.md** : ajout de 5 nouvelles tâches priorisées :
  - **[91]** Sortir automatiquement du mode edit quand on sélectionne en dehors de la cellule (P1)
  - **[92]** Valeurs du menu plock sound par défaut = valeurs globales de l'instrument (P1)
  - **[93]** Investigation : son très écourté intéressant quand slider OSC maintenu (P2)
  - **[94]** Ajouter un paramètre pitch LFO sur les Toms (P2)
  - **[95]** Ajouter un instrument de type MIDI avec MIDI out (P2/P3)

### Validation
- `cargo test` : 90 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installé

---

## 2026-06-09 — Session : Lane Length lock/follow + volumes dB + taille VST fixe

**Session du 2026-06-09** — Builds : `20260609-152742`, `20260609-160617`, `20260609-162726`, `20260609-173417`, `20260609-184928`, `20260609-185930`

### Résumé de la session
- **[64] Lane Length** : implémentation finale du comportement lock/follow pour les longueurs de lane.
  - Par défaut, chaque lane suit `Pattern Length`.
  - Drag sur `Len` = verrouille la lane sur cette valeur (polyrythmie).
  - Si Pattern > valeur verrouillée → lane garde sa valeur.
  - Si Pattern ≤ valeur verrouillée → lane suit le pattern (trop court).
  - Clic droit = "Follow pattern length" pour déverrouiller.
  - Persistance DAW via `LaneLengthLocks` (`lane-locks-v1`).
  - Fix UI : la cellule `Len` affiche la valeur effective, pas la valeur stockée.
- **[75] Volumes en dB** : sliders de volume affichent `-inf dB` à `+6.0 dB`, stockage interne en gain linéaire `0..2`.
- **[89] Taille VST fixe** : fenêtre forcée à `1480×800`, scroll interne dans le Sound Editor.
- **Commits** : 3 commits sur la session (`4c5fccd`, `b500527`, `14bc83d`, `091d979`).

### Validation globale
- `cargo test` : 90 tests lib + 61 tests standalone OK (dernier build `20260609-185930`)
- `build.ps1 -Install` OK sur les builds finaux

---

## 2026-06-09 - Lane Length lock/follow v2 (build 20260609-185930)

**Build:** `20260609-185930`
**Commits:** Sequencer : lane length avec verrouillage — fix affichage effectif

### Changes
- **[64] Fix UI** : la cellule `Len` affiche maintenant la valeur **effective** (pas la valeur stockée). Quand pattern=48 et lane verrouillée à 50, l'UI affiche 48 (car pattern ≤ valeur verrouillée).
- **[64] Lane Length** : comportement inchangé :
  - **Par defaut** : suit `Pattern Length`.
  - **Drag la cellule `Len`** : verrouille sur cette valeur.
  - **Pattern > valeur verrouillee** : garde valeur (polyrythmie).
  - **Pattern <= valeur verrouillee** : suit pattern.
- **[64] Clic droit** : "Follow pattern length" pour déverrouiller.

### Validation
- `cargo test` : 90 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installé

---

## 2026-06-09 - Lane Length lock/follow (build 20260609-184928)

**Build:** `20260609-184928`
**Commits:** Sequencer : lane length avec verrouillage

### Changes
- **[64] Lane Length** : comportement final clarifie :
  - **Par defaut** : chaque lane suit `Pattern Length` (follow).
  - **Drag la cellule `Len`** : la lane se verrouille sur cette valeur (polyrythmie).
  - **Si Pattern > valeur verrouillee** : la lane garde sa valeur (ex: pattern 64, kick 12 → kick sur 12).
  - **Si Pattern <= valeur verrouillee** : la lane suit le pattern (ex: pattern 16, kick 32 → kick sur 16).
- **[64] Clic droit** : "Follow pattern length" pour deverrouiller une lane.
- **[64] Persistance** : `LaneLengthLocks` (masque `AtomicU16` persistant `lane-locks-v1`) conserve l'etat verrouille/deverrouille par session DAW.

### Validation
- `cargo test` : 90 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Lane Length clamp (build 20260609-173417)

**Build:** `20260609-173417`
**Commits:** Sequencer : simplification lane length — clamp au pattern length

### Changes
- **[64] Lane Length** : les longueurs par instrument sont simplement clampees au `Pattern Length` global. Si une lane fait `32` et le pattern passe a `16`, la lane passe a `16`.
- **[64] Suppression** : le systeme d'override persistant (`PersistentTrackLengthOverrides`) a ete retire. Plus de suivi complexe, plus de migration legacy.
- **[64] UI** : la cellule `Len` est un simple `DragValue` borne a `1..master_length`. Pas de menu contextuel, pas d'etat verrouille/deverrouille.

### Validation
- `cargo test` : 87 tests lib + 61 tests standalone OK
- VST3 installe (copie manuelle apres echec permission build.ps1)

---

## 2026-06-09 - Lane Length follow + override (build 20260609-162726)

**Build:** `20260609-162726`
**Commits:** Sequencer : lane length suit pattern length par defaut

### Changes
- **[64] Lane Length** : les longueurs par instrument suivent maintenant automatiquement `Pattern Length` tant qu'elles n'ont pas ete modifiees manuellement.
- **[64] Override manuel** : modifier une cellule `Len` pose un bit d'override persistant pour cette lane, y compris si la valeur choisie est `16` avec un pattern plus long.
- **[64] UI** : clic droit sur une cellule `Len` modifiee permet de revenir a `Follow pattern length`.
- **[64] Migration** : les anciennes sessions sans masque d'override conservent les lanes non-default (`Len != 16`) comme overrides manuels.

### Validation
- `cargo test` : 90 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Volumes instruments en dB (build 20260609-160617)

**Build:** `20260609-160617`
**Commits:** UI : affichage dB pour volumes instruments

### Changes
- **[75] Sound Editor** : le slider `Volume` affiche maintenant une valeur musicale en dB (`-inf dB` a `+6.0 dB`) au lieu du gain lineaire `0..2`.
- **[75] Grille** : les sliders `Vol` des lanes utilisent aussi une courbe dB, tout en stockant toujours le gain lineaire interne.
- **[75] UX** : double-clic sur un slider volume local reset a `0 dB` (unity gain), pas au milieu numerique de la range.

### Validation
- `cargo test` : 86 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Sound Editor Volume unique (build 20260609-152742)

**Build:** `20260609-152742`
**Commits:** UI : volume instrument unique dans Sound Editor

### Changes
- **[75] Sound Editor** : le champ `Volume` data-driven n'est plus rendu dans la section Output, pour eviter un deuxieme slider pour le meme instrument.
- **[75] Ranges** : les definitions internes `StandardField::Volume` passent de `0..1.5` a `0..2.0` pour rester coherentes avec le slider principal et les volumes de lane.
- **[75] UX** : le Sound Editor garde uniquement le slider `Volume` du haut.

### Validation
- `cargo test` : 86 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Taille editeur VST forcee (build 20260609-150545)

**Build:** `20260609-150545`
**Commits:** UI : taille editor forcee a 1480x800

### Changes
- **[89] UI Layout** : ajout d'un mode `fixed_size()` dans le wrapper `ResizableWindow` vendore.
- **[89] Studio One** : l'editeur demande maintenant explicitement `1480x800` meme si le host ou l'etat UI restaure une ancienne hauteur.
- **[89] Regression** : evite que l'ancien auto-resize par hauteur de contenu remonte la fenetre a `850px`.

### Validation
- `cargo test` : 86 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Hauteur editeur VST reduite (build 20260609-145809)

**Build:** `20260609-145809`
**Commits:** UI : hauteur editor fixee a 800 px

### Changes
- **[89] UI Layout** : taille initiale de l'editeur passee de `1480x850` a `1480x800`.
- **[89] Stabilite visuelle** : hauteur minimale de la `ResizableWindow` passee a `800px` avec `resizable(false)`.
- **[89] Sound Editor** : le scroll interne reste actif pour absorber les instruments avec beaucoup de parametres dans la hauteur reduite.

### Validation
- `cargo test` : 86 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Scroll interne Sound Editor (build 20260609-144118)

**Build:** `20260609-144118`
**Commits:** UI : scroll Sound Editor dans hauteur VST fixe

### Changes
- **[89] UI Layout** : le titre `Sound Editor` et les onglets instruments restent fixes dans la colonne droite.
- **[89] Sound Editor** : les controles de synthese sont maintenant enveloppes dans un `ScrollArea::vertical()` limite a la hauteur disponible.
- **[89] Stabilite visuelle** : les instruments avec beaucoup de parametres n'agrandissent plus la fenetre VST fixe `1480x850`.

### Validation
- `cargo test` : 86 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Hauteur editeur VST fixe (build 20260609-141438)

**Build:** `20260609-141438`
**Commits:** UI : hauteur editor fixee a 850 px

### Changes
- **[89] UI Layout** : taille initiale de l'editeur passee de `1480x520` a `1480x850`.
- **[89] Stabilite visuelle** : hauteur minimale de la `ResizableWindow` passee a `850px` avec `resizable(false)` pour eviter les sauts lors des changements d'instruments.

### Validation
- `cargo check`
- `cargo test` : 86 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Step Fusion selection blink (build 20260609-124302)

**Build:** `20260609-124302`
**Commits:** UI Step Fusion : cellule source de selection clignotante

### Changes
- **[87] Fusion Mode** : la premiere cellule selectionnee par Maj+clic apparait maintenant comme une cellule active temporaire (`X` + fond bleu).
- **[87] Visibilite** : la cellule source clignote entre le fond actif bleu et son fond normal, avec une bordure bleue.
- **[87] Interaction** : relacher Maj annule toujours la selection temporaire et restaure l'affichage normal.

### Validation
- `cargo check`
- `cargo test` : 86 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Step Fusion selection highlight (build 20260609-121512)

**Build:** `20260609-121512`
**Commits:** UI Step Fusion : highlight de cellule source pendant selection

### Changes
- **[87] Fusion Mode** : apres le premier Maj+clic d'une creation de fusion, le point central de la cellule source devient bleu.
- **[87] Interaction** : si Maj est relachee avant la deuxieme cellule, la selection temporaire est annulee et le point reprend sa couleur normale.
- **[87] UI** : le highlight reutilise l'etat existant `fusion_selection_start`, sans changer le scheduling audio ni les donnees de fusion.

### Validation
- `cargo check`
- `cargo test` : 86 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Master Volume crash fix (build 20260609-114803)

**Build:** `20260609-114803`
**Commits:** Audio params : smoothing master volume compatible silence

### Changes
- **[88] Crash Studio One** : correction du lissage du slider `Master Volume` en haut a gauche.
- **[88] Audio** : remplacement de `SmoothingStyle::Logarithmic(50.0)` par `SmoothingStyle::Exponential(50.0)`, car le range du gain master inclut `0.0` (`-inf dB`).
- **[88] Regression test** : ajout d'un test verifiant que le smoothing du master volume reste fini depuis le silence.

### Validation
- `cargo check`
- `cargo test` : 86 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Step Fusion double-clic edition (build 20260609-112628)

**Build:** `20260609-112628`
**Commits:** UI Step Fusion : double-clic fusion traite avant le toggle simple

### Changes
- **[87] Interaction** : le double-clic sur une cellule fusionnee ouvre l'edition Fusion avant la logique de clic simple.
- **[87] Regression** : evite que le premier clic du double-clic desactive la cellule source de la fusion.
- **[87] UX** : conserve le clic simple immediat, sans retour au mecanisme de toggle differe/pending toggle rejete.

### Validation
- `cargo check`
- `cargo test` : 85 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Revert toggle differe Step Fusion (build 20260609-105752)

**Build:** `20260609-105752`
**Commits:** UI Step Fusion : retrait du pending toggle du double-clic

### Changes
- **[87] Revert** : suppression du mecanisme de toggle differe ajoute au build `20260609-104936`.
- **[87] Interaction** : retour au comportement immediat precedent pour le clic sur une cellule fusionnee.
- **[87] Suivi** : retrait de l'entree TODO du build rejete `20260609-104936`.

### Validation
- `cargo check`
- `cargo test` : 85 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Step Fusion edit box reservee (build 20260609-102249)

**Build:** `20260609-102249`
**Commits:** UI Step Fusion : panneau d'edition dans une box reservee stable

### Changes
- **[87] Fusion box** : le panneau `Fusion x-y (cells) Steps ... Delete Close` est maintenant dessine dans une boite Fusion fixe sous la grille, a cote du mode plock/fusion.
- **[87] Layout stable** : la boite Fusion est toujours reservee ; son apparition/disparition ne decale plus l'interface.
- **[87] Edition** : cliquer sur le champ `Steps` de la boite ne ferme plus immediatement le mode edition.
- **[87] Clic exterieur** : pendant l'edition, les clics sur la grille sont neutralises ; un clic hors cellule inline et hors boite Fusion ferme l'edition et garde la cellule source active.

### Validation
- `cargo check`
- `cargo test` : 85 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Step Fusion edition inline stable (build 20260609-100205)

**Build:** `20260609-100205`
**Commits:** UI Step Fusion : edition inline contrainte et sortie par clic exterieur

### Changes
- **[87] Edition inline** : le champ `DragValue` du nombre de pulses remplace maintenant le bouton fusionne avec exactement la meme taille, au lieu d'etre dessine en overlay ; la ligne ne se decale plus pendant l'edition.
- **[87] Clic exterieur** : un clic hors de la cellule fusionnee quitte le mode edition et remet la cellule source en mode normal actif.
- **[87] Clavier** : `Enter` et `Escape` quittent aussi le mode edition.

### Validation
- `cargo check`
- `cargo test` : 85 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-09 - Focus Studio One menus avec VST ouvert (build 20260609-094555)

**Build:** `20260609-094555`
**Commits:** Vendor nih-plug-egui : focus clavier Windows non-intrusif

### Changes
- **Windows/Studio One** : le workaround clavier ne force plus `SetFocus(plugin)` a chaque frame quand egui ne saisit pas de texte.
- **Focus host** : `set_keyboard_focus()` ne refocalise le VST que si le focus ou le curseur est deja dans l'editeur, ce qui laisse les menus de Studio One s'ouvrir pendant que le VST est visible.
- **Saisie texte plugin** : la redirection vers la message window reste active quand egui veut une saisie clavier.

### Validation
- `cargo check`
- `cargo test` : 85 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-08 - Step Fusion Copy/Paste Page + x2 (build 20260608-195857)

**Build:** `20260608-195857`
**Commits:** UI : copie des fusions dans les operations de page et de duplication

### Changes
- **[87] Copy Page** : le clipboard de page embarque maintenant les groupes Step Fusion par instrument, avec start/end locaux et nombre de pulses.
- **[87] Paste Page** : remappe les fusions vers la page cible, remplace les anciennes fusions de cette page, conserve l'etat ON/OFF de la cellule de depart et nettoie les plocks couverts.
- **[87] x2** : duplique les groupes Step Fusion avec offset `current_len`, en respectant les limites page-locales.
- **[87] Clear Page** : supprime aussi les fusions de la page videe.

### Validation
- `cargo check`
- `cargo test` : 85 tests lib + 61 tests standalone OK
- `build.ps1 -Install` OK, VST3 installe

---

## 2026-06-08 - Step Fusion Shift detection robuste (build 20260608-194139)

**Build:** `20260608-194139`
**Commits:** UI : detection Maj stable via Win32 GetAsyncKeyState

### Changes
- **[87] Fusion Mode** : l'indicateur `Maj for fusion mode` et le Shift+clic utilisent maintenant une detection centralisee.
- **[87] Windows/Studio One** : ajout d'un fallback Win32 `GetAsyncKeyState()` pour lire l'etat reel de Maj gauche/droite, car les modifiers egui peuvent etre aleatoires dans un VST selon le focus clavier du host.
- **[87] Comportement** : la couleur bleue de l'indicateur et la creation de fusion reposent sur la meme detection.

---

## 2026-06-08 - Step Fusion mode indicator (build 20260608-193357)

**Build:** `20260608-193357`
**Commits:** UI : indicateur Maj pour mode fusion

### Changes
- **[87] UI Fusion Mode** : ajout d'une section a droite du mode plock sous la grille.
- **[87] Feedback clavier** : affiche `Maj for fusion mode` en gris au repos, puis en bleu quand Maj/Shift est maintenu.
- **[87] Guidance** : quand Maj est actif, affiche aussi `Select 2 cells` pour clarifier la creation de fusion.

---

## 2026-06-08 - Step Fusion nettoie les plocks couverts (build 20260608-192613)

**Build:** `20260608-192613`
**Commits:** Step Fusion : suppression des plocks sous cellules couvertes

### Changes
- **[87] Creation de fusion** : lors d'une fusion, les plocks sound et sequencer des cellules internes couvertes sont supprimes.
- **[87] Source unique** : les plocks de la cellule de depart sont conserves, car c'est la seule cellule source lue par l'audio pour la fusion.
- **[87] UX** : evite les plocks caches/inactifs sous une fusion qui pourraient reapparaitre de facon confuse apres suppression de la fusion.

---

## 2026-06-08 - Step Fusion UX polish (build 20260608-191352)

**Build:** `20260608-191352`
**Commits:** Step Fusion : style standard, edition inline, activation par defaut

### Changes
- **[87] Style cellule fusionnee** : le bloc fusionne reprend les couleurs des cellules standard (active, plock, seq-plock, current) au lieu d'utiliser un style bleu/cyan dedie.
- **[87] Edition inline** : double-clic sur une fusion ouvre maintenant l'edition du nombre de pulses directement dans la cellule ; le menu contextuel propose aussi "Edit Fusion Steps".
- **[87] Interaction** : le double-clic n'active/desactive plus accidentellement la fusion avant d'entrer en edition.
- **[87] Creation** : une fusion nouvellement creee est activee par defaut sur sa cellule source.

---

## 2026-06-08 - Step Fusion UI: vraie fusion graphique (build 20260608-190515)

**Build:** `20260608-190515`
**Commits:** Rendu UI Step Fusion en bloc continu

### Changes
- **[87] Step Fusion UI** : les cellules fusionnees sont maintenant rendues comme un seul widget large couvrant toute la plage fusionnee.
- **[87] Alignement grille** : la largeur du bloc fusionne inclut les espacements internes des cellules remplacees, ce qui garde les colonnes suivantes alignees avec la grille 16 pas.
- **[87] Interaction** : le clic, double-clic et menu contextuel restent portes par la fusion, en utilisant la cellule de depart comme source des triggers/plocks.

---

## 2026-06-07 — Step Fusion V2 from scratch (build 20260607-131747)

**Build:** `20260607-131747`
**Commits:** Refonte Step Fusion : grille fixe + pulses audio + stutter seq-plock désactivé

### Changes
- **[87] Step Fusion V2 — grille fixe** : suppression du rendu en bouton large qui supprimait des colonnes et décalait la grid. Les 16 cellules de page restent toujours rendues avec une largeur fixe.
- **[87] Step Fusion V2 — vrai scheduling audio** : une fusion active déclenche `N` pulses régulièrement espacés sur la durée des cellules fusionnées, au lieu de remapper les steps vers d'autres cellules.
- **[87] Step Fusion V2 — source unique** : la cellule de départ porte l'état ON/OFF et les plocks sonores ; les cellules internes ne déclenchent plus indépendamment.
- **[87] Step Fusion V2 — page-local only** : les fusions qui traversent une page 16-step sont rejetées.
- **[87] Step Fusion V2 — stutter seq-plock désactivé** : le stutter est ignoré côté audio sur une fusion, et l'UI du plock séquenceur l'affiche comme indisponible.
- **[87] Temps réel** : sync fusion UI→audio via buffer fixe préalloué (`load_fusions_into`) ; plus de `Vec` alloué dans `process()` pour les fusions.
- **Tests** : ajout de tests séquenceur pour filtrage des fusions invalides et suppression des triggers internes avec métadonnées de pulses.

---

## 2026-06-07 — Step Fusion fixes : audio, 1-cell, visual overflow (build 20260607-103006)

**Build:** `20260607-103006`
**Commits:** Correction audio + UI des cellules fusionnées

### Changes
- **[87] Step Fusion — audio fix** : `map_step_to_cell` prend maintenant `track_length` et fait une recherche modulaire. Une fusion sur les cellules globales 16-19 avec `track_length=16` s'applique correctement aux steps 0-3 (car 16≡0 mod 16).
- **[87] Step Fusion — single-cell filter** : `set_fusions` rejette les fusions à 1 cellule (pas d'effet rythmique).
- **[87] Step Fusion — UI clamp to page** : le rendu des fusions est coupé à la fin de la page courante (pas de débordement visuel). Les cellules appartenant à une fusion commencée avant la page sont sautées.
- **[87] Step Fusion — UI no 1-cell creation** : Shift+clic sur une seule cellule ne crée plus de fusion.

---

## 2026-06-05 — Session Pattern Bank : plocks, Clear, Generate, Presets (build 20260605-180924)

**Build:** `20260605-180924`
**Commits:** Stabilisation complète Pattern Bank + UX Clear + Generate 64 steps + Presets étendus

### Changes
- **[85] Crash retour P1** fixé : `MAX_PLOCK_BYTES` calcul dynamique depuis `FIELD_COUNT`/`INSTRUMENT_COUNT`/`STEP_COUNT` (plus de hardcode 18 fields)
- **[86] Plocks résiduels au changement de slot** fixé : `clear_all()` + détection auto format legacy (18 vs 46 fields) dans `restore_from_buffers()`
- **Bouton Clear** : déplacé après les slots P1-P8, confirmation 2 étapes ("Clr" → "Sure?" rouge clignotant), annulation auto sur clic Save/slot
- **Suppression** du bouton "Clear" de la section Generator (doublon)
- **Plocks effacés** automatiquement sur : preset (Rock/Funk/Disco), Random, Generate
- **Presets Rock/Funk/Disco** étendus sur 64 steps (répétition bar-by-bar)
- **Generator tiling** : Probabilistic/Markov/Classic répètent le motif 16-step sur 4 bars (64 steps). Euclidean inchangé (déjà 64 steps)
- **Generator respecte `pattern_length`** : steps au-delà de la longueur sont effacés après génération

---

## 2026-06-05 — Fix Clear + confirmation deux étapes [58]

**Build:** `20260605-124507`
**Commits:** Correction du bouton Clear qui ne vidait pas la grille + ajout confirmation deux étapes

### Changes
- **Fix** : le bouton "Clr" vidait les plocks mais pas la grille (step masks) — il appelle maintenant `load_pattern_for_ui(pattern, &Pattern::empty())` pour vider aussi la grille
- **Confirmation deux étapes** : premier clic sur "Clr" affiche "Sure?" en rouge clignotant, deuxième clic confirme le clear
- **Annulation auto** : le mode confirm est annulé si on clique sur Save, un slot P1-P8, ou ailleurs
- **Bouton "Clr"** déplacé à droite des slots P1-P8 pour un flux de travail cohérent (Save → Slots → Clear)
- **Suppression** du bouton "Clear" de la section Generator qui faisait doublon

---

## 2026-06-05 — Déplacement du bouton Clear dans la Pattern Bank [58]

**Build:** `20260605-123720`
**Commits:** UI — bouton "Clr" déplacé après les slots P1-P8, suppression du "Clear" de la section Generator

### Changes
- **Bouton "Clr"** déplacé à droite des slots P1-P8 pour un flux de travail cohérent (Save → Slots → Clear)
- **Suppression** du bouton "Clear" de la section Generator qui faisait doublon avec le bouton "Clr" de la Pattern Bank
- Le bouton "Clr" vide les plocks sound + sequencer directement depuis l'UI thread

---

## 2026-06-05 — Fix plocks liés au pattern bank (clear + restore + legacy format) [58]

**Build:** `20260605-094135`
**Commits:** Correction plocks qui restaient du pattern précédent au changement de slot

### Changes
- **Problème** : quand on chargeait un pattern depuis la bank, les plocks du pattern précédent restaient visibles
- **Cause** : `restore_from_buffers()` skipait le restore si `plock_bytes.len()` < `expected_plock_size` (calculé avec `FIELD_COUNT=46`). Les slots sauvegardés avant le passage de `FIELD_COUNT` de 18 à 46 avaient des données trop courtes
- **Fix** :
  - `restore_from_buffers()` et `PatternSlot::restore()` détectent automatiquement le format (18 vs 46 fields) depuis la taille des données
  - `PlockState::clear_all()` et `SequencerPlockState::clear_all()` : vident tous les plocks avant restauration pour éviter les résiduels
  - `load_pattern_from_slot()` appelle `clear_all()` sur les plocks sound et sequencer avant `restore_from_buffers()`
- **Rétrocompatibilité** : les anciens slots (FIELD_COUNT=18) sont correctement restaurés

---

## 2026-06-05 — UI Pattern Bank : slot vide plus sombre + save positionne + header cleanup [58]

**Build:** `20260605-092903`
**Commits:** Améliorations UX Pattern Bank + cleanup header

### Changes
- Slot **vide** : fond `rgb(16, 16, 22)` + bordure `rgb(40, 40, 50)` (beaucoup plus sombre)
- Slot **enregistré non lu** : inchangé `rgb(48, 48, 58)`
- **Save positionne le slot** : après sauvegarde, le slot est automatiquement marqué comme "chargé" (vert)
  - `save_pattern_to_slot()` met à jour `audio_last_loaded_slot`
  - Le slot sauvegardé s'affiche en vert dans l'UI
- **Header bar cleanup** : suppression du bouton play (non fonctionnel) et de l'affichage BPM
  - Gardé : Master Volume, Swing, Groove, toggles (Seq, Choke, Auto-Edit, Song)

---

## 2026-06-05 — Fix crash [85] : buffer overflow dans copy_data_for_restore() [58]

**Build:** `20260605-090814`
**Commits:** Correction crash retour P1 — `MAX_PLOCK_BYTES` under-allocatait de 2.4x

### Changes
- **Cause racine identifiée**
  - `MAX_PLOCK_BYTES` utilisait `18` (ancienne valeur de `FIELD_COUNT`) au lieu de `46` (valeur actuelle)
  - Calcul incorrect : 66 664 bytes alloués vs 159 848 bytes écrits par `capture()`
  - `copy_data_for_restore()` copiait 159 848 bytes dans un buffer de 66 664 → panic/crash Studio One
- **Fix**
  - `MAX_PLOCK_BYTES` et `MAX_SEQ_PLOCK_BYTES` calculés dynamiquement depuis `FIELD_COUNT`, `INSTRUMENT_COUNT`, `STEP_COUNT`
  - Plus de hardcode — les constantes suivent automatiquement les évolutions du modèle de données
  - `copy_data_for_restore()` protégé par `.min()` pour éviter tout overflow futur
- **Tests**
  - 82 tests passent, tests pattern bank validés

---

## 2026-06-04 — Fix race condition Pattern Bank (mutex lock + divergence last_loaded_slot) [58]

**Build:** `20260604-200117`
**Commits:** Correction race condition pattern bank — grid bloqué sur P2 après switch rapide

### Changes
- **Réduction temps de verrou audio thread**
  - `PatternSlot::copy_data_for_restore()` : copie les données du slot sous le lock (court)
  - `restore_from_buffers()` : restauration lock-free depuis des buffers temporaires
  - `load_pattern_from_slot()` ne tient plus le mutex `pattern_bank` pendant le restore (qui touche des milliers d'atomics)
- **Synchronisation `last_loaded_slot` audio→UI**
  - Nouvel atomic `audio_last_loaded_slot` mis à jour par l'audio thread après chaque `load_pattern_from_slot`
  - L'UI thread lit cet atomic à chaque frame et synchronise `state.last_loaded_slot`
  - Élimine la divergence entre l'affichage (UI) et l'état réel (audio) quand on clique rapidement
- **Buffers préalloués**
  - `temp_plock_bytes: [u8; MAX_PLOCK_BYTES]` et `temp_seq_plock_bytes` dans `DrumFlashVst`
  - Zéro allocation dans l'audio thread pendant le restore

---

## 2026-06-04 — Refonte UX Pattern Bank v2 (Save mode 2 étapes + indicateurs dirty/actif) [58]

**Build:** `20260604-193124`
**Commits:** Finalisation UX pattern bank — save/load explicites, indicateurs d'état, position sous grille

### Changes
- **Nouvelle position** : Pattern Bank sous la grille, au-dessus du générateur
- **Interaction Save à 2 étapes**
  - Bouton **"Save"** : clic pour activer le mode save (clignote), puis clic sur un slot P1-P8 pour sauvegarder
  - Désactive le mode save après sauvegarde
- **Click direct sur slot = Load**
  - Slot occupé : charge immédiatement le pattern dans la grille
  - Slot vide : rien (pas de chargement)
- **Indicateurs d'état**
  - Cercle **vert** sur le slot actuellement chargé (`last_loaded_slot`)
  - Étoile `*` sur slot si pattern modifié depuis le dernier load/save (dirty detection)
- **Reset indicateurs**
  - Presets (Rock/Funk/Disco/Clear/Random) et Generate resettent `last_loaded_slot = None`
  - Le pattern n'est plus lié au bank après modification via preset/generate
- **Synchro `pattern_length` audio→UI**
  - `pending_pattern_length: Arc<AtomicI32>` notifie l'UI thread qui applique via `setter.set_parameter()`

---

## 2026-06-04 — Refonte UX Pattern Bank (boutons Save/Load explicites) [58]

**Build:** `20260604-175459`
**Commits:** Correction de l'UX pattern bank — interactions confuses remplacées par des boutons explicites

### Changes
- **Nouvelle interaction Pattern Bank**
  - P1-P8 = simples sélecteurs de slot
  - Bouton **"Save"** explicite : sauvegarde le pattern courant dans le slot sélectionné
  - Bouton **"Load"** explicite : charge le pattern du slot sélectionné (grisé si vide)
- **Indicateurs visuels clairs**
  - Slot occupé = petit point vert + bordure verte
  - Slot sélectionné = fond bleu
  - Slot vide = fond gris foncé
- **Tooltips explicites** au survol de chaque élément
- **Bugfix : `pattern_length` se met à jour au load**
  - L'audio thread notifie l'UI via `pending_pattern_length` atomic
  - L'UI thread applique la valeur via `setter.set_parameter()`
  - Auparavant, charger un pattern de 32 steps dans un contexte de 16 steps laissait le slider bloqué

---

## 2026-06-04 — Stabilisation Pattern Bank (pas d'alloc audio thread + pas de panic) [58]

**Build:** `20260604-170429`

### Changes
- **`PatternSlot::default()` préalloue les buffers**
  - `capture()` utilise `clear()` + `extend_from_slice()` — zéro allocation dans l'audio thread
- **`load_pattern_from_slot()` sans `.unwrap()`** sur le mutex
- **Tests unitaires ajoutés** : capture/restore roundtrip, préallocation, persistance song

---

## 2026-06-04 — Song Mode (chaînage patterns P1-P8) [58]

**Build:** `20260604-164354`
**Commits:** Implémentation du song mode — chaînage séquentiel des patterns

### Changes
- **Nouveau paramètre `song_mode` (BoolParam, ID: `song_mode`)**
  - Default: `false` (pattern unique en boucle — comportement existant)
  - Quand activé: le séquenceur avance automatiquement dans la séquence de patterns
- **Structure `SongSequence` dans `PatternBank`**
  - 64 steps max, chaque step référence un slot P1-P8 (ou vide `-1`)
  - `length`: nombre de steps actifs
  - `loop_enabled`: boucle la séquence à la fin
  - Persistance DAW via le champ existant `pattern-bank-v1`
- **Logique de playback dans `process()`**
  - Détection du wrap de pattern via `loop_count` du séquenceur
  - Au wrap: avance `song_position`, charge le pattern du slot suivant
  - Si fin de séquence et `loop_enabled`: retour au step 0
- **UI: Toggle "Song" dans la header bar**
  - Checkbox à côté des autres toggles (Seq, Choke, Auto-Edit)
  - Quand Song mode actif: le panel generator est remplacé par l'éditeur de séquence
- **Song Editor UI**
  - Grille horizontale de steps (16 par ligne)
  - Click sur un step: cycle P1 → P2 → ... → P8 → vide
  - Right-click: efface le step
  - Bouton "Loop": toggle boucle
  - Contrôles "Len +/-": ajuste la longueur de la séquence
  - Highlight rouge sur le step en cours de lecture

---

## 2026-06-04 — Désactivation séquenceur interne / Mode MIDI thru [60]

**Build:** `20260604-141711`
**Commits:** Implémentation du mode MIDI thru pour pilotage DAW

### Changes
- **Nouveau paramètre `use_internal_sequencer` (BoolParam, ID: `int_seq`)**
  - Default: `true` (séquenceur interne actif — comportement existant)
  - Quand désactivé: le plugin ne génère plus de triggers depuis le séquenceur interne
  - Le plugin passe en mode "MIDI thru": les notes MIDI reçues déclenchent les instruments
- **Mapping MIDI note → voix**
  - Fonction `instrument_registry::voice_idx_from_midi_note(note: u8) -> Option<usize>`
  - Mappe les notes MIDI standards (GM Drum Map) aux 13 voix du plugin
  - Kick=36, Snare=38, HiHat=42, OpenHH=46, Tom1=50, Tom2=47, Tom3=43, Clap=39, Ride=51, Cymbal=49, Snare606=40, B8=35, Perc1=37
- **Traitement des événements MIDI entrants dans `process()`**
  - NoteOn reçu → lookup de la voix correspondante → `trigger()` avec velocity MIDI
  - Hi-hat choke open hi-hat respecté aussi en mode MIDI
  - Les événements MIDI sont forwardés à la sortie (channel 9) comme en mode séquenceur
  - Le test panel (bouton T) continue de fonctionner en mode MIDI
- **UI: Toggle "Seq" dans la header bar**
  - Checkbox à côté de "Choke" et "Auto-Edit"
  - Label court pour ne pas surcharger la barre

---

## 2026-06-04 — Fix trigger_hard() remet active=true (stutter machine-gun)

**Build:** `20260604-130503`
**Commits:** Fix trigger_hard() manquait self.active = true sur toutes les voix

### Changes
- **Bugfix critique : `trigger_hard()` ne remettait pas `self.active = true`**
  - Quand l'enveloppe atteignait 0 entre deux stutters, la voix devenait inactive
  - Les coups suivants du stutter étaient muets → un seul long son au lieu de coups distincts
  - Fix appliqué sur les 11 voix : Kick, Snare, HiHat, OpenHiHat, Tom, Clap, Ride, Cymbal, Snare606, BassDrum808, Perc1
  - Chaque `trigger_hard()` commence maintenant par `self.active = true` avant de hard-retrigger l'enveloppe

---

## 2026-06-04 — trigger_hard() machine-gun retrigger chain

**Build:** `20260604-102713`
**Commits:** Ajout trigger_hard() sur toute la chaîne de voix

### Changes
- **Ajout de `trigger_hard()` pour les répétitions stutter en "machine gun"**
  - `ExpDecayEnvelope::trigger_from_zero()` — redémarre l'enveloppe depuis zéro avec une rampe d'attaque complète
  - `DecayReleaseEnvelope::trigger_hard()` — hard-retrigger des deux stages (decay + release)
  - `Voice::trigger_hard()` — méthode par défaut qui fallback sur `trigger()`
  - `DrumVoiceKind::trigger_hard()` — dispatch vers chaque voix concrète
  - `DrumSynthesizer::trigger_hard()` — API publique pour le séquenceur
  - Implémentations par voix : Kick, Snare, HiHat, OpenHiHat, Tom, Clap, Ride, Cymbal, Snare606, BassDrum808, Perc1
  - Seule l'enveloppe d'amplitude est hard-reset ; les autres états (pitch, filtres) restent continus

---

## 2026-06-04 — Stutter max 16 + espacement BPM-sync

**Build:** `20260604-100501`
**Commits:** Ajustements stutter après retour utilisateur

### Changes
- **Stutter max augmenté de 8 à 16**
  - Slider UI : `1..=16` au lieu de `1..=8`
  - Commentaire struct mis à jour (`1-16`)
- **Espacement stutter recalculé proportionnellement au step**
  - `spacing = step_duration / stutter` (revert du hardcodé `step/4`)
  - Le step_duration est dérivé du BPM du DAW : `sample_rate * 60 / (bpm * 4)`
  - x2 = 2 coups sur le step, x4 = 4 coups, x8 = 8 coups, x16 = 16 coups
  - L'espacement s'adapte automatiquement au tempo du projet

---

## 2026-06-04 — Fix UI conditions Plocks Séquenceur

**Build:** `20260604-092857`
**Commits:** Correction interactive après retour test utilisateur

### Changes
- **Fix ComboBox condition qui revenait sur `Always`**
  - Suppression complète du `ComboBox` imbriqué dans le menu contextuel
  - Remplacement par une grille de boutons/radios visible directement dans le menu
  - Chaque option appelle directement `set_condition()` dans le handler `.clicked()`
  - Le bouton `Create Seq Plock` n'apparaît plus comme état inactif après une modification dans le même frame, ce qui évite d'écraser la sélection par `SequencerStepParams::default()`
- **Sécurisation atomique du `SequencerPlockState`**
  - `set_active()` utilise maintenant `fetch_or` / `fetch_and` au lieu d'un cycle load/store
  - Lectures des champs sequencer plock en `Acquire`, écritures en `Release`
- **Tests ajoutés**
  - `sequencer_step_params_default_is_playable`
  - `sequencer_condition_setter_roundtrips`

---

## 2026-06-03 — Fix Plocks Séquenceur: defaults, conditions, stutter spacing

**Build:** `20260603-211721`
**Commits:** Corrections bugs [59] Plocks Séquenceur

### Changes
- **Fix `SequencerStepParams::default()`**
  - `probability` = `1.0` (was `0.0` — new seq-plocks were silent by default)
  - `stutter_count` = `1` (was `0` — caused no-trigger)
  - `condition` = `Always`, `microtiming_ms` = `0.0`
- **Retrait Fill / NotFill de `StepCondition`**
  - Supprimés de l'enum, du label, de `all()`, des match arms `lib.rs` et persistance
  - Seuls `First` / `NotFirst` restent comme conditions de loop
- **Fix Stutter: espacement temporel entre triggers**
  - `pending_stutters` : file fixe 128 slots `(samples_until, voice_idx, velocity, step)`
  - Chaque trigger stutter est espacé de `step_duration / stutter_count` samples
  - Évite l'écrasement de tous les triggers au même `sample_idx`
  - `fire_voice_trigger()` helper extrait pour uniformiser audio + MIDI

---

## 2026-06-03 — Revue code: Plocks 64 steps, Export MIDI, Atomics, NoteOff timing

**Build:** `20260603-095833`
**Commits:** Revue de code et corrections post-revue

### Changes
- **Plocks: support complet des 64 steps**
  - `PlockMasks` passe de `AtomicU16` à `AtomicU64` (masque d'activation par instrument)
  - Persistance `plock-v1` rétrocompatible : détection auto ancien format (masques u16) vs nouveau (u64)
  - Tests ajoutés : `plock_supports_steps_16_to_63`, `plock_persistence_roundtrips_step_63`
- **Export MIDI: respecte la longueur du pattern**
  - `export_pattern_to_midi_data()` accepte `pattern_length` (1-64 steps)
  - Boutons Export MIDI et Drag passent la longueur courante
  - Test ajouté : `midi_export_includes_steps_beyond_first_page`
- **Fix NoteOff timing hors buffer**
  - `NoteOff` envoyé à `sample_idx` au lieu de `sample_idx + 1` pour éviter un offset égal à la taille du buffer
- **Sécurisation atomics UI → audio**
  - `bump_version()` : `Release` au lieu de `Relaxed`
  - Lecture version côté audio : `Acquire` au lieu de `Relaxed`
  - `PlockMasks.set_active()` : `Release`, `is_active()` : `Acquire`
  - `voice_test_triggers` : `Release` en UI, `Acquire` en audio
- **Nettoyage**
  - `Cargo.toml` : description corrigée "64-step drum sequencer"
  - `ui.rs` : suppression du double `algo` et commentaire dupliqué
  - Section "Dev: Preset Dumps" masquée en build release (`cfg!(debug_assertions)`)

---

## 2026-06-03 — Plocks Séquenceur: Phases 1-5 (Probabilité + Stutter + Conditions + UI)

**Build:** `20260603-205246`
**Commits:** Architecture séquenceur plock complète avec probabilité, stutter, conditions et UI couleurs

### Changes
- **Nouveau système: Sequencer Plocks** (`TODO.md` [59] Phases 1-5)
  - `SequencerPlockState` : stockage lock-free par step × instrument (4 paramètres)
  - `StepCondition` enum : Always / 1st loop / Not 1st / 1/2, 2/2 / 1/3, 2/3, 3/3 / 1/4, 2/4, 3/4, 4/4 / Fill / Not Fill
  - `SequencerStepParams` : probability (0-100%), stutter_count (1-8), condition, microtiming_ms (±50ms)
  - Persistance DAW via `PersistentSequencerPlockState` (champ `seq-plock-v1`)
- **Probabilité (Phase 1)**
  - Slider 0-100% dans le menu contextuel mode "Sequencer"
  - Skip aléatoire dans le callback audio (`next_rand()` LCG)
  - Par défaut 100% (pas de changement de comportement)
- **Stutter (Phase 4)**
  - Slider 1-8x dans le menu contextuel séquenceur
  - Déclenche multiple fois le son sur le même step
- **Conditions (Phase 5)**
  - Combobox avec toutes les conditions dans le menu contextuel
  - Filtrage dans le callback audio basé sur `loop_count`
  - Fonctionne sur le nombre de boucles du pattern
- **UI**
  - Label "Plock mode:" avant le switch
  - Switch "Sound / Sequencer" sous la grille avec couleur adaptative (orange = Sound, violet = Sequencer)
  - Menu contextuel adaptatif : Sound → plocks instruments, Seq → plocks séquenceur
  - Bouton "Create Seq Plock" / "Clear Seq Plock"
- **Couleurs par mode**
  - Mode Sound : plocks instruments en rouge/orange (inchangé)
  - Mode Sequencer : plocks séquenceur en violet (#9333EA) visibles uniquement en mode Seq
  - Les steps affichent les couleurs correspondant au mode actif uniquement

---

## 2026-06-03 — Archivage PoC web + Documentation + Cleanup labels UI

**Build:** `20260603-171338`
**Commits:** Archivage PoC web, création docs infrastructure/utilisateur, cleanup labels

### Changes
- **Archivage du PoC web**
  - Déplacement de `index.html` et `index.js` vers `archive/web-poc/`
  - Le plugin VST3 est désormais le seul produit actif
  - README.md mis à jour avec la nouvelle structure du repo
- **Documentation**
  - `docs/infrastructure.md` créé — guide build, architecture technique, tests, déploiement
  - `docs/user-guide.md` créé — guide utilisateur complet (UI, plocks, export, multi-out, analog)
- **Cleanup labels UI**
  - Suppression des préfixes `--` devant "Link to global", "Linked to global", "Mixed"
  - Correction "Snapshot Snapshot" → "Snapshot"
  - Correction "Random Random" → "Random"
  - Correction `Dump failed: { }` → `Dump failed: {}`

---

## 2026-06-03 — Copier/coller de plock + cleanup labels UI

**Build:** `20260603-145433`
**Commits:** Plock copy/paste + audit et correction des labels UI

### Changes
- **Copier/coller de plock individuel** (`TODO.md` [61b])
  - Bouton "Copy Plock" dans le menu contextuel d'une step avec plock existant
  - Bouton "Paste Plock" disponible quand le clipboard contient un plock du même instrument
  - Stockage dans `EditorUIState.plock_clipboard` (`SinglePlockClipboard`)
  - Le collage écrase le plock existant sur la step cible
  - Protection multi-instrument : on ne colle que si l'instrument correspond
  - Disponible à la fois dans le mode "création" (step sans plock) et "édition" (step avec plock)
- **Cleanup labels UI**
  - Suppression des préfixes `--` devant "Link to global", "Linked to global", "Mixed"
  - Correction "Snapshot Snapshot current settings" → "Snapshot current settings"
  - Correction "Random Random" → "Random"
  - Correction "-' Clear plock" → "Clear plock"
  - Correction format string `Dump failed: { }` → `Dump failed: {}`

---

## 2026-06-03 — Slider Analog: défaut 0.3 sur instruments opérationnels

**Build:** `20260603-121232`
**Commits:** Post-revue — Valeur par défaut du slider Analog

### Changes
- **Slider "Analog" passe à 0.3 par défaut sur 7 instruments**
  - Kick, Snare, Tom1, Tom2, Tom3, Cymbal, BassDrum808
  - Correspond au drift opérationnel (pas une alternance binaire)
  - Les instruments avec analog fixé/inactif restent à 1.0 (HiHat, OpenHiHat, Clap, Ride, Snare606, Zap)
- **`instrument_registry.rs`** : `sound_settings_default()` retourne `analog: 0.3` pour les 7 instruments concernés
- **`synthesis/mod.rs`** : `VoiceSettings::default()` et `default_for_instrument()` alignés sur 0.3
- **`ADDING_AN_INSTRUMENT.md`** : convention Analog ajoutée — 0.3 si opérationnel, 1.0 si fixé/inactif
- **135 tests passent**, build VST3 installé

---

## 2026-06-02 — UI: Layout page navigation + x2 + LED lecture

**Build:** `20260602-202855`
**Commits:** À venir

### Changes
- **Slider "Len" déplacé vers la ligne des pages**
  - Retiré du header bar, positionné à côté des boutons 1-4
  - Plus logique : la longueur est liée à la pagination
- **Boutons presets de longueur 16/32/48/64**
  - Accès rapide aux longueurs standard
  - Le bouton actif est surligné en bleu
- **Bouton x2 (doubler le pattern)**
  - Copie les steps 0..len vers len..2×len
  - Copie aussi les parameter locks (plocks)
  - Grisé quand len > 32 (limite 64 steps)
- **LED rouge sous la page en cours de lecture**
  - Petit cercle rouge sous le bouton de page actif dans le séquenceur
  - Indépendant du highlight bleu de la page affichée

---

## 2026-06-02 — Kick: 3 types de click fonctionnels

**Build:** `20260602-174136`
**Commits:** `8188cc6` + fix

### Changes
- **Kick: 3 types de click (Soft/Medium/Hard)**
  - Ajout paramètre `kick_click_type` dans `DrumFlashParams` (special_index: 6)
  - Dropdown UI avec labels "Soft / Medium / Hard" au lieu d'un slider numérique
  - **Fix bug critique** : `set_settings()` ne recréait pas le `ClickGenerator` quand `click_type` changeait
  - Valeurs exagérées pour différenciation audible :
    - Soft: 30ms decay, 80% noise, 0.4 level (feutré)
    - Medium: 10ms decay, 30% noise, 1.0 level (standard)
    - Hard: 2ms decay, 0% noise, 2.5 level (agressif)

---

## 2026-06-02 — Copier/Coller de pages avec parameter locks

**Build:** `20260602-155542`
**Commits:** `93886a3`

### Changes
- **Copy/Paste de pages avec parameter locks (plocks)**
  - Copy Page : copie les triggers + tous les plocks de la page
  - Paste Page : restaure les triggers + les plocks
  - Seuls les steps avec plocks sont stockés (optimisation mémoire)
  - Structures `PlockClipboardEntry` et `PageClipboard` ajoutées

---

## 2026-06-02 — Menu contextuel pages + Clear plocks

**Build:** `20260602-152305`
**Commits:** `44ceed5`

### Changes
- **Menu contextuel sur les boutons de page (1-4)**
  - Copy Page : copie les 16 steps dans le presse-papiers
  - Paste Page : colle le presse-papiers dans la page cible
  - Clear Page : efface les triggers ET les plocks de la page
- **Fix : Clear Page efface aussi les plocks**
  - Appelle `plock.clear()` pour chaque instrument et chaque step de la page

### Tests
- Build et installation OK

---

## 2026-06-02 — Fix : slider Len restauré dans la barre d'en-tête

**Build:** `20260602-145233`
**Commits:** `786e368`

### Changes
- **Slider de longueur (Len) restauré**
  - Le slider était dans `draw_top_bar()` qui n'est plus appelé
  - Déplacé dans `draw_header_bar()` entre Swing et Groove
  - Pages 1-4 et Follow toujours visibles dans la grille

### Tests
- Build et installation OK

---

## 2026-06-02 — UI : largeur augmentée + fix Sound Editor

**Build:** `20260602-143954`
**Commits:** `0db6acc`

### Changes
- **Largeur de fenêtre augmentée** : 1400 → 1480 px
  - Colonne gauche : 860 → 900 px
  - Colonne droite : 520 → 560 px
  - Gap entre colonnes : 12 → 20 px
  - Boutons P1..P8 moins tronqués dans la barre supérieure
- **Fix : Sound Editor de nouveau visible**
  - ScrollArea mal configuré masquait le Sound Editor (hauteur ~0)
  - Retrait du ScrollArea temporairement pour restaurer la visibilité

### Tests
- Build et installation OK
- Sound Editor visible et fonctionnel

---

## 2026-06-02 — Fix : suppression du vide en bas de l'UI

**Build:** `20260602-141849`
**Commits:** `8c17e54`

### Changes
- **Auto-resize de la hauteur de fenêtre** selon le contenu réel
  - `ResizableWindow` mesure la hauteur du contenu après chaque frame
  - Ajuste automatiquement la taille de la fenêtre quand `resizable=false`
  - Élimine le vide noir de ~300px en bas de l'interface

### Tests
- Build et installation OK
- UI s'affiche correctement sans espace vide en bas

---

## 2026-06-02 — Layout UI : 2 colonnes (Option A)

**Build:** `20260602-103224`
**Commits:** `XXXXXXX`

### Changes
- **Nouveau layout 2 colonnes** (conforme au LAYOUT.md)
  - **Barre haute** : Flash Drum v0.2 | ▶ | BPM | Master | Swing | Mode | Choke | Auto-Edit | P1..P8
  - **Colonne gauche** (~850px) : Séquenceur (grille 13×16 avec pagination 64) + Générateur
  - **Colonne droite** (~550px) : Éditeur de son (onglets + sound panel)
  - Toute la logique existante conservée (plock, pattern, sound settings, test, etc.)
- Fondations pour le design system et le schema data-driven
  - `src/ui/design_system.rs` : tokens visuels + widgets de base
  - `src/ui/schema.rs` : ParamSpec, Section, Category, schemas par instrument

### Tests
- Build et installation OK

---

## 2026-06-02 — Renommage : Drum Flash → Flash Drum

**Build:** `20260602-085637`
**Commits:** `XXXXXXX`

### Changes
- **Renommage global de la marque** : Drum Flash → Flash Drum
  - Nom du plugin affiché dans le DAW : `Flash Drum`
  - Titre de fenêtre UI : `Flash Drum`
  - Fenêtre drag-drop MIDI : `Flash Drum MIDI Drag`
  - Dossiers utilisateur : `Documents/Flash Drum/exports` et `Documents/Flash Drum/preset_dumps`
  - Scripts build/verif/install : titres mis à jour
  - Documentation (README, AGENTS, ADDING_AN_INSTRUMENT, STUDIO_ONE_MULTI_OUT)
  - `Cargo.toml` authors, `bundle.toml` name
  - **Non modifié** : `VST3_CLASS_ID = DrumFlashPlugin1` (gelé pour compatibilité DAW)
  - **Non modifié** : nom du dossier racine du repo (`E:\Dev\Projets\Drum Flash`)

### Tests
- Build et installation OK
- Plugin s'affiche comme `Flash Drum` dans le DAW

---

## 2026-06-01 — Session: Pattern 64 steps avec pagination style Digitakt

**Build:** `20260601-175002`
**Commits:** `XXXXXXX`

### Changes
- **Pattern étendu à 64 steps** (4 pages × 16 steps)
  - `STEP_COUNT` : 16 → 64 (`pattern.rs`, `plock.rs`, `lib.rs`)
  - Le séquenceur supporte une `master_length` globale (1-64 steps)
  - Chaque track `length_*` passe de max 16 à max 64
- **Pagination UI** (`ui.rs`)
  - 4 boutons de page (1-2-3-4) au-dessus de la grille
  - Mode **Follow** : la page affichée suit automatiquement la tête de lecture
  - Mode **Free** : navigation manuelle entre les pages
  - La grille affiche toujours 16 steps selon la page courante
- **Persistance** (`lib.rs`, `pattern.rs`)
  - Nouveau format `pattern-v2` avec `PatternMasks` wrapper pour `serde_arrays`
  - Migration automatique `pattern-v1` (16 steps) → `pattern-v2` (64 steps)
  - Migration legacy `st01..st16` → `pattern-v2` (padding avec zéros)
- **Fix stack overflow** (`plock.rs`)
  - `PlockValues` et `PlockFieldMasks` alloués sur le heap (`Vec`) au lieu de la stack

### Tests
- 73 lib tests pass
- Build installé dans le dossier VST3 système

---

## 2026-06-01 — Session: Fix range volume master (coherence 0.0-2.0)

**Build:** `20260601-171606`
**Commits:** `XXXXXXX`

### Changes
- **Uniformisation range volume** (`lib.rs`)
  - `master_volume` : range changé de `0.0..1.5` à `0.0..2.0`
  - Cohérent avec les sliders de lane (`0.0..=2.0`) et le volume instrument (`0.0..=2.0`)

### Tests
- Build installé dans le dossier VST3 système

---

## 2026-06-01 — Session: Fix focus fenêtre plugin bloque Windows

**Build:** `20260601-170350`
**Commits:** `XXXXXXX`

### Changes
- **Correction du vol de focus Windows** (`editor.rs` — `win_keyboard::set_keyboard_focus`)
  - **Problème** : `SetFocus` était appelé à chaque frame même quand l'utilisateur avait switché vers une autre application (navigateur, explorer, etc.)
  - **Cause** : `AttachThreadInput` + `SetFocus` forçaient le focus à revenir vers le plugin indépendamment de la fenêtre active
  - **Fix** : vérification que le plugin (ou sa fenêtre parent DAW) est bien la fenêtre au premier plan (`GetForegroundWindow()`) avant d'appeler `SetFocus`
  - Si l'utilisateur a switché vers une autre app, le plugin ne vole plus le focus

### Tests
- Build installé dans le dossier VST3 système

---

## 2026-06-01 — Session: Fix complet corruption UTF-8 UI (séparateurs et caractères spéciaux)

**Build:** `20260601-165420`
**Commits:** `XXXXXXX`

### Changes
- **Correction complète des caractères corrompus** (`ui.rs` + `envelope_viz.rs`)
  - Suppression de 698 séquences box-drawing corrompues (`━` → `-`)
  - Correction de séquences em-dash/en-dash mal encodées (`—`, `–` → `-`)
  - Correction de caractères accentués double-encodés (`é` → `e`)
  - Remplacement des émojis résiduels par du texte ASCII :
    - `🎲` → `Random`
    - `🗑` → `Clear`
    - `📸` → `Snapshot`
    - `↺` → `Undo`
  - **Cause** : double encodage UTF-8 (UTF-8 → Latin-1 → UTF-8) lors de manipulations PowerShell
  - **Prévention** : utilisation exclusive de caractères ASCII dans les labels de boutons et commentaires

### Tests
- 73 lib tests pass
- Build installé dans le dossier VST3 système

---

## 2026-06-01 — Session: Fix corruption UTF-8 UI (émojis et symboles)

**Build:** `20260601-163923`
**Commits:** `XXXXXXX`

### Changes
- **Correction corruption caractères UI** (`ui.rs`)
  - Remplacement des émojis corrompus (🔗, 📸, 🎲, 🗑) par du texte ASCII :
    - "🔗 Link" → "Link"
    - "📸 Snapshot" → "Snapshot" 
    - "🎲 Random" → "Random"
    - "🗑 Clear" → "Clear"
  - Remplacement des symboles de navigation corrompus :
    - "◀" (précédent) → "<"
    - "▶" (suivant) → ">"
    - "↺" (reset) → "R"
  - Remplacement des séparateurs box-drawing (─) par des tirets simples
  - Remplacement des em-dashes (—) par des tirets
  - **Cause** : encodage UTF-8 → Windows-1252 lors de manipulations PowerShell
  - **Prévention** : utilisation exclusive de caractères ASCII dans les labels de boutons

### Tests
- 73 lib tests pass
- Build installé dans le dossier VST3 système

---

## 2026-06-01 — Session: AnalogDrift sur Snare606 + Perc1

**Build:** `20260601-100457`
**Commits:** `XXXXXXX`

### Changes
- **Snare606** : ajout du drift analogique (`AnalogDrift`) — le slider Analog est maintenant fonctionnel :
  - `pitch` → détune la fréquence du résonateur bridged-T par coup (±3.5 %)
  - `level` → variation de niveau par coup (±10 %)
  - `time` → variation du decay/release par coup (±20 %)
  - Mode digital (`analog < 0.5`) = bit-identical, pas de drift.
- **Perc1** : ajout du drift analogique (`AnalogDrift`) — le slider Analog est maintenant fonctionnel :
  - `pitch` → détune la fréquence du sweep FM par coup
  - `level` → variation de niveau par coup
  - `time` → variation du decay/release par coup
  - Mode digital = bit-identical.
- Audit complet : Kick, Snare, Tom ont déjà le drift ; HiHat/OpenHiHat/Ride/Cymbal/Clap masquent le slider ; Kick808 a son propre comportement cold-start.

### Tests
- 73 lib tests pass
- Build installé dans le dossier VST3 système

---

## 2026-05-31 — Session: Task 71 — sécurisation anti-click des autres voix

**Build:** `20260531-184528`
**Commits:** `XXXXXXX`

### Changes
Application du pattern anti-click validé sur la BD à toutes les voix tonales / exposées :
- **perc1** : supprimé le **reset de phase inconditionnel** (à chaque trigger) → reset au cold-start seulement. + plancher d'attaque + DC-blockers L/R.
- **snare**, **tom** : le reset phase/filtre du mode digital ne se fait plus que sur cold-start ; + **drift analog** (slider exposé : hauteur/niveau/temps d'enveloppe par coup) ; + plancher + DC.
- **snare606** : reset résonateur/filtres → cold-start only ; + plancher + DC.
- **hihat** : pas de reset de phase (déjà ok) ; biquad peaking recalculé **seulement si la fréquence change** ; + plancher + DC.
- **snare / snare606 / hihat** recréaient leur enveloppe d'amplitude à chaque `set_settings` (appelé avant chaque trigger) → l'enveloppe repartait de 0 = click au retrigger. Corrigé via **setters** (préserve l'état de queue).
- Nouveau helper partagé **`AnalogDrift`** dans `dsp.rs` (drift pitch/level/temps ; mode digital = facteurs à 1.0).
- ride / cymbal / clap / open_hihat / kick_808 : déjà click-safe (pas de reset de phase), **non modifiés**.

### Tests
- 73 lib tests pass (nouveau garde-fou `perc1_no_click_on_retrigger_during_tail` : edge au retrigger = 0.004 → phase continue).
- Build installé dans le dossier VST3 système.

---

## 2026-05-31 — Session: Vrai fix du click parasite BD + drift analogique

**Build:** `20260531-155232`
**Commits:** `XXXXXXX`

### Diagnostic (mesuré, pas supposé)
- Le « click parasite » sur changement de hauteur n'était PAS en mode analog (mesuré propre : saut au raccord ~0.001–0.003) mais dans le chemin **digital** (`analog < 0.5`) : reset de phase sur une queue sonore + un **crossfade mathématiquement faux** (snapshot de phase figé, ratio inversé, saut brutal au sample 8). Saut mesuré ~0.20 filtre ouvert.
- Le filtre par défaut très bas (30 Hz) **masquait** le défaut → d'où l'intermittence (« revient plus ou moins fort »).
- Le test de click existant ne mesurait que l'énergie HF 3–20 kHz → **aveugle** à une discontinuité de phase basse fréquence (sortait 0.81× la baseline).

### Changes (`kick.rs`, `dsp.rs`)
- **Suppression du reset de phase en retrigger + suppression totale du crossfade cassé.** La phase n'est jamais resetée sur une queue vivante (les oscillateurs sont des accumulateurs de phase → un changement de fréquence est sans click par nature).
- **Reset au démarrage à froid uniquement** (`!was_active`) : phase + filtre + smoothers + dc_block remis à zéro → attaque propre même à 0 ms.
- **Plancher anti-click sur l'attaque d'amplitude** `MIN_AMP_ATTACK_MS = 0.5` (un attack de 0 ms = une marche = un click par définition).
- **Bug digital corrigé** : `pitch_env.trigger()` plafonnait le sweep à +1 Hz → remplacé par `trigger_reset_to(pitch_peak)`.
- **Mode analog/digital re-rendu utile** : digital = identique au bit près à chaque coup ; analog = drift par coup (hauteur ±3.5 %, niveau ±10 %, temps d'enveloppe decay+release ±20 % — la longueur de queue varie ~624–906 ms, le plus audible).
- `dsp.rs` : ajout `ExpDecayEnvelope::trigger_reset_to`, ré-ajout `SquareOsc::reset_phase`, retrait des getters morts du crossfade.

### Tests
- 72 lib tests pass. Nouveaux garde-fous : `test_kick_no_click_on_plock_retrigger_either_mode`, `test_kick_zero_attack_no_click`, `test_kick_analog_drifts_digital_is_stable` (mesure : digital diff 0.0, analog diff > 0), + rendu WAV digital.
- Build installé dans le dossier VST3 système.

### À suivre
- Appliquer le même pattern anti-click + le sens analog=drift/digital=stable aux autres voix tonales : **perc1, snare, tom, snare606, hihat**.

---

## 2026-05-30 — Session: Fix click BD sur changement de hauteur (plock)

**Build:** `20260530-195702`
**Commits:** `XXXXXXX`

### Changes
- **Anti-click kick sur plock frequency** (`kick.rs`)
  - `FREQ_SMOOTH_MS` : 0.1 ms → **2.0 ms** (lissage de la fréquence d'oscillateur)
  - Crossfade digital mode : 2 → **8 échantillons** (transition de phase plus douce)
  - Ajout d'un `filter_cutoff_smoother` pour éviter les sauts sur `filter_freq`
  - `update_derived_params` ne touche plus le filtre directement (smoothed dans `process_sample`)
  - Test unitaire `test_kick_plock_frequency_change_no_click` qui reproduit le scénario
- **Fix root cause: boucle de version dans `iter_samples`** (`lib.rs`)
  - `sound_settings_state.version` était vérifiée **à chaque échantillon** dans `process()`
  - Si un trigger avec plock se produisait, puis la version changeait (modif UI), les settings globaux écrasaient le plock dans le même buffer → discontinuité d'un échantillon = click
  - Déplacée **avant** `iter_samples`, exécutée une fois par buffer
  - Test de rendu audio `test_kick_plock_click_audio_render` génère un WAV + analyse HF

### Tests
- 67 lib tests pass (2 nouveaux)
- Build installé dans le dossier VST3 système

---

## 2026-05-30 — Session: Réparation ui.rs + Masquage slider Analog

**Build:** `20260530-174031`
**Commits:** `XXXXXXX`

### Changes
- **Réparation fichier ui.rs corrompu** (session précédente plantée)
  - Suppression de ~2500 lignes dupliquées dans la section Preset Dumps
  - Suppression du bloc `if Analog` mal formé dans le match des paramètres sliders
  - Restauration de la structure correcte du match Slider/Checkbox depuis git
- **Masquage du slider Analog** pour 6 instruments (HiHat, OpenHiHat, Ride, Cymbal, Perc1, Zap)
  - Slider masqué dans le Sound Panel (remplacé par placeholder 0.0 dans les dumps)
  - Seuls Kick, Snare, Tom, Snare606, Kick808 et B8 exposent le paramètre Analog

### Tests
- Compilation propre : 0 erreur, 6 warnings (unused_variables/methods)
- Build installé dans le dossier VST3 système

---

## 2026-05-29 — Session: Correction corruption UTF-8 dans l'interface

**Build:** `20260529-174106`
**Commits:** `XXXXXXX`

### Changes
- **Fix caractères ésotériques dans l'UI** (ui.rs, lib.rs)
  - 49 occurrences de caractères corrompus remplacés par les bons caractères Unicode
  - Émojis restaurés : ◀, ▶, 🎲, 🔗, 📸, 🔀, ↺, 🗑
  - Caractères accentués restaurés : é, è, à, ç, ê, ô, ù, î, ï
  - Séparateurs et flèches restaurés : —, →, ─, ■
  - Cause : manipulations PowerShell précédentes en encodage Windows-1252 au lieu d'UTF-8
  - Fix via script Python avec mapping byte UTF-8 explicite

### Tests
- 58 lib tests + 44 standalone tests pass
- Build installé

---

## 2026-05-29 — Session: Tests de stress du séquenceur et documentation

**Build:** `20260530-154620`
**Commits:** `7be01c1` et suivants

### Changes
- **Tests de stress du séquenceur** (sequencer/stress_tests.rs)
  - 6 tests de stress implémentés couvrant :
    * `test_long_session_stability` : stabilité sur 1 minute (extensible à 1h)
    * `test_complex_pattern_changes` : changements dynamiques de patterns
    * `test_daw_sync_scenarios` : synchronisation play/stop/seek
    * `test_high_cpu_load_patterns` : patterns denses à haute charge
    * `test_groove_timing_stability` : stabilité du timing avec différents grooves
    * `test_track_push_pull_stability` : décalages de piste (push/pull)
  - Tous les tests passent : 6/6 nouveaux tests + 59/59 tests existants
  - Couverture étendue : longue durée, charge CPU, synchronisation DAW

- **Analyse complète mode Analog vs Digital** (documentée dans TODO.md)
  - 5 instruments utilisent le mode analog/digital (Kick, Kick808, Snare, Snare606, Tom)
  - 7 instruments toujours en mode "analog" (Clap, HiHat, OpenHiHat, Ride, Cymbal, Perc1, Zap)
  - Documentation des comportements et recommandations d'utilisation

### Tests
- 65 lib tests + 51 standalone tests pass (incluant les 6 nouveaux tests de stress)
- Build prêt pour installation et test dans Studio One
- Validation complète de la stabilité du séquenceur
  - Cela créait un pic de amplitude massif (step de 0.468) superposé à la tail
  - Fix : ne retrigger le click que sur un cold start (!was_active), pas sur un retrigger
  - Les tests confirment : max step passe de 0.468 à 0.0036 avec le fix
  - Tous les tests existants passent (6 kick tests + 52 autres)

### Tests
- 58 lib tests + 44 standalone tests pass
- Build installé et prêt à tester

---

## 2026-05-29 — Session: Correction focus clavier Windows (solution officielle)

**Build:** `20260529-135735`
**Commits:** `XXXXXXX`

### Changes
- **Correction focus clavier** (Windows uniquement) — Solution officielle egui-baseview
  - Problème connu : les événements clavier sont capturés par le DAW parent au lieu de la fenêtre enfant du plugin
  - Référence : [baseview#192](https://github.com/RustAudio/baseview/issues/192), [egui-baseview#20](https://github.com/BillyDM/egui-baseview/issues/20)
  - Solution : activation de la feature windows_keyboard_workaround dans egui-baseview
  - Cette feature appelle window.focus() automatiquement quand des événements de saisie sont détectés
  - Modification : endor/nih-plug/nih_plug_egui/Cargo.toml — ajout de la feature aux features par défaut

### Tests
- Build installé et prêt à tester dans Studio One / Reaper

---

## 2026-05-29 — Session: Correction focus clavier Windows + Preset dumps Phase 1a

**Build:** `20260529-124136`
**Commits:** `XXXXXXX`

### Changes
- **Correction focus clavier** (Windows uniquement)
  - Le DAW capturait les événements clavier et ne les transmettait pas au plugin
  - SetFocus appelé automatiquement sur le HWND de la fenêtre du plugin à chaque frame
  - PLUGIN_HWND stocké dans une variable statique publique (
ih_plug_egui::editor.rs)
  - Fonction publique 
ih_plug_egui::ensure_window_focus() exposée
  - Appel systématique dans ui.rs::update callback via ensure_keyboard_focus()
- **Preset dump dev tools** (Phase 1a)
  - Section "Dev: Preset Dumps" dans le Sound Panel
  - Dump/Load/Delete de presets JSON dans Documents/Flash Drum/preset_dumps/
  - serde_json ajouté aux dépendances

### Tests
- 58 lib tests + 44 standalone tests pass
- Build installé et prêt à tester

---

## 2026-05-29 — Session: Dev tools preset dumps (Phase 1a)

**Build:** `20260529-095403`
**Commits:** `XXXXXXX`

### Changes
- **Preset dump dev tools** (preset_dumps.rs, ui.rs)
  - Collapsible "Dev: Preset Dumps" section in Sound Panel
  - **Dump** : captures current instrument settings (13 standards + algo + specials) to JSON
  - **Load** : restores dumped settings + switches to target instrument tab
  - **Delete** : removes dump file
  - Files stored in Documents/Flash Drum/preset_dumps/
- **New dependency** : serde_json = "1.0" (Cargo.toml)
- **New module** : src/preset_dumps.rs (dump/list/load/delete preset JSONs)

### Tests
- 58 lib tests + 44 standalone tests pass
- Build installed and ready for factory preset authoring

---

## 2026-05-28 â€” Session: UI polish + polyrhythm fix + generator roles

**Build:** `20260528-175015`
**Commits:** `XXXXXXX`

### Changes
- **Volume at top of Sound Editor** (`ui.rs`)
  - Dedicated group with separator before OSC/ENV/FILTER/SAT/OUTPUT families
  - Large slider (0.0â€“2.0) right under "Sound Editor" heading
- **Per-lane volume in pattern grid** (`ui.rs`)
  - Compact 40px slider before Mute/Solo/Test buttons
  - Reads/writes `sound_settings.instruments[inst].volume` directly
- **Plock colour coding** (`ui.rs`)
  - **Orange** (255, 140, 0) â†’ Link mode or mixed plock
  - **Red** (220, 50, 50) â†’ Full snapshot
  - Darker variants for inactive steps
- **True polyrhythm** (`sequencer/mod.rs`)
  - Independent `step_counter` per track, incremented on master-step transition
  - Fixes identical bars bug with `master_step % length`
  - Tracks resync at LCM(master, track_length)
- **Steps beyond lane_length erased** (`ui.rs`)
  - Complete visual removal (no button, no background) for clarity
- **Pattern generator roles enriched** (`generator/styles.rs`)
  - Rock style: Snare 606 backbeat layer, 808 Kick downbeat reinforcement, Perc1 crash/FX accents
  - All 13 instruments now have musically meaningful roles
- **Grid spacing** (`ui.rs`)
  - Steps in horizontal containers with 6px spacing
  - Header labels aligned with exact column widths
- **Bugfix: first step not read on play** (`sequencer/mod.rs`)
  - `play()` and `force_step0_trigger()` initialise `step_counter` to `length - 1`
- **Deployment rule added** (`AGENTS.md`)
  - Systematic build + install after every task completion

### Tests
- 58 lib tests + 44 standalone tests pass
- Multiple builds tested and installed throughout the session

---

## 2026-05-28 â€” Generator roles enriched + polyrhythm fix + dimmed steps

**Build:** `20260528-154125`
**Commits:** `XXXXXXX`

### Changes
- **Pattern generator roles enriched for Rock style** (`styles.rs`)
  - Snare 606: backbeat layer (steps 4, 12, 6, 10) with 35% probability
  - 808 Kick: sub-bass reinforcement on downbeats 0 and 8 only
  - Perc1: crash/FX accents (steps 0, 14, 15, 7, 11) with 20% probability
  - All 13 instruments now have musically meaningful roles (no more user-only)
- **True polyrhythm with independent step counters** (`sequencer/mod.rs`)
  - Each track maintains its own `step_counter` incremented on master-step transition
  - Fixes the bug where `current_step = master_step % length` repeated identically every bar
  - Tracks now cycle independently and resync at LCM(master, track_length)
- **Dimmed steps beyond track length** (`ui.rs`)
  - Steps beyond `lane_length` are completely erased (no button, no background)
  - Active steps beyond length shown in dark blue for clarity

### Tests
- 58 lib tests + 44 standalone tests pass
- Build OK, bundle generated, installed to system VST3 folder

---

## 2026-05-28 â€” UI improvements: per-lane volume & plock colour coding

**Build:** `20260528-142648`
**Commits:** `XXXXXXX`

### Changes
- **Volume moved to top of Sound Editor** (`draw_sound_panel`)
  - Large `LocalParamSlider` (0.0â€“2.0) displayed right under the "Sound Editor" heading
  - No longer buried inside the Output family group
- **Per-lane volume control in pattern grid** (`draw_grid`)
  - Compact 40 px slider next to each instrument label (before Mute/Solo/Test)
  - Reads/writes `sound_settings.instruments[inst].volume` directly
  - Calls `bump_version()` on change so the audio thread picks it up
- **Plock colour coding: link vs snapshot** (`draw_grid`)
  - **Orange** (255, 140, 0) â†’ Link mode or mixed plock (`field_mask == 0` or partial)
  - **Red** (220, 50, 50) â†’ Full snapshot (`field_mask == all_bits`)
  - Darker variants for inactive steps with plock only
  - Makes it immediately obvious which steps are fully frozen vs. following globals

### Tests
- Build OK, bundle generated, installed to system VST3 folder
- 0 new compiler errors (5 pre-existing warnings only)

---

## 2026-05-27 â€” Bugfix B8 + Cymbal shimmer & noise colour

**Build:** `20260527-202249`
**Commits:** `XXXXXXX`

### Changes
- **Bugfix B8 silent after CY param change** (`ExpDecayEnvelope::set_attack_ms`)
  - Division-by-zero when `attack_time` shortened to 0 during active ramp â†’ permanently corrupted envelope with NaN
  - Fix: snap immediately to `attack_peak` and clear `attack_remaining` when zeroed mid-ramp
  - Test button "T" now calls `set_voice_settings` before `trigger` (was using stale params)
- **Cymbal Sound Panel refactor**
  - Removed unused `frequency` parameter (noise-based voice, no oscillator)
  - Added `Shimmer Freq` (1â€“50 Hz, default 15 Hz) â€” modulates FM shimmer LFO rate
  - Added `Noise Type` combobox: White / Pink / Brown / Blue
    - `PinkNoise` (Voss-McCartney), `BrownNoise` (integrator), `BlueNoise` (differentiator) in `dsp.rs`
    - Independent L/R generators for stereo mode, no shared state
  - `CymbalSettings` now stores `shimmer_freq` and `noise_type` via `special[0..1]`
  - Retro-compatibility: old plock snapshots saved `special[0]=0.5` â†’ now interpreted as 0.5 Hz shimmer (slow, nearly static)

### Tests
- 54 lib tests pass, 41 standalone tests pass
- New: `shimmer_produces_varying_filter_cutoff`, `set_settings_updates_shimmer_freq`, `cymbal_shimmer_through_drum_synthesizer`

---

## 2026-05-26 â€” Saturation generalised to all 13 instruments

**Build:** `20260526-101659`
**Commits:** `XXXXXXX`

### Changes
- **Saturation on all 13 voices**: Kick, Snare, HiHat, OpenHiHat, Tom1-3, Clap, Ride, Cymbal, Snare606, BassDrum808, Perc1
- **Dedicated SAT section** in Sound Panel (`ParamFamily::Saturation`) â€” no longer mixed in OSC/OUTPUT
- **Algorithm names displayed** in combobox (SoftClip, Valve, Transistor, HardClip, Tape) instead of numbers
- **Pre-Filter checkbox** now functional â€” routes saturation before or after the filter chain
- **Per-instrument special params** using saturation slots in `special[8]` array
  - Instruments with existing specials (Snap, Echo, Stick, etc.) append saturation after
  - Instruments without specials use indices 0-4
  - BassDrum808 limited to 4 saturation params (no Pre-Filter slot due to 8-element array)
- **65 new FloatParam** declarations in `DrumFlashParams` (5 params Ã— 13 instruments)

---

## 2026-05-23 â€” Saturation / distortion per instrument (Snare 606)

**Build:** `20260523-211642`
**Commits:** `XXXXXXX`

### Changes
- **New saturation module** (`saturation.rs`) with 5 distinct algorithms:
  - **SoftClip** â€” smooth tanh, warm and musical
  - **Valve** â€” strong asymmetry, tube glow, even harmonics
  - **Transistor** â€” germanium grit, crunchy, emphasizes highs (+35% positive side)
  - **HardClip** â€” brutal digital clipping, aggressive and square
  - **Tape** â€” soft compression "glue", smooth transient taming
- **Saturation exposed in Sound Panel** for Snare 606 (S6):
  - Saturation Type (0-5, step 1)
  - Saturation Amount (0-1, drive mapped 1Ã—..20Ã—)
  - Saturation Mix (0-1, dry/wet)
  - Saturation Output Gain (0.5-2.0, makeup)
  - Saturation Pre-Filter â˜‘ (checkbox toggle, post-filter by default)
- **Auto-edit enabled by default** (`BoolParam::new("Auto Edit", true)`)
- **Hold parameter restored** on Snare 606 (was missing from `SNARE606_STD`)
- Special params use slots 3-7 of `special[8]` for saturation (indices 0-2 remain resonance/tone/snap)

---

## 2026-05-23 â€” Snare 606 body enhancement (v4)

**Build:** `20260523-154654`  
**Commits:** `XXXXXXX`

### Changes
- **Hold parameter exposed** in the Sound Panel UI (ENV group). Default 8 ms for a thicker body; user can tweak from 0 to 0.5 s.
- (Retains v3 changes: user-controllable hold, body oscillator, boosted body gain, raw noise excitation, snap envelope, revised mix, tuned defaults.)
- All 43 tests pass; `cargo check --all-targets` clean.

---

## 2026-05-23 â€” Snare 606 body enhancement (v3)

**Build:** `20260523-102533`  
**Commits:** `XXXXXXX`

### Changes
- **Hold now user-controllable** via the Hold parameter (default 8 ms for thicker body). The envelope stays at peak for `hold` seconds before decay starts.
- **Default hold increased** from 0 ms to 8 ms to give a thicker, more rounded body out of the box.
- (Retains v2 changes: body oscillator, boosted body gain, raw noise excitation, snap envelope, revised mix, tuned defaults.)
- All 43 tests pass; `cargo check --all-targets` clean.

---

## 2026-05-23 â€” Snare 606 body enhancement (v2)

**Build:** `20260523-100824`  
**Commits:** `XXXXXXX`

### Changes
- **Body oscillator added**: pure `SineOsc` at resonator frequency mixed with raw noise as excitation (`excitation = noise + sine * 0.6`). Gives the resonator a tonal fundamental to resonate with â€” much closer to the real TR-606 VCO+bridged-T topology.
- **Body gain boosted**: `tone * 1.2` (was `tone * 0.7`). More weight when tone is up.
- (Retains v1 changes: raw noise excitation, snap envelope, revised mix, tuned defaults.)
- All 43 tests pass; `cargo check --all-targets` clean.

---

## 2026-05-23 â€” Snare 606 punch overhaul (v1)

**Build:** `20260523-095847`  
**Commits:** `XXXXXXX`

### Changes
- **Snare 606 signal chain rework** for more punch and closer TR-606 character:
  - **Raw noise excitation**: the bridged-T resonator is now driven by unfiltered white noise (previously the noise was softened by a LP before hitting the resonator, smearing the transient).
  - **Dedicated snap envelope**: ultra-short burst (0.2 ms attack, 3 ms decay) on raw noise for the percussive attack that defines the 606 snare.
  - **Revised mix architecture**: body (resonator) + wires (HP-filtered softened noise) + snap (raw noise burst), each with independent gain.
  - **Body gain now scales 0..0.7** (was 0.4..1.0), so tone=0 gives a pure wires+snap sound.
- **Defaults tuned** for a tighter, more aggressive sound:
  - decay 0.7 s â†’ 0.25 s
  - filter_freq 3000 Hz â†’ 8000 Hz
  - tone 0.55 â†’ 0.4
  - snap 0.3 â†’ 0.6
- All 43 tests pass; `cargo check --all-targets` clean.

---

## 2026-05-23 â€” Session : revert [54], docs update

**Build:** `20260523-092208`  
**Commits:** `520e6d8`, `b604ae8`

### Changes
- Update `ADDING_AN_INSTRUMENT.md` for typed settings ([39] generalization).
- Attempt [54] Alt+mouse precision input on ParamSlider + egui::Slider.
- Revert [54] : egui::Slider is a closed widget, cannot reliably intercept Alt+drag. Custom bar+DragValue replacement broke UX.
- Decision : [54] requires a custom widget built from scratch (separate bar + value text).

---

## 2026-05-21 â€” [39] Typed per-instrument settings (all 13 voices)

**Build:** `20260521-213022`  
**Commits:** `fcde87c`

### Changes
- Generalize typed settings structs to all 13 instruments (Kick prototype â†’ all voices).
- New settings files: `SnareSettings`, `HiHatSettings`, `OpenHiHatSettings`, `TomSettings`, `ClapSettings`, `RideSettings`, `CymbalSettings`, `Snare606Settings`, `Kick808Settings`, `Perc1Settings`.
- Each voice refactored to store its typed struct instead of `VoiceSettings` + opaque `special[N]`.
- `VoiceSettings` remains the persistence boundary; conversions happen in `set_settings()` with zero-allocation stack copies.
- All 43 tests pass; bit-identical guarantee maintained.

---

## 2026-05-21 â€” [39] Prototype Kick : typed per-instrument settings

**Build:** `20260521-201743`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- New typed settings struct for Kick (`KickSettings`) replacing opaque `special[0]` access.
- `KickSettings` contains named fields for all standard and special parameters used by the Kick voice.
- `From<VoiceSettings>` and `Into<VoiceSettings>` implementations for seamless conversion at the persistence boundary.
- `KickVoice` refactored to store `KickSettings` internally; `Voice::set_settings` wrapper handles conversion.
- Round-trip test (`kick_settings_roundtrip_preserves_all_fields`) verifies no data loss.
- All existing kick tests pass unchanged â€” confirms bit-identical behavior.
- No change to `plock-v1` format, `DrumFlashParams`, or automation IDs.

---

## 2026-05-21 - External MIDI drag helper

### Changes
- Add `drum-pattern-midi-drag-helper.exe`, a Windows helper bin that performs OLE `DoDragDrop` outside the DAW process.
- Re-enable `Drag`: the plugin exports the MIDI file, then opens a tiny topmost `Drag MIDI` helper window with the exported `.mid` path.
- Update `build.ps1` to copy the helper next to the VST3 DLL in the bundle/install.
- Keep MIDI file export available through the `MIDI` button and `Copy Path`.
- Polish the helper window into a compact rounded drag handle instead of a raw Windows-looking box.

### Notes
- Direct in-process OLE drag crashed Studio One; the helper isolates that risk from the host.
- The previous invisible helper launch did not provide a reliable Windows drag source. Drag now starts from the helper window itself.

---

## 2026-05-21 â€” Perc1 Hold wiring

### Fixes
- Wire Perc1 `hold` into its amplitude `DecayReleaseEnvelope` on creation and settings updates.
- Add a regression test confirming Perc1 Hold extends the active envelope duration.

---

## 2026-05-21 â€” Targeted stereo controls

### Changes
- Expose Stereo in the Sound Panel for Snare606 without exposing it on B8.
- Keep Kick, B8 and Toms mono-focused in the registry.
- Fix Snare606 resonance retuning so both left and right resonators update when resonance changes.
- Add a Snare606 stereo unit test that verifies stereo mode produces independent L/R channels.

---

## 2026-05-21 â€” Per-instrument Attack parameter

### Changes
- Add `attack` to `VoiceSettings` and expose it in the Sound Panel ENV group for every instrument.
- Wire Attack into each amplitude `DecayReleaseEnvelope`, preserving the existing anti-click ramp defaults per voice.
- Update the amplitude envelope graph and legend to show full A-H-D-R shape.
- Persist sound settings with 13 fields per instrument while migrating old 12-field states.
- Extend plocks with Attack as appended field 18, preserving legacy field 12 for old Clap Echo compatibility.

---

## 2026-05-20 â€” Plock Snapshot vs Link mode

**Build:** `20260520-211700`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- **Plock per-field masks** (`PlockFieldMasks`) : each plock step now tracks which fields are explicitly overridden via an 18-bit `u32` mask.
- **Snapshot mode** (default) : "ðŸ“¸ Snapshot current settings" copies all global values and locks them â€” previous behavior.
- **Link mode** (new) : "ðŸ”— Link to global" activates the plock without copying values; only fields you subsequently modify override the live global settings.
- **`get_settings` merge** : audio thread builds global `VoiceSettings`, then merges with plock â€” overridden fields come from plock storage, unmodified fields fall back to globals.
- **Plock editor UI** :
  - Mode indicator : `ðŸ”— Linked`, `ðŸ“¸ Full snapshot`, or `ðŸ”€ Mixed`.
  - Bold labels for overridden fields, weak labels for linked fields.
  - `â†º` reset button per field to revert to global (clears the bit).
  - Per-field `set_field` writes only the changed field instead of rewriting the entire `VoiceSettings`.
- **Persistence retro-compatibility** : old presets without field masks load as full snapshots (all bits set).
- New unit tests : `link_mode_returns_global`, `merge_takes_modified_fields`, `set_field_only_sets_one_bit`, `clear_field_unlinks_without_clearing_plock`, `clear_removes_field_mask`.

---

## 2026-05-20 â€” Sound Panel redesign (families + interactive envelope viz)

**Build:** `20260520-123040`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- Sound Panel fully data-driven from `instrument_registry.rs`:
  - New `ParamFamily` enum (Osc / Env / Filter / Output) with `StandardParamDef` metadata (range, log scale, suffix, checkbox).
  - Parameters grouped per family with titled frames.
  - Removed legacy `InstrumentCapabilities` â€” parameter visibility is now encoded in `standard_params` slices.
- Interactive envelope visualizations:
  - `draw_amp_envelope` : AHDSR-style curve with colour-coded phases (Hold=cyan, Decay=blue, Release=purple). Attack phase is hidden when no Attack parameter exists.
  - `draw_filter_envelope` : dedicated filter-env curve (orange) inside the FILTER family group.
  - Layout horizontal : params on the left, graph on the right.
  - Real-time update when moving Decay / Release / Curve sliders.
- Fixed decay slider ranges that were clamping long-decay voices (Ride 1.2s, Cymbal 2.0s).

---

## 2026-05-19 â€” Perc1 refactor (Zap â†’ Perc1)

**Build:** `20260519-191344`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- Rename Zap â†’ Perc1 (`perc1.rs`, `DrumVoice::Perc1`, label `"P1"`, all params `perc1_*`).
- Migrate Perc1 `amp_env` from `ExpDecayEnvelope` to `DecayReleaseEnvelope` â€” Release slider is now wired.
- Fix `set_settings` anti-click invariant: use `set_decay()` / `set_release()` / `set_curve()` instead of recreating envelopes.
- Add `filter` + `filter_env` to Perc1 with additive cutoff formula.
- Fix latent bug in `voice_settings_for`: index 12 now correctly reads `algo_perc1`.
- Update plock tests, MIDI export tests, generator comments, and algo registry for Perc1.

### Known issues
- Perc1 Release and other parameters reported as non-responsive in Studio One â€” under investigation ([50]).

---

## 2026-05-19 â€” Revert stable + documentation

**Build:** `20260519-163250`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- Revert code to stable commit `5ae1286` (Zap voice) after critical bugs identified in Perc1 commit `8d56e72` (envelope recreation in `set_settings`, broken release/filter env, hardcoded plock menu).
- Rebuild and reinstall VST3 bundle.
- Create `ADDING_AN_INSTRUMENT.md` â€” complete guide for adding new synthesis voices (architecture, step-by-step checklist, anti-patterns).
- Merge `CLAUDE.md` into `AGENTS.md` for unified agent documentation.
- Synchronize `BACKLOG_VST.md` and `TODO.md`.

### Known issues to fix
- Perc1 needs clean re-implementation: do not recreate envelopes in `set_settings`, migrate to `DecayReleaseEnvelope`, make plock menu data-driven.

---

## 2026-05-16 â€” Mix Bus + plock fix + B8 + conditional params

**Build:** `20260516-205054`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- Per-instrument Mix Bus checkbox (route to Main Mix on/off, independent of Mute).
- Parameter Locks format expanded: `FIELD_COUNT` 12 â†’ 14 (fields 12 = clap_echo, 13 = algo).
- Fix root cause of lost plock echo: `set_special_param()` removed from `process()`, special params now propagated only at trigger time.
- Sound Panel hides inactive parameters per instrument via `InstrumentCapabilities`.
- New instrument B8 (TR-808 Bass Drum) with accent, snap, pitch drop, analog, release, click tone.

---

## 2026-05-15 â€” B8 click tone + plock B8 fix + anti-click

**Build:** `20260515-124610`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- Dedicated LP filter for B8 click tone (100â€“8000 Hz), plockable (field 17).
- Plock B8 fix: special params (accent/snap/pitch_drop/click_tone) stored in fields 14â€“17.
- Attack ramp 1.5 ms on B8 envelope + cold-start-only phase reset + DcBlocker + freq_smoother.
- Cross-DAW validation: plugin loads in Reaper, audio stable.
- Warnings reduced: 17 â†’ 0 (`cargo check --all-targets` clean).

---

## 2026-05-14 â€” DecayReleaseEnvelope + Snare 606 + Clap rework

**Build:** `20260514-220658`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- Bi-stage `DecayReleaseEnvelope` (decay + release) with persistent retrigger (`trigger_at_peak`).
- Hold phase between attack and decay for Snare/HiHat/OpenHH/Snare606.
- Analog-style continuity: no phase/filter/noise reset on retrigger.
- Kick: additive pitch sweep + freq smoother + DcBlocker.
- Clap rework: bandpass, snap transient, 4 bursts with irregular timing, Echo slider (0â€“3).
- New instrument: Snare 606 (TR-606 grey-box) with resonance, tone, snap.
- Fix crash on 11th voice: `IntRange` div-by-zero + index bounds + step mask hardcode.

---

## 2026-05-13 â€” Modular synthesis + groove + generators + UI polish

**Build:** `20260513-202946`  
**VST3 Class ID:** `DrumFlashPlugin1`

### Changes
- Modular `Voice` architecture with `set_algo()` and `set_special_param()`.
- Kick: 3 algos (Sine/Square/FM) + click transient.
- Snare: 3 algos (Synth/Noise/Layered) + snap param.
- New voices: Clap, Ride, Cymbal.
- Groove engine: Straight, Swing 16th, Shuffle, MPC Style.
- Push/pull per instrument, humanize per instrument.
- Pattern generators: Euclidean, Markov, Probabilistic.
- MIDI export to `Documents/Flash Drum/exports/`.
- UI: BoolParam â†’ checkbox, EnumParam â†’ combobox, algo â†’ named combobox.
- Sound panel per instrument with frequency, decay, volume, filter, algo, special params.

---

## 2026-05-11 â€” Grid persistence + Studio One save/restore fix

**Build:** `20260511-091259`  
**VST3 Class ID:** `DrumFlashPlugin1`  
**SHA-256:** `62AA5FCC445FEFDBC1E30196E614BCAED53A61C9F9EB2AB9BD5A4E1C5C510CEF`

### Changes
- Grid persisted via `pattern-v1` field (serialized from `SharedPattern`).
- Migration from legacy hidden params `st01`â€“`st16` to `pattern-v1`.
- Vendored `nih-plug` wrapper saves/restores state on both `IComponent` and `IEditController`.
- Studio One multi-out validated: `getRoutingInfo()` maps event input to main audio output.
- DAW sync validated: play, stop, tempo, repositionnement.
- Presets Rock, Funk, Disco.
- Mutes and solos per instrument.



