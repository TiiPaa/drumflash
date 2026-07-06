# Audit complet — Move

**IMPORTANT — Environnement Windows**
Tu es sur Windows. Les commandes Unix suivantes NE FONCTIONNENT PAS :
- grep, find, cat, head, tail, sed, awk, wc, xargs, chmod, ls
Utilise a la place :
- Pour lire un fichier : ouvre-le directement avec tes outils integres (read file, etc.)
- Pour chercher dans le code : utilise tes outils de recherche integres (search, grep tool, etc.)
- Pour lister des fichiers : utilise tes outils de navigation (list directory, etc.)
- Si tu dois absolument lancer une commande shell : PowerShell (Get-Content, Select-String, Get-ChildItem)
NE TENTE PAS de commandes Unix. Elles echoueront silencieusement ou avec une erreur.

Effectue un audit complet de ce projet : code, architecture ET infrastructure.
Explore tous les fichiers du projet, comprends l'architecture, et produis un rapport exhaustif.

Tu es libre d'utiliser tous les outils, skills, agents, commandes, ressources et competences a ta disposition pour mener a bien cet audit. N'hesite pas a lire les fichiers, naviguer dans l'arborescence, executer des commandes d'analyse, consulter la documentation, ou tout autre moyen que tu juges utile pour produire le rapport le plus complet et precis possible.

## Instructions

### Methodologie

1. **Analyser le contexte** : langages, frameworks, type de projet, patterns detectes.
2. **Explorer systematiquement** les fichiers sources, configs, dependances et tests.
3. **Cartographier l'infrastructure** : serveurs, services externes, bases de donnees, flux de donnees, CI/CD, deploiement, monitoring.
4. **Verifier la documentation existante** : si le projet contient des documents d'architecture ou d'infrastructure (README, docs/, ARCHITECTURE.md, diagrammes, docker-compose, .env.example, etc.), les comparer avec l'etat reel du code. Signaler toute divergence (documentation obsolete, composants documentes mais absents, services reels non documentes).
5. **Structurer les findings** par priorite (critique > important > suggestion > hors_securite).
6. **Appliquer la checklist** securite, performance, maintenabilite, tests, infrastructure.

### Checklist obligatoire

**Securite :**
- Validation et sanitisation des entrees utilisateur
- Pas de secrets hardcodes (cles API, mots de passe)
- Gestion securisee des erreurs (pas de stack trace exposee)
- Protection contre SQLi / XSS / CSRF selon le contexte
- Principe du moindre privilege
- Controle d'acces sur les actions sensibles

**Auth / OAuth / Impersonation (OBLIGATOIRE si SSO ou API tierce detectes) :**
- Identifier les fournisseurs d'identite utilises (Google, Microsoft, Okta, Auth0, GitHub...)
- Lister les scopes OAuth demandes : sont-ils minimaux pour la finalite annoncee ?
  Tout scope au-dela de `openid email profile` doit etre justifie par un usage explicite.
