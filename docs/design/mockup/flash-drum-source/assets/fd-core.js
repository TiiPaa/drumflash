/* Flash Drum — shared control + render library (window.FDCore) */
(function () {
  const FD = window.FD;
  function el(tag, cls, props) {
    const n = document.createElement(tag);
    if (cls) n.className = cls;
    if (props) Object.assign(n, props);
    return n;
  }
  const clamp = (v, a, b) => Math.min(b, Math.max(a, v));

  // ---- horizontal slider -------------------------------------------------
  function slider(spec, onChange) {
    const wrap = el('div', 'ctl ctl--slider');
    const lab = el('span', 'ctl__label', { textContent: spec.label });
    const track = el('div', 'sld');
    const fill = el('div', 'sld__fill');
    const knob = el('div', 'sld__knob');
    track.append(fill, knob);
    const val = el('span', 'ctl__val');
    wrap.append(lab, track, val);
    let v = spec.value;
    const fmt = spec.fmt || (x => x.toFixed(2));
    function render() {
      const t = (v - spec.min) / (spec.max - spec.min);
      fill.style.width = (t * 100) + '%';
      knob.style.left = (t * 100) + '%';
      val.textContent = fmt(v) + (spec.unit ? ' ' + spec.unit : '');
    }
    function setFromX(clientX) {
      const r = track.getBoundingClientRect();
      let t = clamp((clientX - r.left) / r.width, 0, 1);
      v = spec.min + t * (spec.max - spec.min);
      if (spec.step) v = Math.round(v / spec.step) * spec.step;
      v = clamp(v, spec.min, spec.max);
      render();
      onChange && onChange(v);
    }
    let drag = false;
    track.addEventListener('pointerdown', e => { drag = true; track.setPointerCapture(e.pointerId); setFromX(e.clientX); });
    track.addEventListener('pointermove', e => { if (drag) setFromX(e.clientX); });
    track.addEventListener('pointerup', e => { drag = false; });
    render();
    wrap.api = { get: () => v, set: x => { v = x; render(); } };
    return wrap;
  }

  // ---- rotary knob -------------------------------------------------------
  function knob(spec, onChange) {
    const wrap = el('div', 'ctl ctl--knob');
    const dial = el('div', 'knb');
    const SZ = 46, R = 18, CX = SZ / 2, CY = SZ / 2;
    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    svg.setAttribute('viewBox', `0 0 ${SZ} ${SZ}`);
    const A0 = 135, A1 = 405; // sweep degrees
    const polar = (deg, rad) => {
      const a = (deg - 90) * Math.PI / 180;
      return [CX + rad * Math.cos(a), CY + rad * Math.sin(a)];
    };
    const arc = (deg0, deg1, rad) => {
      const [x0, y0] = polar(deg0, rad); const [x1, y1] = polar(deg1, rad);
      const large = (deg1 - deg0) % 360 > 180 ? 1 : 0;
      return `M ${x0} ${y0} A ${rad} ${rad} 0 ${large} 1 ${x1} ${y1}`;
    };
    const bg = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    bg.setAttribute('d', arc(A0, A1, R)); bg.setAttribute('class', 'knb__bg');
    const fg = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    fg.setAttribute('class', 'knb__fg');
    const tick = document.createElementNS('http://www.w3.org/2000/svg', 'line');
    tick.setAttribute('class', 'knb__tick');
    svg.append(bg, fg, tick);
    dial.append(svg);
    const lab = el('span', 'ctl__label', { textContent: spec.label });
    const val = el('span', 'ctl__val');
    wrap.append(dial, lab, val);
    let v = spec.value;
    const fmt = spec.fmt || (x => x.toFixed(2));
    function render() {
      const t = (v - spec.min) / (spec.max - spec.min);
      const ang = A0 + t * (A1 - A0);
      fg.setAttribute('d', arc(A0, ang, R));
      const [tx, ty] = polar(ang, R - 3); const [bx, by] = polar(ang, 6);
      tick.setAttribute('x1', bx); tick.setAttribute('y1', by);
      tick.setAttribute('x2', tx); tick.setAttribute('y2', ty);
      val.textContent = fmt(v) + (spec.unit ? ' ' + spec.unit : '');
    }
    let drag = false, lastY = 0;
    dial.addEventListener('pointerdown', e => { drag = true; lastY = e.clientY; dial.setPointerCapture(e.pointerId); });
    dial.addEventListener('pointermove', e => {
      if (!drag) return;
      const dy = lastY - e.clientY; lastY = e.clientY;
      v = clamp(v + dy / 140 * (spec.max - spec.min), spec.min, spec.max);
      if (spec.step) v = Math.round(v / spec.step) * spec.step;
      render(); onChange && onChange(v);
    });
    dial.addEventListener('pointerup', () => { drag = false; });
    render();
    wrap.api = { get: () => v, set: x => { v = x; render(); } };
    return wrap;
  }

  // ---- switch ------------------------------------------------------------
  function toggle(spec, onChange) {
    const wrap = el('div', 'ctl ctl--switch');
    const lab = el('span', 'ctl__label', { textContent: spec.label });
    const sw = el('button', 'sw');
    let v = spec.value;
    const render = () => sw.classList.toggle('sw--on', !!v);
    sw.addEventListener('click', () => { v = !v; render(); onChange && onChange(v); });
    wrap.append(lab, sw);
    render();
    wrap.api = { get: () => v, set: x => { v = x; render(); } };
    return wrap;
  }

  // ---- select ------------------------------------------------------------
  function select(spec, onChange) {
    const wrap = el('div', 'ctl ctl--select');
    const lab = el('span', 'ctl__label', { textContent: spec.label });
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
    sel.addEventListener('click', () => menu.classList.toggle('open'));
    document.addEventListener('click', e => { if (!sel.contains(e.target)) menu.classList.remove('open'); });
    wrap.append(lab, sel);
    wrap.api = { get: () => cur.textContent, set: x => { cur.textContent = x; } };
    return wrap;
  }

  // ---- ADSR graph --------------------------------------------------------
  function drawADSR(canvas, p) {
    const dpr = window.devicePixelRatio || 1;
    const w = canvas.clientWidth, h = canvas.clientHeight;
    if (!w || !h) { requestAnimationFrame(() => drawADSR(canvas, p)); return; }
    canvas.width = w * dpr; canvas.height = h * dpr;
    const ctx = canvas.getContext('2d');
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);
    const padX = 10, padY = 12, gx = w - padX * 2, gy = h - padY * 2;
    const baseY = h - padY, topY = padY;
    // grid
    ctx.strokeStyle = 'rgba(255,255,255,0.05)'; ctx.lineWidth = 1;
    for (let i = 0; i <= 4; i++) { const x = padX + gx * i / 4; ctx.beginPath(); ctx.moveTo(x, topY); ctx.lineTo(x, baseY); ctx.stroke(); }
    const a = Math.max(0.02, p.attack ?? 0.1), d = Math.max(0.05, p.decay ?? 0.5), r = Math.max(0.05, p.release ?? 0.5);
    const tot = a + d + r;
    const xA = padX + gx * (a / tot);
    const xD = padX + gx * ((a + d) / tot);
    const xR = padX + gx;
    const dc = (p.decayC ?? 0.55), rc = (p.relC ?? 0.4);
    function curveSeg(x0, y0, x1, y1, k, color, sustainFloor) {
      ctx.beginPath(); ctx.strokeStyle = color; ctx.lineWidth = 2;
      const N = 40;
      for (let i = 0; i <= N; i++) {
        const t = i / N;
        const e = Math.pow(t, 1 + k); // ease
        const x = x0 + (x1 - x0) * t;
        const y = y0 + (y1 - y0) * e;
        i ? ctx.lineTo(x, y) : ctx.moveTo(x, y);
      }
      ctx.stroke();
    }
    // attack (amber)
    ctx.beginPath(); ctx.strokeStyle = '#f5b42c'; ctx.lineWidth = 2;
    ctx.moveTo(padX, baseY); ctx.lineTo(xA, topY); ctx.stroke();
    // decay (blue) to ~30% sustain
    const susY = topY + gy * 0.7;
    curveSeg(xA, topY, xD, susY, dc, '#3b9eff');
    // release (purple)
    curveSeg(xD, susY, xR, baseY, rc, '#b97bff');
    // labels
    ctx.fillStyle = 'rgba(255,255,255,0.55)'; ctx.font = '10px ui-monospace, monospace';
    ctx.fillText('A', xA - 4, topY + 11);
    ctx.fillText('D', xD - 10, susY - 4);
    ctx.fillText('R', xR - 14, baseY - 4);
  }

  // ---- dynamic editor ----------------------------------------------------
  // style: 'slider' | 'knob'. cols layout from schema.
  function buildEditor(container, instId, opts = {}) {
    container.innerHTML = '';
    const inst = FD.instruments.find(i => i.id === instId);
    const schema = FD.schemaFor(inst.cat);
    const p = FD.params[instId];
    let adsrCanvas = null;
    const refreshADSR = () => adsrCanvas && drawADSR(adsrCanvas, p);
    schema.forEach(sec => {
      const block = el('section', 'edsec');
      const head = el('div', 'edsec__h');
      head.append(el('span', 'edsec__t', { textContent: sec.title }));
      block.append(head);
      const body = el('div', 'edsec__b');
      body.style.setProperty('--cols', sec.cols || 1);
      let useKnob = opts.style === 'knob' && sec.title !== 'Level';
      if (opts.knobFor) useKnob = opts.knobFor(sec);
      (sec.items || []).forEach(c => {
        let node;
        if (c.kind === 'slider') node = (useKnob ? knob : slider)(c, v => { p[c.key] = v; if (['attack','decay','decayC','release','relC'].includes(c.key)) refreshADSR(); });
        else if (c.kind === 'select') node = select(c, v => { p[c.key] = v; });
        else if (c.kind === 'switch') node = toggle(c, v => { p[c.key] = v; });
        if (node) body.append(node);
      });
      block.append(body);
      // ADSR visual attached to envelope section
      if (sec.adsr) {
        const fig = el('div', 'adsr');
        adsrCanvas = el('canvas', 'adsr__c');
        const legend = el('div', 'adsr__leg');
        legend.innerHTML = '<i class="dot dot--a"></i>A <i class="dot dot--d"></i>D <i class="dot dot--r"></i>R';
        fig.append(adsrCanvas, legend);
        block.append(fig);
      }
      container.append(block);
    });
    requestAnimationFrame(refreshADSR);
    setTimeout(refreshADSR, 80);
    if (document.fonts && document.fonts.ready) document.fonts.ready.then(refreshADSR);
  }

  // ---- sequencer ---------------------------------------------------------
  function buildSequencer(container, opts = {}) {
    container.innerHTML = '';
    const grid = el('div', 'seq');
    const showExtras = opts.showExtras !== false;
    const showVol = opts.showVol !== false;
    const colorRows = !!opts.colorRows;
    const onSelect = opts.onSelect || (() => {});
    let selected = opts.selected;

    // header row
    const head = el('div', 'seq__row seq__row--head');
    head.append(el('div', 'seq__name'));
    if (showVol) head.append(el('div', 'seq__vol', { textContent: 'Vol' }));
    head.append(el('div', 'seq__ms'));
    const steps = el('div', 'seq__steps');
    FD.stepLabels.forEach(n => {
      const c = el('div', 'seq__steplab', { textContent: n });
      if ((n - 1) % 4 === 0) c.classList.add('is-beat');
      steps.append(c);
    });
    head.append(steps);
    if (showExtras) {
      head.append(el('div', 'seq__extra', { textContent: 'Hum' }));
      head.append(el('div', 'seq__extra', { textContent: 'Push' }));
      head.append(el('div', 'seq__extra', { textContent: 'Len' }));
    }
    grid.append(head);

    const rowEls = {};
    FD.instruments.forEach(inst => {
      const row = el('div', 'seq__row');
      row.dataset.id = inst.id;
      if (colorRows) row.style.setProperty('--ihue', inst.hue);
      const name = el('button', 'seq__name', { textContent: inst.id, title: inst.name });
      name.addEventListener('click', () => { selectRow(inst.id); onSelect(inst.id); });
      row.append(name);
      if (showVol) {
        const vwrap = el('div', 'seq__vol');
        const vt = el('div', 'minisld'); const vf = el('div', 'minisld__f');
        vt.append(vf); vf.style.width = (FD.lanes[inst.id].vol * 100) + '%';
        let vd = false;
        const setV = cx => { const r = vt.getBoundingClientRect(); const t = clamp((cx - r.left) / r.width, 0, 1); FD.lanes[inst.id].vol = t; vf.style.width = (t * 100) + '%'; };
        vt.addEventListener('pointerdown', e => { vd = true; vt.setPointerCapture(e.pointerId); setV(e.clientX); });
        vt.addEventListener('pointermove', e => { if (vd) setV(e.clientX); });
        vt.addEventListener('pointerup', () => vd = false);
        vwrap.append(vt); row.append(vwrap);
      }
      const ms = el('div', 'seq__ms');
      const m = el('button', 'tag tag--m', { textContent: 'M' });
      const s = el('button', 'tag tag--s', { textContent: 'S' });
      m.addEventListener('click', () => { FD.lanes[inst.id].mute = !FD.lanes[inst.id].mute; m.classList.toggle('on', FD.lanes[inst.id].mute); });
      s.addEventListener('click', () => { FD.lanes[inst.id].solo = !FD.lanes[inst.id].solo; s.classList.toggle('on', FD.lanes[inst.id].solo); });
      ms.append(m, s); row.append(ms);

      const stepsW = el('div', 'seq__steps');
      const cells = [];
      FD.pattern[inst.id].forEach((st, i) => {
        const cell = el('button', 'step');
        if ((i) % 4 === 0) cell.classList.add('is-beat');
        const setState = v => { cell.dataset.s = v; };
        setState(st);
        cell.addEventListener('click', () => {
          let v = (+cell.dataset.s + 1) % 3;
          FD.pattern[inst.id][i] = v; setState(v);
        });
        cells.push(cell); stepsW.append(cell);
      });
      row.append(stepsW);

      if (showExtras) {
        ['hum', 'push', 'len'].forEach(k => {
          const ex = el('div', 'seq__extra');
          if (k === 'len') ex.append(el('span', 'ex__num', { textContent: FD.lanes[inst.id].len }));
          else if (k === 'push') ex.append(el('span', 'ex__num', { textContent: '0 ms' }));
          else { const b = el('div', 'minisld minisld--dim'); const f = el('div', 'minisld__f'); f.style.width = '8%'; b.append(f); ex.append(b); }
          row.append(ex);
        });
      }
      grid.append(row);
      rowEls[inst.id] = row;
    });

    function selectRow(id) {
      selected = id;
      Object.values(rowEls).forEach(r => r.classList.toggle('is-sel', r.dataset.id === id));
    }
    if (selected) selectRow(selected);
    container.append(grid);

    // playhead
    let playCol = -1;
    function setPlayhead(col) {
      grid.querySelectorAll('.step.is-play').forEach(c => c.classList.remove('is-play'));
      grid.querySelectorAll('.seq__steplab.is-play').forEach(c => c.classList.remove('is-play'));
      playCol = col;
      if (col < 0) return;
      grid.querySelectorAll('.seq__row:not(.seq__row--head)').forEach(r => {
        const cell = r.querySelectorAll('.step')[col];
        if (cell) cell.classList.add('is-play');
      });
      const lab = grid.querySelectorAll('.seq__steplab')[col];
      if (lab) lab.classList.add('is-play');
    }
    return { selectRow, setPlayhead, grid };
  }

  window.FDCore = { el, slider, knob, toggle, select, drawADSR, buildEditor, buildSequencer };
})();
