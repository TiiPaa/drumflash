# Drum Rack — algorithmes de synthèse

Analyse technique du moteur audio de `drumrack.html` (Lumlux Art / FDR-Sound).
Document de référence pour comprendre ou réimplémenter les quatre voix.

Source analysée : fichier monolithique, 1551 lignes, HTML + CSS + JS inline, aucune
dépendance externe. Le moteur repose entièrement sur l'API Web Audio native.

---

## 1. Principe général

Le moteur est un **graphe Web Audio temps réel**, reconstruit à chaque frappe.
Il n'y a ni buffer pré-calculé, ni synthèse échantillon par échantillon, ni sample.
Chaque hit instancie des nœuds (`OscillatorNode`, `BufferSource`, `BiquadFilter`,
`GainNode`, `WaveShaperNode`), programme leurs automations à un temps absolu `t`, puis
laisse le garbage collector faire le ménage après `stop()`.

Conséquence architecturale importante : **le même code sert à l'audition et à l'export**.
Le rendu WAV instancie un `OfflineAudioContext`, reconstruit le même graphe dedans, et
ordonnance les mêmes appels. Il n'existe pas de second chemin de rendu à maintenir en
cohérence avec le premier.

Topologie globale :

```
voix ──► lane (gain/mute/solo) ──► master ──► limiteur (soft clip) ──► destination
```

Une lane par voix, quatre au total : `kick`, `snare`, `hat`, `clap`.

---

## 2. Primitives partagées

### 2.1 Générateur de bruit

Un buffer de bruit blanc uniforme de 1,2 s est généré une seule fois par contexte audio
et mémorisé dans une `WeakMap` indexée par le contexte. Chaque instance de voix crée un
`BufferSource` en boucle sur ce buffer partagé, avec une variation :

```
playbackRate = 0.9 + random() * 0.2
```

Deux effets. D'abord chaque frappe lit le buffer à une vitesse légèrement différente, ce
qui décale le spectre de ±10 % et évite l'effet « machine » d'un bruit strictement
identique à chaque hit. Ensuite, comme la boucle démarre à une phase différente selon
l'instant de déclenchement, deux hats consécutifs ne sont jamais bit-à-bit identiques.

C'est de l'humanisation à coût nul : un seul buffer en mémoire, aucun calcul par hit.

### 2.2 Saturation douce (`tanh`)

Table de transfert de 1024 points pour `WaveShaperNode`, normalisée pour que l'entrée
pleine échelle ressorte à pleine échelle :

```
k     = 1 + amount * 40
f(x)  = tanh(k·x) / tanh(k)
```

avec `x ∈ [-1, 1]` et `amount ∈ [0, 1]`. Oversampling 4x sur toutes les instances.

La plage de `k` va de 1 (quasi linéaire) à 41 (écrêtage dur). La normalisation par
`tanh(k)` garantit que le knob agit sur le contenu harmonique et pas sur le niveau de
sortie. La même fonction sert de saturation de voix (kick) et de limiteur master, avec
des valeurs de `amount` très différentes.

### 2.3 Enveloppe d'amplitude AD

Toutes les voix utilisent la même enveloppe attaque/décroissance :

```
gain(t)         = 0.0001                      (setValueAtTime)
gain(t+a)       = peak                        (rampe linéaire)
gain(t+a+dec)   = 0.0001                      (rampe exponentielle)
```

Attaque par défaut 1 ms. Le plancher à 0.0001 au lieu de 0 est obligatoire :
`exponentialRampToValueAtTime` refuse une cible nulle. L'attaque linéaire et la
décroissance exponentielle sont le comportement percussif attendu.

### 2.4 Table d'onde en phase cosinus

C'est la pièce la plus intéressante du moteur.

Un `OscillatorNode` de type `sine` démarre à phase nulle : la première alternance met un
quart de cycle à atteindre son maximum. Sur un kick de 50 Hz, ce quart de cycle représente
5 ms — largement audible, et l'amplitude atteinte dépend du moment exact du déclenchement
par rapport à l'enveloppe. Le transitoire n'est pas reproductible.

