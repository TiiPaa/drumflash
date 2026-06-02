/**
 * Flash Drum — Source de vérité des paramètres par catégorie d'instrument
 *
 * Ce fichier définit le schéma de chaque instrument (sections + ParamSpec).
 * L'éditeur de son le consomme pour se reconstruire dynamiquement.
 * 🔒 Ne jamais coder de paramètre en dur dans le layout — toujours parcourir ce schéma.
 */

const CtlKind = {
  Slider: 'slider',
  Select: 'select',
  Switch: 'switch',
};

// ============================================================
// Paramètres standards (réutilisables dans plusieurs sections)
// ============================================================

const STD_VOLUME = {
  label: 'Volume', key: 'volume', kind: CtlKind.Slider,
  min: 0, max: 2, step: 0.01, default: 0.8, unit: ''
};

const STD_PAN = {
  label: 'Pan', key: 'pan', kind: CtlKind.Slider,
  min: -1, max: 1, step: 0.01, default: 0, unit: ''
};

const STD_ATTACK = {
  label: 'Attack', key: 'attack_ms', kind: CtlKind.Slider,
  min: 0, max: 500, step: 1, default: 1.5, unit: 'ms'
};

const STD_DECAY = {
  label: 'Decay', key: 'decay', kind: CtlKind.Slider,
  min: 0.01, max: 2, step: 0.01, default: 0.3, unit: 's'
};

const STD_RELEASE = {
  label: 'Release', key: 'release', kind: CtlKind.Slider,
  min: 0.01, max: 2, step: 0.01, default: 0.2, unit: 's'
};

const STD_SUSTAIN = {
  label: 'Sustain', key: 'sustain', kind: CtlKind.Slider,
  min: 0, max: 1, step: 0.01, default: 0, unit: ''
};

const STD_CUTOFF = {
  label: 'Cutoff', key: 'filter_freq', kind: CtlKind.Slider,
  min: 20, max: 20000, step: 1, default: 8000, unit: 'Hz', log: true
};

const STD_RESONANCE = {
  label: 'Resonance', key: 'filter_res', kind: CtlKind.Slider,
  min: 0, max: 1, step: 0.01, default: 0, unit: ''
};

const STD_DRIVE = {
  label: 'Drive', key: 'sat_drive', kind: CtlKind.Slider,
  min: 0, max: 20, step: 0.1, default: 0, unit: 'x'
};

const STD_SAT_TYPE = {
  label: 'Type', key: 'sat_type', kind: CtlKind.Select,
  min: 0, max: 4, step: 1, default: 0, unit: '',
  options: ['SoftClip', 'Valve', 'Transistor', 'HardClip', 'Tape']
};

const STD_SAT_MIX = {
  label: 'Mix', key: 'sat_mix', kind: CtlKind.Slider,
  min: 0, max: 1, step: 0.01, default: 1, unit: ''
};

const STD_OUTPUT_GAIN = {
  label: 'Output Gain', key: 'output_gain', kind: CtlKind.Slider,
  min: 0, max: 2, step: 0.01, default: 1, unit: ''
};

const STD_MIX_BUS = {
  label: 'Mix Bus', key: 'mix_bus', kind: CtlKind.Switch,
  min: 0, max: 1, step: 1, default: 1, unit: ''
};

// ============================================================
// Sections communes à tous les instruments
// ============================================================

const COMMON_SECTIONS = {
  level: {
    title: 'Level',
    cols: 2,
    items: [STD_VOLUME, STD_PAN],
    has_adsr: false,
  },
  envelope: {
    title: 'Envelope',
    cols: 4,
    items: [STD_ATTACK, STD_DECAY, STD_SUSTAIN, STD_RELEASE],
    has_adsr: true,
  },
  filter: {
    title: 'Filter',
    cols: 2,
    items: [STD_CUTOFF, STD_RESONANCE],
    has_adsr: false,
  },
  saturation: {
    title: 'Saturation',
    cols: 3,
    items: [STD_SAT_TYPE, STD_DRIVE, STD_SAT_MIX],
    has_adsr: false,
  },
  output: {
    title: 'Output',
    cols: 2,
    items: [STD_OUTPUT_GAIN, STD_MIX_BUS],
    has_adsr: false,
  },
};

// ============================================================
// Sections source par catégorie
// ============================================================