- Detection delegation domain-wide / service account avec subject (impersonation a l'echelle de l'organisation)
- Si l'application realise des actions "en tant que" l'utilisateur via une API tierce :
  * Qui peut declencher quelles actions (controles d'autorisation cote serveur) ?
  * Y a-t-il un audit log de ces actions (qui, quand, quoi, sur quel compte cible) ?
  * Le consentement utilisateur est-il explicite et granulaire (pas de scope cache) ?
  * Possibilite de revocation des tokens / refresh tokens stockes ?
- Stockage des tokens : en clair en base ? chiffres ? scope minimum ?
- Gestion des sessions : duree de vie raisonnable, revocation possible ?
- Detection automation / bot : un compte de service est-il utilise pour faire des actions
  qui devraient etre tracees a un humain ?

**Performance :**
- Pas de requetes N+1
- Pas de boucles O(n^2) evitables
- Ressources liberees correctement (connexions, fichiers, memoire)
- Batching envisage pour les ecritures en volume
- Pas d'await sequentiels evitables dans les boucles

**Maintenabilite :**
- Responsabilite unique par fonction/methode
- Nommage clair et explicite
- Pas de magic numbers/strings
- Pas de code mort (imports inutilises, branches inatteignables)
- Pas de duplication structurelle entre fichiers proches
- Contrats API/UI coherents

**Tests :**
- Cas nominaux couverts
- Edge cases testes (null, vide, limites)
- Pour chaque finding important, identifier le test a ajouter

**Infrastructure (OBLIGATOIRE — section distincte) :**
- Identifier tous les services externes utilises (APIs, BDD, stockage, auth, email, CDN...)
- Decrire la stack technique (hosting, runtime, OS, conteneurs...)
- Documenter les flux de donnees (qui appelle quoi, dans quel sens)
- Analyser la configuration de deploiement (CI/CD, scripts, Dockerfile, docker-compose...)
- Evaluer le monitoring et l'observabilite (logs, metriques, alertes)
- Verifier la gestion des secrets en production (env vars, vault, config files)
- Evaluer la resilience (single points of failure, backups, failover)
- Analyser la scalabilite (bottlenecks, limites connues)

**Hors securite (obligatoire meme si pas de faille) :**
- Code mort et imports inutilises
- Redondances et duplications
- Optimisations pertinentes
- Dette de qualite visible

### Format de chaque finding

Chaque finding doit etre precis et actionnable :
- **categorie** : code|infrastructure (pour distinguer les findings)
- **fichier** : chemin relatif exact du fichier concerne
- **ligne** : numero de ligne si possible
- **probleme** : description courte et precise
- **pourquoi** : impact concret (pas de generalite vague)
- **suggestion** : solution concrete avec exemple de code si pertinent
- **confiance** : haute (certain), moyenne (probable), faible (hypothese)
- **contexte_finding** : facteurs contextuels que TU OBSERVES dans le code (pas de devinette). Ils servent a moduler la criticite reelle de l'ecart.

#### Facteurs contextuels (contexte_finding)

Pour CHAQUE finding, evalue ces 4 facteurs a partir de ce que tu vois reellement dans le code. Si tu ne peux pas trancher, mets "inconnu".

- **exposition** : ou se situe le code concerne dans la surface d'attaque ?
  - `public` : accessible sans authentification (route publique, endpoint ouvert)
  - `authentifie` : derriere un login / une session
  - `interne` : reseau interne, admin, VPN, job batch — non expose publiquement
  - `inconnu` : impossible a determiner depuis le code
- **exploitabilite** : a quel point l'ecart est facile a declencher ?
  - `triviale` : exploitable directement, sans condition particuliere
  - `conditionnelle` : necessite des conditions specifiques (role precis, donnee particuliere, timing)
  - `theorique` : possible en theorie mais tres difficile en pratique
  - `inconnu`
- **mitigations** : des protections sont-elles deja en place autour de l'ecart ?
  - `aucune` : rien ne protege
  - `partielle` : protection incomplete (validation partielle, echappement partiel)
  - `robuste` : protection solide en amont (ORM parametre, CSP stricte, validation forte)
  - `inconnu`
- **donnees_impactees** : quelles donnees sont en jeu ?
  - `pii_sensibles` : donnees personnelles / sensibles (RGPD), secrets, credentials
  - `metier` : donnees metier non personnelles
  - `publiques` : donnees deja publiques ou non sensibles
  - `inconnu`

### Regles

- Minimum 10 findings pour un projet non trivial
- Inclure au moins 2 points positifs
- Le verdict doit etre justifie par les scores
- Les scores de 0 a 10 doivent refleter l'etat reel (pas de complaisance)
- Les quick_wins sont des actions faisables en moins de 30 minutes

## SORTIE OBLIGATOIRE — NE PAS IGNORER

**OBLIGATION ABSOLUE** : Tu DOIS ecrire le fichier **audit_cr/claude-code.json** sur le disque.
C'est la finalite de cette tache. Sans ce fichier, l'audit est considere comme echoue.

Cree le dossier audit_cr/ s'il n'existe pas.
Utilise les outils d'ecriture de fichier a ta disposition (write_file, fs, echo, etc.).
**ECRASE le fichier s'il existe deja** — c'est un nouveau audit, l'ancien doit etre remplace.

Le fichier doit contenir UNIQUEMENT un objet JSON valide (pas de texte avant/apres, pas de markdown) :

```json
{
  "contexte": {
    "description": "Description concise de l'application : son utilite, ses fonctionnalites majeures et ses utilisateurs cibles. Maximum 4 lignes.",
    "langages": [],
    "frameworks": [],
    "type_projet": "",
    "patterns_detectes": []
  },
  "architecture": {
    "resume": "Description synthetique de l'architecture globale du projet (3-5 phrases)",
    "composants": ["Liste des composants principaux et leur role"],
    "patterns": ["Patterns architecturaux detectes (MVC, microservices, monolithe, event-driven...)"],
    "points_forts": ["Ce qui est bien concu"],
    "points_faibles": ["Ce qui pose probleme ou manque"],
    "diagramme_textuel": "Representation ASCII/textuelle des relations entre composants (optionnel)"
  },
  "documentation_divergences": [
    {
      "document": "Chemin du fichier de documentation concerne",
      "constat": "Ce que dit la documentation",
      "realite": "Ce que montre le code",
      "impact": "Consequence de cette divergence"
    }
  ],
  "infrastructure": {
    "resume": "Description synthetique de l'infrastructure (3-5 phrases)",
    "stack": ["Technologie 1", "Technologie 2"],
    "services_externes": ["Service 1 — role", "Service 2 — role"],
    "flux_donnees": ["Source -> Traitement -> Destination"],
    "deploiement": "Description du mode de deploiement et CI/CD",
    "monitoring": "Description du monitoring et observabilite",
    "secrets_management": "Comment sont geres les secrets",
    "resilience": "Points de defaillance, backups, failover",
    "scalabilite": "Limites et bottlenecks identifies"
  },
  "findings": [
    {
      "categorie": "code|infrastructure",
      "priorite": "critique|important|suggestion|hors_securite",
      "fichier": "chemin/relatif/fichier.ext",
      "ligne": "42",
      "probleme": "Description courte et precise",
      "pourquoi": "Impact concret si non corrige",
      "suggestion": "Solution concrete avec exemple",
      "confiance": "haute|moyenne|faible",
      "contexte_finding": {
        "exposition": "public|authentifie|interne|inconnu",
        "exploitabilite": "triviale|conditionnelle|theorique|inconnu",
        "mitigations": "aucune|partielle|robuste|inconnu",
        "donnees_impactees": "pii_sensibles|metier|publiques|inconnu"
      }
    }
  ],
  "points_positifs": [
    "Point positif 1",
    "Point positif 2"
  ],
  "scores": {
    "securite": 0,
    "performance": 0,
    "architecture": 0,
    "qualite_code": 0,
    "bugs_potentiels": 0,
    "dependances": 0,
    "tests": 0,
    "infrastructure": 0,
    "global": 0
  },
  "resume": {
    "verdict": "Pret|A retravailler|Refactoring necessaire",
    "top_3_actions": [
      "Action prioritaire 1",
      "Action prioritaire 2",
      "Action prioritaire 3"
    ],
    "quick_wins": [
      "Quick win faisable en <30min"
    ],
    "risk_matrix": [
      {"finding": "Description", "impact": "Eleve|Moyen|Faible", "probabilite": "Elevee|Moyenne|Faible"}
    ]
  }
}
```

Les scores sont des nombres de 0 a 10. Langue: francais. Les éléments techniques peuvent rester en anglais.
