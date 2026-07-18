// dom.render.test.mjs — headless verification that the G3 DOM/FieldSim pass in
// app.mjs actually RENDERS kernel/field state into the DOM (zero browser, zero
// deps). It installs a minimal DOM shim on globalThis, dynamically imports
// app.mjs (which runs PASS A console driver + PASS B DOM render into the shim),
// then asserts:
//   • debug cards were filled from kernel exports (not left at the "–" default),
//     AND the rendered value equals a fresh independent kernel call (kernel is
//     the source of truth — the DOM mirrors kernel math, no JS re-impl).
//   • the #cv canvas received a REAL FieldSim frame (len === SIM_W*SIM_H*4, not
//     all-zero) via putImageData + a drawImage blit — proving the Sea render
//     executed, not the static fallback.
//   • the order surface: place → PENDING + money line; press CONFIRMED → status
//     advances; press DELIVERED (illegal) → flash bad, status unchanged.
//
// This is a GREEN gate for G3: if any assertion fails the process exits nonzero,
// exactly like kernel.test.mjs (no fake-green).
//
// Run:  node web/src/dom.render.test.mjs   (from repo root)

const isNode = typeof process !== 'undefined' && !!(process.versions && process.versions.node);
if (!isNode) {
  console.error('dom.render.test.mjs is a Node headless harness — run under node.');
  process.exit(2);
}

// ── DOM shim ────────────────────────────────────────────────────────────────
const registry = new Map();
const putLog = [];        // every putImageData(data,w,h) call
const drawLog = [];       // every drawImage call
let nanBail = false;

function makeCtx(ownerEl) {
  return {
    canvas: ownerEl,
    putImageData(img, x, y) {
      putLog.push({ data: Array.from(img.data), w: img.width, h: img.height });
    },
    drawImage() { drawLog.push({}); },
    createLinearGradient() { return { addColorStop() {} }; },
    set fillStyle(_) {}, get fillStyle() { return ''; },
    fillRect() {},
    clearRect() {},
  };
}

class El {
  constructor(tag) {
    this.tagName = tag;
    this.children = [];
    this._text = tag === 'canvas' ? '' : '–'; // mirrors index.html default "–"
    this.className = '';
    this.hidden = false;
    this.onclick = null;
    this.disabled = false;
    this._html = '';
    this.width = 0;
    this.height = 0;
    this.style = {};
  }
  set textContent(v) { this._text = String(v); }
  get textContent() { return this._text; }
  set innerHTML(v) { this._html = String(v); if (v === '') this.children = []; }
  get innerHTML() { return this._html; }
  appendChild(c) { this.children.push(c); return c; }
  getContext() { return makeCtx(this); }
  setAttribute(k, v) { this['__attr_' + k] = v; }
  getAttribute(k) { return this['__attr_' + k]; }
  addEventListener() {}
}

function mk(id, tag = 'div', opts = {}) {
  const e = new El(tag);
  e.id = id;
  if (opts.hidden) e.hidden = true;
  if (opts.width) e.width = opts.width;
  if (opts.height) e.height = opts.height;
  registry.set(id, e);
  return e;
}

// Cards present in index.html
mk('rem'); mk('snap'); mk('seg');
mk('rho'); mk('gap'); mk('fie'); mk('drift');
mk('fsm'); mk('acyc');
mk('boot-banner', 'div', { hidden: true });
mk('cv', 'canvas', { width: 800, height: 160 });
mk('tick', 'button');
mk('order-status'); mk('order-line'); mk('order-actions'); mk('order-flash', 'div', { hidden: true });
mk('place', 'button');

const documentShim = {
  getElementById: (id) => registry.get(id) || null,
  createElement: (tag) => new El(tag),
};

globalThis.document = documentShim;
globalThis.ImageData = class { constructor(data, w, h) { this.data = data; this.width = w; this.height = h; } };
globalThis.matchMedia = () => ({ matches: true }); // reduced-motion → single deterministic blit
globalThis.requestAnimationFrame = () => {};        // no animation loop in headless

// ── assert helpers ────────────────────────────────────────────────────────────
let failures = 0;
function ok(cond, msg) {
  if (cond) console.log('ok  -', msg);
  else { console.error('FAIL:', msg); failures++; }
}
function bail(msg) {
  console.error('FATAL:', msg);
  process.exit(1);
}

// ── run app.mjs into the shim ─────────────────────────────────────────────────
let appModule;
try {
  appModule = await import('./app.mjs');
} catch (e) {
  bail('app.mjs threw while rendering into the DOM shim: ' + (e && e.stack ? e.stack : e));
}

// ── assertions ──────────────────────────────────────────────────────────────
import {
  geo_progress_js, spectral_flat_js, spectral_classify_drift_js, fsm_graph_report_js,
  place_order_js, apply_event_js, estimate_order_total_js, boot_verify_fsm_js,
} from './lib/kernel/kernel_client.mjs';
import { parseJsonResult, validateProgressOut, validateSpectralFlat, validateFsmReport, validateOrderOut, validateEstimateOut } from './lib/kernel/contracts.mjs';

const POLY = '[[0,0],[10,0]]';
const MAT = '[[1,0],[0,1]]';

// 1) boot banner hidden (boot gate OK on live graph)
const banner = registry.get('boot-banner');
ok(banner.hidden === true, 'boot banner stays hidden (boot_verify_fsm_js OK)');

// 2) geo card populated from kernel + matches an independent kernel call
const rem = registry.get('rem');
 ok(rem.textContent !== '–' && rem.textContent.trim() !== '', 'geo #rem card was populated (not "–")');
