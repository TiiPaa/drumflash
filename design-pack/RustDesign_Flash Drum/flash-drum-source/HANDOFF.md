# HANDOFF — index d'intégration (à lire en premier)

Tout ce qu'il faut pour intégrer le design Skeuo dans le plugin egui. **Checklist de couverture** : chaque besoin → le fichier qui y répond.

| Besoin d'intégration | Fichier |
|---|---|
| **Cible visuelle** (à quoi ça doit ressembler) | `png/reference-full-ui.png` (UI complète 2×) + `index.html` dans un navigateur |
| **Cotes mesurées** (tailles, radius, couleurs, ombres — computed styles réels) | `SPEC-COMPUTED.md` ⭐ |
| **Radius de chaque élément** (dont pads = 4 px) | `RADIUS.md` |
| **Recettes matières Skeuo** (dégradés, keycaps, pads, LCD, états on/off) | `SKEUO.md` |
| **Code Rust du thème (à coller)** | `rust/skeuo_theme.rs` + `rust/skeuo_widgets.rs` + `rust/README.md` ⭐ |
| **Textures bakées** (pads, keycaps, LEDs, capuchon, switch, GENERATE, touches lane — PNG 4×) | `png/*.png` + `png/README.md` |
| **Squelette egui du layout** | `egui_layout_reference.rs` (layout seulement — le look Skeuo vient de `rust/` + textures) |
| **Layout & assemblage** (zones, invariants 🔒, comportements playhead/pages/follow) | `LAYOUT.md` |
| **Schémas de paramètres par moteur** (plages, steps, défauts, formats) | `assets/fd-data.js` (`schemaFor` / `oscFor` / `seqPlockSchema` / `seqConditions`) |
| **Comportements des widgets** (drag, p-lock popups, fusion, tooltips) | `assets/fd-core.js` + `CHANGES.md` |
| **Delta depuis le build 20260721** (quoi implémenter) | `CHANGES.md` |
| **Courbe ADSR** (formule exacte) | `DESIGN-SYSTEM.md` §5 |
| **Fonts** (IBM Plex Sans + Mono, à embarquer, OFL) | github.com/IBM/plex → `FontDefinitions` (poids : Sans 400–700, Mono 400–600) |
| Thèmes alternatifs conservés (non retenus) | `GRAPHITE.md`, `graphite-tokens.json`, `SKINS-PROPOSALS.md`, `assets/fd-graphite.css`, `assets/fd-skins-l214.css` |

## Pièges connus
- `egui_layout_reference.rs` et `DESIGN-SYSTEM.md` §1–3 datent du **thème plat** : leurs constantes de couleurs/radius sont périmées pour Skeuo → utiliser `SPEC-COMPUTED.md` / `RADIUS.md`.
- Le chiffre de pulses (fusion) et le contour playhead ne sont **pas** dans les textures de pads : à dessiner par-dessus (texte + `Stroke`).
- egui n'a pas de dégradés natifs : pads = images ; grands fonds = 2–3 rects superposés suffisent (la référence PNG fait foi).
