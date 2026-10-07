// THE CANVAS LOADER (Wave CV, crates/dowiz-canvas): the only JavaScript between a surface's
// wasm and the browser. It owns exactly five things and nothing else:
//   1. the ONE <canvas>: its backing store is innerWidth x innerHeight x devicePixelRatio, set
//      from the WINDOW, never from the element's own size (memory sea-canvas-doubles-every-frame:
//      a size read back from the element and multiplied again grew a canvas to 3.3e7 px);
//   2. text widths for Rust (`txt_measure`, Canvas2D measureText, system fonts);
//   3. replaying the command buffer on Canvas2D (cmd.rs is the format; `replay` its one reader);
//   4. input: pointer, wheel and Enter go to wasm; a tap's consequence comes back as an intent;
//   5. the keyboard for a focused text field: EditContext where the browser has it (zero
//      elements), else ONE transient input element that exists only while the field is focused
//      (operator 2026-10-06; the dom-count gate counts it).
// Styles are set through CSSOM, which the page's CSP (`style-src 'self'`) does not govern; there
// is no inline <style> (memory dowiz-csp-blocked-every-stylesheet).
//
// WebGL2 (?gl=1) is NOT built in CV1: this box's headless Chromium rasterises nothing on WebGL
// (memory webgl-renders-nothing-on-the-box), so a GL path could not be verified here. ?gl=1 says
// so on the console and draws with Canvas2D.
//
// ASCII QUOTES ONLY (DOWIZ-COMMON-RULES rule 11).

const FONT = 'system-ui, -apple-system, Roboto, "Segoe UI", sans-serif';
const KIND = { 1: ['email', 'username'], 2: ['password', 'current-password'], 3: ['text', 'one-time-code'], 4: ['text', 'off'] };

