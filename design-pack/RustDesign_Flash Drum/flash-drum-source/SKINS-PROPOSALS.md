# Skins — 3 propositions (juillet 2026)

Trois nouveaux skins pour Flash Drum, **compatibles tels quels** avec le système de skins
runtime (mêmes tokens que `skins.json` — aucun changement de code, juste 3 entrées de plus
dans le registre). Palettes ancrées dans le **design system L214** (orange `#ff6028`,
anthracite `#201f22`, miaulet `#5f68d9`, beige `#f9f7f5`, + gammes yellow/green/pink).

Valeurs prêtes à coller : `skins-proposals.json`. Prévisualisation interactive :
`index.html?skin=paper|encre|miaulet` (ou le canvas `Flash Drum — Skins L214.html`).

---

## 1. Paper — studio clair

Le seul vrai pas de côté : un **skin clair**, rare dans les plugins audio.

- Fond beige chaud `#f9f7f5`, panneaux blancs, contrôles `anthracite-100`.
- Accent principal (`blue`) = **miaulet** `#5f68d9` ; hits violets, sélection `miaulet-100`.
- P-locks sound = **orange** `#ff6028` (plein), snap = `orange-700` ; seq-plocks = **pink-900** `#d9566c`.
- Texte = anthracite (`#201f22` → `#9e9e9d`).
- Attention portage egui : les "glows" n'existent pas (déjà le cas) ; sur fond clair les
  états actifs reposent uniquement sur le remplissage — déjà conforme à la règle
  « p-lock actif = cellule pleine ».

Usage : sessions de jour, projection/streaming, accessibilité.

## 2. Encre — anthracite + orange

Le skin « brand » : le dark actuel réchauffé aux neutres L214, avec **l'orange comme
accent principal** (playhead, hits, boutons, GENERATE).

- Surfaces = gamme anthracite (`#201f22`, `#282828`…), bordures `anthracite-500`.
- `blue` (rôle accent) = orange-500 ; les p-locks sound passent en **orange-300** `#ff9777`
  (plus clair que les hits — même logique que le couple blue/pl_link d'Ember).
- Seq-plocks = **periwinkle** `#828aea` (contraste froid sur base chaude).
- Mute = yellow-700, solo/loaded = green-700.

Usage : identité forte, chaleureuse, très différenciante face aux VST bleus/gris.

## 3. Miaulet — nuit periwinkle

Un dark teinté **violet periwinkle** (ni bleu Midnight, ni brun Ember).

- Surfaces bleu-violet profond (`#121324` → `#24264a`), bordures assorties.
- Accent = **miaulet-300** `#a0a6ee` (hits, pages, GENERATE) ; sélection lane = texte sombre
  sur accent clair.
- P-locks sound = orange `#ff6028` (claque sur la base froide) ; seq-plocks = **rose**
  `#d9566c` (le violet étant pris par l'accent).
- Jaunes chauds pour mute/drag-target.

Usage : successeur naturel de Midnight avec plus de personnalité.

---

## Correspondance L214 (traçabilité)

| Rôle skin | Paper | Encre | Miaulet |
|---|---|---|---|
| Accent (`blue`) | miaulet-500 | orange-500 | miaulet-300 |
| P-lock sound | orange-500 | orange-300 | orange-500 |
| Seq p-lock | pink-900 | miaulet-400 | pink-900 |
| Surfaces | beige-100 / blanc / anthracite-100 | anthracite-700/600 | miaulet-900 assombri |
| Texte | anthracite-700→300 | anthracite-100→400 | teinté periwinkle |
| Solo / succès | green-800 | green-700 | green-600 |
| Mute / warning | yellow-800 | yellow-700 | yellow-500 |

Typo et géométrie inchangées (IBM Plex, tokens de `TOKENS.md`) — les skins ne touchent
que la couleur, comme Dark/Midnight/Ember.
