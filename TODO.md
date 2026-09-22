> Ce fichier ne contient que ce qui reste **a faire ou en cours**.
> Tout ce qui est termine vit dans [DONE.md](DONE.md).

## Nouvelles tâches — session 2026-08-26

> Ticketisation des notes utilisateur (`docs/notes/notes.txt`). Priorité : quick wins d'abord, features moyennes ensuite, gros chantier en fin.

### P1 — Quick wins
- [ ] [209] **Equilibrer les volumes par defaut de tous les instruments** - les charleys sont generalement trop forts. Analyser les defauts du registre (`sound_settings_default[2]` = volume, et le `VoiceSettings::<voix>()` correspondant) sur les **25 voix**, mesurer le niveau reellement produit par chacune a ses reglages d'usine, puis proposer un jeu de volumes coherent. **Mesure d'abord** : un volume ne dit rien du niveau percu, il depend de l'enveloppe, du filtre et de la saturation de la voix - c'est ce qui a rendu [207] concluant. Precedent : [169] avait remonte le clap de 0,7 a 1,0 a l'oreille, sans mesure. **Touche le son de toutes les voix, donc arbitrage utilisateur avant d'appliquer**, et les sessions existantes ne doivent pas etre reecrites (meme regle que [193] [203] [204]).

## Nouvelles tâches — session 2026-08-14

### P2/P3 — Gros chantiers
- [ ] [166] **REPRENDRE ICI** — **Mixer le stutter avec les cellules fusionnées** — étudier attentivement l'interaction stutter × fusion (actuellement exclusifs) : sémantique temporelle, rendu audio, export MIDI [158], UI/plocks. **Bien étudier le point avant de coder.**

### Idées notées (2026-08-16)
- [ ] [173] **Presets d'usine de départ** — composer et embarquer les premiers presets factory (instruments/patterns/grids/songs) via l'outil « Export factory (dev) » + `assets/presets/` + `factory_presets.rs`.

---

## Nouvelles tâches — session 2026-07-29

### Features moyennes (P2)
- [ ] [146] **Enveloppes exponentielles négatives** — pour des attaques plus claquantes (courbe d'attaque exp inversée, par voix ou global ?).

### Grosses features (P2/P3)
- [ ] [152] **Instrument Ambiant** — voix jouant des bouts de samples d'ambiances noisy avec offset aléatoire (dépend de l'infra sampler [83] ?).

---

## [SKEUO] Refonte visuelle « hardware » (pack designer RustDesign_Flash Drum, 2026-07-23)

> Pack de référence : `design-pack/RustDesign_Flash Drum/flash-drum-source/`.
> Docs autoritaires : `HANDOFF.md` (index), `SPEC-COMPUTED.md` ⭐ (cotes mesurées), `RADIUS.md`, `SKEUO.md` (recettes), `rust/skeuo_theme.rs` + `rust/skeuo_widgets.rs` ⭐ (code egui clé en main), `png/` (textures + `reference-full-ui.png` = cible).
> Stratégie : porter les 2 fichiers Rust du designer comme module `skeuo` (theme + widgets), garder notre layout, remplacer le *rendu* de chaque élément par ses fonctions (`pad`, `keycap`, `generate_button`, `hslider`, `led`, `lcd_frame`, `well`). Le module « ne fait que le look ».

### Ordre proposé (1 build testable par étape)

1. SK-1 (débloquer les pads) → 2. SK-2/SK-3 (palette + fonds) → 3. SK-4/SK-5 (keycaps) → 4. SK-6..SK-10 (contrôles) → 5. SK-11..SK-16 (comportement).

---

