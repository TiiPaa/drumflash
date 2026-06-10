# Assets graphiques nécessaires

## Icônes (16×16 à 24×24, style minimaliste)

### Transport
- [ ] Play (triangle)
- [ ] Stop (carré)
- [ ] Pause (deux barres)
- [ ] Record (cercle rouge)

### Actions
- [ ] Save (disquette)
- [ ] Load (dossier ou flèche vers le haut)
- [ ] Generate (dés)
- [ ] Random (shuffle/flèches croisées)
- [ ] Clear (croix ou gomme)
- [ ] Copy (deux documents)
- [ ] Paste (presse-papier)
- [ ] Delete (poubelle)

### MIDI
- [ ] Export MIDI (note musicale + flèche)
- [ ] Drag MIDI (main + note)

### État
- [ ] Link (chaîne)
- [ ] Snapshot (appareil photo)
- [ ] Locked (cadenas fermé)
- [ ] Unlocked (cadenas ouvert)
- [ ] Mute (haut-parleur barré)
- [ ] Solo (écouteur)

### Navigation
- [ ] Previous (flèche gauche)
- [ ] Next (flèche droite)
- [ ] First (double flèche gauche)
- [ ] Last (double flèche droite)

### Instruments (miniatures 12×12 pour la grille)
- [ ] Kick (peau de grosse caisse)
- [ ] Snare (caisse claire)
- [ ] HiHat (charleston)
- [ ] OpenHH (charleston ouverte)
- [ ] Tom (tom)
- [ ] Clap (clap)
- [ ] Ride (ride)
- [ ] Cymbal (crash)

## Widgets custom

### Step Cell
- [ ] État vide (fond)
- [ ] État actif (fond + glow)
- [ ] État playback (pulse/bordure)
- [ ] État plock link (indicateur ambre)
- [ ] État plock snapshot (indicateur rouge)
- [ ] État fusion (bordure + groupe)
- [ ] État sélection (bordure bleue + highlight)

### Slider
- [ ] Track (ligne de fond)
- [ ] Fill (partie remplie)
- [ ] Thumb (poignée)
- [ ] Hover state
- [ ] Active state

### Toggle / Switch
- [ ] Off (fond gris, thumb à gauche)
- [ ] On (fond bleu/vert, thumb à droite)
- [ ] Transition

### Button
- [ ] Default (fond panel, bordure subtile)
- [ ] Hover (fond légèrement plus clair)
- [ ] Active/Pressed (fond encore plus clair)
- [ ] Disabled (fond grisé, texte faint)

### Combo Box
- [ ] Closed (fond panel, flèche bas)
- [ ] Open (liste déroulante)
- [ ] Hover item
- [ ] Selected item

### LED
- [ ] Off (cercle sombre)
- [ ] On (cercle coloré + glow)
- [ ] Blink (animation)

## Backgrounds & Panels

- [ ] Fond global (subtle texture ou gradient très léger)
- [ ] Panel principal (avec ombre portée subtile)
- [ ] Panel secondaire
- [ ] Separator lines
- [ ] Scrollbar custom

## Visualisations

- [ ] Enveloppe amplitude (ligne avec fill)
- [ ] Enveloppe filtre (ligne avec fill)
- [ ] Grid background (subtle lignes de beat)

## Animations (specs)

- [ ] Step playback pulse (timing, easing)
- [ ] Plock creation feedback
- [ ] Fusion selection blink
- [ ] Button press feedback
- [ ] Slider drag feedback
- [ ] Page change transition

## Notes

- Tous les assets doivent être **vectoriels** ou dessinables avec des primitives (rect, circle, line)
- Pas d'images raster (PNG/JPG) — egui préfère le dessin procédural
- Palette : voir DESIGN-BRIEF.md section "Palette de couleurs"
- Style : minimaliste, professionnel, inspiré hardware drum machines
