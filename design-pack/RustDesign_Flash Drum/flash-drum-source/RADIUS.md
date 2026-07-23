# Radius — valeurs exactes (thème Skeuo, px CSS = points egui, échelle 1.0)

Source : `assets/fd-skeuo.css` (+ `fd-base.css` pour ce qui n'est pas surchargé).

## Grille
| Élément | Taille | Radius |
|---|---|---|
| **Pad du grid (.step)** | 44 × 26 (flex) — h 21 en base | **4 px** |
| Bouton nom de lane (.seq__name) | 52 × 21 | **4 px** |
| Tags M / S / T (.tag) | 17 × 17 | **3 px** |
| Mini-sliders Vol / Hum / Push (.minisld) | h 6 | **3 px** |

## Contrôles
| Élément | Radius |
|---|---|
| Slider : piste + fill (.sld, h 5) | **3 px** |
| Slider : poignée (.sld__knob, 12 × 19) | **3 px** |
| Boutons keycap (pages, slots Px, qbtn, chips, onglets, toggles, selects, segmented, GENERATE) | **5 px** |
| Segmented containers (.seg) | **5 px** (boutons internes 5 px, séparés par bord 1px #17171b) |

## Surfaces
| Élément | Radius |
|---|---|
| Panneaux / plaques (.panel) | **7 px** |
| Puits de la grille (.seqwrap, encastré dans la plaque) | **5 px** |
| Écran LCD ADSR (.adsr) | **4 px** |
| Popups (p-lock, menu de lane) | **7 px** |
| Blocs Song (.sblock) | **5 px** |

## Règle générale
3 px (micro : tags, sliders) → 4 px (pads, touches, écran) → 5 px (keycaps, puits, blocs) → 7 px (plaques, popups). Bordures : 1 px partout.