La solution retenue : construire un `PeriodicWave` dont **toute l'énergie est dans les
coefficients réels**, les coefficients imaginaires étant nuls. La forme d'onde résultante
est un cosinus, qui démarre à son maximum. La première alternance pousse donc toujours
dans le même sens et avec la même amplitude, quelle que soit la frappe.

```js
parts = [0, 1, h*0.50, h*0.28, h*0.15, h*0.08]   // DC, fondamentale, partiels 2..5
sum   = Σ parts
real  = parts / sum
imag  = 0
createPeriodicWave(real, imag, { disableNormalization: true })
```

Le paramètre `h` (knob Harmonics, 0 à 1) pilote l'amplitude des partiels supérieurs selon
une décroissance approximativement en `1/n`. La division par `sum` est ce qui rend le knob
utilisable : sans elle, l'amplitude crête de la forme d'onde double presque entre `h=0` et
`h=1`, et le contrôle se comporterait comme un volume au lieu d'un contrôle de timbre.

Les tables sont mémorisées dans un cache à double niveau (`WeakMap` par contexte, `Map`
par valeur de `h` quantifiée sur 40 pas). `createPeriodicWave` est coûteux et serait
appelé à chaque frappe sans ce cache.

---

## 3. Kick

Trois couches sommées sur un bus commun, puis saturées et filtrées ensemble.

```
click ─┐
punch ─┼─► bus (gain = vélocité) ─► waveShaper(drive) ─► HP 28 Hz ─► LP 7 kHz ─► gate ─► lane
sub   ─┘
```

Le fait que la saturation soit **après la somme** et non par couche est un choix de
caractère : le click intermodule avec le punch et le sub à l'intérieur du non-linéaire.
C'est ce qui donne au kick sa cohésion, mais c'est aussi ce qui fait que monter le drive
écrase l'attaque au lieu de la renforcer.

### 3.1 Couche a — click

Bruit blanc → bandpass 3000 Hz, Q 0.8 → enveloppe de décroissance 2 ms → bus.
Source coupée à 30 ms. Niveau `click × 0.9`. Couche omise si `click < 0.001`.

Le Q très bas laisse passer une large bande : ce n'est pas un ping résonnant mais un
souffle bref, qui se lit comme le contact du battoir.

### 3.2 Couche b — punch

Oscillateur avec la table d'onde de la section 2.4, dont la fréquence suit l'enveloppe
dessinée par l'utilisateur. Enveloppe d'amplitude : pic 1.0, attaque 0,8 ms,
décroissance `pdec`.

**Enveloppe de pitch.** Représentée par une liste de nœuds `{ t: ms, f: Hz, c: bend }`,
maximum 6 nœuds, `t ∈ [0, 400] ms`, `f ∈ [24, 420] Hz`. Le dernier nœud est la note
racine, verrouillée bidirectionnellement sur le knob Tune.

L'interpolation à l'intérieur d'un segment `[a, b]` :

```
p  = (ms - a.t) / (b.t - a.t)              p ∈ [0, 1]
p' = p ^ 2^(a.c · 2.2)                     déformation de l'axe temporel
f  = a.f · (b.f / a.f) ^ p'                interpolation logarithmique en fréquence
```

Deux décisions à retenir :

**L'interpolation est logarithmique en fréquence**, donc linéaire en demi-tons. Une chute
de 200 Hz à 50 Hz passe par 100 Hz à mi-parcours, pas par 125 Hz. C'est ce qui rend la
courbe musicalement lisible et permet d'afficher une note plutôt qu'une fréquence.

**Le bend déforme l'axe du temps, pas l'axe des fréquences.** Le paramètre `c ∈ [-1, 1]`
produit un exposant entre 2^-2.2 ≈ 0.217 et 2^2.2 ≈ 4.59. À `c = 0` l'exposant vaut 1 et
l'interpolation est une glissade régulière en demi-tons. À `c > 0` la chute est
front-loaded : l'essentiel du parcours se fait dans les premiers pourcents du segment.
À `c < 0` elle s'étale.

