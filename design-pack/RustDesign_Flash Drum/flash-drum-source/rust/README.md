# rust/ — thème Skeuo prêt à coller

- **`skeuo_theme.rs`** — toutes les constantes (couleurs `Color32`, radius, tailles, typo) issues de `SPEC-COMPUTED.md`.
- **`skeuo_widgets.rs`** — fonctions de dessin : `pad()` (textures `png/pad-*.png` + overlays playhead/fusion/hors-longueur), `keycap()` (Rest/PressedBlue/PressedAmber), `generate_button()`, `hslider()` (capuchon strié), `led()`, `lcd_frame()`, `well()` — et la recette `vgrad()` qui approxime les dégradés verticaux par bandes (écrite une fois, réutilisée partout).

## Intégration
1. `Cargo.toml` : `egui_extras = { version = "…", features = ["image"] }` (+ `image` crate avec feature `png`).
2. Au démarrage : `egui_extras::install_image_loaders(ctx);`
3. Copier `png/` à côté de `rust/` (les `include_image!` pointent vers `../png/…`) — ou ajuster les chemins.
4. Fonts : IBM Plex Sans + Mono via `FontDefinitions` (github.com/IBM/plex, OFL).

## Compatibilité API
Écrit pour **egui ≥ 0.29** (`CornerRadius`, `StrokeKind`). Sur 0.26–0.28 :
`CornerRadius::same(x as u8)` → `Rounding::same(x)`, supprimer l'argument `StrokeKind`, `Image::corner_radius` → `Image::rounding`, `lerp_to_gamma` existe depuis 0.24.

Non couvert ici (à garder tel quel côté app) : layout des zones (voir `LAYOUT.md` / `egui_layout_reference.rs`), logique des popups. Ce module ne fait que le **look**.
