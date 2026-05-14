# Mise a jour - 2026-05-14

## Etat de fin de session

Plugin VST3 Rust installe avec le build:

```text
20260514-220658
```

## Nouveautes de la session

### Synthese — anti-click & enveloppes
- Refonte de l'enveloppe d'amplitude : `DecayReleaseEnvelope` bi-stage
  (decay rapide + release tail), avec courbes `decay_curve` et
  `release_curve` reglables. Le retrigger est persistant
  (`trigger_at_peak`) — au lieu de jumper, l'envelope rampe doucement
  vers la cible. Resout les clics au retrigger sur queue ringing.
- Phase **Hold** entre attaque et decay sur Snare / HiHat / Open HH /
  Snare 606. Slider visible dans le panneau Sound Settings, range 0..0.5 s.
- Toutes les voix retirent les `osc.reset()`, `filter.reset()` et
  `noise.reseed()` au trigger pour preserver l'etat analogique
  (continuite de phase, pas de transient parasite).
- Kick : pitch additif persistant (au lieu de multiplicatif) +
  `OnePoleSmoother` sur la frequence + `DcBlocker` en sortie. Elimine
  le saut de phase / cutoff au retrigger.
- Velocity smoother par voix : absorbe les sauts de gain hit-a-hit
  (humanize, velocity layers).

### Clap retravaille
- Bandpass resserre, couple au filter slider (HP * 2.5).
- Snap transient HP-filtre (3.5 kHz) pour caractere "papier / sec".
- 4 bursts a timing irregulier (0/10/25/50 ms), LP shift par burst
  pour la diffusion / petite salle.
- Snap re-trigger a chaque burst (pas seulement au premier) — chaque
  echo est percu comme un impact distinct.
- Nouveau slider **Echo** (0..3) qui scale les timings et la diffusion.

### 11e instrument : Snare 606 (TR-606 grey-box)
- Modele analogique du circuit TR-606 : white noise → LP softener (3 kHz)
  → Swing-VCA envelope → bridged-T resonator (Biquad bandpass) → mix
  avec dry noise (snare wires).
- Voix dediee dans `synthesis/snare606.rs` avec son fichier propre.
- 11e ligne dans la grille du sequencer (label `S6`).
- 3 parametres dedies : Resonance (Q 0.5..12), Tone (LP color), Snap
  (wires balance). Plus les controles standards (Frequency, Decay,
  Hold, Release, etc.).
- Layout VST3 : maintenu a 10 sorties aux pour preserver la
  compatibilite des sessions DAW existantes. La voix est mixee dans le
  Main Mix uniquement.

### Bugs critiques fixes
- Crash a l'instanciation avec 11e voix : `IntRange::Linear { min:0, max:0 }`
  sur `algo_snare606` → division par zero dans nih-plug.
- Index out of bounds dans UI : 3 arrays (`hums`, `pushes`, `lengths`)
  de taille 10 alors que `INSTRUMENT_LABELS` est passe a 11.
- Step mask hardcode a `0x3ff` (10 bits) — empechait l'activation des
  steps sur la 11e voix. Derive maintenant de `INSTRUMENT_COUNT`.

## Build installe courant

- Build UI: `20260514-220658`
- VST3 class ID: `DrumFlashPlugin1`
- Binaire installe: `C:\Program Files\Common Files\VST3\drum-pattern-vst.vst3\Contents\x86_64-win\drum-pattern-vst.vst3`