const gp = parseJsonResult(geo_progress_js(POLY, 5, 1));
const gpv = gp.ok ? validateProgressOut(gp.value) : gp;
ok(gpv.ok && rem.textContent === `${gpv.value.remaining_m}`, `#rem mirrors kernel geo value (${rem.textContent} == ${gpv.value.remaining_m})`);
const snap = registry.get('snap');
ok(snap.textContent !== '–' && /, /.test(snap.textContent), 'geo #snap card populated with "lat, lng"');
const seg = registry.get('seg');
ok(seg.textContent !== '–', 'geo #seg card populated');

// 3) spectral card populated + matches independent kernel call
const rho = registry.get('rho');
const sf = parseJsonResult(spectral_flat_js(MAT));
const sfv = sf.ok ? validateSpectralFlat(sf.value) : sf;
ok(sfv.ok && rho.textContent === `${sfv.value.rho}`, `#rho mirrors kernel spectral_flat value (${rho.textContent} == ${sfv.value.rho})`);
ok(registry.get('gap').textContent !== '–', 'spectral #gap populated');
ok(registry.get('fie').textContent !== '–', 'spectral #fie populated');
const drift = spectral_classify_drift_js(MAT);
ok(drift.ok && registry.get('drift').textContent === drift.value, `#drift mirrors kernel classify (${registry.get('drift').textContent})`);

// 4) FSM card populated + matches independent kernel call
const fr = parseJsonResult(fsm_graph_report_js());
const frv = fr.ok ? validateFsmReport(fr.value) : fr;
const fsm = registry.get('fsm');
ok(frv.ok && fsm.textContent === `${frv.value.vertices} / ${frv.value.edges}`, `#fsm mirrors kernel fsm report (${fsm.textContent})`);
ok(registry.get('acyc').textContent === `${frv.value.is_acyclic}`, '#acyc mirrors kernel is_acyclic');

// 5) Sea FieldSim actually rendered a real frame to #cv (not the static fallback)
const SIM_W = 200, SIM_H = 40;
ok(putLog.length >= 1, `FieldSim frame was putImageData'd to canvas (${putLog.length} call(s))`);
const frame = putLog[putLog.length - 1];
ok(frame && frame.data.length === SIM_W * SIM_H * 4, `frame is real FieldSim dims (${frame ? frame.data.length : 0} === ${SIM_W * SIM_H * 4})`);
ok(frame && frame.w === SIM_W && frame.h === SIM_H, `frame ImageData sized to sim resolution (${frame && frame.w}x${frame && frame.h})`);
ok(frame && frame.data.some((b) => b !== 0), 'FieldSim frame has non-zero pixels (real sim, not blank)');
ok(drawLog.length >= 1, `canvas blit (drawImage) executed (${drawLog.length} call(s))`);

// 6) order surface: place → PENDING + money line
const place = registry.get('place');
ok(place && typeof place.onclick === 'function', 'place-order button wired');
place.onclick();
const status = registry.get('order-status');
const line = registry.get('order-line');
ok(status.textContent === 'PENDING', `placed order → status PENDING (got "${status.textContent}")`);
// money values are derived from the kernel independently — the DOM must mirror
// them (no JS math). Order subtotal = 2*500 + 300 = 1300 minor.
const CFG = JSON.stringify({
  is_pickup: false, delivery_fee_flat: 200, tax_rate: 0.1, price_includes_tax: false,
  has_distance_tiers: false, min_order_value: 1000,
});
const indep = parseJsonResult(estimate_order_total_js(1300, CFG));
const indepV = indep.ok ? validateEstimateOut(indep.value) : indep;
ok(indepV.ok, `independent kernel estimate at subtotal=1300 (got ${JSON.stringify(indepV).slice(0, 70)})`);
const expSub = (1300 / 100).toFixed(2);
const expFee = indepV.value.delivery_fee == null ? '—' : (indepV.value.delivery_fee / 100).toFixed(2);
const expTax = indepV.value.tax_total == null ? '—' : (indepV.value.tax_total / 100).toFixed(2);
ok(line.textContent.includes(`subtotal ${expSub}`), `money line mirrors kernel subtotal (got "${line.textContent}", expected ${expSub})`);
ok(line.textContent.includes(`est fee ${expFee}`), `money line mirrors kernel estimate delivery_fee (got "${line.textContent}", expected ${expFee})`);
ok(line.textContent.includes(`est tax ${expTax}`), `money line mirrors kernel estimate tax (got "${line.textContent}", expected ${expTax})`);

// 7) advance via candidate action CONFIRMED → status advances
const actions = registry.get('order-actions');
ok(actions.children.length === 4, `candidate action buttons rendered (${actions.children.length})`);
const confirmBtn = actions.children.find((b) => b.textContent === 'CONFIRMED');
ok(!!confirmBtn && typeof confirmBtn.onclick === 'function', 'CONFIRMED candidate button present + wired');
confirmBtn.onclick();
ok(status.textContent === 'CONFIRMED', `CONFIRMED press advanced FSM (got "${status.textContent}")`);

// 8) illegal transition DELIVERED → flash bad, state unchanged
const deliverBtn = actions.children.find((b) => b.textContent === 'DELIVERED');
deliverBtn.onclick();
ok(status.textContent === 'CONFIRMED', 'illegal DELIVERED press left status unchanged (kernel sole authority)');
const flashEl = registry.get('order-flash');
ok(flashEl.hidden === false && flashEl.className === 'bad', `illegal press surfaced a bad flash (got "${flashEl.textContent}", class=${flashEl.className})`);

// ── verdict ──────────────────────────────────────────────────────────────────
if (failures > 0) {
  console.error(`\nG3 DOM RENDER FAIL: ${failures} assertion(s) failed.`);
  process.exit(1);
}
console.log('\nG3 DOM RENDER GREEN — DOM populated from kernel state; Sea FieldSim blitted a real frame; order places/advances/refuses illegal.');
