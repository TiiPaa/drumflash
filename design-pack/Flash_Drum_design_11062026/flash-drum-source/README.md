# Flash Drum — Pack de design (handoff dev)

Maquette de référence **fonctionnelle** de l'interface Flash Drum (plugin VST3, moteur Rust + UI egui).
Sert de **spec visuelle et comportementale** à reproduire en egui — ce n'est pas du code de production,
mais tout est dimensionné, coloré et câblé pour lever les ambiguïtés.

Fenêtre cible : **1480 × 800 px, fixe** (échelle 1.0 → 1px maquette = 1pt egui).

---

## Contenu

| Fichier | Rôle |
|---|---|
| `index.html` | La maquette interactive. Ouvrir dans un navigateur Chromium. |
| `assets/fd-data.js` | **Source de vérité données** : registre de moteurs (`ENGINES`), schémas de paramètres par moteur, lanes, pattern (64 pas), transport, p-locks. |
| `assets/fd-core.js` | Rendu + widgets : sliders/knobs/selects/switch, dessin ADSR, séquenceur paginé, menus p-lock & lane, export MIDI. |
| `assets/fd-base.css` | Tous les tokens visuels (palette, tailles, états) — référence pixel. |
| `DESIGN-SYSTEM.md` | **Tokens** : couleurs (hex → `Color32`), typo, rayons, système de contrôles coordonné, états, ADSR, états de cellule p-lock, norme de casse. |
| `LAYOUT.md` | **Architecture & layout** : invariants 🔒, structure 2 colonnes, séquenceur modulaire, éditeur dynamique, structs Rust suggérées. |

> Commence par `DESIGN-SYSTEM.md` (le quoi) puis `LAYOUT.md` (le où / comment). Le code JS donne les
> valeurs exactes (plages, pas, défauts) quand un doute subsiste.

---

## Lancer la maquette

Ouvrir `index.html` dans un navigateur (Chrome/Edge de préférence pour le drag-out MIDI natif).
Aucune dépendance à installer ; la police IBM Plex est chargée depuis Google Fonts (connexion requise
pour le rendu typographique exact — sinon fallback système).

À tester pour comprendre les comportements :
- **Pas** : clic = on/off ; **clic droit** = menu p-lock (Sound) ou seq-plock (Sequencer).
- **P-Lock Mode** (Sound / Sequencer) : modes d'affichage exclusifs (couleurs ≠, voir DESIGN-SYSTEM §6).
- **Lanes** : clic-droit sur le nom = assigner un moteur / renommer / retirer ; poignée ⠿ = drag pour réordonner ; **+ Add module**.
- **Éditeur** (droite) : change selon le moteur de la lane (synth ≠ sample ≠ MIDI).
- **Pages 1-4** + Len (1–64) + Follow ; **Generator | Song** (panneau partagé) ; **Export / Drag MIDI**.

---

## Architecture (résumé pour le portage egui)

**Tout est piloté par la donnée — aucun paramètre codé en dur dans l'UI.**

```rust
// Registre de moteurs : type -> (label, groupe, schéma)
struct Engine { label: &str, group: EngineGroup, schema: fn() -> Vec<Section> }
// ENGINES: synth-kick/snare/tom/hat/cymbal/clap/perc, sample, samplefx, midi

// Un contrôle
enum CtlKind { Slider, Freq, Select, Switch }
struct ParamSpec { label, key, kind, min, max, step, default, unit, options }
struct Section { title, items: Vec<ParamSpec>, adsr: bool, is_volume: bool }

// Une lane = slot assignable (4 au départ, cap = 14, réordonnable)
struct Lane { id, tag, name, engine: Option<EngineType>,
              pattern: Pattern, params: ParamStore, settings: LaneSettings }

// Pattern : 64 pas, paginé par 16
struct Step { hit: bool, plock: PlockKind /*none|link|snapshot*/, seq: bool }
// + fusion: Vec<{start, len, pulses}>

fn schema_for_engine(engine) -> Vec<Section>
fn assign_engine(lane, engine)              // remplace + reset params
fn add_lane(engine) / remove_lane(id) / move_lane(id, to_index)   // cap 14, min 1
```

L'éditeur boucle simplement sur `schema_for_engine(lane.engine)` et rend chaque `Section`/`ParamSpec`
avec le widget correspondant — ajouter un moteur ou un paramètre = éditer la donnée, jamais le layout.

Détails complets (formule ADSR, mapping MIDI/GM, états de cellule, comportement playhead/pages/Follow) :
voir `DESIGN-SYSTEM.md` et `LAYOUT.md`.
