/* Flash Drum — shared control + render library v2. window.FDCore */
(function () {
  const FD = window.FD;
  function el(tag, cls, props) {
    const n = document.createElement(tag);
    if (cls) n.className = cls;
    if (props) Object.assign(n, props);
    return n;
  }
  const clamp = (v, a, b) => Math.min(b, Math.max(a, v));

  // ---- slider ----
  function slider(spec, onChange) {
    const wrap = el('div', 'ctl ctl--slider');
    const lab = el('span', 'ctl__label', { textContent: spec.label });
    const track = el('div', 'sld');
    const fill = el('div', 'sld__fill'); const knob = el('div', 'sld__knob');
    track.append(fill, knob);
    const val = el('span', 'ctl__val');
    wrap.append(lab, track, val);
    let v = spec.value; const fmt = spec.fmt || (x => x.toFixed(2));
    function render() {
      const t = (v - spec.min) / (spec.max - spec.min);
      fill.style.width = (t * 100) + '%'; knob.style.left = (t * 100) + '%';
      val.textContent = fmt(v) + (spec.unit ? ' ' + spec.unit : '');
    }
    function setX(cx) {
      const r = track.getBoundingClientRect();
      let t = clamp((cx - r.left) / r.width, 0, 1);
      v = spec.min + t * (spec.max - spec.min);
      if (spec.step) v = Math.round(v / spec.step) * spec.step;
      v = clamp(v, spec.min, spec.max); render(); onChange && onChange(v);
    }
    let drag = false;
    track.addEventListener('pointerdown', e => { drag = true; track.setPointerCapture(e.pointerId); setX(e.clientX); });
    track.addEventListener('pointermove', e => { if (drag) setX(e.clientX); });
    track.addEventListener('pointerup', () => drag = false);
    track.addEventListener('dblclick', () => { v = spec.value; render(); onChange && onChange(v); });
    track.title = 'Double-clic : valeur par défaut';
    render();
    wrap.api = { get: () => v, set: x => { v = x; render(); } };
    return wrap;
  }

  // ---- freq (label + Notes toggle + slider + value) ----
  function freq(spec, onChange) {
    const wrap = el('div', 'ctl ctl--freq');
    const lab = el('span', 'ctl__label', { textContent: spec.label });
    const notes = el('button', 'notes', { textContent: 'Notes' });
    const dec = el('button', 'stepbtn stepbtn--l', { textContent: '◂', title: '-1 demi-ton' });
    const inc = el('button', 'stepbtn', { textContent: '▸', title: '+1 demi-ton' });
    const track = el('div', 'sld'); const fill = el('div', 'sld__fill'); const knob = el('div', 'sld__knob');
    track.append(fill, knob);
    const val = el('span', 'ctl__val');
    wrap.append(lab, notes, track, dec, val, inc);
    let v = spec.value, asNotes = false; const fmt = spec.fmt || (x => x.toFixed(2));
    const NOTE = ['C','C#','D','D#','E','F','F#','G','G#','A','A#','B'];
    const toNote = hz => { const m = Math.round(69 + 12 * Math.log2(hz / 440)); return NOTE[((m % 12) + 12) % 12] + (Math.floor(m / 12) - 1); };
    function render() {
      const t = (v - spec.min) / (spec.max - spec.min);
      fill.style.width = (t * 100) + '%'; knob.style.left = (t * 100) + '%';
      val.textContent = asNotes ? toNote(v) : fmt(v);
    }
    notes.addEventListener('click', () => { asNotes = !asNotes; notes.classList.toggle('on', asNotes); wrap.classList.toggle('notes-on', asNotes); render(); });
    function stepNote(d) {
      const m = Math.round(69 + 12 * Math.log2(v / 440)) + d;
      v = clamp(440 * Math.pow(2, (m - 69) / 12), spec.min, spec.max);
      render(); onChange && onChange(v);
    }
    dec.addEventListener('click', () => stepNote(-1));
    inc.addEventListener('click', () => stepNote(1));
    function setX(cx) { const r = track.getBoundingClientRect(); let t = clamp((cx - r.left) / r.width, 0, 1);
      v = spec.min + t * (spec.max - spec.min); if (spec.step) v = Math.round(v / spec.step) * spec.step; render(); onChange && onChange(v); }
    let drag = false;
    track.addEventListener('pointerdown', e => { drag = true; track.setPointerCapture(e.pointerId); setX(e.clientX); });
    track.addEventListener('pointermove', e => { if (drag) setX(e.clientX); });
    track.addEventListener('pointerup', () => drag = false);
    track.addEventListener('dblclick', () => { v = spec.value; render(); onChange && onChange(v); });
    track.title = 'Double-clic : valeur par défaut';
    render();
    return wrap;
  }

  // ---- switch ----
  function toggle(spec, onChange) {
    const wrap = el('div', 'ctl ctl--switch');
    const lab = el('span', 'ctl__label', { textContent: spec.label });
    const sw = el('button', 'sw');
    let v = spec.value; const render = () => sw.classList.toggle('sw--on', !!v);
    sw.addEventListener('click', () => { v = !v; render(); onChange && onChange(v); });
    wrap.append(lab, sw); render();
    wrap.api = { get: () => v, set: x => { v = x; render(); } };
    return wrap;
  }

  // ---- select ----
  function select(spec, onChange) {
    const wrap = el('div', 'ctl ctl--select');
    const lab = el('span', 'ctl__label', { textContent: spec.label });
    if (!spec.label) lab.style.display = 'none';
    const sel = el('div', 'selbox', { tabIndex: 0 });
    const cur = el('span', 'selbox__cur', { textContent: spec.value });
    const car = el('span', 'selbox__car', { textContent: '▾' });
    const menu = el('div', 'selbox__menu');
    spec.options.forEach(o => {
      const it = el('div', 'selbox__opt', { textContent: o });
      it.addEventListener('click', e => { e.stopPropagation(); cur.textContent = o; menu.classList.remove('open'); onChange && onChange(o); });
      menu.append(it);
    });
    sel.append(cur, car, menu);
    sel.addEventListener('click', () => {
      const willOpen = !menu.classList.contains('open');
      menu.classList.toggle('open');
      if (willOpen) {
        menu.classList.remove('up');
        const r = menu.getBoundingClientRect();
        if (r.bottom > window.innerHeight - 8) menu.classList.add('up');
      }
    });
    document.addEventListener('click', e => { if (!sel.contains(e.target)) menu.classList.remove('open'); });
    wrap.append(lab, sel);
    return wrap;
  }

  function ctlFor(c, onChange) {
    if (c.kind === 'slider') return slider(c, onChange);
    if (c.kind === 'freq') return freq(c, onChange);
    if (c.kind === 'select') return select(c, onChange);
    if (c.kind === 'switch') return toggle(c, onChange);
    return el('div');
  }

  // ---- ADSR graph ----
  function drawADSR(canvas, p) {
    const dpr = window.devicePixelRatio || 1;
    const w = canvas.clientWidth, h = canvas.clientHeight;
    if (!w || !h) { requestAnimationFrame(() => drawADSR(canvas, p)); return; }
    canvas.width = w * dpr; canvas.height = h * dpr;
    const ctx = canvas.getContext('2d'); ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);
    const padX = 12, padY = 12, gx = w - padX * 2, gy = h - padY * 2;
    const baseY = h - padY, topY = padY;
    ctx.strokeStyle = 'rgba(255,255,255,.05)'; ctx.lineWidth = 1;
    for (let i = 0; i <= 4; i++) { const x = padX + gx * i / 4; ctx.beginPath(); ctx.moveTo(x, topY); ctx.lineTo(x, baseY); ctx.stroke(); }
    const a = Math.max(0.02, p.attack ?? 0.1), d = Math.max(0.05, p.decay ?? 0.5), r = Math.max(0.05, p.release ?? 0.5);
    const tot = a + d + r;
    const xA = padX + gx * (a / tot), xD = padX + gx * ((a + d) / tot), xR = padX + gx;
    const dc = (p.decayC ?? 2) / 2, rc = (p.relC ?? 2) / 2;
    const susY = topY + gy * 0.62;
    function curve(x0, y0, x1, y1, k, color) {
      ctx.beginPath(); ctx.strokeStyle = color; ctx.lineWidth = 2;
      const N = 40;
      for (let i = 0; i <= N; i++) { const t = i / N; const e = Math.pow(t, 1 + k);
        const x = x0 + (x1 - x0) * t, y = y0 + (y1 - y0) * e; i ? ctx.lineTo(x, y) : ctx.moveTo(x, y); }
      ctx.stroke();
    }
    ctx.beginPath(); ctx.strokeStyle = '#fbbf24'; ctx.lineWidth = 2; ctx.moveTo(padX, baseY); ctx.lineTo(xA, topY); ctx.stroke();
    curve(xA, topY, xD, susY, dc, '#4a9eff');
    curve(xD, susY, xR, baseY, rc, '#a855f7');
  }

  // ---- editor (Sound tab) ----
  function buildEditor(container, instId) {
    container.innerHTML = '';
    const inst = FD.instruments.find(i => i.id === instId);
    if (!inst.engine) {
      const empty = el('div', 'ed-empty');
      empty.innerHTML = '<div class="ed-empty__i">—</div><div class="ed-empty__t">Empty lane</div>'
        + '<div class="ed-empty__s">Assign an engine to this lane to edit its sound.</div>';
      container.append(empty);
      return;
    }
    const schema = FD.schemaForEngine(inst.engine);
    const p = FD.params[instId];
    let adsrCanvas = null;
    const refreshADSR = () => adsrCanvas && drawADSR(adsrCanvas, p);
    schema.forEach(sec => {
      const block = el('section', 'edsec' + (sec.kind === 'volume' ? ' edsec--volume' : ''));
      if (sec.title) block.append(el('div', 'edsec__h', { textContent: sec.title }));
      const body = el('div', 'edsec__b');
      sec.items.forEach(c => {
        const node = ctlFor(c, v => { p[c.key] = v; if (['attack','decay','decayC','release','relC'].includes(c.key)) refreshADSR(); });
        body.append(node);
      });
      if (sec.adsr) {
        const cols = el('div', 'edsec__cols');
        const fig = el('div', 'adsr');
        adsrCanvas = el('canvas', 'adsr__c');
        const leg = el('div', 'adsr__leg');
        leg.innerHTML = '<i class="dot dot--a"></i>A&nbsp;&nbsp;<i class="dot dot--d"></i>D&nbsp;&nbsp;<i class="dot dot--r"></i>R';
        fig.append(adsrCanvas, leg);
        cols.append(body, fig);
        block.append(cols);
      } else {
        block.append(body);
      }
      container.append(block);
    });
    requestAnimationFrame(refreshADSR); setTimeout(refreshADSR, 90);
    if (document.fonts && document.fonts.ready) document.fonts.ready.then(refreshADSR);
  }

  // ---- p-lock context menu ----
  let plkEl = null;
  function ensurePlk() {
    if (plkEl) return plkEl;
    plkEl = el('div', 'plk');
    document.body.append(plkEl);
    document.addEventListener('click', e => { if (plkEl && !plkEl.contains(e.target)) plkEl.classList.remove('open'); });
    document.addEventListener('contextmenu', e => { if (plkEl && plkEl.contains(e.target)) e.preventDefault(); });
    return plkEl;
  }
  function openPlock(x, y, instId, stepAbs, mode, paint) {
    const m = ensurePlk(); m.innerHTML = '';
    paint = paint || (() => {});
    const inst = FD.instruments.find(i => i.id === instId);
    const lane = FD.pattern[instId];
    const plState = lane.plock[stepAbs];
    const hasSeq = !!lane.seq[stepAbs];
    const h = el('div', 'plk__h');
    const pin = el('span', 'pin');
    pin.style.background = mode === 'Sequencer'
      ? (hasSeq ? 'var(--seqpl)' : 'var(--faint)')
      : (plState === 'snapshot' ? 'var(--pl-snap)' : plState === 'link' ? 'var(--pl-link)' : 'var(--faint)');
    h.append(pin, el('span', null, { textContent: (mode === 'Sequencer' ? 'Seq Plock ' : 'Plock ') + inst.name }));
    h.append(el('span', 'step-n', { textContent: 'Step ' + (stepAbs + 1) }));
    m.append(h);
    const opt = (label, fn, danger) => {
      const b = el('button', 'plk__opt' + (danger ? ' plk__opt--danger' : ''), { textContent: label });
      b.addEventListener('click', () => fn());
      return b;
    };
    const reopen = () => openPlock(x, y, instId, stepAbs, mode, paint);
    const close = () => m.classList.remove('open');

    if (mode === 'Sequencer') {
      // popup unique (cf. design original) : Mode Inactive/Active + params + grille + action en bas
      const modeLine = el('div', 'plk__mode');
      modeLine.append(el('span', null, { textContent: 'Mode' }));
      modeLine.append(el('span', 'plk__modeval', { textContent: hasSeq ? 'Active' : 'Inactive' }));
      m.append(modeLine);
      m.append(plkRow(FD.seqPlockSchema[0], true)); // Probability
      m.append(plkRow(FD.seqPlockSchema[1], true)); // Stutter
      m.append(el('div', 'plk__lab', { textContent: 'Condition' }));
      const grid = el('div', 'plk__cond');
      FD.seqConditions.forEach(o => {
        const b = el('button', 'plk__cbtn' + (o === 'Always' ? ' on' : ''), { textContent: o });
        b.addEventListener('click', () => { grid.querySelectorAll('.plk__cbtn').forEach(x => x.classList.remove('on')); b.classList.add('on'); });
        grid.append(b);
      });
      m.append(grid);
      const act = el('button', 'plk__create' + (hasSeq ? ' plk__create--danger' : ''), { textContent: hasSeq ? 'Clear Seq Plock' : 'Create Seq Plock' });
      act.addEventListener('click', () => {
        if (hasSeq) { lane.seq[stepAbs] = 0; paint(); close(); }
        else { lane.seq[stepAbs] = 1; if (!lane.hits[stepAbs]) lane.hits[stepAbs] = 1; paint(); reopen(); }
      });
      m.append(act);
    } else {
      if (plState === 'none') {
        // pas de p-lock : menu de création
        m.append(el('div', 'plk__state', { textContent: 'No plock on this step' }));
        m.append(opt('Link to Global', () => { lane.plock[stepAbs] = 'link'; paint(); reopen(); }));
        m.append(opt('Snapshot Current Settings', () => { lane.plock[stepAbs] = 'snapshot'; paint(); reopen(); }));
        m.append(opt('Paste Plock', () => { lane.plock[stepAbs] = 'snapshot'; paint(); close(); }));
      } else {
        // p-lock existant : édition
        const modeLine = el('div', 'plk__mode');
        modeLine.append(el('span', null, { textContent: plState === 'snapshot' ? 'Full Snapshot' : 'Linked' }));
        modeLine.append(select({ options: ['Link to global', 'Snapshot'], value: plState === 'snapshot' ? 'Snapshot' : 'Link to global' },
          v => { lane.plock[stepAbs] = v === 'Snapshot' ? 'snapshot' : 'link'; paint(); reopen(); }));
        m.append(modeLine);
        // Volume first, then the instrument's sliders
        const items = [{ kind: 'slider', label: 'Volume', key: 'vol', min: -60, max: 6, step: 0.1, value: FD.params[instId].vol, fmt: v => v.toFixed(1) }];
        FD.schemaForEngine(inst.engine).forEach(sec => sec.items.forEach(c => {
          if (c.key && c.key !== 'vol' && (c.kind === 'slider' || c.kind === 'freq')) items.push({ ...c, value: FD.params[instId][c.key] });
        }));
        items.slice(0, 8).forEach(c => m.append(plkRow(c)));
        const foot = el('div', 'plk__foot');
        foot.append(el('button', null, { textContent: 'Copy Plock' }), el('button', null, { textContent: 'Paste Plock' }));
        const clr = el('button', 'plk__foot--danger', { textContent: 'Clear' });
        clr.addEventListener('click', () => { lane.plock[stepAbs] = 'none'; paint(); close(); });
        foot.append(clr);
        m.append(foot);
      }
    }
    m.style.left = Math.min(x, window.innerWidth - 250) + 'px';
    m.style.top = Math.min(y, window.innerHeight - m.offsetHeight - 20) + 'px';
    m.classList.add('open');
    // reposition after layout
    requestAnimationFrame(() => { m.style.top = Math.min(y, window.innerHeight - m.offsetHeight - 12) + 'px'; });
  }
  function plkRow(c, noUndo) {
    const row = el('div', 'plk__row');
    // inside the narrow popup, render freq as a plain slider (no Notes toggle)
    const spec = { ...c, kind: c.kind === 'freq' ? 'slider' : c.kind, fmt: c.fmt || (v => v.toFixed(2)) };
    row.append(ctlFor(spec, () => {}));
    if (!noUndo) row.append(el('button', 'undo', { textContent: '↺', title: 'Reset' }));
    return row;
  }

  // ---- lane assignment menu (right-click a lane name) ----
  let laneEl = null;
  function ensureLane() {
    if (laneEl) return laneEl;
    laneEl = el('div', 'lanemenu');
    document.body.append(laneEl);
    document.addEventListener('click', e => { if (laneEl && !laneEl.contains(e.target)) laneEl.classList.remove('open'); });
    document.addEventListener('contextmenu', e => { if (laneEl && laneEl.contains(e.target)) e.preventDefault(); });
    return laneEl;
  }
  function openLaneMenu(x, y, instId, onChange, onStruct) {
    const m = ensureLane(); m.innerHTML = '';
    const inst = FD.instruments.find(i => i.id === instId);
    m.append(el('div', 'lanemenu__h', { textContent: 'Lane ' + (inst.tag || inst.id) }));
    // rename
    const nameRow = el('div', 'lanemenu__row');
    nameRow.append(el('span', 'lanemenu__k', { textContent: 'Name' }));
    const input = el('input', 'lanemenu__name'); input.type = 'text'; input.value = inst.name;
    input.addEventListener('input', () => { FD.renameLane(instId, input.value); onChange(instId); });
    nameRow.append(input); m.append(nameRow);
    // engine picker
    m.append(el('div', 'lanemenu__lab', { textContent: 'Engine' }));
    const groups = FD.engineList();
    const wrap = el('div', 'lanemenu__engines');
    Object.keys(groups).forEach(g => {
      wrap.append(el('div', 'lanemenu__grp', { textContent: g }));
      groups[g].forEach(e => {
        const b = el('button', 'lanemenu__eng' + (inst.engine === e.type ? ' on' : ''), { textContent: e.label });
        b.addEventListener('click', () => { FD.assignEngine(instId, e.type); onChange(instId); (onStruct || (() => {}))(); m.classList.remove('open'); });
        wrap.append(b);
      });
    });
    m.append(wrap);
    // actions de lane (parité alpha)
    const acts = el('div', 'lanemenu__acts');
    [['Copy Lane', () => { FD._laneClip = instId; }],
     ['Paste Lane', () => { if (FD._laneClip && FD._laneClip !== instId) { const src = FD.pattern[FD._laneClip]; const dst = FD.pattern[instId]; dst.hits = [...src.hits]; dst.plock = [...src.plock]; dst.seq = [...src.seq]; dst.fusion = src.fusion.map(f => ({ ...f })); (onStruct || (() => {}))(); } }],
     ['Randomize', () => { const L = FD.pattern[instId]; const len = FD.lanes[instId].len; for (let s = 0; s < len; s++) L.hits[s] = Math.random() < 0.28 ? 1 : 0; (onStruct || (() => {}))(); }],
     ['Clear Lane', () => { const L = FD.pattern[instId]; L.hits.fill(0); L.plock.fill('none'); L.seq.fill(0); L.fusion.length = 0; (onStruct || (() => {}))(); }]]
    .forEach(([label, fn]) => {
      const b = el('button', 'lanemenu__act', { textContent: label });
      b.addEventListener('click', () => { fn(); m.classList.remove('open'); });
      acts.append(b);
    });
    m.append(acts);
    // remove (kept ≥ 1 lane)
    const rm = el('button', 'lanemenu__clear', { textContent: 'Remove lane' });
    if (FD.instruments.length <= 1) rm.disabled = true;
    rm.addEventListener('click', () => { FD.removeLane(instId); (onStruct || (() => {}))(); m.classList.remove('open'); });
    m.append(rm);
    m.classList.add('open');
    m.style.left = Math.min(x, window.innerWidth - 230) + 'px';
    m.style.top = '0px';
    requestAnimationFrame(() => { m.style.top = Math.max(8, Math.min(y, window.innerHeight - m.offsetHeight - 12)) + 'px'; });
  }

  // ---- value tip (visible pendant hover ET drag) ----
  let tipEl = null;
  function showTip(x, y, text) {
    if (!tipEl) { tipEl = el('div', 'fd-tip'); document.body.append(tipEl); }
    tipEl.textContent = text;
    tipEl.style.left = x + 'px'; tipEl.style.top = (y - 8) + 'px';
    tipEl.classList.add('show');
  }
  function hideTip() { if (tipEl) tipEl.classList.remove('show'); }

  // ---- sequencer (paged, p-locks, fusion, modular lanes) ----
  function buildSequencer(container, opts = {}) {
    container.innerHTML = '';
    const grid = el('div', 'seq');
    const onSelect = opts.onSelect || (() => {});
    const onLanesChange = opts.onLanesChange || (() => {});
    let selected = opts.selected, page = 1, mode = FD.transport.plockMode;
    const PAGE = FD.PAGE;

    // header (built once)
    const head = el('div', 'seq__row seq__row--head');
    head.append(el('div', 'seq__grip'));
    head.append(el('div', 'seq__name'));
    head.append(el('div', 'seq__vol', { textContent: 'Vol' }));
    const mstH = el('div', 'seq__mst'); mstH.style.flex = '0 0 auto';
    ['M', 'S', 'T'].forEach(t => { const s = el('div'); s.style.cssText = 'width:17px;text-align:center;font:500 9px/1 var(--mono);color:var(--ink3)'; s.textContent = t; mstH.append(s); });
    head.append(mstH);
    const lblW = el('div', 'seq__steps');
    FD.stepLabels.forEach(n => { const c = el('div', 'seq__steplab', { textContent: n }); if ((n - 1) % 4 === 0) c.classList.add('is-beat'); lblW.append(c); });
    head.append(lblW);
    ['Hum', 'Push', 'Len'].forEach(t => head.append(el('div', 'seq__extra', { textContent: t })));
    grid.append(head);
    head._lbls = [...lblW.children];

    // body holds the lane rows (rebuilt on add/remove/reorder)
    const body = el('div', 'seq__body');
    grid.append(body);

    // "+ Add module" row
    const addRow = el('div', 'seq__addrow');
    const addBtn = el('button', 'seq__add', { textContent: '+  Add module' });
    addBtn.addEventListener('click', e => {
      if (FD.instruments.length >= FD.LANE_COUNT) return;
      openAddMenu(e.clientX, e.clientY, type => {
        const it = FD.addLane(type);
        if (it) { renderRows(); select(it.id); onSelect(it.id); onLanesChange(); }
      });
    });
    addRow.append(addBtn);
    grid.append(addRow);

    let rowEls = {}, cellEls = {}, lenEls = {};

    function buildRow(inst) {
      const L = FD.pattern[inst.id], lane = FD.lanes[inst.id];
      const row = el('div', 'seq__row'); row.dataset.id = inst.id;
      if (!inst.engine) row.classList.add('is-empty');
      // drag grip (reorder)
      const grip = el('div', 'seq__grip', { textContent: '⠿', title: 'Drag to reorder' });
      grip.addEventListener('pointerdown', () => { row.draggable = true; });
      grip.addEventListener('pointerup', () => { row.draggable = false; });
      row.addEventListener('dragstart', e => { dragId = inst.id; row.classList.add('dragging'); e.dataTransfer.effectAllowed = 'move'; try { e.dataTransfer.setData('text/plain', inst.id); } catch (_) {} });
      row.addEventListener('dragend', () => { row.classList.remove('dragging'); row.draggable = false; clearDropMarks(); });
      row.addEventListener('dragover', e => { e.preventDefault(); markDrop(row, e.clientY); });
      row.addEventListener('drop', e => { e.preventDefault(); doDrop(inst.id, row, e.clientY); });
      row.append(grip);
      // name (select + assign menu)
      const name = el('button', 'seq__name', { textContent: inst.name, title: inst.name + ' · ' + FD.engineLabel(inst.engine) + '  (right-click to assign)' });
      name.addEventListener('click', () => { select(inst.id); onSelect(inst.id); });
      name.addEventListener('contextmenu', e => { e.preventDefault(); select(inst.id); onSelect(inst.id); openLaneMenu(e.clientX, e.clientY, inst.id, opts.onLaneChange || (() => {}), () => { renderRows(); onLanesChange(); }); });
      row.append(name);
      // vol
      const vwrap = el('div', 'seq__vol'); const vt = el('div', 'minisld'); const vf = el('div', 'minisld__f');
      vt.append(vf); vf.style.width = (lane.vol * 100) + '%';
      let vd = false; const setV = cx => { const r = vt.getBoundingClientRect(); lane.vol = clamp((cx - r.left) / r.width, 0, 1); vf.style.width = (lane.vol * 100) + '%'; };
      vt.addEventListener('pointerdown', e => { vd = true; vt.setPointerCapture(e.pointerId); setV(e.clientX); });
      vt.addEventListener('pointermove', e => { if (vd) setV(e.clientX); });
      vt.addEventListener('pointerup', () => vd = false);
      vwrap.append(vt); row.append(vwrap);
      // M S T
      const mst = el('div', 'seq__mst');
      const mk = (cls, k) => { const b = el('button', 'tag ' + cls, { textContent: cls.slice(-1).toUpperCase() }); b.classList.toggle('on', !!lane[k]); b.addEventListener('click', () => { lane[k] = !lane[k]; b.classList.toggle('on', lane[k]); }); return b; };
      mst.append(mk('tag--m', 'mute'), mk('tag--s', 'solo'), mk('tag--t', 'trig'));
      row.append(mst);
      // steps
      const stepsW = el('div', 'seq__steps');
      const cells = [];
      for (let i = 0; i < PAGE; i++) {
        const cell = el('button', 'step'); if (i % 4 === 0) cell.classList.add('is-beat');
        cell.addEventListener('click', () => {
          const abs = (page - 1) * PAGE + i; if (abs >= lane.len) return;
          if (mode === 'Sequencer') { L.seq[abs] = L.seq[abs] ? 0 : 1; if (L.seq[abs]) L.hits[abs] = 1; }
          else { L.hits[abs] = L.hits[abs] ? 0 : 1; }
          paintCell(inst.id, i, abs);
        });
        cell.addEventListener('contextmenu', e => {
          e.preventDefault(); const abs = (page - 1) * PAGE + i; if (abs >= lane.len) return;
          select(inst.id); onSelect(inst.id); openPlock(e.clientX, e.clientY, inst.id, abs, mode, () => paintCell(inst.id, i, abs));
        });
        cells.push(cell); stepsW.append(cell);
      }
      row.append(stepsW);
      // lane vide : pastille +N → popup Add Module
      if (!inst.engine) {
        const pill = el('button', 'lane-addpill', { textContent: '+' + (FD.instruments.indexOf(inst) + 1), title: 'Assign a module to this lane' });
        pill.addEventListener('click', e => {
          e.stopPropagation();
          openAddMenu(e.clientX, e.clientY, type => {
            FD.assignEngine(inst.id, type);
            (opts.onLaneChange || (() => {}))(inst.id);
            renderRows(); onLanesChange();
          });
        });
        stepsW.style.position = 'relative';
        stepsW.append(pill);
      }
      cellEls[inst.id] = cells;
      // extras
      const hum = el('div', 'seq__extra'); const hb = el('div', 'minisld minisld--dim'); const hf = el('div', 'minisld__f');
      const humText = () => 'Hum ' + Math.round(lane.hum * 100) + ' %';
      const paintHum = () => { hf.style.width = (lane.hum * 100) + '%'; };
      let hd = false;
      const setHum = cx => { const r = hb.getBoundingClientRect(); lane.hum = clamp((cx - r.left) / r.width, 0, 1); paintHum(); };
      hb.addEventListener('pointerdown', e => { hd = true; hb.setPointerCapture(e.pointerId); setHum(e.clientX); showTip(e.clientX, hb.getBoundingClientRect().top, humText()); });
      hb.addEventListener('pointermove', e => { if (hd) setHum(e.clientX); showTip(e.clientX, hb.getBoundingClientRect().top, humText()); });
      hb.addEventListener('pointerup', () => hd = false);
      hb.addEventListener('pointerleave', () => { if (!hd) hideTip(); });
      hb.append(hf); hb.style.width = '34px'; paintHum(); hum.append(hb); row.append(hum);
      // Push : mini-slider bipolaire (±50 ms), comme le code original
      const push = el('div', 'seq__extra');
      const pb = el('div', 'minisld'); pb.style.width = '34px';
      const pf = el('div', 'minisld__f');
      const pushText = () => 'Push ' + (lane.push > 0 ? '+' : '') + lane.push + ' ms';
      const paintPush = () => { pf.style.width = ((lane.push + 50) / 100 * 100) + '%'; };
      let pd = false;
      const setPush = cx => { const r = pb.getBoundingClientRect(); const t = clamp((cx - r.left) / r.width, 0, 1); lane.push = Math.round(t * 100 - 50); paintPush(); };
      pb.addEventListener('pointerdown', e => { pd = true; pb.setPointerCapture(e.pointerId); setPush(e.clientX); showTip(e.clientX, pb.getBoundingClientRect().top, pushText()); });
      pb.addEventListener('pointermove', e => { if (pd) setPush(e.clientX); showTip(e.clientX, pb.getBoundingClientRect().top, pushText()); });
      pb.addEventListener('pointerup', () => pd = false);
      pb.addEventListener('pointerleave', () => { if (!pd) hideTip(); });
      pb.append(pf); paintPush(); push.append(pb); row.append(push);
      // Len : champ éditable clavier + souris
      const len = el('div', 'seq__extra');
      const lenN = el('input', 'ex__input');
      lenN.type = 'number'; lenN.min = 1; lenN.max = FD.STEPS; lenN.value = lane.len;
      lenN.addEventListener('change', () => {
        let v = Math.round(+lenN.value || lane.len);
        v = Math.max(1, Math.min(FD.STEPS, v));
        lenN.value = v; lane.len = v; renderPage();
      });
      lenN.addEventListener('keydown', e => e.stopPropagation());
      len.append(lenN); row.append(len);
      lenEls[inst.id] = lenN;
      body.append(row); rowEls[inst.id] = row;
    }

    function renderRows() {
      body.innerHTML = ''; rowEls = {}; cellEls = {}; lenEls = {};
      FD.instruments.forEach(buildRow);
      addBtn.disabled = FD.instruments.length >= FD.LANE_COUNT;
      addBtn.textContent = addBtn.disabled ? 'Max ' + FD.LANE_COUNT + ' modules' : '+  Add module';
      if (selected && !rowEls[selected]) selected = FD.instruments[0] && FD.instruments[0].id;
      applySelected(); setMode(mode); renderPage();
    }

    // ---- drag reorder helpers ----
    let dragId = null;
    function clearDropMarks() { body.querySelectorAll('.drop-above,.drop-below').forEach(r => r.classList.remove('drop-above', 'drop-below')); }
    function markDrop(row, y) {
      if (row.dataset.id === dragId) return;
      clearDropMarks();
      const r = row.getBoundingClientRect();
      row.classList.add(y < r.top + r.height / 2 ? 'drop-above' : 'drop-below');
    }
    function doDrop(targetId, row, y) {
      clearDropMarks();
      if (!dragId || dragId === targetId) return;
      const r = row.getBoundingClientRect();
      const before = y < r.top + r.height / 2;
      let idx = FD.instruments.findIndex(i => i.id === targetId);
      if (!before) idx += 1;
      FD.moveLane(dragId, idx);
      const moved = dragId; dragId = null;
      renderRows(); select(moved); onSelect(moved); onLanesChange();
    }

    function paintCell(id, i, abs) {
      const L = FD.pattern[id], lane = FD.lanes[id], cell = cellEls[id][i];
      cell.className = 'step' + (i % 4 === 0 ? ' is-beat' : '');
      if (abs >= lane.len) { cell.style.opacity = '.28'; cell.style.pointerEvents = 'none'; return; }
      cell.style.opacity = ''; cell.style.pointerEvents = '';
      const hit = !!L.hits[abs];
      cell.dataset.s = hit ? 1 : 0;
      if (mode === 'Sequencer') {
        if (L.seq[abs]) cell.classList.add(hit ? 'st-seqhit' : 'st-seqoff');
        else if (hit) cell.classList.add('st-hit');
      } else {
        const pl = L.plock[abs];
        if (hit) cell.classList.add(pl === 'link' ? 'st-link' : pl === 'snapshot' ? 'st-snap' : 'st-hit');
        else if (pl === 'link') cell.classList.add('st-link-off');
        else if (pl === 'snapshot') cell.classList.add('st-snap-off');
      }
      const f = L.fusion.find(g => abs >= g.start - 1 && abs < g.start - 1 + g.len);
      if (f) { cell.classList.add('fuse'); if (abs === f.start - 1) { cell.classList.add('fuse-start'); cell.dataset.pulses = f.pulses; } else cell.classList.add('fuse-mid'); }
    }
    function renderPage() {
      FD.instruments.forEach(inst => {
        for (let i = 0; i < PAGE; i++) paintCell(inst.id, i, (page - 1) * PAGE + i);
        if (lenEls[inst.id]) lenEls[inst.id].value = FD.lanes[inst.id].len;
      });
      head._lbls.forEach((l, i) => { l.textContent = (page - 1) * PAGE + i + 1; });
    }
    function applySelected() { Object.values(rowEls).forEach(r => r.classList.toggle('is-sel', r.dataset.id === selected)); }
    function select(id) { selected = id; applySelected(); }
    function setPage(p) { page = p; renderPage(); }
    function setMode(mo) { mode = mo; FD.transport.plockMode = mo; grid.classList.toggle('seq--seqmode', mo === 'Sequencer'); grid.classList.toggle('seq--soundmode', mo === 'Sound'); renderPage(); }

    renderRows();
    container.append(grid);

    function setPlayhead(absCol) {
      grid.querySelectorAll('.step.is-play, .seq__steplab.is-play').forEach(c => c.classList.remove('is-play'));
      if (absCol < 0) return;
      const p = Math.floor(absCol / PAGE) + 1;
      if (FD.transport.follow && p !== page) setPage(p);
      if (p !== page) return;
      const i = absCol % PAGE;
      FD.instruments.forEach(inst => { const c = cellEls[inst.id][i]; if (c) c.classList.add('is-play'); });
      if (head._lbls[i]) head._lbls[i].classList.add('is-play');
    }
    return { select, setPage, getPage: () => page, setMode, setPlayhead, renderPage, renderRows, grid };
  }

  // ---- add-module menu (engine picker) ----
  let addEl = null;
  function ensureAdd() {
    if (addEl) return addEl;
    addEl = el('div', 'lanemenu');
    document.body.append(addEl);
    document.addEventListener('click', e => { if (addEl && !addEl.contains(e.target) && !e.target.classList.contains('seq__add')) addEl.classList.remove('open'); });
    return addEl;
  }
  function openAddMenu(x, y, onPick) {
    const m = ensureAdd(); m.innerHTML = '';
    m.append(el('div', 'lanemenu__h', { textContent: 'Add module' }));
    const groups = FD.engineList();
    const wrap = el('div', 'lanemenu__engines');
    Object.keys(groups).forEach(g => {
      wrap.append(el('div', 'lanemenu__grp', { textContent: g }));
      groups[g].forEach(e => {
        const b = el('button', 'lanemenu__eng', { textContent: e.label });
        b.addEventListener('click', () => { m.classList.remove('open'); onPick(e.type); });
        wrap.append(b);
      });
    });
    m.append(wrap);
    m.classList.add('open');
    m.style.left = Math.min(x, window.innerWidth - 230) + 'px';
    m.style.top = '0px';
    requestAnimationFrame(() => { m.style.top = Math.max(8, Math.min(y, window.innerHeight - m.offsetHeight - 12)) + 'px'; });
  }

  // ---- pattern → MIDI (General MIDI drum map, channel 10) ----
  // lanes are dynamic; map by engine/index to a sensible GM note
  const GM_BY_TAG = { BD:36, SD:38, HH:42, OH:46, TOM:45, CY:49, CP:39, PC:37, SMP:35, FX:40, MI:50 };
  const GM_FALLBACK = [36,38,42,46,50,45,48,41,39,51,49,37,40,43];
  function midiNote(inst, idx) { return GM_BY_TAG[inst.tag] || GM_FALLBACK[idx % GM_FALLBACK.length]; }
  function noteCount() {
    let n = 0; const len = FD.transport.len;
    FD.instruments.forEach(it => { if (FD.lanes[it.id].mute) return; const L = FD.pattern[it.id]; for (let s = 0; s < len; s++) if (L.hits[s]) n++; });
    return n;
  }
  function patternToMidi() {
    const div = 96, six = div / 4, len = FD.transport.len;
    const evs = [];
    FD.instruments.forEach((it, idx) => {
      if (FD.lanes[it.id].mute || !it.engine) return;
      const L = FD.pattern[it.id], note = midiNote(it, idx);
      for (let s = 0; s < len; s++) if (L.hits[s]) {
        const t = s * six, vel = L.plock[s] !== 'none' ? 120 : 100;
        evs.push({ t, on: 1, note, vel }); evs.push({ t: t + six - 2, on: 0, note, vel: 0 });
      }
    });
    evs.sort((a, b) => a.t - b.t || a.on - b.on);
    const body = []; let last = 0;
    const vlq = v => { const b = [v & 0x7f]; v >>= 7; while (v) { b.unshift((v & 0x7f) | 0x80); v >>= 7; } return b; };
    evs.forEach(e => { vlq(e.t - last).forEach(x => body.push(x)); last = e.t; body.push(e.on ? 0x99 : 0x89, e.note, e.vel); });
    vlq(0).forEach(x => body.push(x)); body.push(0xff, 0x2f, 0x00);
    const tl = body.length;
    const head = [0x4d,0x54,0x68,0x64, 0,0,0,6, 0,0, 0,1, (div>>8)&0xff, div&0xff];
    const trk = [0x4d,0x54,0x72,0x6b, (tl>>24)&0xff,(tl>>16)&0xff,(tl>>8)&0xff,tl&0xff, ...body];
    return new Uint8Array([...head, ...trk]);
  }
  function midiDataUrl() { let s = ''; patternToMidi().forEach(b => s += String.fromCharCode(b)); return 'data:audio/midi;base64,' + btoa(s); }

  window.FDCore = { el, slider, freq, toggle, select, drawADSR, buildEditor, buildSequencer, openPlock, openLaneMenu, patternToMidi, midiDataUrl, noteCount };
})();
