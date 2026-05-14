# Synthèse DSP pour kicks TR-606 / TR-808 / TR-909

## 1. Principe général

Le moteur recommandé est un moteur **grey-box DSP** à voix mono persistante.

Idée centrale :

- une voix garde ses états internes
- un nouveau step **refrappe** le système au lieu de le détruire et de le recréer
- on sépare clairement :
  - **transient** = clic voulu, attaque, agressivité
  - **body** = grave, résonance, queue

Objectif :

- garder l'attaque initiale
- éviter le clic numérique parasite
- garder un comportement crédible quand les steps sont très rapprochés

## 2. Architecture commune

Blocs conseillés :

- `transient generator`
  - pulse court
  - ou burst bruité filtré
  - ou click synthétique band-limited
- `body generator`
  - oscillateur sinus à phase continue
  - ou résonateur amorti
- `pitch envelope`
  - décroissance exponentielle rapide
- `amplitude envelope`
  - décroissance exponentielle ou bi-exponentielle
- `tail duck`
  - léger duck de l'ancienne queue si les steps sont serrés
- `saturation légère`
- `dc blocker`
- `oversampling` si non-linéarités fortes

Sortie :

```text
output = transient + body
```

## 3. Règle de retrig

Au retrig :

- on déclenche un **nouveau transient**
- on relance les enveloppes depuis leur **état courant**
- on garde la continuité de la phase ou du résonateur
- on évite les resets violents de fréquence, phase ou amplitude

Formule mentale utile :

> Il faut refrapper un système déjà en mouvement, pas recréer la voix à zéro.

## 4. Pseudo-code commun

```text
state:
    transient
    amp_env
    pitch_env
    body
    tail_duck
    smoother_freq
    dc_block
    saturator

onTrigger(velocity):
    transient.trigger()
    pitch_env.retrigger_from_current(start_freq - base_freq)
    amp_env.retrigger_from_current(velocity)
    body.inject_energy(retrigger_amount)
    tail_duck.trigger()

processSample():
    target_freq = base_freq + pitch_env.next()
    freq = smoother_freq.process(target_freq)

    transient_sample = transient.next()
    body_sample = body.process(freq) * amp_env.next()
    body_sample *= tail_duck.gain()

    y = transient_sample + body_sample
    y = saturator.process(y)
    y = dc_block.process(y)
    return y
```

## 5. Profil TR-808

### Description littérale

La 808 doit rester :

- ronde
- profonde
- longue
- propre dans le sub
- avec une attaque présente mais pas dure

### Réglages typiques

- body principal très sinusoïdal
- pitch drop doux
- decay longue
- transient faible à moyenne
- saturation faible

### Paramètres de départ

```text
baseFreq     = 45..60 Hz
startFreq    = 110..180 Hz
pitchDecay   = 20..45 ms
ampDecay     = 300..1200 ms
transient    = faible, court, doux
drive        = faible
```

## 6. Profil TR-909

### Description littérale

La 909 doit être :

- plus agressive
- plus punchy
- plus courte que la 808
- plus mordante dans l'attaque
- plus présente dans le médium

### Réglages typiques

- transient plus fort
- pitch sweep plus rapide
- body plus court
- saturation plus présente
- plus de composante click/knock

### Paramètres de départ

```text
baseFreq     = 50..65 Hz
startFreq    = 150..260 Hz
pitchDecay   = 8..20 ms
ampDecay     = 120..350 ms
transient    = moyen à fort, plus brillant
drive        = moyen
```

## 7. Profil TR-606

### Description littérale

La 606 doit rester :

- plus petite
- plus sèche
- plus courte
- moins sub que la 808
- plus compacte et plus machine

### Réglages typiques

- body court
- pitch sweep plus modeste
- transient sec
- peu de saturation
- plus de médium grave perçu

### Paramètres de départ

```text
baseFreq     = 55..75 Hz
startFreq    = 100..160 Hz
pitchDecay   = 10..25 ms
ampDecay     = 80..220 ms
transient    = moyen, sec
drive        = faible à légère
```

## 8. Évolution vers d'autres oscillateurs

Oui, les algos peuvent évoluer.

### Recommandation générale

Ne pas remplacer brutalement le body principal par un oscillateur carré.

Mieux vaut :

- garder une base propre (`sine` ou résonateur)
- ajouter une **harmonic layer** optionnelle

```text
output = transient + sine_body + harmonic_layer
```

### Intérêt par oscillateur

- `sine`
  - meilleure base pour 808
  - très bonne base pour 606 et 909 aussi
- `triangle`
  - peut donner un peu plus de matière sans être trop dur
- `square`
  - utile pour plus d'harmoniques et d'agressivité
  - à filtrer ou doser prudemment
- `pulse`
  - très utile pour un transient ou une harmonic layer vive
- `shaped sine`
  - excellent compromis pour enrichir sans casser la base

### Recommandation par machine

- **808** : sine-first, harmonic layer très subtile
- **909** : sine + harmonic layer plus assumée
- **606** : body court + petite coloration harmonique possible

## 9. Direction de dev recommandée

### V1

- transient séparé
- body sinus / résonateur
- pitch env exponentielle
- amp env exponentielle
- retrig propre
- tail duck léger
- saturation douce

### V2

- harmonic layer optionnelle
- square / pulse / triangle / shaped sine
- plus de caractère pour 909 / 606

### V3

- oversampling ciblé
- circuit-inspired parameter mapping
- WDF ou state-space si besoin de fidélité plus puriste
