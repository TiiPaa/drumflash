# Mode Analog — Documentation de référence

> Ce document décrit le fonctionnement du paramètre **Analog** dans Flash Drum, son impact sonore par instrument, et les bonnes pratiques d'utilisation.

---

## Qu'est-ce que le mode Analog ?

Le slider **Analog** contrôle le comportement de l'oscillateur et de l'enveloppe à chaque trigger :

- **`analog >= 0.5`** → Mode **Analog**
  - Phase de l'oscillateur **préservée** (pas de reset)
  - Enveloppe relancée depuis sa valeur courante via `trigger_at_peak()`
  - **Drift aléatoire** appliqué à chaque coup (pitch, niveau, enveloppe)
  - Son organique, continu, imprévisible — inspiré des TR-808/909

- **`analog < 0.5`** → Mode **Digital**
  - Phase de l'oscillateur **réinitialisée** à 0 (avec crossfade anti-click sur 2 samples)
  - Enveloppe repart de zéro via `trigger()`
  - Aucun drift — chaque hit est identique au bit près
  - Son propre, répétable, précis — inspiré des drum machines numériques

> **Note importante** : le slider Analog agit comme un **seuil binaire** (0.5), pas comme un contrôle progressif de l'intensité du drift. La valeur affichée indique principalement la préférence par défaut de l'instrument.

---

## Valeurs par défaut actuelles

| Instrument | Défaut | Type | Drift opérationnel ? |
|-----------|--------|------|---------------------|
| Kick | **0.3** | Hybride | **Oui** (très audible) |
| Snare | **0.3** | Hybride | Oui (subtil) |
| Tom1 / Tom2 / Tom3 | **0.3** | Hybride | Oui (subtil) |
| Cymbal | **0.3** | Hybride | Oui (subtil) |
| BassDrum808 (B8) | **0.3** | Hybride | Oui (subtil) |
| HiHat | **1.0** | Analog fixé | Non (continuité requise) |
| OpenHiHat | **1.0** | Analog fixé | Non (continuité requise) |
| Clap | **1.0** | Analog fixé | Non (continuité requise) |
| Ride | **1.0** | Analog fixé | Non (continuité requise) |
| Snare606 | **1.0** | Analog fixé | Non (continuité requise) |
| Perc1 | **1.0** | Analog fixé | Non (continuité requise) |

### Pourquoi 0.3 par défaut sur certains instruments ?

La valeur **0.3** place l'instrument en mode **Digital** (`< 0.5`) par défaut, mais signale qu'il possède un drift analogique opérationnel si l'utilisateur passe le slider au-dessus de 0.5.

Les instruments à **1.0** sont en mode **Analog** permanent — le slider est présent mais le comportement analogique est fixé pour des raisons techniques (continuité de phase obligatoire sur les bruits longs).

---

## Amplitude du drift par instrument

Le drift analogique n'est **pas uniforme** d'un instrument à l'autre :

### Très audible
- **Kick** : ±3.5% pitch, ±10% niveau, ±20% temps d'enveloppe
  - Le changement de fréquence est immédiatement perceptible
  - Recommandé à 1.0 pour House/Disco, 0.0-0.3 pour Techno

### Subtil (~7.5% pitch max)
- **Snare** : Variation de pitch du body + noise non reseedé
- **Tom1-3** : Variation de pitch de l'oscillateur
- **BassDrum808** : Phase préservée + drift de fréquence modéré
- **Cymbal** : Variation du shimmer et du niveau

> Sur ces instruments, le drift est audible en A/B mais discret dans un mix. Il ajoute de la vie sans déstabiliser la groove.

### Aucun drift (analog fixé)
- **HiHat / OpenHiHat / Ride / Clap / Snare606 / Perc1**
  - Le slider Analog contrôle uniquement la continuité de phase (pas de réinitialisation)
  - Aucune variation aléatoire de pitch ou niveau
  - Passer de 0.3 à 1.0 ne change pas le caractère sonore, seulement la réponse au retrigger

---

## Implémentation technique par instrument

### Kick (kick.rs)
- Analog : `self.osc.phase` préservé, `self.noise_osc.phase` préservé
- Digital : Crossfade entre ancienne et nouvelle phase sur 2 samples
- Drift : fréquence ±3.5%, niveau ±10%, decay ±20%