## Fonctionnalites P3 (Avancees / Complexes)
- [ ] [69] Creer un instrument percussif a base de wavetables — phase recherche et prototypage (Complexité: Élevée, 2-4 semaines, P3)
- [~] [83] **Instruments sampler TR-606 multisamplé** — **1er instrument livré : BD6smp (build 20260802-160117)**
  - [x] Nouveau type de voix "Sampler" (multisample) — `synthesis/sample_bank.rs` + `synthesis/bd606.rs`
  - [x] Sélection aléatoire du layer à chaque trigger (sans répétition immédiate) pour simuler l'imperfection analogique
  - [x] Chargement de samples WAV embarqués (`include_bytes!` + `OnceLock`, zéro alloc audio thread)
  - [~] Étendre aux autres instruments de la 606 (SD, HH, …) — même infra, il suffit d'ajouter les WAV
    - [x] SD6smp (Snare 606)
    - [x] CH6smp (Closed Hi-hat 606, note MIDI 42) — build 20260804-170126, `wav/CH.wav` → `assets/ch606.wav`
    - [x] OH6smp (Open Hi-hat 606, note MIDI 46) - livre par [208] (build 20260909-144136)
    - [ ] Cymbal, ... restants
- [ ] [84] **Instruments sampler Yamaha RX11**
  - Même architecture sampler que [83] avec le kit RX11
  - 4 layers par son pour l'effet analog random
  - Dépend de [83] (infrastructure sampler)
  - **Complexité : Moyenne, 2-3 semaines, P3**

## Nouveaux éléments (À prioriser)
- [ ] [56] Ajouter une percussion de type Tom Simmons (Complexité: Moyenne, 3-5 jours)
  - Créer un nouveau module de synthèse
  - Ajouter l'instrument dans le registre des instruments
  - Créer les paramètres spécifiques et l'interface utilisateur
  - Intégrer dans le système de mixage et de sortie audio

## [100] Redesign UI complet (design pack 2026-06-11) — EN COURS

> **Livrable designer** : `design-pack/Flash_Drum_design_11062026/flash-drum-source/`
> Fichiers clés : `DESIGN-SYSTEM.md` (tokens), `LAYOUT.md` (architecture), `assets/fd-data.js` (schémas moteurs)

### Architecture (invariants du design)

- **Système de lanes modulaires** : 4 lanes au départ (BD/SD/HH/TOM), ajoutables jusqu'à 14, réordonnables par drag
- **Registre de moteurs** : Synth (kick/snare/tom/hat/cymbal/clap/perc), Sample, Sample FX, MIDI Out
- **Éditeur dynamique** : contenu reconstruit selon le moteur assigné, aucun paramètre codé en dur
- **Séparation données ↔ rendu** : ajouter instrument/paramètre = éditer une donnée

### Notes

- **Volume** : range -60 dB à +6 dB (actuellement 0..2 linéaire, à convertir)
- **Norme de casse** : Title Case partout
- **Pas de gradients** : aplats + ombres/glow subtils
- **Contrainte egui** : tout en primitives (rect, cercle, texte), pas d'images

---

## Investigation & Features (A prioriser)
- [ ] [94] **Ajouter un parametre pitch LFO sur les Toms** (P2, Synthese)
  - Intensite, Rate, Type de LFO (sine/triangle/square/saw), arrivee progressive
  - Permet des variations de hauteur dynamiques sur les toms
  - Complexite : Moyenne, 3-5 jours
- [ ] [95] **Ajouter un instrument de type MIDI (avec MIDI out)** (P2/P3, Architecture MIDI)
  - Voix virtuelle qui envoie des NoteOn/NoteOff MIDI sur une sortie MIDI externe
  - Pas de synthese interne, juste du routage MIDI
  - Permet de declencher des instruments externes depuis le sequencer
  - Complexite : Moyenne-Elevee, 1-2 semaines

## Plan d'action — Audit code review 2026-07-18

### [AUDIT-CR-4] P3 — Dette structurelle (à planifier)
- [ ] **[BUG-LANE-DESYNC]** Décalage de tête de lecture entre lanes au changement Song/Pattern + changement de pattern — en attente de l'isolation du déclencheur par l'utilisateur.
