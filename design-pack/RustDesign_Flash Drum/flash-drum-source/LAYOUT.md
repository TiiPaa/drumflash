# Flash Drum — Layout & assemblage (egui) · v2

> Aligné sur l'alpha 0.1.0 + design pack dev. Décrit *où vont les choses* et *quels
> paramètres expose chaque instrument*. Prérequis : toolkit de `DESIGN-SYSTEM.md`.
> Réf. pixel : `variation-a.html`.
>
> **Deux niveaux :** 🔒 **INVARIANT** = architecture, à respecter dès maintenant (coder
> contre = travail jeté). 🔓 **AJUSTABLE** = cotes/détail, peut bouger sans rien casser.
> **Fenêtre fixe 1480 × 800.**

---

## 0. Invariants en un coup d'œil

| 🔒 | Décision d'architecture |
|----|--------------------------|
| 1 | **2 colonnes** : gauche = séquenceur + page-bar + p-lock-bar + patterns + générateur (+ Song arranger en bas si activé) ; droite = **Sound Editor** (édition de synthèse par instrument). |
| 2 | **Éditeur dynamique** : son contenu se reconstruit selon l'instrument. **Aucun paramètre codé en dur** — on parcourt `schema_for(cat)` (donnée). |
| 3 | **Sélection d'instrument unique partagée** : ligne du séquenceur ⇄ onglets de l'éditeur ⇄ menu p-lock pointent le même `selected`. |
| 4 | **Pattern = 64 pas, paginé par 16** (4 pages). `len ∈ [1,64]` global + par lane ; pages = `ceil(len/16)` ; pas hors-longueur grisés. |
| 5 | **P-locks = cœur produit.** Deux modes (`Sound` / `Sequencer`), états visuels par cellule (DESIGN-SYSTEM §6), édition par **menu clic-droit**. Modèle : `hits[64]`, `plock[64]∈{none,link,snapshot}`, `seq[64]`, `fusion[]`. |
| 6 | **Séparation données ↔ rendu** : ajouter instrument/paramètre = éditer une donnée, pas le layout. |
| 7 | **Système modulaire de lanes.** Démarre avec **4 lanes de base** (BD/SD/HH/TOM) ; l'utilisateur **ajoute** des modules (jusqu'à un **cap de 14**), les **retire**, et les **réordonne verticalement** (drag par la poignée). Une lane = `{id, tag, name, engine}` ; `engine` pointe un **registre de moteurs** (`ENGINES`). Réassigner = changer `engine` + reset params. |
| 8 | **Layout 100 % fixe** (sauf le scroll interne du Sound Editor, *by design*). La zone séquenceur **réserve toujours la hauteur de 14 lanes** (`min-height` du corps), donc page-bar, p-lock-bar, patterns, générateur et Song restent à position fixe quel que soit le nombre de lanes actives. Aucun bloc ne se redimensionne. |

---

## 1. Layout général (1480 × 800) 🔒 structure / 🔓 cotes

```
┌ HEADER (full width, h44) ─────────────────────────────────────────────┐
│ FLASH DRUM build │ ▶ ● │ Master │ Swing │ Groove ▾ │ Seq Choke Auto Song │
├──────────────────────────────────────────────┬───────────────────────┤
│ COLONNE GAUCHE (flex, ~910)                    │ COLONNE DROITE (568)   │
│ ┌ page-bar : Page 1-4 · Follow · Len+presets ┐ │ Sound Editor + tabs    │
│ ┌ SÉQUENCEUR (13 × 16 visibles) ────────────┐ │ [Sound|Pattern|Song]   │
│ p-lock mode (Sound/Seq) · Fusion             │ onglets instruments(13)│
│ Patterns [P:n S:n] · Save · P1-8 · Clr       │ ┌ éditeur (scroll) ────┐│
│ Générateur (presets + A/B/Mix/Dens/Var + GEN)│ │ Volume·OSC·ENV·FILTER ││
│                                               │ │ ·SAT·OUTPUT          ││
└──────────────────────────────────────────────┴───────────────────────┘
```

> ⚠️ Le brief ASCII donnait séquenceur ~580 / éditeur ~850 ; l'alpha réelle (et cette
> maquette) font l'inverse — **séquenceur large à gauche, éditeur ~568 à droite**. Choisi
> car la grille est l'élément principal. 🔓 Les largeurs exactes restent ajustables.

| Zone | Valeur | |
|---|---|---|
| Header | h44, padding 0×14, bord bas LINE, fond PANEL | 🔓 |
| Colonne gauche | flex, padding 11×14, gap vertical 10, bord droit LINE | 🔒 contenu / 🔓 cotes |
| Colonne droite | 568, fond PANEL | 🔒 = éditeur / 🔓 largeur |
| Panneau | PANEL, bord LINE, r9 | 🔓 |

---

## 2. Header (gauche → droite)

`FLASH DRUM` `build` │ **▶/■** **●**(rec) │ Master(slider dB) │ Swing(slider %) │ Groove(select) │ **Seq: Internal | Ext MIDI** (segmented) │ **Choke Auto-Edit** (toggles LED).

