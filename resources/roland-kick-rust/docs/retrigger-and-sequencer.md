# Retrig, séquenceur et clics parasites

## 1. Pourquoi le clic anormal apparaît

Un clic parasite apparaît quand le moteur provoque une **discontinuité** :

- saut brutal d'amplitude
- saut brutal de phase
- saut brutal de fréquence
- reset violent d'un état interne

Le transient voulu doit rester, mais les sauts numériques doivent disparaître.

## 2. Règle de conception

Quand un nouveau step arrive alors que l'ancien n'a pas fini :

- on garde les états internes
- on déclenche un nouveau transient
- on relance proprement amp/pitch
- on évite le hard reset global

La séparation la plus utile reste :

- `transient` = attaque voulue
- `body` = queue/résonance stabilisée

## 3. Voix mono à état continu

Pour un kick, la stratégie recommandée est :

- une voix mono persistante par instrument
- phase continue ou résonateur continu
- enveloppes relancées depuis leur état courant
- micro-duck de l'ancienne queue seulement si nécessaire

## 4. Séquenceur sample-accurate

Ne pas quantifier tous les triggers au début du buffer.

Prévoir des événements horodatés dans le buffer :

```text
Event {
    sample_offset,
    event_kind,
}
```

Puis, lors du rendu bloc :

- parcourir les samples
- déclencher les événements exactement au sample prévu

## 5. Micro-smoothing utile

Lisser seulement ce qui est sensible :

- fréquence instantanée
- gain du body
- dry/wet ou drive si automatisés

Éviter de lisser tout le kick globalement, sinon on perd l'attaque.

## 6. Stratégie de sécurité si les steps sont très serrés

Quand les triggers sont vraiment proches :

- transient neuf inchangé
- body ancien légèrement ducké pendant quelques samples ou quelques millisecondes

Exemple :

```text
tail_duck = 0.10 .. 0.35
tail_duck_ms = 0.2 .. 1.5 ms
```

## 7. Tests à prévoir

Créer un test spécifique de retrig serré :

- triggers à 5 ms, 10 ms, 20 ms, 30 ms
- vérifier :
  - pas de `NaN`
  - pas de `Inf`
  - pas de pic aberrant
  - pas de bruit HF anormal
  - attaque toujours présente

## 8. Conclusion pratique

La stratégie saine n'est pas :

> couper puis recréer la voix

mais :

> déclencher un nouveau transient et réinjecter de l'énergie dans une voix déjà vivante
