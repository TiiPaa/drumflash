# Flash Drum — Design Pack

## Pour le designer UI

Ce pack contient tout ce dont vous avez besoin pour redesigner l'interface de Flash Drum.

## Contenu

- **`DESIGN-BRIEF.md`** — Brief complet avec la structure de l'UI, la palette, les problèmes identifiés, et les livrables attendus
- **`ui-screenshots/`** — Dossier pour les captures d'écran (à ajouter manuellement depuis le plugin ouvert dans Studio One)
- **`assets-needed.md`** — Liste des assets graphiques nécessaires

## Comment utiliser ce pack

1. Lire `DESIGN-BRIEF.md` pour comprendre la structure actuelle
2. Ouvrir le plugin VST3 dans un DAW (Studio One, Reaper) pour voir l'UI en action
3. Prendre des captures d'écran des écrans principaux et les mettre dans `ui-screenshots/`
4. Concevoir les maquettes en respectant les contraintes techniques (egui, 1480×800)
5. Fournir les livrables listés dans le brief

## Points d'attention

- **Framework : egui (Rust)** — pas de HTML/CSS, tout est code
- **Pas d'images externes** — tout doit être dessiné avec des primitives (rect, cercle, texte)
- **Performance critique** — 60fps minimum, pas d'effets lourds
- **Dark theme obligatoire**

## Contact

Pour toute question technique sur les contraintes egui, consulter :
- `drum-pattern-vst/src/ui.rs` — code source de l'UI principale
- `drum-pattern-vst/src/ui/design_system.rs` — tokens visuels actuels

---

*Généré le 2026-06-10*