- **Seq source** : `Internal` = le séquenceur interne joue la grille ; `Ext MIDI` = suit le MIDI entrant du DAW (le plugin ne séquence plus, il est joué par l'hôte). Segmented explicite avec LED, pas un simple toggle « Seq ».
- Transport ▶/● = **ajout** (le brief listait son absence comme problème). Play vert actif, Rec rouge actif.
- 🔒 présence transport + clock source + toggles ; 🔓 ordre/cotes.

---

## 3. Page / Length bar 🔒 (fonctionnalité réintroduite)

`Page [1][2][3][4]` · `Follow ON/OFF` · `Len:` slider(1–64) · presets `16 32 48 64` · `×2`.

- Nb de pages = `ceil(len/16)`, reconstruit quand `len` change. Page active = fond BLUE.
- **LED rouge** sous le bouton de la page en lecture.
- **Follow** : ON = la page suit le playhead en lecture ; OFF = on reste sur la page éditée.
- **×2** : double `len` (et duplique le contenu) ; **désactivé si len > 32**.
- Presets surlignés quand `len` == valeur.

---

## 4. Séquenceur 🔒 grille / 🔓 cotes

1 en-tête + **lanes dynamiques** : on démarre avec **4 lanes de base** (BD/SD/HH/TOM), l'utilisateur en **ajoute** jusqu'à un **cap de 14**, en **retire**, et les **réordonne**. Chaque ligne = une lane portant un moteur. `tag` court dans le bouton de nom, `name` complet en tooltip.

### Lane assignable / modulaire 🔒
- **Poignée ⠀⠿** à gauche = **drag pour réordonner** verticalement (indicateur bleu au survol d'une cible).
- **Clic gauche** sur le nom = sélectionne la lane (édition + grille).
- **Clic droit** sur le nom = menu : champ **Name**, **moteurs groupés** (Synth / Sampler / MIDI), **Remove lane** (gardé ≥ 1 lane).
- **Rangée « + Add module »** sous les lanes : ouvre le sélecteur de moteur groupé et **ajoute** une lane (désactivée au cap de 14).
- Le moteur est aussi changeable depuis le sélecteur **Engine ▾** de l'en-tête éditeur (§6).

### Une ligne
`[Nom 34]  [Vol mini 56]  [M][S][T 17px]  [16 pas · flex]  [Hum 44][Push 44][Len 44]`

| Élément | Valeur |
|---|---|
| Hauteur ligne / en-tête | 24 / 16 |
| Gap vertical / horizontal | 3 / 7 |
| Tags M/S/T | 17×17 (M=mute amber, S=solo green, **T=trig/audition** blue) |
| Nom de lane | bouton 34px, `tag` mono ; clic = sélection, clic-droit = menu d'assignation |
| Pas | h21 r4, 16 colonnes flex, gap 3 — états : DESIGN-SYSTEM §6 |
| Hum | mini-slider grisé · Push | `±N ms` (0 = INK3) · Len | nombre |

### Interactions 🔒
- **Clic gauche** : toggle hit. **Clic droit** : menu p-lock (§5). **Maj+glisser** : sélection pour Fusion.
- Mode Sequencer actif → clic gère le seq-plock (violet).
- Pages : seuls les 16 pas de la page courante sont visibles ; pas ≥ `len` grisés (opacité .28, non cliquables).

### Playhead 🔒 comportement
`ms_par_pas = 60000 / bpm / 4` (double-croches ; tempo = hôte VST). Avance `current_step` sur 0..len, `request_repaint()`. Si **Follow**, change de page automatiquement. Rendu : DESIGN-SYSTEM §6.

---

## 5. P-lock — modes & menu contextuel 🔒🔒🔒

> Le point le plus structurant. À traiter comme un **système de données**, pas du layout.

**Toggle de mode** (sous la grille) : segmented `Sound | Sequencer`. Onglet actif Sound = orange (PL_LINK), Sequencer = violet (SEQPL). `Fusion` = bouton séparé + hint « Maj+glisser ».

**Menu clic-droit** (largeur ~236, P_ACTIVE, r9, ombre) :

### Mode Sound
```
● Plock <Instrument>                        Step N
─ Mode: [Link to global ▾ / Snapshot] ───────────
Volume       [────●──]  3.2   ↺      ← EN PREMIER
Frequency    [Notes][──●─]  175  ↺
Click Level  [──●───]  0.51    ↺
Attack / Decay / Decay Curve / Release …   ↺
─────────────────────────────────────────────────
[Copy Plock]            [Paste Plock]
```
Les lignes sont générées **depuis le schéma de l'instrument** (Volume forcé en tête), chacune avec un bouton **↺ undo**. Le `Mode` choisit link-vers-global vs snapshot figé.

### Mode Sequencer
`Probability (0–100%)` · `Stutter (0–8)` · `Condition (Always / 1:2 / 2:2 / 1:4 / Fill / !Fill)` · `Micro-timing (±50 ms)` — chacun avec ↺.

---

## 6. Sound Editor (colonne droite) 🔒 dynamique / 🔓 cotes

En-tête : `Sound Editor` + nom de l'instrument + sélecteur **Engine ▾** (à droite) pour réassigner le moteur de la lane courante. **Pas d'onglets** (le panneau édite uniquement
la synthèse de l'instrument courant ; le pattern s'édite dans la grille à gauche, le song en
bas à gauche). Puis **onglets instruments** (grille 14). Puis zone **scroll**.

### Sections (single column : label · slider · valeur)
`schema_for_engine(engine)` → le contenu dépend du moteur. Le synth donne **Volume** (en tête, sans titre) · **OSC** (varie par cat) · **ENV** (+ ADSR inline) · **FILTER** · **SAT** · **OUTPUT** ; les moteurs Sample / MIDI ont des sections différentes (voir ci-dessous).

### Moteurs (registre `ENGINES`)
| Groupe | Moteur | Sections spécifiques |
|---|---|---|
| Synth | Kick | OSC: Frequency(+Notes), Click Level, Click Type, Algorithm · FILTER LP |
| Synth | Tom | OSC: Frequency(+Notes), Pitch Bend, Click Level, Algorithm · LP |
| Synth | Snare | OSC: Tone Freq, Noise Mix, Snap, Body · LP |
| Synth | Hat / Cymbal | OSC: Tone, Metal Decay, Color, Shimmer · **HP** |
| Synth | Clap | OSC: Pitch(+Notes), Spread, Count · LP |
| Synth | Perc | OSC: Pitch(+Notes), Decay, Noise · LP |
| Sampler | **Sample** | SAMPLE: Sample, Start, End, Reverse, Loop · PITCH: Tune, Fine, Key Track · ENV · LP · SAT · OUTPUT |
| Sampler | **Sample FX** | SAMPLE: Sample, Start, Choke · SHAPE: Decay, Tone, Drive · LP · OUTPUT |
| MIDI | **MIDI Out** | MIDI: Channel, Note, Velocity, Gate · MOD: CC Number, CC Value *(pas d'ENV/FILTER/SAT)* |

### Song arranger 🔓
Le **Générateur** et le **Song** partagent **un seul panneau** en bas à gauche (mutuellement exclusifs) : un segmented **[Generator | Song]** bascule l'un *à la place* de l'autre — on n'utilise jamais les deux en même temps, ce qui économise la hauteur. L'onglet **Song** affiche la chaîne de blocs `pattern × répétitions` (clic sur le nom = cycle, stépper +/− répétitions, retrait, `+` pour ajouter), un résumé `n blocks · n patterns · n bars`, et un toggle **Song Enabled** (lecture en mode morceau).

### Struct de données (source de vérité) — modulaire
```rust
enum CtlKind { Slider, Freq, Select, Switch }
struct ParamSpec { label:&str, key:&str, kind:CtlKind,
                   min:f32, max:f32, step:f32, default:f32, unit:&str, options:&[&str] }
struct Section { title:&str, items:&[ParamSpec], adsr:bool, is_volume:bool }

// Registre de moteurs : type -> (label, groupe, schéma)
struct Engine { label:&str, group:EngineGroup, schema: fn() -> Vec<Section> }
// ENGINES: Map<EngineType, Engine>   (synth-kick, …, sample, samplefx, midi)

// Lane = slot assignable (liste dynamique : 4 au départ, cap à 14)
struct Lane { id:LaneId, tag:&str, name:String, engine:Option<EngineType>,
              pattern:Pattern, params:ParamStore, settings:LaneSettings }
// fn schema_for_engine(engine) -> Vec<Section>
// fn assign_engine(lane, engine)   // remplace engine + reset params
// fn add_lane(engine) / remove_lane(id) / move_lane(id, to_index)   // cap = 14, min = 1 aux défauts
```
> Plages/pas/défauts + définition complète des moteurs : `assets/fd-data.js` → `ENGINES` / `schemaForEngine` / `oscFor` / `defaultParams` / `assignEngine`.

---

## 7. Générateur & Patterns (bas gauche)

- **Patterns** (barre du pattern courant) : `Patterns` · Save · slots P1–P8 (point vert si occupé, BLUE si actif) · Clr · *(à droite)* **Export MIDI** + **Drag MIDI**.
- **Generator / Song** : un seul panneau partagé, segmented **[Generator | Song]** (voir §6). Generator = combo type + **A**/**B** (morphing de styles) + `Mix` + `Dens` + `Var` + `⟳ Random` + **GENERATE**. Song = arrangeur de chaîne de patterns.

### MIDI export / drag-out 🔒 comportement
Le pattern courant → fichier MIDI type-0, **canal 10** (batterie), division 96 PPQ, pas = double-croche (24 ticks). Mapping GM : BD 36, SD 38, HH 42, OH 46, T1 48, T2 45, T3 41, CL 39, RD 51, CY 49, S6 40, B8 35, P1 37. Vélocité 100, **120 si p-lock**. Lanes muettes exclues, longueur = `len`. Côté egui/nih-plug : sérialiser ces octets et exposer un **drag-source** (la poignée « Drag MIDI ») + une action **Export** (écriture fichier). Réf. implémentation : `assets/fd-core.js` → `patternToMidi()`.

*Réf. pixel : `variation-a.html`.*