## Commande de build recommandee

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
.\build.ps1 -Install
```

---

# Mise a jour - 2026-05-13

## Etat de fin de session

Le plugin VST3 Rust est installe avec le build:

```text
20260513-202946
```

Le numero de build est affiche dans l'interface du plugin pour verifier que Studio One charge bien le binaire attendu.

## Nouveautes de la session

### Synthese modulaire
- Architecture `Voice` modulaire avec `set_algo()` et `set_special_param()`.
- Kick: 3 algorithmes (Sine, Square, FM) + click transient controle par slider.
- Snare: 3 algorithmes (Synth, Noise, Layered) + parametre `snap` (osc/noise ratio).
- Nouvelles voix: Clap, Ride, Cymbal (synthese dediee via `dsp.rs`).
- Registre des algos et special params dans `special_params.rs`.

### Sequencer avance
- Moteur de groove: Straight, Swing 16th, Shuffle, MPC Style.
- Push/pull par instrument (-50ms a +50ms).
- Humanize par instrument (intensite aleatoire de velocite).
- Generateurs de pattern: Euclidean, Markov, Probabilistic.
- Export MIDI vers `Documents/Drum Flash/exports/`.

### UI amelioree
- BoolParam -> checkbox (`hihat_chokes_oh`).
- EnumParam -> combobox (`groove_type`, `generator_type`, `style_primary/secondary`).
- IntParam algo -> combobox avec noms (`Sine`, `Square`, `FM`...).
- Panneau de synthese par instrument avec frequence, decay, volume, filter, algo, special params.

### Corrections
- Defaults de decay: Snare 0.47, HiHat 0.36, Open HH 0.66.
- Hi-Hat Choke: option pour couper Open HH quand Closed HH trigger.
- Step skips rares: `sync_to_host` ne se declenche plus qu'au play start ou en cas de seek > 0.2 beat.

## Commande de build recommandee

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
.\build.ps1 -Install
```

## Build installe courant

- Build UI: `20260513-202946`
- VST3 class ID: `DrumFlashPlugin1`
- Binaire installe: `C:\Program Files\Common Files\VST3\drum-pattern-vst.vst3\Contents\x86_64-win\drum-pattern-vst.vst3`

---

# Mise a jour - 2026-05-11

## Etat de fin de session

Le plugin VST3 Rust est installe avec le build:

```text
20260511-091259
```

Le numero de build est affiche dans l'interface du plugin pour verifier que Studio One charge bien le binaire attendu.

## Corrections importantes

- Sync DAW validee: play, stop, tempo, repositionnement.
- Interface egui utilisable avec grille 16x7 editable.
- Presets Rock, Funk et Disco branches.
- Mutes et solos par instrument branches.
- Multi-out Studio One fonctionnel.
- Diagnostic de sauvegarde/reouverture Studio One effectue.
- Parametres classiques (`master_vol`, `bpm`, mutes/solos) sauvegardes/restaures.
- Probleme de grille isole: les anciens parametres caches `st01` a `st16` ne changeaient pas lors des clics.
- Grille maintenant sauvegardee via le champ persistant `pattern-v1`, directement depuis `SharedPattern`.
- Migration ajoutee depuis les anciens etats `st01` a `st16` vers `pattern-v1`.

## Correctif multi-out Studio One

Le multi-out a ete debloque par le patch `nih-plug` vendore dans:

```powershell
drum-pattern-vst\vendor\nih-plug\src\wrapper\vst3\wrapper.rs
```

Points requis pour Studio One:

- `get_unit_by_bus()` doit retourner le root unit pour les bus audio/event valides.
- `set_bus_arrangements()` doit accepter les activations progressives des sorties.
- `set_bus_arrangements()` doit accepter un pointeur d'entree audio nul quand `num_ins == 0`.
- La validation des buffers doit ignorer les sorties auxiliaires non activees.
- `getRoutingInfo()` doit relier l'entree event/MIDI instrument a la sortie audio principale.
- L'etat VST3 est sauvegarde/restaure cote `IComponent` et `IEditController`.

Le build courant utilise `VST3_CLASS_ID = DrumFlashPlugin1`.

## Correctif sauvegarde/reouverture

Les 16 pas du pattern sont maintenant serialises dans le champ persistant `pattern-v1`.
Ce champ lit directement `SharedPattern`, qui est l'etat reel utilise par l'UI et le sequenceur.
Les anciens parametres caches `st01` a `st16` restent disponibles pour migrer les songs deja
sauvegardees avant ce correctif.

Tests valides:

```text
cargo test: 16 tests OK
```

Binaire installe:

```text
C:\Program Files\Common Files\VST3\drum-pattern-vst.vst3
SHA-256: 62AA5FCC445FEFDBC1E30196E614BCAED53A61C9F9EB2AB9BD5A4E1C5C510CEF
```

## Commande de build recommandee

```powershell
cd "E:\Dev\Projets\Drum Flash\drum-pattern-vst"
.\build.ps1 -Install
```

Le build utilise la dependance vendoree `drum-pattern-vst/vendor/nih-plug`; il ne modifie plus le
checkout Cargo global.