### Snare (snare.rs)
- Analog : Phase préservée + noise generator NON reseedé
- Digital : Phase réinitialisée + noise generator reseedé
- Drift : pitch du body ±7.5%

### Tom (tom.rs)
- Analog : Phase préservée pour un son plus naturel
- Digital : Réinitialisation pour un son plus synthétique
- Drift : pitch ±7.5%

### BassDrum808 (kick_808.rs)
- Analog : Phase préservée, simule le comportement du circuit original
- Digital : Réinitialisation complète ("cold start" comme l'original 808)
- Drift : pitch modéré, enveloppe légèrement variable

### HiHat / OpenHiHat / Ride / Cymbal
- Toujours analog (pas de reset de phase possible sans coupure audible)
- Pas de drift aléatoire (le bruit blanc est suffisamment variable)

### Clap
- Toujours analog (0.3 = continuité des oscillateurs pour le son réaliste)
- Pas de drift (l'enveloppe en burst est déjà imprévisible)

---

## Recommandations par style musical

### Classic House (Kerri Chandler style)
- Kick: 0.9 (légèrement digital pour la précision)
- Snare: 1.0 (full analog pour le groove)
- HiHat: 1.0 (toujours analog)
- Tom: 0.8 (presque analog)
- Clap: 0.3 (défaut)
- Groove: Swing16 à 55%

### Detroit Techno (Jeff Mills style)
- Kick: 0.2 (très digital pour la précision)
- Snare: 0.3 (légèrement analog pour le corps)
- HiHat: 1.0 (toujours analog)
- Tom: 0.4 (mi-chemin)
- Groove: Straight (pas de swing)

### Drum & Bass (LTJ Bukem style)
- Kick: 0.7 (analog pour les retriggers rapides)
- Snare: 0.8 (presque analog pour le groove)
- HiHat: 1.0 (toujours analog)
- Tom: 0.9 (presque analog)
- Groove: Shuffle à 40%

### Minimal Techno (Richie Hawtin style)
- Kick: 0.1 (très digital)
- Snare: 0.2 (très digital)
- HiHat: 1.0 (toujours analog)
- Tom: 0.3 (digital)
- Groove: Straight (pas de swing)

---

## Astuces et conseils avancés

1. **Automatisation du paramètre Analog**
   - Automatisez le slider Analog pendant un breakdown
   - Passez de digital (précis) à analog (organique) pour un effet dramatique

2. **Per-instrument settings**
   - Chaque instrument peut avoir sa propre valeur analog
   - Exemple puissant : Kick digital (0.2) + Snare analog (1.0)

3. **Density et Analog**
   - Patterns denses (>120 BPM, 16e notes) → privilégiez analog (évite la fatigue)
   - Patterns clairsemés (<110 BPM, 8e notes) → digital fonctionne bien

4. **Velocity interaction**
   - En mode analog : la velocity affecte davantage le timbre
   - En mode digital : la velocity affecte davantage le volume

5. **Test de perceptibilité**
   - Pour vérifier si le drift est audible sur un instrument : créez un pattern de 16e notes à 120 BPM, écoutez en solo, basculez entre 0.0 et 1.0
   - Le Kick montre immédiatement la différence ; les Toms nécessitent une écoute attentive

---

## Dépannage

| Problème | Cause probable | Solution |
|----------|---------------|----------|
| "Mon kick sonne différent à chaque hit" | Mode analog actif avec drift élevé | Passez en digital (0.0) pour une consistance parfaite |
| "Mon pattern dense sonne mécanique" | Trop d'instruments en digital | Passez Kick/Snare en analog (≥0.5) pour plus de groove |
| "Le slider Analog ne change rien sur HiHat" | Analog fixé (pas de drift implémenté) | Normal — HiHat nécessite la continuité pour éviter les clicks |
| "Le drift sur Tom est trop subtil" | Amplitude du drift limitée (~7.5%) | Essayez de combiner avec le paramètre Push/Pull stéréo pour plus de variation |

---

## Notes pour les développeurs

- Le drift analogique est appliqué dans `AnalogDrift::apply()` (helper partagé)
- Chaque instrument définit ses propres coefficients de drift dans son module de synthèse
- Le seuil de 0.5 est codé en dur dans chaque voix (`if analog >= 0.5`)
- Les instruments "fixés" ignorent le seuil et traitent toujours le signal comme analog (pas de reset phase)
- Le drift est calculé **par trigger**, pas par sample — coût CPU négligeable
