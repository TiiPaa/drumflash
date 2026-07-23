# Changements UI depuis le build 20260721 (revue de juillet)

Delta à implémenter, dans l'ordre des zones. Réf. pixel : `index.html` (thème Skeuo par défaut).

## Header
- Groupe **Seq Mode** : segmented `Internal / Ext MIDI` + toggle **MIDI Pat** regroupés sous ce label (MIDI Pat grisé + éteint en Ext MIDI).
- **Auto-Edit déplacé dans Settings** — le header ne garde que Choke.
- **⚙ Settings** (droite) : popup 300px — Auto-Edit (switch), Default Analog, Global MIDI Channel, Skin.
- Tous les segmented (`Internal/Ext MIDI`, `Sound/Sequencer`, `Generator/Song`) : **moitiés symétriques** (largeur = libellé le plus long), texte centré.

## Page bar
- LED **rouge en coin haut-droit** du bouton de page en cours de lecture (plus de point sous le bouton).
- Len global : slider + valeur `N steps` + presets 16/32/48/64 + ×2 (inchangé).

## Grille
- Bouton de lane = **nom complet** (52px, tronqué, tooltip nom + moteur). Renommage live depuis Track ou le menu de lane.
- **Lane vide** : pads quasi éteints (14 %, pointillé), contrôles estompés, pastille **+N bleue** centrée → popup Add Module. Bouton nom : bordure pleine discrète (pas de pointillé).
- **Menu clic-droit de lane** : Name, Engine (groupé), **Copy Lane / Paste Lane / Randomize / Clear Lane**, Remove lane.
- **Hum** : mini-slider éditable, **bulle de valeur live** au survol et pendant le drag (« Hum 65 % »).
- **Push** : **mini-slider bipolaire ±50 ms** (centre = 0), même bulle live (« Push +12 ms »).
- **Len (par lane)** : **champ numérique** éditable clavier (saisie, ↑↓) borné 1–64.
- **Double-clic sur tout slider = valeur par défaut** (éditeur, header, générateur, popups).

## Patterns (sur sa propre plaque)
- `Patterns · Save · P1–P8 ←→ ⟳ Random · Clr │ Export MIDI · Drag MIDI`.
- **Random** = remplissage aléatoire du pattern courant, **dissocié du générateur**.
- **Clr** = style « danger silencieux » (gris, rouge au survol), éloigné des slots.
- Slots : pastille verte coin haut-droit = occupé (4px).

## Generator | Song (panneau partagé, largeur fixe)
- Generator sur 2 rangées : `Type │ A ↔ Mix ↔ B` puis `Dens · Var — GENERATE`.
- Song inchangé (blocs compacts 62px, ×N, scroll horizontal).

## Sound Editor (droite)
- Onglets **Sound / Track** pleine largeur ; en-tête `Sound Editor · Slot N – nom`.
- **Plus de rangée de sélecteurs d'instruments** (sélection via la grille uniquement).
- Onglet Track : Name (6 car.), Instrument, Routing (Main + Out No Aux/1-14), MIDI (Channel, Note), Sequencing (Length + Lock).
- Dropdowns : **largeur fixe 170px**, alignés à droite ; menus plafonnés 220px (scroll) et **s'ouvrent vers le haut** près du bord bas.
- Mode **Notes** : flèches **◂ ▸** = ±1 demi-ton exact, groupe centré.
- Graphe ADSR : plus de lettres A/D/R dans le graphe (légende du bas uniquement).

## Popups p-lock (état ≠ menu)
- **Sound, pas sans p-lock** : « No plock on this step » + **Link to Global** / **Snapshot Current Settings** / **Paste Plock**.
- **Sound, p-lock existant** : indicateur **Linked / Full Snapshot** + sélecteur de mode, sliders par champ (Volume en tête), Copy / Paste / **Clear**.
- **Sequencer** : popup unique — `Mode Inactive/Active` (violet) · Probability (%) · Stutter (1–16, ×N) · **Condition en grille 3 colonnes** (Always · 1st loop only · Not 1st loop · 1/2 · 2/2 · 1/3 · 2/3 · 3/3 · 1/4 · 2/4 · 3/4 · 4/4 — pilules, sélection violette) · bouton pleine largeur **Create Seq Plock** (inactif) / **Clear Seq Plock** (actif). Pas de micro-timing.

## Divers
- Poignée de slider : plus de transition sur la position (**suivi instantané du drag**, le bug de décalage est corrigé). En Skeuo le capuchon strié (12×19) est **toujours visible** ; l'apparition au survol ne concerne que le thème plat.
- Proposition de disposition « plaque unique » (page bar + grille + p-lock bar dans le panneau séquenceur) : `index.html?layout=b` — à valider.
