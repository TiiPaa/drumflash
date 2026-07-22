# Skins — Flash Drum

3 skins intégrés, switchables à l'exécution (Settings → Skin). Toutes les couleurs de l'UI passent par ces tokens ; aucune couleur n'est codée en dur dans l'interface. Version JSON exploitable : `skins.json`.

Rôles des tokens :

- **Surfaces** : `bg` (fond), `panel` (panneaux), `panel2` (contrôles), `panel3`, `p_hover`, `p_active` (états), `line`, `line2` (bordures)
- **Accents** : `blue` (accent principal : playhead, actif, boutons), `green` (solo/loaded), `red` (erreurs), `amber` (mute)
- **P-locks** : `pl_link` (sound p-lock actif), `pl_link_dim` / `pl_snap_dim` (p-lock sur step inactive), `seqpl` / `seqpl_dim` (sequencer p-lock)
- **Texte** : `ink`, `ink2`, `ink3`, `faint`
- **Cellules grille** : `cell_empty_beat`, `cell_empty_off`, `cell_current`, `cell_disabled`, `fusion_fill`, `cell_seqpl_off`, `cell_pl_snap_off`, `cell_pl_link_off`, `song_empty`
- **Feedback** : `danger` / `danger_dim` / `danger_soft` (actions destructives), `drag_target`, `handle` (poignées sliders), `mute_fill`, `solo_fill`
- **Graphes** : `envelope_bg`, `envelope_curve`

---

## Dark (défaut — palette d'origine)

| Token | RGB | Token | RGB |
|---|---|---|---|
| bg | 10, 10, 15 | ink | 232, 232, 240 |
| panel | 20, 20, 25 | ink2 | 156, 163, 175 |
| panel2 | 28, 28, 36 | ink3 | 107, 114, 128 |
| panel3 | 24, 24, 30 | faint | 75, 85, 99 |
| p_hover | 36, 36, 48 | blue | 74, 158, 255 |
| p_active | 42, 42, 56 | green | 74, 222, 128 |
| line | 42, 42, 53 | red | 248, 113, 113 |
| line2 | 58, 58, 72 | amber | 251, 191, 36 |
| pl_link | 255, 140, 0 | pl_link_dim | 180, 100, 0 |
| pl_snap_dim | 160, 30, 30 | seqpl | 168, 85, 247 |
| seqpl_dim | 120, 60, 180 | | |
| cell_empty_beat | 35, 35, 44 | cell_empty_off | 27, 27, 34 |
| cell_current | 48, 48, 60 | cell_disabled | 10, 10, 14 |
| fusion_fill | 20, 34, 58 | cell_seqpl_off | 28, 18, 48 |
| cell_pl_snap_off | 36, 16, 16 | cell_pl_link_off | 36, 26, 8 |
| song_empty | 18, 18, 24 | | |
| danger | 255, 80, 80 | danger_dim | 180, 60, 60 |
| danger_soft | 255, 120, 120 | drag_target | 255, 200, 80 |
| handle | 238, 242, 248 | mute_fill | 26, 18, 6 |
| solo_fill | 6, 32, 15 | envelope_bg | 12, 12, 17 |
| envelope_curve | 255, 160, 60 | | |

## Midnight

| Token | RGB | Token | RGB |
|---|---|---|---|
| bg | 8, 10, 18 | ink | 226, 232, 240 |
| panel | 14, 18, 30 | ink2 | 148, 163, 184 |
| panel2 | 20, 26, 40 | ink3 | 100, 116, 139 |
| panel3 | 18, 23, 36 | faint | 71, 85, 105 |
| p_hover | 26, 34, 52 | blue | 96, 165, 250 |
| p_active | 32, 42, 62 | green | 74, 222, 128 |
| line | 30, 38, 56 | red | 248, 113, 113 |
| line2 | 40, 50, 72 | amber | 251, 191, 36 |
| pl_link | 255, 150, 60 | pl_link_dim | 190, 110, 30 |
| pl_snap_dim | 170, 50, 60 | seqpl | 150, 110, 250 |
| seqpl_dim | 105, 80, 185 | | |
| cell_empty_beat | 28, 34, 48 | cell_empty_off | 22, 27, 40 |
| cell_current | 44, 54, 74 | cell_disabled | 8, 10, 16 |
| fusion_fill | 18, 32, 60 | cell_seqpl_off | 24, 20, 52 |
| cell_pl_snap_off | 40, 18, 22 | cell_pl_link_off | 40, 26, 12 |
| song_empty | 14, 17, 26 | | |
| danger | 255, 90, 90 | danger_dim | 170, 60, 60 |
| danger_soft | 255, 130, 130 | drag_target | 250, 204, 21 |
| handle | 226, 232, 240 | mute_fill | 30, 22, 10 |
| solo_fill | 8, 34, 20 | envelope_bg | 10, 12, 20 |
| envelope_curve | 96, 165, 250 | | |

## Ember

| Token | RGB | Token | RGB |
|---|---|---|---|
| bg | 16, 10, 8 | ink | 245, 235, 225 |
| panel | 26, 16, 12 | ink2 | 180, 160, 145 |
| panel2 | 36, 22, 16 | ink3 | 135, 118, 105 |
| panel3 | 30, 19, 14 | faint | 100, 88, 78 |
| p_hover | 48, 30, 20 | blue | 251, 146, 60 |
| p_active | 58, 38, 26 | green | 134, 239, 172 |
| line | 52, 34, 24 | red | 248, 113, 113 |
| line2 | 70, 46, 32 | amber | 251, 191, 36 |
| pl_link | 255, 120, 40 | pl_link_dim | 190, 90, 20 |
| pl_snap_dim | 170, 40, 30 | seqpl | 217, 120, 239 |
| seqpl_dim | 150, 85, 170 | | |
| cell_empty_beat | 44, 32, 26 | cell_empty_off | 34, 26, 22 |
| cell_current | 70, 52, 40 | cell_disabled | 14, 10, 8 |
| fusion_fill | 50, 32, 20 | cell_seqpl_off | 46, 24, 44 |
| cell_pl_snap_off | 44, 18, 16 | cell_pl_link_off | 48, 30, 14 |
| song_empty | 26, 17, 14 | | |
| danger | 255, 90, 80 | danger_dim | 175, 60, 50 |
| danger_soft | 255, 130, 115 | drag_target | 251, 191, 36 |
| handle | 245, 235, 225 | mute_fill | 40, 24, 8 |
| solo_fill | 10, 36, 18 | envelope_bg | 18, 12, 10 |
| envelope_curve | 251, 146, 60 | | |
