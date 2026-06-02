/* Flash Drum — shared data model (vanilla, attaches to window.FD) */
(function () {
  // ---- Instruments -------------------------------------------------------
  // hue used for color-coded variation. cat drives the dynamic editor schema.
  const instruments = [
    { id: 'BD', name: 'Bass Drum',  cat: 'kick',   hue: 18  },
    { id: 'SD', name: 'Snare',      cat: 'snare',  hue: 42  },
    { id: 'HH', name: 'Closed Hat', cat: 'hat',    hue: 200 },
    { id: 'OH', name: 'Open Hat',   cat: 'hat',    hue: 188 },
    { id: 'T1', name: 'Tom 1',      cat: 'tom',    hue: 12  },
    { id: 'T2', name: 'Tom 2',      cat: 'tom',    hue: 350 },
    { id: 'T3', name: 'Tom 3',      cat: 'tom',    hue: 330 },
    { id: 'CL', name: 'Clap',       cat: 'clap',   hue: 56  },
    { id: 'RD', name: 'Ride',       cat: 'cymbal', hue: 168 },
    { id: 'CY', name: 'Crash',      cat: 'cymbal', hue: 150 },
    { id: 'S6', name: 'Shaker',     cat: 'perc',   hue: 268 },
    { id: 'B8', name: 'Perc 808',   cat: 'perc',   hue: 288 },
    { id: 'P1', name: 'Perc 1',     cat: 'perc',   hue: 240 },
  ];

  // ---- Pattern -----------------------------------------------------------
  // 16 steps per instrument. 0 = off, 1 = hit, 2 = accent (orange).
  const blank = () => new Array(16).fill(0);
  const fromList = (hits, accents = []) => {
    const a = blank();
    hits.forEach(i => { a[i - 1] = 1; });
    accents.forEach(i => { a[i - 1] = 2; });
    return a;
  };

  const pattern = {
    BD: fromList([3, 11, 14], [1, 7]),
    SD: fromList([13], [5]),
    HH: fromList([3, 7, 11, 15], [1, 9]),
    OH: fromList([4, 12]),
    T1: blank(),
    T2: blank(),
    T3: blank(),
    CL: fromList([5, 13]),
    RD: blank(),
    CY: fromList([1]),
    S6: fromList([4, 8, 16], [12]),
    B8: fromList([1, 7], [11]),
    P1: blank(),
  };

  // Per-instrument lane settings shown in the sequencer (vol, mute, solo, etc.)
  const lanes = {};
  instruments.forEach((it, i) => {
    lanes[it.id] = {
      vol: 0.55 + (i % 5) * 0.08,
      mute: false,
      solo: false,
      hum: 0,
      push: 0,
      len: 16,
    };
  });

  // ---- Dynamic editor schemas -------------------------------------------
  // Each instrument category exposes a different parameter set, proving the
  // editor is dynamic. Control kinds: slider | select | switch | adsr | div
  const S = (label, key, opts = {}) => ({ kind: 'slider', label, key,
    min: opts.min ?? 0, max: opts.max ?? 1, step: opts.step ?? 0.01,
    value: opts.value ?? 0.5, unit: opts.unit ?? '', fmt: opts.fmt });
  const SEL = (label, key, options, value) => ({ kind: 'select', label, key, options, value });
  const SW = (label, key, value = false) => ({ kind: 'switch', label, key, value });
  const ADSR = () => ({ kind: 'adsr' });

  const outputSection = {
    title: 'Output', cols: 2, items: [
      S('Volume', 'outVol', { value: 0.9 }),
      S('Analog', 'analog', { value: 1, fmt: v => v.toFixed(2) }),
      SW('Mix', 'outMix', true),
    ],
  };
  const satSection = {
    title: 'Saturation', cols: 2, items: [
      SEL('Type', 'satType', ['None', 'Tape', 'Tube', 'Hard', 'Fold'], 'None'),
      S('Amount', 'satAmt', { value: 0, max: 100, step: 1, fmt: v => Math.round(v) }),
      S('Mix', 'satMix', { value: 1 }),
      S('Out Gain', 'satGain', { value: 1, fmt: v => v.toFixed(2) }),
      SW('Pre-Filter', 'satPre', false),
    ],
  };
  const filterSection = (label = 'Filter (LP)') => ({
    title: 'Filter', cols: 1, items: [
      S(label, 'filter', { value: 0.42, max: 100, step: 0.5, fmt: v => v.toFixed(1) + ' Hz' }),
      S('Resonance', 'reso', { value: 0.2 }),
    ],
  });
  const envSection = {
    title: 'Envelope', cols: 2, adsr: true, items: [
      S('Attack', 'attack', { value: 0.0, fmt: v => v.toFixed(2) }),
      S('Decay', 'decay', { value: 0.5, fmt: v => v.toFixed(2) }),
      S('Decay Curve', 'decayC', { value: 0.55, max: 8, step: 0.05, fmt: v => v.toFixed(2) }),
      S('Release', 'release', { value: 0.5, fmt: v => v.toFixed(2) }),
      S('Rel. Curve', 'relC', { value: 0.4, max: 8, step: 0.05, fmt: v => v.toFixed(2) }),
    ],
  };

  function schemaFor(cat) {
    const head = { title: 'Level', cols: 1, items: [ S('Volume', 'vol', { value: 0.9 }) ] };
    let osc;
    if (cat === 'kick' || cat === 'tom') {
      osc = { title: 'Oscillator', cols: 2, items: [
        S('Frequency', 'freq', { value: 0.32, max: 200, step: 0.5, fmt: v => v.toFixed(2) }),
        S('Click', 'click', { value: 0.5 }),
        SEL('Algorithm', 'algo', ['Sine', 'Triangle', 'Saw', 'Square'], 'Sine'),
        ...(cat === 'tom' ? [ S('Pitch Bend', 'bend', { value: 0.3 }) ] : []),
      ] };
    } else if (cat === 'snare') {
      osc = { title: 'Body + Noise', cols: 2, items: [
        S('Tone Freq', 'freq', { value: 0.4, max: 200, step: 0.5, fmt: v => v.toFixed(1) }),
        S('Noise Mix', 'noise', { value: 0.6 }),
        S('Snap', 'snap', { value: 0.45 }),
        SEL('Body', 'algo', ['Sine', 'Triangle', 'Noise'], 'Triangle'),
      ] };
    } else if (cat === 'hat' || cat === 'cymbal') {
      osc = { title: 'Metal / Noise', cols: 2, items: [
        S('Tone', 'tone', { value: 0.62 }),
        S('Decay', 'mdecay', { value: cat === 'hat' ? 0.25 : 0.7 }),
        S('Color', 'color', { value: 0.5 }),
        S('Shimmer', 'shimmer', { value: 0.4 }),
      ] };
    } else { // perc / clap
      osc = { title: cat === 'clap' ? 'Clap Engine' : 'Source', cols: 2, items: [
        S('Pitch', 'freq', { value: 0.5, max: 200, step: 0.5, fmt: v => v.toFixed(1) }),
        ...(cat === 'clap'
          ? [ S('Spread', 'spread', { value: 0.5 }), S('Count', 'count', { value: 0.4, max: 8, step: 1, fmt: v => Math.round(v) }) ]
          : [ S('Decay', 'mdecay', { value: 0.4 }), S('Noise', 'noise', { value: 0.3 }) ]),
      ] };
    }
    const flt = cat === 'hat' || cat === 'cymbal'
      ? filterSection('Filter (HP)') : filterSection('Filter (LP)');
    return [ head, osc, envSection, flt, satSection, outputSection ];
  }

  // pre-compute live param values per instrument from defaults
  const params = {};
  instruments.forEach(it => {
    const obj = {};
    schemaFor(it.cat).forEach(sec => sec.items && sec.items.forEach(c => {
      if (c.key) obj[c.key] = c.value;
    }));
    params[it.id] = obj;
  });

  // ---- transport / header state -----------------------------------------
  const transport = {
    vol: -1.94, swing: 0, swingMode: 'Swing 16th',
    choke: true, autoEdit: true, groove: false,
    song: 'P1', bpm: 124,
    genType: 'Probabilistic', genStyle: 'Rock', mix: 0, dens: 0.7, varAmt: 0.3,
  };

  window.FD = {
    instruments, pattern, lanes, params, transport,
    schemaFor, blank, fromList,
    stepLabels: Array.from({ length: 16 }, (_, i) => i + 1),
  };
})();
