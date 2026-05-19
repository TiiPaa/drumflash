# Test du plugin

## But de ce document

Decrire une procedure de test compatible avec l'etat reel du plugin Rust actuel.

Il ne faut pas l'utiliser comme preuve de parite avec le PoC web. Cette parite n'est pas atteinte aujourd'hui.

## Pre-conditions

- le plugin compile en `release`
- le bundle VST3 est genere dans `drum-pattern-vst/build/`
- le DAW ou l'hote VST rescanne les plugins

## Ce qu'il est raisonnable de tester maintenant

- le plugin se charge sans crash
- l'interface affiche le numero de build
- une sortie Main Mix stereo est produite
- le sequenceur interne declenche bien des evenements
- les patterns Rock, Funk et Disco se chargent
- la grille 16x7 est editable en temps reel
- les mutes et solos par instrument repondent
- la sync DAW suit play, stop, tempo et repositionnement
- les sorties multi-out Studio One sont activables

## Ce qui reste hors validation V1

- vraie sortie MIDI plugin
- export MIDI plugin
- equivalence sonore complete avec la web app
- validation dans d'autres DAWs que Studio One

## Procedure de test minimale

1. Compiler et installer:

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
.\build.ps1 -Install
```

2. Dans Studio One:
- rescanner les plugins
- charger `Drum Flash`
- ouvrir l'interface et noter le build affiche
- lancer la lecture du projet
- verifier qu'un pattern audio simple est audible
- ouvrir le menu de sorties instrument et activer les sorties separees

## Regression multi-out Studio One

Cette procedure doit etre relancee apres toute modification de:
- `drum-pattern-vst/src/lib.rs`
- `drum-pattern-vst/src/sequencer/`
- `drum-pattern-vst/src/synthesis/`
- `drum-pattern-vst/vendor/nih-plug/src/wrapper/vst3/wrapper.rs`
- `drum-pattern-vst/build.ps1`

### Preparation

1. Fermer Studio One.
2. Compiler et installer:

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
.\build.ps1 -Install
```

3. Noter le build ID affiche par le script.
4. Ouvrir Studio One et rescanner les plugins VST3 si necessaire.
5. Verifier qu'une seule copie systeme est presente:

```powershell
Get-ChildItem "C:\Program Files\Common Files\VST3" -Recurse -Filter "drum-pattern-vst.vst3"
```

Resultat attendu: uniquement le bundle
`C:\Program Files\Common Files\VST3\drum-pattern-vst.vst3` et son binaire interne.

### Projet Studio One neuf

1. Creer un projet vide.
2. Ajouter `Drum Flash` comme instrument.
3. Ouvrir l'interface du plugin.
4. Verifier que le build ID affiche correspond au build installe.
5. Lancer la lecture.
6. Verifier que le Main Mix est audible avant activation des sorties separees.
7. Dans le panneau Instruments/Console, ouvrir la liste des sorties du plugin.
8. Activer toutes les sorties:
   - Main Mix
   - Kick
   - Snare
   - Hi-Hat
   - Open HH
   - Tom 1
   - Tom 2
   - Tom 3

Resultats attendus:
- les sorties separees sont cliquables, pas grisees
- le Main Mix reste audible apres activation des sorties separees
- chaque bus separe recoit uniquement sa voix correspondante
- aucun bus separe ne recoit de note fantome hi-hat quand le pas est desactive

### Mapping des voix

Utiliser un pattern simple ou editer la grille pour isoler chaque instrument.

Pour chaque ligne d'instrument:
1. Activer un seul pas sur un seul instrument.
2. Desactiver ou vider les autres instruments.
3. Lancer la lecture.
4. Observer le Main Mix et le bus separe correspondant.

Resultat attendu:
- Kick sort sur Kick et Main Mix
- Snare sort sur Snare et Main Mix
- Hi-Hat sort sur Hi-Hat et Main Mix
- Open HH sort sur Open HH et Main Mix
- Tom 1 sort sur Tom 1 et Main Mix
- Tom 2 sort sur Tom 2 et Main Mix
- Tom 3 sort sur Tom 3 et Main Mix
- les autres bus separes restent silencieux

### Mutes et solos

1. Charger le preset Rock.
2. Activer toutes les sorties separees.
3. Muter Kick.
4. Verifier que Kick disparait du Main Mix et du bus Kick.
5. Retirer le mute Kick.
6. Solo Snare.
7. Verifier que seule Snare reste audible dans le Main Mix et sur le bus Snare.
8. Retirer le solo Snare.
9. Solo Kick et Snare ensemble.
10. Verifier que Kick et Snare restent audibles, et que les autres voix sont coupees.

Resultat attendu: mutes et solos affectent de facon coherente le Main Mix et les sorties separees.

### Sauvegarde et reouverture

Etat du correctif 2026-05-11: le build `20260511-091259` sauvegarde la grille dans le
champ persistant `pattern-v1`, directement depuis `SharedPattern`. Le diagnostic Studio One
a montre que les parametres classiques etaient deja restaures et que le probleme etait limite
aux anciens parametres caches de grille `st01` a `st16`. Cette procedure doit confirmer le
resultat dans Studio One.

1. Garder toutes les sorties separees activees.
2. Modifier le pattern dans la grille, par exemple ajouter Kick au pas 3 et retirer un Hi-Hat.
3. Charger un preset different, puis refaire une modification manuelle dans la grille.
4. Modifier `Master Volume`.
5. Modifier `Fallback BPM`.
6. Activer au moins un mute et un solo.
7. Sauvegarder le projet Studio One.
8. Fermer le projet.
9. Rouvrir le projet.
10. Lancer la lecture.

Resultats attendus:
- le plugin se recharge sans erreur
- le build ID reste celui attendu
- les sorties precedemment activees sont toujours disponibles
- le Main Mix et les bus separes produisent le meme routing qu'avant sauvegarde
- le pattern edite dans la grille est restaure
- le preset charge et les modifications manuelles restent audibles
- `Master Volume` et `Fallback BPM` sont restaures
- les mutes et solos sont restaures

### Projet Studio One existant

Ouvrir un projet cree avec `VST3_CLASS_ID = DrumFlashPlugin1`.

Resultats attendus:
- le plugin existant est retrouve par Studio One
- l'interface s'ouvre
- le build ID correspond au build installe
- les sorties separees restent activables
- le projet joue sans devoir remplacer manuellement le plugin

### Autre DAW

Repeter le test minimal dans au moins un autre hote VST3.

Resultats minimum attendus:
- le plugin est detecte comme instrument VST3
- l'interface s'ouvre
- le Main Mix produit de l'audio
- les sorties separees sont visibles ou activables si le DAW supporte les instruments multi-out
- si le DAW ne supporte pas le multi-out instrument de la meme facon, noter le comportement exact

## Resultats attendus aujourd'hui

- audio present
- pattern de base audible
- variation audible du niveau master
- BPM de secours utilisable si le host ne fournit pas le tempo
- sorties multi-out activables dans Studio One
- chaque bus instrument recoit sa voix correspondante

## Resultats a traiter comme non acquis

Toute modification au patch `nih-plug` multi-out doit etre retestee dans Studio One avec un
nouveau build visible dans l'interface.