const SOURCE_KICK = {
  title: 'Oscillator',
  cols: 3,
  items: [
    { label: 'Frequency', key: 'frequency', kind: CtlKind.Slider,
      min: 20, max: 200, step: 1, default: 60, unit: 'Hz' },
    { label: 'Click', key: 'click', kind: CtlKind.Slider,
      min: 0, max: 1, step: 0.01, default: 0.5, unit: '' },
    { label: 'Algorithm', key: 'algo', kind: CtlKind.Select,
      min: 0, max: 3, step: 1, default: 0, unit: '',
      options: ['Sine', 'Triangle', 'Saw', 'Square'] },
  ],
  has_adsr: false,
};

const SOURCE_TOM = {
  title: 'Oscillator',
  cols: 4,
  items: [
    { label: 'Frequency', key: 'frequency', kind: CtlKind.Slider,
      min: 40, max: 400, step: 1, default: 120, unit: 'Hz' },
    { label: 'Click', key: 'click', kind: CtlKind.Slider,
      min: 0, max: 1, step: 0.01, default: 0.3, unit: '' },
    { label: 'Algorithm', key: 'algo', kind: CtlKind.Select,
      min: 0, max: 3, step: 1, default: 0, unit: '',
      options: ['Sine', 'Triangle', 'Saw', 'Square'] },
    { label: 'Pitch Bend', key: 'pitch_bend', kind: CtlKind.Slider,
      min: 0, max: 1, step: 0.01, default: 0.2, unit: '' },
  ],
  has_adsr: false,
};

const SOURCE_SNARE = {
  title: 'Body + Noise',
  cols: 4,
  items: [
    { label: 'Tone Freq', key: 'tone_freq', kind: CtlKind.Slider,
      min: 100, max: 1000, step: 1, default: 250, unit: 'Hz' },
    { label: 'Noise Mix', key: 'noise_mix', kind: CtlKind.Slider,
      min: 0, max: 1, step: 0.01, default: 0.5, unit: '' },
    { label: 'Snap', key: 'snap', kind: CtlKind.Slider,
      min: 0, max: 1, step: 0.01, default: 0.6, unit: '' },
    { label: 'Body', key: 'body_type', kind: CtlKind.Select,
      min: 0, max: 2, step: 1, default: 0, unit: '',
      options: ['Synth', 'Noise', 'Layered'] },
  ],
  has_adsr: false,
};

const SOURCE_HAT = {
  title: 'Metal / Noise',
  cols: 4,
  items: [
    { label: 'Tone', key: 'tone', kind: CtlKind.Slider,
      min: 0, max: 1, step: 0.01, default: 0.5, unit: '' },
    { label: 'Decay', key: 'hat_decay', kind: CtlKind.Slider,
      min: 0.01, max: 1, step: 0.01, default: 0.15, unit: 's' },
    { label: 'Color', key: 'color', kind: CtlKind.Slider,
      min: 0, max: 1, step: 0.01, default: 0.5, unit: '' },
    { label: 'Shimmer', key: 'shimmer', kind: CtlKind.Slider,
      min: 0, max: 1, step: 0.01, default: 0, unit: '' },
  ],
  has_adsr: false,
};

const SOURCE_CYMBAL = {
  title: 'Metal / Noise',
  cols: 4,
  items: [
    { label: 'Tone', key: 'tone', kind: CtlKind.Slider,
      min: 0, max: 1, step: 0.01, default: 0.5, unit: '' },
    { label: 'Decay', key: 'cymbal_decay', kind: CtlKind.Slider,
      min: 0.1, max: 4, step: 0.01, default: 1.2, unit: 's' },
    { label: 'Color', key: 'color', kind: CtlKind.Slider,
      min: 0, max: 1, step: 0.01, default: 0.5, unit: '' },
    { label: 'Shimmer', key: 'shimmer', kind: CtlKind.Slider,
      min: 0, max: 1, step: 0.01, default: 0.3, unit: '' },
  ],
  has_adsr: false,
};

const SOURCE_CLAP = {
  title: 'Clap Engine',
  cols: 3,
  items: [
    { label: 'Pitch', key: 'clap_pitch', kind: CtlKind.Slider,
      min: 0.5, max: 2, step: 0.01, default: 1, unit: '' },
    { label: 'Spread', key: 'clap_spread', kind: CtlKind.Slider,
      min: 0, max: 1, step: 0.01, default: 0.3, unit: '' },
    { label: 'Count', key: 'clap_count', kind: CtlKind.Slider,
      min: 1, max: 8, step: 1, default: 3, unit: '' },
  ],
  has_adsr: false,
};