C'est exactement la différence entre un kick dur et un kick profond, avec les mêmes
fréquences de départ et d'arrivée.

**Programmation de l'automation.** La courbe est échantillonnée en 512 points et poussée
par `setValueCurveAtTime`. La spécification Web Audio interdit tout autre événement
d'automation dans la plage temporelle couverte par une courbe ; Chrome lève
`NotSupportedError`. Le code ne programme donc aucun `setValueAtTime` sur ce paramètre, et
enveloppe l'appel dans un `try/catch` avec repli sur un `exponentialRampToValueAtTime`
entre la première valeur et la racine.

### 3.3 Couche c — sub

Oscillateur avec la même table d'onde à `h = 0`, soit un cosinus pur — aucun partiel, le
bas du spectre reste propre. Fréquence **fixe** à `tune`, jamais enveloppée.

Point à souligner : le sub ne suit pas la courbe de pitch. Seule la couche punch descend.
Le sub est une note tenue courte qui démarre en même temps et porte le poids.
Enveloppe : pic `slev`, attaque 3 ms, décroissance `sdec`. Couche omise si `slev < 0.001`.

L'attaque de 3 ms, plus lente que celle du punch, évite que le sub ajoute son propre
transitoire et brouille l'attaque.

### 3.4 Post-traitement et choke

Après saturation : passe-haut 28 Hz (élimine le DC et l'infra-basse inutile qui
consommerait du headroom), passe-bas 7 kHz (rattrape la dureté générée par le drive).

Un `GainNode` de gate en fin de chaîne assure deux fonctions :

