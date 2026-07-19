# Guide Utilisateur — Flash Drum

> Guide rapide pour utiliser le plugin Flash Drum dans votre DAW.

---

## Installation

1. **Télécharger** le dernier build (voir `CHANGELOG.md`)
2. **Lancer** `build.ps1 -Install` (ou copier manuellement le `.vst3` dans le dossier VST3 système)
3. **Scanner** les plugins dans votre DAW
4. **Insérer** Flash Drum sur une piste instrument

## Interface

### Barre supérieure

- **▶ Play** — Démarre le séquenceur (sync avec le DAW)
- **BPM** — Tempo du séquenceur (sync avec le transport hôte)
- **Choke** — Active le choke HiHat/OpenHiHat
- **Auto-Edit** — Active l'édition automatique des réglages

### Grille de séquence (64 steps)

- **4 pages** de 16 steps (boutons 1-2-3-4)
- **13 voix de synthèse** dans **14 slots modulaires** (Kick, Snare, HiHat, OpenHiHat, Tom1-3, Clap, Ride, Cymbal, Snare606, 808 Kick, Perc1)
- **Navigation** : cliquez sur les boutons de page ou activez **Follow** pour suivre la lecture
- **Longueur** : ajustable de 1 à 64 steps (slider Len + boutons rapides 16/32/48/64)
- **x2** : Double la longueur du pattern en copiant les steps existants

### Édition des steps

- **Clic gauche** — Active/désactive un trigger sur la step
- **Clic droit** — Ouvre le menu des **Parameter Locks** (plocks)

#### Parameter Locks (Plocks)

Modifiez un paramètre de synthèse pour une step spécifique :

1. **Clic droit** sur une step
2. **Snapshot** — Copie tous les réglages actuels (mode figé)
3. **Link** — Suit les réglages globaux (mode dynamique)
4. Modifiez les paramètres individuels (fréquence, decay, volume, filter...)
5. **Copy Plock / Paste Plock** — Copiez un plock vers une autre step

Les plocks s'affichent en **orange** (link/mixte) ou **rouge** (snapshot) sur la grille.

### Sound Editor

- **Volume** — Niveau de sortie (0.0 - 2.0)
- **Decay** — Temps de déclin
- **Filter** — Fréquence de coupure (logarithmique)
- **Attack** — Temps d'attaque
- **Release** — Temps de relâchement
- **Analog** — Seuil analog/digital (≥0.5 = analog avec drift)
- **Stereo** — Largeur stéréo
- **Saturation** — Type, Drive, Mix (5 algorithmes)
- **Paramètres spéciaux** — Par instrument (click type pour Kick, shimmer pour Cymbal...)

#### Bouton Test (T)

Déclenche le son de l'instrument isolé pour pré-écouter les réglages.

### Générateur de patterns

- **Type** : Classic / Euclidean / Markov / Probabilistic
- **A / B** : Variantes de pattern
- **Mix** : Mélange A/B
- **Density** : Densité des notes
- **Variation** : Degré de variation
- **GENERATE** : Génère un nouveau pattern

### Export

- **MIDI** — Exporte le pattern en fichier `.mid` dans `Documents/Flash Drum/exports`
- **Drag** — Ouvre une fenêtre de drag-and-drop pour glisser le MIDI directement dans le DAW

## Multi-sorties

Flash Drum propose **14 sorties stéréo aux** (`Out 1`..`Out 14`) + le Main Mix.

### Configuration dans Studio One

1. Ouvrir le **mixeur** (F3)
2. Cliquer sur l'icône **⚙️** de la piste Flash Drum
3. Activer les sorties désirées (Kick, Snare, HiHat...)
4. Chaque sortie devient une piste audio séparée pour le mixage

## Modes Analog / Digital

Chaque instrument possède un slider **Analog** :

- **Analog (≥0.5)** — Phase préservée, drift aléatoire (pitch ±3.5%, niveau ±10%)
- **Digital (<0.5)** — Phase réinitialisée, son identique à chaque hit

**Recommandations** :
- Kick à 0.3-1.0 selon le style (House=1.0, Techno=0.2)
- HiHat/Ride/Cymbal toujours analog (fixé à 1.0)
- Snare/Tom/B8 à 0.3 (drift subtil)

Voir `docs/analog-mode.md` pour les détails complets.

## Groove

- **Swing** — Retarde les steps impaires (0-100%)
- **Groove Type** : Straight / Swing16 / Shuffle / MPC
- **Humanize** — Variation de velocity par piste (pas de timing)
- **Push/Pull** — Décalage stéréo gauche/droite par piste

## Sauvegarde

L'état complet du plugin est sauvegardé **automatiquement** dans le projet du DAW :
- Grille 64 steps
- Plocks (parameter locks)
- Réglages de synthèse par slot
- Paramètres globaux (BPM, swing, etc.)

**Pas besoin de sauvegarde manuelle** — rouvrez votre projet et tout est restauré.

## Raccourcis clavier

- **Shift + clic sur slider** — Fine-tuning (précision augmentée)
- **Double-clic sur valeur** — Saisie manuelle de la valeur

## Conseils de workflow

1. **Commencez** avec le générateur (bouton Random ou Generate) pour obtenir une base
2. **Affinez** la grille manuellement (clic gauche pour ajouter/supprimer des hits)
3. **Ajoutez des plocks** (clic droit) pour varier les sons sur certaines steps
4. **Réglez le mix** avec les sorties séparées dans le DAW
5. **Exportez en MIDI** pour éditer ultérieurement dans le DAW

## Dépannage

| Problème | Solution |
|----------|----------|
| Pas de son | Vérifier que le séquenceur est en lecture (▶) ou que le DAW joue |
| Clicks/pops | Vérifier la taille du buffer audio dans le DAW (augmenter si besoin) |
| Sauvegarde non restaurée | Vérifier que le build ID correspond (compatibilité VST3) |
| Drag MIDI ne fonctionne pas | Vérifier que `drum-pattern-midi-drag-helper.exe` est présent |
| Focus bloqué | Redémarrer le DAW après installation du plugin |

---

> Pour les détails techniques, voir `AGENTS.md` et `docs/infrastructure.md`
