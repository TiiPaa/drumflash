/* Flash Drum — shared data model v3 (modular engines + 14 assignable lanes). window.FD */
(function () {
  const STEPS = 64;
  const PAGE = 16;
  const LANE_COUNT = 14;

  // ---- step model --------------------------------------------------------
  const blankLane = () => ({
    hits: new Array(STEPS).fill(0),
    plock: new Array(STEPS).fill('none'),
    seq: new Array(STEPS).fill(0),
    fusion: [],
  });
  const lane = (hits, opts = {}) => {
    const L = blankLane();
    hits.forEach(i => { L.hits[i - 1] = 1; });
    (opts.link || []).forEach(i => { L.hits[i - 1] = 1; L.plock[i - 1] = 'link'; });
    (opts.snap || []).forEach(i => { L.hits[i - 1] = 1; L.plock[i - 1] = 'snapshot'; });
    (opts.seq || []).forEach(i => { L.seq[i - 1] = 1; });
    (opts.fusion || []).forEach(f => L.fusion.push(f));
    return L;
  };

  // ---- control spec helpers ---------------------------------------------
  const S = (label, key, o = {}) => ({ kind: 'slider', label, key,
    min: o.min ?? 0, max: o.max ?? 1, step: o.step ?? 0.01, value: o.value ?? 0.5,
    unit: o.unit ?? '', fmt: o.fmt });
  const FREQ = (label, key, o = {}) => ({ ...S(label, key, o), kind: 'freq' });
  const SEL = (label, key, options, value) => ({ kind: 'select', label, key, options, value });
  const SW = (label, key, value = false) => ({ kind: 'switch', label, key, value });
  const f2 = v => v.toFixed(2);
  const f1 = v => v.toFixed(1);
  const f4 = v => v.toFixed(4);
  const SAMPLES = ['808 Kick', '909 Kick', 'Vinyl Kick', 'Clap 1', 'Rimshot', 'Cowbell', 'Vox Chop', 'Foley Hit'];

  // ---- shared sections ---------------------------------------------------
  const volSection = { title: '', kind: 'volume', items: [
    S('Volume', 'vol', { min: -60, max: 6, step: 0.1, value: 3.2, unit: 'dB', fmt: f1 }),
  ] };
  const envSection = { title: 'Envelope', adsr: true, items: [
    S('Attack', 'attack', { min: 0, max: 4, step: 0.01, value: 0.01, fmt: f2 }),
    S('Decay', 'decay', { min: 0, max: 4, step: 0.01, value: 0.71, fmt: f2 }),
    S('Decay Curve', 'decayC', { min: 0, max: 8, step: 0.01, value: 5.43, fmt: f2 }),
    S('Release', 'release', { min: 0, max: 4, step: 0.01, value: 1.05, fmt: f2 }),
    S('Release Curve', 'relC', { min: 0, max: 8, step: 0.01, value: 3.77, fmt: f2 }),
  ] };
  const satSection = { title: 'Saturation', items: [
    SEL('Saturation Type', 'satType', ['None', 'Tape', 'Tube', 'Hard', 'Fold'], 'None'),
    S('Saturation Amount', 'satAmt', { value: 0.39, fmt: v => v.toFixed(3) }),
    S('Saturation Mix', 'satMix', { value: 0.21, fmt: v => v.toFixed(3) }),
    S('Saturation Output Gain', 'satGain', { min: 0, max: 2, step: 0.01, value: 1, fmt: f2 }),
    SW('Saturation Pre-Filter', 'satPre', false),
  ] };
  const outSection = { title: 'Output', items: [
    S('Analog', 'analog', { min: 0, max: 1, step: 0.01, value: 0, fmt: f2 }),
    SW('Mix', 'mix', true),
  ] };
  const filterLP = { title: 'Filter', items: [ S('Filter (LP)', 'filter', { min: 20, max: 18000, step: 0.1, value: 757.1, fmt: f2 }) ] };
  const filterHP = { title: 'Filter', items: [ S('Filter (HP)', 'filter', { min: 20, max: 18000, step: 0.1, value: 320, fmt: f2 }) ] };

  function oscFor(cat) {
    if (cat === 'kick') return { title: 'Oscillator', items: [
      FREQ('Frequency', 'freq', { min: 20, max: 2000, step: 0.01, value: 175.57, fmt: f2 }),
      S('Click Level', 'clickLvl', { value: 0.515, fmt: f4 }),
      SEL('Click Type', 'clickType', ['Soft', 'Medium', 'Hard'], 'Hard'),
      SEL('Algorithm', 'algo', ['Sine', 'Triangle', 'Saw', 'Square', 'FM'], 'Sine'),
    ] };
    if (cat === 'tom') return { title: 'Oscillator', items: [
      FREQ('Frequency', 'freq', { min: 20, max: 2000, step: 0.01, value: 140, fmt: f2 }),
      S('Pitch Bend', 'bend', { value: 0.3, fmt: f2 }),
      S('Click Level', 'clickLvl', { value: 0.3, fmt: f4 }),
      SEL('Algorithm', 'algo', ['Sine', 'Triangle', 'FM'], 'Sine'),
    ] };
    if (cat === 'snare') return { title: 'Oscillator', items: [
      FREQ('Tone Freq', 'freq', { min: 80, max: 1200, step: 0.01, value: 238, fmt: f2 }),
      S('Noise Mix', 'noise', { value: 0.6, fmt: f2 }),
      S('Snap', 'snap', { value: 0.45, fmt: f2 }),
      SEL('Body', 'algo', ['Sine', 'Triangle', 'Noise'], 'Triangle'),
    ] };
    if (cat === 'hat' || cat === 'cymbal') return { title: 'Oscillator', items: [
      S('Tone', 'tone', { value: 0.62, fmt: f2 }),
      S('Metal Decay', 'mdecay', { value: cat === 'hat' ? 0.25 : 0.7, fmt: f2 }),
      S('Color', 'color', { value: 0.5, fmt: f2 }),
      S('Shimmer', 'shimmer', { value: 0.4, fmt: f2 }),
    ] };
    if (cat === 'clap') return { title: 'Oscillator', items: [
      FREQ('Pitch', 'freq', { min: 80, max: 1500, step: 0.01, value: 520, fmt: f2 }),
      S('Spread', 'spread', { value: 0.5, fmt: f2 }),
      S('Count', 'count', { min: 1, max: 8, step: 1, value: 4, fmt: v => '' + Math.round(v) }),
    ] };
    return { title: 'Oscillator', items: [
      FREQ('Pitch', 'freq', { min: 40, max: 2000, step: 0.01, value: 660, fmt: f2 }),
      S('Decay', 'mdecay', { value: 0.4, fmt: f2 }),
      S('Noise', 'noise', { value: 0.3, fmt: f2 }),
    ] };
  }

  // ---- engine registry ---------------------------------------------------
  // Each engine = a parameter schema (array of sections). The editor renders
  // whatever schema the assigned engine returns — fully dynamic.
  const synth = cat => () => [ volSection, oscFor(cat), envSection,
    (cat === 'hat' || cat === 'cymbal') ? filterHP : filterLP, satSection, outSection ];

  function schemaSample() {
    return [
      volSection,
      { title: 'Sample', items: [
        SEL('Sample', 'sample', SAMPLES, SAMPLES[0]),
        S('Start', 'start', { value: 0, fmt: f2 }),
        S('End', 'end', { value: 1, fmt: f2 }),
        SW('Reverse', 'reverse', false),
        SW('Loop', 'loop', false),
      ] },
      { title: 'Pitch', items: [
        S('Tune', 'tune', { min: -24, max: 24, step: 1, value: 0, unit: 'st', fmt: v => (v > 0 ? '+' : '') + Math.round(v) }),
        S('Fine', 'fine', { min: -100, max: 100, step: 1, value: 0, unit: 'ct', fmt: v => (v > 0 ? '+' : '') + Math.round(v) }),
        SW('Key Track', 'keytrack', true),
      ] },
      envSection, filterLP, satSection, outSection,
    ];
  }
  function schemaSampleFx() {
    return [
      volSection,
      { title: 'Sample', items: [
        SEL('Sample', 'sample', SAMPLES, SAMPLES[6]),
        S('Start', 'start', { value: 0, fmt: f2 }),
        SW('Choke', 'choke', true),
      ] },
      { title: 'Shape', items: [
        S('Decay', 'decay', { min: 0, max: 4, step: 0.01, value: 0.6, fmt: f2 }),
        S('Tone', 'tone', { value: 0.5, fmt: f2 }),
        S('Drive', 'drive', { value: 0.2, fmt: f2 }),
      ] },
      filterLP, outSection,
    ];
  }
  function schemaMidi() {
    const NOTES = ['C1','C2','C3','C#3','D3','E3','F3','G3','A3','C4'];
    return [
      { title: 'MIDI', items: [
        S('Channel', 'channel', { min: 1, max: 16, step: 1, value: 10, fmt: v => Math.round(v) }),
        SEL('Note', 'note', NOTES, 'C3'),
        S('Velocity', 'velocity', { min: 1, max: 127, step: 1, value: 100, fmt: v => Math.round(v) }),
        S('Gate', 'gate', { min: 0, max: 1, step: 0.01, value: 0.5, fmt: f2 }),
      ] },
      { title: 'Modulation', items: [
        S('CC Number', 'ccNum', { min: 0, max: 127, step: 1, value: 74, fmt: v => Math.round(v) }),
        S('CC Value', 'ccVal', { min: 0, max: 127, step: 1, value: 64, fmt: v => Math.round(v) }),
      ] },
    ];
  }

  const ENGINES = {
    'synth-kick':   { label: 'Kick Synth',   group: 'Synth',   build: synth('kick') },
    'synth-snare':  { label: 'Snare Synth',  group: 'Synth',   build: synth('snare') },
    'synth-tom':    { label: 'Tom Synth',    group: 'Synth',   build: synth('tom') },
    'synth-hat':    { label: 'Hat Synth',    group: 'Synth',   build: synth('hat') },
    'synth-cymbal': { label: 'Cymbal Synth', group: 'Synth',   build: synth('cymbal') },
    'synth-clap':   { label: 'Clap Synth',   group: 'Synth',   build: synth('clap') },
    'synth-perc':   { label: 'Perc Synth',   group: 'Synth',   build: synth('perc') },
    'sample':       { label: 'Sample',       group: 'Sampler', build: schemaSample },
    'samplefx':     { label: 'Sample FX',    group: 'Sampler', build: schemaSampleFx },
    'midi':         { label: 'MIDI Out',     group: 'MIDI',    build: schemaMidi },
  };
  function schemaForEngine(type) { return type && ENGINES[type] ? ENGINES[type].build() : []; }
  function engineLabel(type) { return type && ENGINES[type] ? ENGINES[type].label : 'Empty'; }
  function engineList() {
    const groups = {};
    Object.keys(ENGINES).forEach(type => {
      const g = ENGINES[type].group;
      (groups[g] = groups[g] || []).push({ type, label: ENGINES[type].label });
    });
    return groups; // { Synth:[...], Sampler:[...], MIDI:[...] }
  }
  function defaultParams(type) {
    const obj = {};
    schemaForEngine(type).forEach(sec => sec.items.forEach(c => { if (c.key) obj[c.key] = c.value; }));
    return obj;
  }

  // ---- assignable lanes (start with 4, add up to LANE_COUNT, reorderable) --
  // A lane = a slot { id, tag, name, engine }. engine === null => empty slot.
  const TAG_FOR = {
    'synth-kick': 'BD', 'synth-snare': 'SD', 'synth-hat': 'HH', 'synth-tom': 'TOM',
    'synth-cymbal': 'CY', 'synth-clap': 'CP', 'synth-perc': 'PC',
    'sample': 'SMP', 'samplefx': 'FX', 'midi': 'MI', '': '--',
  };
  function tagFor(engine) { return TAG_FOR[engine || ''] || 'LN'; }

  const instruments = [
    { id: 'L1', tag: 'BD',  name: 'Kick',       engine: 'synth-kick'  },
    { id: 'L2', tag: 'SD',  name: 'Snare',      engine: 'synth-snare' },
    { id: 'L3', tag: 'HH',  name: 'Closed Hat', engine: 'synth-hat'   },
    { id: 'L4', tag: 'TOM', name: 'Tom',        engine: 'synth-tom'   },
  ];

  // ---- pattern (keyed by lane id) ----------------------------------------
  const pattern = {
    L1: lane([3, 11], { link: [12], snap: [1, 7], fusion: [{ start: 13, len: 2, pulses: 3 }] }),
    L2: lane([5, 13], { seq: [5] }),
    L3: lane([3, 7, 15], { link: [5, 9], seq: [11] }),
    L4: lane([7]),
  };
  pattern.L1.hits[4] = 1; pattern.L1.hits[8] = 1;
  pattern.L3.hits[4] = 1; pattern.L3.hits[8] = 1;

  // ---- per-lane sequencer settings ---------------------------------------
  const lanes = {};
  const makeSettings = (i = 0) => ({
    vol: 0.5 + (i % 5) * 0.09, mute: false, solo: false, trig: false,
    hum: i === 0 ? 0.65 : 0, push: 0, len: 48,
  });
  instruments.forEach((it, i) => { lanes[it.id] = makeSettings(i); });

  // ---- live param store --------------------------------------------------
  const params = {};
  instruments.forEach(it => { params[it.id] = defaultParams(it.engine); });

  // ---- structural ops (add / remove / reorder / assign) ------------------
  let laneSeq = instruments.length;
  function addLane(engine) {
    if (instruments.length >= LANE_COUNT) return null;
    laneSeq += 1;
    const id = 'L' + (laneSeq + 100); // unique id space, avoids clashes after removals
    const it = { id, tag: tagFor(engine), name: engine ? engineLabel(engine) : 'Empty', engine: engine || null };
    instruments.push(it);
    pattern[id] = blankLane();
    params[id] = engine ? defaultParams(engine) : {};
    lanes[id] = makeSettings(instruments.length - 1);
    return it;
  }
  function removeLane(id) {
    if (instruments.length <= 1) return;
    const i = instruments.findIndex(x => x.id === id);
    if (i < 0) return;
    instruments.splice(i, 1);
    delete pattern[id]; delete params[id]; delete lanes[id];
  }
  function moveLane(id, toIndex) {
    const i = instruments.findIndex(x => x.id === id);
    if (i < 0) return;
    const [it] = instruments.splice(i, 1);
    const clamp = Math.max(0, Math.min(instruments.length, toIndex > i ? toIndex - 1 : toIndex));
    instruments.splice(clamp, 0, it);
  }

  // assign / clear a lane's engine (resets that lane's params to engine defaults)
  function assignEngine(id, type) {
    const it = instruments.find(x => x.id === id);
    if (!it) return;
    it.engine = type || null;
    it.tag = tagFor(type);
    params[id] = type ? defaultParams(type) : {};
  }
  function renameLane(id, name) { const it = instruments.find(x => x.id === id); if (it) it.name = name; }

  // ---- transport ---------------------------------------------------------
  const transport = {
    vol: -0.07, swing: 0, groove: 'Swing 16th',
    seqSource: 'int', choke: true, autoEdit: true, song: false,
    len: 48, page: 1, follow: false,
    plockMode: 'Sound',
    activePattern: 'P1',
    genType: 'Probabilistic', genStyleA: 'Rock', genStyleB: 'Funk',
    genMix: 0, genDens: 0.7, genVar: 0.3,
  };

  // sequencer p-lock params (context menu in Sequencer mode)
  const seqPlockSchema = [
    S('Probability', 'prob', { min: 0, max: 100, step: 1, value: 100, unit: '%', fmt: v => Math.round(v) }),
    S('Stutter', 'stutter', { min: 0, max: 8, step: 1, value: 0, fmt: v => Math.round(v) }),
    SEL('Condition', 'cond', ['Always', '1:2', '2:2', '1:4', 'Fill', '!Fill'], 'Always'),
    S('Micro-timing', 'micro', { min: -50, max: 50, step: 1, value: 0, unit: 'ms', fmt: v => (v > 0 ? '+' : '') + Math.round(v) }),
  ];

  window.FD = {
    instruments, pattern, lanes, params, transport,
    ENGINES, schemaForEngine, engineLabel, engineList, defaultParams,
    assignEngine, renameLane, addLane, removeLane, moveLane, tagFor,
    oscFor, seqPlockSchema,
    STEPS, PAGE, LANE_COUNT,
    stepLabels: Array.from({ length: PAGE }, (_, i) => i + 1),
  };
})();
