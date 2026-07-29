# LOGOTYPE — Flash Drum

Marque du header, zone haut-gauche. **Verrou : deux fontes, deux rôles.**

## Construction

Deux mots accolés, sans espace ni séparateur, alignés sur la **même ligne de base**.

| | FLASH | DRUM |
|---|---|---|
| Fonte | **Saira Condensed** | **Barlow Condensed** |
| Graisse | 500 (medium) | 700 (bold) |
| Corps | 24 pt | 24 pt |
| Interlettrage | +0.02em | +0.02em |
| Couleur | `#9fa2ac` (gris moyen) | `#ffffff` |
| Inclinaison | **oblique −11°**, pivot en bas à gauche | aucune (droit) |
| Marge | 4 pt à droite (compense l'oblique) | — |

Aucun point, aucun filet, aucun ornement : **le contraste des deux lettrages articule le nom**. FLASH file (étroit, léger, penché), DRUM encaisse (large, gras, droit).

À droite du logotype, à **11 pt**, la version en IBM Plex Mono 500 9.5 pt `#6b7280` : `v0.1.0 · <build>`.

## En egui

```rust
// FLASH — oblique par cisaillement du mesh de texte (egui n'a pas d'italique synthétique)
// Poser le galley dans un mesh, puis appliquer skew_x = tan(11°) ≈ 0.1944 sur chaque vertex,
// pivot = coin bas-gauche du galley :
//   v.pos.x -= (baseline_y - v.pos.y) * 0.1944;
let skew = (11.0_f32).to_radians().tan();
// DRUM — texte normal, aucune transformation.
```

Si le cisaillement pose problème, deux replis acceptables, dans cet ordre : embarquer un **vrai italique** de Saira Condensed (le rendu est plus propre qu'un faux oblique), ou **baker le logotype en bitmap** — il est fixe, donc un seul PNG suffit (voir `png/README.md`).

## Fontes à embarquer

- **Saira Condensed** 500 — SIL OFL, `fonts.google.com/specimen/Saira+Condensed`
- **Barlow Condensed** 700 — SIL OFL, `fonts.google.com/specimen/Barlow+Condensed`

Ces deux fontes servent **uniquement au logotype**. Toute l'UI reste en IBM Plex Sans / Mono.

## Interdits

- Ne pas incliner DRUM (l'opposition droit/penché est le principe même).
- Ne pas mettre d'orange dans le logotype : l'orange est réservé aux p-locks *link* de la grille.
- Ne pas ajouter d'espace entre les deux mots, ni de point, ni de barre.
- Ne pas uniformiser les deux fontes.

## Variantes écartées (conservées pour mémoire)

`Flash Drum - Logotype.html` documente les pistes explorées — plaque gravée, badge FD, rétroéclairé, empilé, et les déclinaisons de lettrage. La maquette peut les rejouer via `?wm=a`, `?wm=b`, `?wm=b1`, `?wm=b2`, `?wm=b3`.