**Choke.** Le gate du kick précédent est ramené à zéro en 5 ms via `cancelAndHoldAtTime`
(avec repli sur `cancelScheduledValues` sur les moteurs qui ne l'implémentent pas). Un
seul kick sonne à la fois. Indispensable dès que `sdec` dépasse la durée d'un pas.

**Fade de fin.** Le gate descend à zéro en 4 ms après la fin de la queue la plus longue,
pour qu'aucune coupe abrupte de source ne produise un clic.

---

## 4. Snare

Deux composantes mixées par un crossfade unique, sans traitement commun.

**Composante bruit** — bruit blanc → bandpass à `tone`, Q 0.7 → enveloppe de décroissance
`dec`. Niveau `vel × snap`.

**Composante corps** — deux oscillateurs triangle aux ratios **1 et 1.83** par rapport à
`tune`. Niveau `vel × (1 - snap)`, respectivement pondérés 0.9 et 0.5, avec une
décroissance de `dec / 2`.

Le ratio 1.83 est inharmonique. C'est un choix classique de synthèse de caisse claire :
une membrane circulaire ne produit pas de série harmonique entière, et un intervalle
non entier évite que les deux oscillateurs se lisent comme une note.

Le knob Snap n'est donc pas un contrôle de brillance mais un crossfade strict entre timbre
et corps : les deux composantes se partagent le niveau. La décroissance moitié plus courte
du corps fait que le bruit domine toujours la queue.

---

## 5. Hi-hat

La voix la plus économe du moteur : une seule source, deux filtres.

```
bruit ─► HP à tune × 0.8 ─► BP à tune, Q 6 ─► enveloppe ─► lane
```

Décroissance : `dec × (1 + open × 6)`, soit un facteur 1 à 7. Niveau `vel × 0.55`.

À noter : il n'y a **pas** de synthèse métallique à six oscillateurs carrés en ratios
inharmoniques, méthode habituelle pour approcher une 808. Le caractère métallique vient
uniquement de la résonance du bandpass à Q 6, appliqué à du bruit large bande.

Il n'y a pas non plus de choke entre hat ouvert et hat fermé, puisqu'il n'y a qu'une seule
voix de hat : le knob Open est un simple multiplicateur de décroissance. Deux hats qui se
chevauchent sonnent simultanément.

---

## 6. Clap

Quatre bursts de bruit dans un bandpass partagé (`tone`, Q 1.3).

```
burst 0 :  t + 0·spread        décroissance 20 ms   niveau vel × 0.7
burst 1 :  t + 1·spread        décroissance 20 ms   niveau vel × 0.7
burst 2 :  t + 2·spread        décroissance 20 ms   niveau vel × 0.7
queue   :  t + 2·spread        décroissance dec     niveau vel × 0.45
```

C'est la structure canonique du clap 909 : la perception de « plusieurs mains » vient de
la répétition rapide, et le corps du son vient de la queue plus longue qui démarre avec le
dernier burst. Le fait que les quatre passent par le **même** instance de bandpass est
important — ils partagent l'état du filtre, donc la résonance du burst précédent colore le
suivant.

Avec `spread` réglable de 5 à 40 ms, la voix couvre du flam serré au roulement.

---

## 7. Bus, mix et limiteur

Chaque voix a sa lane : un `GainNode` dont le gain vaut `vol × on`, où `on` intègre la
logique mute/solo (si au moins une voix est en solo, toutes les autres sont à zéro).

Le master est un `GainNode` suivi d'un `WaveShaperNode` utilisant la fonction de la
section 2.2 avec `amount = 0.08`, soit `k ≈ 4.2`.

Ce « limiteur » est un soft clip statique : pas de détection d'enveloppe, pas de lookahead,
pas de constante de temps. Il ne réduit pas le gain, il déforme. En pratique il attrape les
crêtes et empêche l'écrêtage numérique de la carte son, mais en cas de surcharge il ajoute
de la distorsion au lieu de ducker. C'est un choix cohérent avec le reste du moteur — pas
de dynamique, pas d'état à gérer — mais c'est le point à remplacer en premier si on veut
un master propre.

---

## 8. Rendu offline et export WAV

Le rendu réutilise le graphe temps réel :

1. Instancier un `OfflineAudioContext` à 44,1 kHz de durée calculée.
2. Sauvegarder la référence globale au rig, en construire un nouveau dans le contexte
   offline, l'assigner à la globale.
3. Appeler les fonctions de synthèse normalement — elles écrivent dans le contexte offline
   sans le savoir.
4. Restaurer la référence globale (dans un `finally` pour l'export kick).
5. `startRendering()`.

Pour l'export du kick seul, la durée du buffer est `max(pdec, sdec, envMs) + 120 ms`.
La prise en compte de `envMs` est nécessaire : une courbe de pitch longue avec des
décroissances d'amplitude courtes serait tronquée si on ne dimensionnait que sur les
enveloppes d'amplitude.

Pour l'export de boucle : `bars × 16 × stepDur + 1,2 s` de queue, premier hit à 50 ms.

**Encodage WAV.** En-tête RIFF/WAVE de 44 octets écrit à la main via `DataView`, PCM 16
bits entier signé, little-endian, entrelacé. La conversion float → entier utilise
correctement l'asymétrie du format :

```
s < 0  →  s × 0x8000        (plage négative : -32768)
s ≥ 0  →  s × 0x7fff        (plage positive : +32767)
```

Multiplier uniformément par 32768 déborderait sur les crêtes positives.

---

## 9. Sérialisation d'état

Aucun `localStorage`. L'état complet — tempo, swing, master, patterns, tous les paramètres
de voix, tous les nœuds d'enveloppe, la matrice de mix — est sérialisé en JSON compact,
encodé en base64 puis converti en base64url (`+` → `-`, `/` → `_`, padding retiré) et écrit
dans le hash de l'URL.

Les valeurs sont arrondies avant sérialisation (3 décimales pour les paramètres, 2 pour
les fréquences d'enveloppe) pour limiter la longueur de l'URL.

Le code gère le cas où `history.replaceState` est refusé — page `file://`, blob, iframe
sandboxée — en conservant l'état encodé en mémoire et en cessant d'écrire dans l'adresse
après le premier refus, au lieu de lever une exception à chaque déplacement de knob.

---

## 10. Plages de paramètres

### Kick

| Paramètre | Clé | Min | Max | Défaut | Rôle |
|---|---|---|---|---|---|
| Tune | `tune` | 32 Hz | 65 Hz | 49 | racine, verrouillée sur le dernier nœud d'enveloppe |
| Punch | `pdec` | 60 ms | 600 ms | 280 | décroissance de la couche accordée |
| Sub decay | `sdec` | 80 ms | 1200 ms | 480 | décroissance du sub |
| Sub level | `slev` | 0 | 1 | 0.80 | niveau du sub |
| Click | `click` | 0 | 1 | 0.35 | niveau du transitoire |
| Harmonics | `harm` | 0 | 1 | 0.18 | partiels 2 à 5 de la couche punch |
| Drive | `drive` | 0 | 1 | 0.45 | `k` de la saturation, 1 à 41 |

Enveloppe de pitch : `t ∈ [0, 400] ms`, `f ∈ [24, 420] Hz`, 6 nœuds maximum,
bend `c ∈ [-1, 1]`.

### Snare

| Paramètre | Clé | Min | Max | Défaut |
|---|---|---|---|---|
| Tune | `tune` | 120 Hz | 260 Hz | 180 |
| Decay | `dec` | 60 ms | 400 ms | 180 |
| Snap | `snap` | 0 | 1 | 0.60 |
| Tone | `tone` | 600 Hz | 4000 Hz | 1600 |

### Hi-hat

| Paramètre | Clé | Min | Max | Défaut |
|---|---|---|---|---|
| Tune | `tune` | 4 kHz | 12 kHz | 8000 |
| Decay | `dec` | 20 ms | 200 ms | 45 |
| Open | `open` | 0 | 1 | 0.15 |

### Clap

| Paramètre | Clé | Min | Max | Défaut |
|---|---|---|---|---|
| Spread | `spread` | 5 ms | 40 ms | 12 |
| Decay | `dec` | 60 ms | 400 ms | 160 |
| Tone | `tone` | 600 Hz | 2500 Hz | 1100 |

---

## 11. Observations pour une réimplémentation

**Ce qui mérite d'être repris tel quel :**

- La phase cosinus par `PeriodicWave` pour tout transitoire percussif. Le gain en
  reproductibilité est réel et le coût est nul une fois la table en cache.
- La normalisation par la somme des partiels sur le contrôle d'harmoniques. Sans elle,
  n'importe quel knob de timbre additif devient un knob de volume déguisé.
- L'interpolation logarithmique de l'enveloppe de pitch avec déformation de l'axe temporel.
  Séparer « où on va » de « à quelle vitesse on y va » est ce qui rend l'éditeur utilisable.
- La randomisation du `playbackRate` sur le buffer de bruit partagé.
- Un seul chemin de synthèse pour l'audition et le rendu, via substitution du contexte.

**Ce qui est un compromis assumé :**

- La saturation sur le bus commun plutôt que par couche. Donne de la cohésion mais empêche
  de driver le corps sans écraser le click.
- Le sub à fréquence fixe. Simplifie beaucoup, mais interdit les kicks où le sub lui-même
  descend — courant en hardstyle et en dubstep.
- Le limiteur en soft clip statique. Suffisant en usage normal, mauvais en surcharge.
- La voix de hat unique, sans paire ouvert/fermé et donc sans choke.

**Contraintes Web Audio à connaître avant de coder ça :**

- `exponentialRampToValueAtTime` refuse la cible zéro : plancher à 0.0001.
- Aucun événement d'automation n'est autorisé dans la plage temporelle d'un
  `setValueCurveAtTime` ; Chrome lève `NotSupportedError`.
- `cancelAndHoldAtTime` n'est pas universellement implémenté, prévoir le repli.
- `createPeriodicWave` est coûteux, quantifier et mettre en cache.
- Sur iOS, aucun son avant une interaction utilisateur ; le contexte doit être repris
  depuis `suspended`.