const SOURCE_PERC = {
  title: 'Source',
  cols: 3,
  items: [
    { label: 'Pitch', key: 'pitch', kind: CtlKind.Slider,
      min: 20, max: 2000, step: 1, default: 440, unit: 'Hz', log: true },
    { label: 'Decay', key: 'perc_decay', kind: CtlKind.Slider,
      min: 0.01, max: 1, step: 0.01, default: 0.2, unit: 's' },
    { label: 'Noise', key: 'noise_amount', kind: CtlKind.Slider,
      min: 0, max: 1, step: 0.01, default: 0, unit: '' },
  ],
  has_adsr: false,
};

// ============================================================
// Assemblage par catégorie
// ============================================================

const SCHEMAS = {
  kick: [
    SOURCE_KICK,
    COMMON_SECTIONS.level,
    COMMON_SECTIONS.envelope,
    COMMON_SECTIONS.filter,
    COMMON_SECTIONS.saturation,
    COMMON_SECTIONS.output,
  ],
  tom: [
    SOURCE_TOM,
    COMMON_SECTIONS.level,
    COMMON_SECTIONS.envelope,
    COMMON_SECTIONS.filter,
    COMMON_SECTIONS.saturation,
    COMMON_SECTIONS.output,
  ],
  snare: [
    SOURCE_SNARE,
    COMMON_SECTIONS.level,
    COMMON_SECTIONS.envelope,
    COMMON_SECTIONS.filter,
    COMMON_SECTIONS.saturation,
    COMMON_SECTIONS.output,
  ],
  hat: [
    SOURCE_HAT,
    COMMON_SECTIONS.level,
    COMMON_SECTIONS.envelope,
    // Pas de filter LP sur hat → HP natif dans le moteur
    COMMON_SECTIONS.saturation,
    COMMON_SECTIONS.output,
  ],
  cymbal: [
    SOURCE_CYMBAL,
    COMMON_SECTIONS.level,
    COMMON_SECTIONS.envelope,
    // Pas de filter LP sur cymbal → HP natif
    COMMON_SECTIONS.saturation,
    COMMON_SECTIONS.output,
  ],
  clap: [
    SOURCE_CLAP,
    COMMON_SECTIONS.level,
    COMMON_SECTIONS.envelope,
    COMMON_SECTIONS.filter,
    COMMON_SECTIONS.saturation,
    COMMON_SECTIONS.output,
  ],
  perc: [
    SOURCE_PERC,
    COMMON_SECTIONS.level,
    COMMON_SECTIONS.envelope,
    COMMON_SECTIONS.filter,
    COMMON_SECTIONS.saturation,
    COMMON_SECTIONS.output,
  ],
};

// ============================================================
// Mapping instrument → catégorie
// ============================================================

const INSTRUMENT_CATEGORIES = {
  BD: 'kick',
  SD: 'snare',
  HH: 'hat',
  OH: 'hat',
  T1: 'tom',
  T2: 'tom',
  T3: 'tom',
  CL: 'clap',
  RD: 'cymbal',
  CY: 'cymbal',
  S6: 'perc',
  B8: 'perc',
  P1: 'perc',
};

// ============================================================
// API publique
// ============================================================

/**
 * Retourne le schéma complet (sections + ParamSpec) pour une catégorie.
 * @param {string} category — 'kick' | 'tom' | 'snare' | 'hat' | 'cymbal' | 'clap' | 'perc'
 * @returns {Array<Section>}
 */
function schemaFor(category) {
  return SCHEMAS[category] || SCHEMAS.perc;
}

/**
 * Retourne le schéma pour un instrument par son code (BD, SD, HH...).
 * @param {string} instrumentCode
 * @returns {Array<Section>}
 */
function schemaForInstrument(instrumentCode) {
  const cat = INSTRUMENT_CATEGORIES[instrumentCode];
  return schemaFor(cat);
}

// Export pour module ou usage global
if (typeof module !== 'undefined' && module.exports) {
  module.exports = { schemaFor, schemaForInstrument, CtlKind, SCHEMAS, INSTRUMENT_CATEGORIES };
}