export async function start(canvas, wasmUrl, { lang = 0, dark = 1, restoring = 0, onIntent = () => {} } = {}) {
  const t0 = performance.now();
  const q = new URL(location.href).searchParams;
  const dbg = { t0, frames: [], backend: '2d', transient: 0 };
  if (q.get('gl')) console.warn('canvas: the WebGL2 path is not built yet (CV1); drawing with Canvas2D');
  const de = document.documentElement, body = document.body;
  for (const [el, k, v] of [[de, 'height', '100%'], [body, 'margin', '0'], [body, 'height', '100%'], [body, 'overflow', 'hidden'],
    [canvas, 'display', 'block'], [canvas, 'width', '100vw'], [canvas, 'height', '100vh'], [canvas, 'touchAction', 'none'], [canvas, 'outline', 'none']]) el.style[k] = v;
  canvas.tabIndex = 0;

  let ctx = canvas.getContext('2d', { alpha: false });
  const enc = new TextEncoder(), dec = new TextDecoder();
  let mem = null, ex = null, dpr = 0, w = 0, h = 0, font = '', fill = -1;
  const u8 = () => new Uint8Array(mem.buffer);
  const str = (p, n) => dec.decode(u8().subarray(p, p + n));
  const css = c => `rgba(${c >>> 24},${(c >>> 16) & 255},${(c >>> 8) & 255},${(c & 255) / 255})`;
  const setFont = (px, wt) => { const f = `${wt} ${px}px ${FONT}`; if (f !== font) { ctx.font = f; font = f; } };
  const setFill = c => { if (c !== fill) { ctx.fillStyle = css(c); fill = c; } };

  const env = {
    txt_measure(p, n, px, wt) { setFont(px, wt); return ctx.measureText(str(p, n)).width; },
  };
  // `wasmUrl`: a URL, or a WebAssembly.Module promise (board.js compiles it early).
  const instance = typeof wasmUrl === 'string' ? (await WebAssembly.instantiateStreaming(fetch(wasmUrl), { env })).instance
    : await WebAssembly.instantiate(await wasmUrl, { env });
  ex = instance.exports; mem = ex.memory;
  dbg.t_instantiate_ms = performance.now() - t0;

  /// Write a string into wasm's input buffer; its byte length.
  const put = s => enc.encodeInto(String(s ?? ''), u8().subarray(ex.inbuf(), ex.inbuf() + ex.inbuf_cap())).written;

  function size() {
    const d = devicePixelRatio || 1;
    w = Math.max(1, Math.round(innerWidth)); h = Math.max(1, Math.round(innerHeight));
    canvas.width = Math.round(w * d); canvas.height = Math.round(h * d);
    ctx.setTransform(d, 0, 0, d, 0, 0); ctx.textBaseline = 'top'; font = ''; fill = -1;
    if (d !== dpr && dpr) ex.forget_widths();
    dpr = d;
    ex.resize(w, h);
  }

  function replay(n) {
    const W = new Int32Array(mem.buffer, ex.cmd_ptr(), n);
    for (let i = 0; i < n;) {
      const op = W[i];
      if (op === 1) {
        const [x, y, rw, rh, r, c] = [W[i + 1], W[i + 2], W[i + 3], W[i + 4], W[i + 5], W[i + 6]];
        setFill(c);
        if (r > 0 && ctx.roundRect) { ctx.beginPath(); ctx.roundRect(x, y, rw, rh, r); ctx.fill(); } else ctx.fillRect(x, y, rw, rh);
        i += 8;
      } else if (op === 2) {
        setFont(W[i + 5], W[i + 6]); setFill(W[i + 7]);
        ctx.fillText(str(W[i + 1], W[i + 2]), W[i + 3], W[i + 4]);
        i += 8;
      } else if (op === 3) {
        ctx.save(); ctx.beginPath(); ctx.rect(W[i + 1], W[i + 2], W[i + 3], W[i + 4]); ctx.clip(); i += 5;
      } else if (op === 4) {
        ctx.restore(); font = ''; fill = -1; i += 1;
      } else if (op === 5) {
        const lw = W[i + 7], o = lw / 2;
        ctx.strokeStyle = css(W[i + 6]); ctx.lineWidth = lw; ctx.beginPath();
        if (ctx.roundRect) ctx.roundRect(W[i + 1] + o, W[i + 2] + o, W[i + 3] - lw, W[i + 4] - lw, W[i + 5]); else ctx.rect(W[i + 1] + o, W[i + 2] + o, W[i + 3] - lw, W[i + 4] - lw);
        ctx.stroke(); i += 8;
      } else { console.error('canvas: unknown op', op, 'at', i); break; }
    }
  }

  let raf = 0, lastNow = Date.now();
  /// One frame now. `now` defaults to the wall clock (ticket ages); a restore passes the old one.
  function draw(now = Date.now()) {
    lastNow = now;
    const a = performance.now();
    replay(ex.frame(now));
    dbg.frames.push(performance.now() - a);
    if (dbg.frames.length > 600) dbg.frames.splice(0, 300);
  }
  const ask = () => { if (!raf) raf = requestAnimationFrame(() => { raf = 0; draw(); }); };

  // ── the keyboard ───────────────────────────────────────────────────────────
  const useEC = 'EditContext' in window && !q.get('noec');
  let ec = null, input = null;
  const fieldValue = f => str(ex.field_ptr(f), ex.field_len(f));
  const setField = (f, v) => { ex.field_set(f, put(v)); ask(); };
  function dropInput() { if (input) { const el = input; input = null; el.remove(); } }
  function focusField(f, kind, rect) {
    const v = fieldValue(f);
    if (useEC) {
      if (!ec) {
        ec = new EditContext(); canvas.editContext = ec;
        ec.addEventListener('textupdate', () => { const g = ex.focused(); if (g !== 255) setField(g, ec.text); });
      }
      ec.updateText(0, ec.text.length, v); ec.updateSelection(v.length, v.length);
      const b = new DOMRect(rect[0], rect[1], rect[2], rect[3]);
      ec.updateControlBounds(b); ec.updateSelectionBounds(b);
      canvas.focus();
      return;
    }
    dropInput();
    const el = document.createElement('input');
    const [type, ac] = KIND[kind] || KIND[3];
    el.type = type; el.autocomplete = ac; el.value = v; el.setAttribute('autocapitalize', 'off'); el.spellcheck = false;
    for (const [k, val] of [['position', 'fixed'], ['left', rect[0] + 'px'], ['top', rect[1] + 'px'], ['width', rect[2] + 'px'],
      ['height', rect[3] + 'px'], ['fontSize', '16px'], ['opacity', '0'], ['border', '0'], ['padding', '0']]) el.style[k] = val;
    el.addEventListener('input', () => setField(f, el.value));
    el.addEventListener('keydown', e => { if (e.key === 'Enter') { e.preventDefault(); ex.key_enter(); drain(); } });
    el.addEventListener('blur', () => { if (input === el) { dropInput(); ex.blur(); ask(); } });
    input = el; dbg.transient += 1;
    body.appendChild(el); el.focus();
  }
  function blurField() { dropInput(); if (ec && document.activeElement === canvas) canvas.blur(); }

  // ── intents: what a tap asked for ──────────────────────────────────────────
  function drain() {
    for (let k = 0; k < 8; k++) {
      const I = new Int32Array(mem.buffer, ex.intent(), 8);
      const kind = I[0];
      if (!kind) break;
      if (kind === 6) focusField(I[1], I[2], [I[4], I[5], I[6], I[7]]);
      else if (kind === 7) blurField();
      const text = I[3] > 0 ? str(I[2], I[3]) : '';
      try { onIntent(kind, I[1], text); } catch (e) { console.error('canvas intent', kind, e); }
    }
    ask();
  }

  const P = (k, e) => { if (ex.pointer(k, Math.round(e.clientX), Math.round(e.clientY))) draw(); drain(); };
  canvas.addEventListener('pointerdown', e => { try { canvas.setPointerCapture(e.pointerId); } catch {} P(0, e); });
  canvas.addEventListener('pointermove', e => { if (e.buttons) P(1, e); });
  canvas.addEventListener('pointerup', e => P(2, e));
  canvas.addEventListener('pointercancel', e => P(3, e));
  canvas.addEventListener('wheel', e => { e.preventDefault(); ex.wheel(Math.round(e.deltaY)); ask(); }, { passive: false });
  canvas.addEventListener('keydown', e => { if (e.key === 'Enter') { ex.key_enter(); drain(); } });
  addEventListener('resize', () => { size(); draw(); });

  // ── context loss: the scene lives in Rust, so a restore is one frame ──────
  // Canvas2D can lose its backing store (Chrome fires contextlost/contextrestored on a 2D
  // canvas); everything drawn is gone and the context's state is reset. Rust still holds the
  // whole scene, so a restore re-sizes and redraws the SAME frame (same `now`): the
  // context-loss gate compares its hash and pixels with the frame before the loss.
  canvas.addEventListener('contextlost', e => { e.preventDefault(); dbg.lost = (dbg.lost || 0) + 1; });
  canvas.addEventListener('contextrestored', () => { dbg.restored = (dbg.restored || 0) + 1; size(); draw(lastNow); });

  ex.init(1, 1, dark ? 1 : 0, lang);
  if (restoring) ex.session(2, 0, 0, 0);   // "Loading", not the login form
  size();
  draw();
  dbg.t_first_frame_ms = performance.now() - t0;
  Object.assign(dbg, {
    ex, ask, draw, put, drain,
    redraw: () => draw(lastNow),
    hash: () => ex.frame_hash() >>> 0,
    stats: () => Array.from(new Uint32Array(mem.buffer, ex.stats(), 10)),
    tours: () => str(ex.inbuf(), ex.tour_list()).split('\n').filter(Boolean),
    bumpRect: id => { const r = new Int32Array(mem.buffer, ex.bump_rect(put(id)), 5); return r[0] ? { x: r[1], y: r[2], w: r[3], h: r[4] } : null; },
    tourRect: name => { const r = new Int32Array(mem.buffer, ex.tour_rect(put(name)), 5); return r[0] ? { x: r[1], y: r[2], w: r[3], h: r[4] } : null; },
  });
  return dbg;
}
