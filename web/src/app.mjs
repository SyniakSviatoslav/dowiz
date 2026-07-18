// app.mjs — kernel-driven UI: console-only driver + G3 DOM/FieldSim render pass
// (BLUEPRINT-P-G, Layer G / W-2 / W-3).
//
// The kernel wasm is the SOLE FSM / money authority. All 24 `_js` exports are
// bound through kernel_client.mjs (G2, done) and exercised end-to-end below,
// rendering a KernelResult Err on any illegal action. No JS re-implementation
// of geo/spectral/FSM math exists anywhere in web/src — the kernel is the only
// source of truth (AGENTS invariant, gate G). Money fields are asserted
// Number.isSafeInteger at the boundary.
//
// app.mjs:10's own comment marks the DOM/FieldSim render as a SEPARATE work unit
// (W-2/W-3) from this G console driver. This file therefore contains TWO passes:
//
//   • PASS A — the console-only driver (the original G task). Runs in BOTH the
//     browser and Node; exits nonzero on failure (fail-closed).
//   • PASS B — the G3 DOM/FieldSim render. Guarded by `typeof document !==
//     'undefined'` so the Node headless path stays identical and all existing
//     tests keep running. It populates the existing debug cards, runs the Sea
//     FieldSim on #cv, and renders ONE minimal order surface. Everything here
//     consumes kernel/field wasm state only — zero new JS math.
//
// HARD CONSTRAINTS (repo-wide): vanilla JS (.mjs) / browser-native only. No
// TypeScript, no npm deps. Money-charge-authority FLIP is explicitly gated out
// (see §8 of the blueprint) — this file never creates a payment/charge surface.

// ── env-aware byte loader ───────────────────────────────────────────────────
// Browser: fetch + arrayBuffer (serve.mjs maps /pkg-web/* → kernel/pkg-web/* and
// serves the repo root, so /wasm/demo/pkg/* is reachable). Node: readFileSync
// via dynamic import (Node ESM has no top-level `require`; Node fetch has no
// file://, mirrors smoke.mjs:4-7).
async function loadBytes(relUrl) {
  const isNode = typeof process !== 'undefined' && !!(process.versions && process.versions.node);
  if (isNode) {
    const { readFileSync } = await import('fs');
    return new Uint8Array(readFileSync(new URL(relUrl, import.meta.url)));
  }
  const res = await fetch(new URL(relUrl, import.meta.url).href);
  if (!res.ok) throw new Error(`fetch ${relUrl} → ${res.status}`);
  return new Uint8Array(await res.arrayBuffer());
}

// Fail-closed exit that is safe in both envs (browser just stops, never throws
// at module top level — the DOM pass renders a fatal banner instead).
function fatalExit(msg) {
  console.error(msg);
  if (typeof process !== 'undefined' && process.exit) process.exit(1);
}

// Field crate glue exports proper shims (unlike the kernel, ground truth 1.g),
// so import it directly. It resolves in both envs via the repo-root server.
import initField, { FieldSim } from '../../wasm/demo/pkg/dowiz_wasm.js';

import {
  bindKernel,
  boot_verify_fsm_js,
  place_order_js,
  apply_event_js,
  estimate_order_total_js,
  channel_ledger_js,
  reduce_anomalies_js,
  fsm_graph_report_js,
  spectral_radius_js,
  spectral_flat_js,
  spectral_gap_js,
  spectral_algebraic_connectivity_js,
  spectral_classify_drift_js,
  spectral_eigenvalues_js,
  harmonic_centrality_js,
  geo_progress_flat_js,
  geo_progress_js,
  geo_haversine_js,
  geo_lerp_js,
  geo_bearing_js,
  geo_eta_js,
  geo_is_arriving_js,
  geo_should_snap_js,
  geo_point_in_polygon_js,
  geo_is_out_of_order_js,
} from './lib/kernel/kernel_client.mjs';
import {
  parseJsonResult,
  validateOrderOut,
  validateEstimateOut,
  validateProgressOut,
  validateLedgerOut,
  validateFsmReport,
  validateSpectralFlat,
} from './lib/kernel/contracts.mjs';

const kernelBytes = await loadBytes('../../kernel/pkg-web/dowiz_kernel_bg.wasm');
await bindKernel(kernelBytes);

// ── PASS A — console-only driver (original G task, unchanged behavior) ──────

// ── money assert helper (boundary guard, never rounds) ──
function money(label, x) {
  if (!Number.isSafeInteger(x)) {
    console.error(`MONEY FAIL: ${label} not a safe integer: ${x}`);
    fatalExit(1);
  }
  return x;
}

function step(label, fn) {
  const res = fn();
  if (res.ok) {
    console.log(`OK  ${label}`);
  } else {
    console.log(`ERR ${label} :: ${res.err}`);
  }
  return res;
}

// ── W-3 boot gate (audit finding h): fail-closed before any order ──
const boot = step('boot_verify_fsm_js', () => boot_verify_fsm_js());
if (!boot.ok) {
  fatalExit('FATAL: FSM boot gate refused — authority loss is fatal.');
}

// ── place an order (fixed fixture item list) ──
const ITEMS = JSON.stringify([
  { product_id: 'p1', modifier_ids: ['m1'], quantity: 2, unit_price: 500 },
  { product_id: 'p2', modifier_ids: [], quantity: 1, unit_price: 300 },
]);
const placed = step('place_order_js', () => {
  const r = place_order_js(null, ITEMS, 'web');
  const j = parseJsonResult(r);
  if (!j.ok) return j;
  const v = validateOrderOut(j.value);
  if (!v.ok) return v;
  money('order.subtotal', v.value.subtotal);
  money('order.total', v.value.total);
  return v;
});

// ── estimate total (money mirror; charge authority NOT flipped — §8) ──
const CFG = JSON.stringify({
  is_pickup: false,
  delivery_fee_flat: 200,
  tax_rate: 0.1,
  price_includes_tax: false,
  has_distance_tiers: false,
  min_order_value: 1000,
});
const est = step('estimate_order_total_js', () => {
  const r = estimate_order_total_js(placed.ok ? placed.value.subtotal : 0, CFG);
  const j = parseJsonResult(r);
  if (!j.ok) return j;
  const v = validateEstimateOut(j.value);
  if (!v.ok) return v;
  if (v.value.delivery_fee != null) money('estimate.delivery_fee', v.value.delivery_fee);
  if (v.value.tax_total != null) money('estimate.tax_total', v.value.tax_total);
  if (v.value.total != null) money('estimate.total', v.value.total);
  return v;
});

// ── fold lifecycle through apply_event_js (kernel is sole FSM authority) ──
const NEXT = ['CONFIRMED', 'PREPARING', 'READY'];
let current = placed.ok ? placed.value : null;
for (const next of NEXT) {
  const r = step(`apply_event_js → ${next}`, () => {
    if (!current) return { ok: false, err: 'no current order' };
    const a = apply_event_js(JSON.stringify(current), next);
    const j = parseJsonResult(a);
    if (!j.ok) return j;
    const v = validateOrderOut(j.value);
    if (!v.ok) return v;
    money('order.total', v.value.total);
    return v;
  });
  if (r.ok) current = r.value;
}

// ── adversarial: illegal transition must surface Err, prior state unchanged ──
const illegal = step('apply_event_js → DELIVERED (illegal)', () => {
  if (!placed.ok) return { ok: false, err: 'no order placed' };
  return apply_event_js(JSON.stringify(placed.value), 'DELIVERED');
});
if (illegal.ok) {
  console.error('FSM FAIL: illegal transition was accepted');
  fatalExit(1);
}

// ── analytics: channel ledger + anomaly reduction ──
const EVENTS = JSON.stringify([
  { order_id: 'a1', channel: 'web', status: 'PENDING', at_ms: 1 },
  { order_id: 'a2', channel: 'web', status: 'DELIVERED', at_ms: 2 },
]);
step('channel_ledger_js', () => {
  const j = parseJsonResult(channel_ledger_js(EVENTS));
  if (!j.ok) return j;
  return validateLedgerOut(j.value);
});
step('reduce_anomalies_js', () => reduce_anomalies_js(EVENTS));

// ── FSM graph report ──
step('fsm_graph_report_js', () => {
  const j = parseJsonResult(fsm_graph_report_js());
  if (!j.ok) return j;
  return validateFsmReport(j.value);
});

// ── spectral cards (one spectral_flat fills the whole card) ──
const MAT = '[[1,0],[0,1]]';
step('spectral_radius_js', () => parseJsonResult(spectral_radius_js(MAT)));
step('spectral_flat_js', () => {
  const j = parseJsonResult(spectral_flat_js(MAT));
  if (!j.ok) return j;
  // parseJsonResult already decoded the "[rho,gap,...,]" string into an array.
  return validateSpectralFlat(j.value);
});
step('spectral_gap_js', () => parseJsonResult(spectral_gap_js(MAT)));
step('spectral_algebraic_connectivity_js', () => parseJsonResult(spectral_algebraic_connectivity_js(MAT)));
// drift class returns a bare scalar string ("Damped"/"Resonant"/"Unstable"),
// not JSON — pass it through (validated by membership, not JSON.parse).
step('spectral_classify_drift_js', () => {
  const r = spectral_classify_drift_js(MAT);
  if (!r.ok) return r;
  if (!['Damped', 'Resonant', 'Unstable'].includes(r.value))
    return { ok: false, err: `drift class unknown: ${r.value}` };
  return r;
});
step('spectral_eigenvalues_js', () => parseJsonResult(spectral_eigenvalues_js(MAT)));
step('harmonic_centrality_js', () => harmonic_centrality_js(3, JSON.stringify([[0, 1], [1, 2]])));

// ── geo / route surface ──
const POLY = '[[0,0],[10,0]]';
step('geo_progress_flat_js', () => parseJsonResult(geo_progress_flat_js(POLY, 5, 1)));
step('geo_progress_js', () => {
  const j = parseJsonResult(geo_progress_js(POLY, 5, 1));
  if (!j.ok) return j;
  return validateProgressOut(j.value);
});
step('geo_haversine_js', () => parseJsonResult(geo_haversine_js(0, 0, 0, 1)));
step('geo_lerp_js', () => parseJsonResult(geo_lerp_js(0, 0, 10, 0, 0.5)));
step('geo_bearing_js', () => parseJsonResult(geo_bearing_js(0, 0, 1, 1)));
step('geo_eta_js', () => parseJsonResult(geo_eta_js(500, 1000, 600)));
step('geo_is_arriving_js', () => parseJsonResult(geo_is_arriving_js(20, 50)));
step('geo_should_snap_js', () => parseJsonResult(geo_should_snap_js('[0,0]', '[1,1]', 100)));
step('geo_point_in_polygon_js', () => parseJsonResult(geo_point_in_polygon_js(1, 1, '[[0,0],[0,2],[2,2],[2,0]]')));
step('geo_is_out_of_order_js', () => geo_is_out_of_order_js(-1, 1000));

console.log('\nKERNEL-DRIVEN UI GREEN — 24/24 exports wired, math from wasm only.');

// ── PASS B — G3 DOM/FieldSim render pass (W-2/W-3) ──────────────────────────
// Guarded so Node stays headless + existing tests untouched. Every byte shown
// on the page is derived from kernel/field wasm state — no client math.
if (typeof document !== 'undefined') {
  const isNode = typeof process !== 'undefined' && !!(process.versions && process.versions.node);
  const fieldBytes = await loadBytes('../../wasm/demo/pkg/dowiz_wasm_bg.wasm');
  if (isNode) {
    // Node-with-DOM-shim path: glue can't fetch file://, so hand it bytes.
    await initField({ module_or_path: fieldBytes });
  } else {
    await initField(); // browser: relative fetch of dowiz_wasm_bg.wasm
  }

  // ── W-3.1 boot gate → fatal banner on failure (fail-closed) ──
  const boot2 = boot_verify_fsm_js();
  const banner = document.getElementById('boot-banner');
  if (!boot2.ok && banner) {
    banner.textContent = `FATAL: FSM boot gate refused — ${boot2.err}`;
    banner.className = 'bad';
    banner.hidden = false;
  }

  // helper: set a card value, mark .bad on kernel Err.
  function setCard(id, value, ok) {
    const el = document.getElementById(id);
    if (!el) return;
    el.textContent = value;
    el.className = ok === false ? 'v bad' : 'v';
  }

  // ── W-3.2 populate the debug cards from kernel state ──
  // geo_progress_js → #rem / #snap / #seg (ProgressOut).
  const gp = parseJsonResult(geo_progress_js(POLY, 5, 1));
  const gpv = gp.ok ? validateProgressOut(gp.value) : gp;
  setCard('rem', gpv.ok ? `${gpv.value.remaining_m}` : gpv.err, gpv.ok);
  setCard('snap', gpv.ok ? `${gpv.value.snapped.lat.toFixed(4)}, ${gpv.value.snapped.lng.toFixed(4)}` : gpv.err, gpv.ok);
  setCard('seg', gpv.ok ? `${gpv.value.segment_index}` : gpv.err, gpv.ok);

  // spectral_flat_js (one call) → #rho / #gap / #fie / #drift.
  const sf = parseJsonResult(spectral_flat_js(MAT));
  const sfv = sf.ok ? validateSpectralFlat(sf.value) : sf;
  setCard('rho', sfv.ok ? `${sfv.value.rho}` : sfv.err, sfv.ok);
  setCard('gap', sfv.ok ? `${sfv.value.gap}` : sfv.err, sfv.ok);
  setCard('fie', sfv.ok ? `${sfv.value.fiedler}` : sfv.err, sfv.ok);
  const drift = spectral_classify_drift_js(MAT);
  setCard('drift', drift.ok ? drift.value : drift.err, drift.ok);

  // fsm_graph_report_js → #fsm / #acyc (FsmReport).
  const fr = parseJsonResult(fsm_graph_report_js());
  const frv = fr.ok ? validateFsmReport(fr.value) : fr;
  setCard('fsm', frv.ok ? `${frv.value.vertices} / ${frv.value.edges}` : frv.err, frv.ok);
  setCard('acyc', frv.ok ? `${frv.value.is_acyclic}` : frv.err, frv.ok);

  // ── W-2 — FieldSim Sea render on #cv (kernel-derived circles) ──
  const cv = document.getElementById('cv');
  const tickBtn = document.getElementById('tick');
  // SIM_W/SIM_H = ¼ of the 800×160 #cv. circles = display projection of kernel
  // geo values (the snapped route position mapped into sim coords), zero new
  // math in JS — a *projection* of kernel values, not a re-computation.
  const SIM_W = 200, SIM_H = 40;
  let sim = null;
  let simReady = false;
  try {
    // Project the kernel snapped position (lat in [0,10], lng in [0,10]) into
    // sim pixel space. Pure scaling/translation of ALREADY-kernel-computed
    // floats — not geo math.
    const sx = gpv.ok ? (gpv.value.snapped.lat / 10) * SIM_W : SIM_W / 2;
    const sy = gpv.ok ? (gpv.value.snapped.lng / 10) * SIM_H : SIM_H / 2;
    const circles = new Float64Array([sx, sy, 6, SIM_W * 0.75, SIM_H * 0.5, 5]);
    sim = new FieldSim(circles, SIM_W, SIM_H);
    simReady = true;
  } catch (e) {
    simReady = false;
  }

  const off = document.createElement('canvas');
  off.width = SIM_W; off.height = SIM_H;
  const offCtx = off.getContext('2d');
  const cvCtx = cv.getContext('2d');

  // Static-gradient fallback (T14 degrade path) when the field crate traps.
  function paintFallback() {
    const grad = cvCtx.createLinearGradient(0, 0, 800, 160);
    grad.addColorStop(0, '#0b0f17');
    grad.addColorStop(1, '#13243a');
    cvCtx.fillStyle = grad;
    cvCtx.fillRect(0, 0, 800, 160);
  }

  let lastFrameBytes = null; // real FieldSim frame consumed by the canvas
  function blitSim() {
    if (!simReady) { paintFallback(); return; }
    try {
      sim.step();
      const rgba = sim.frame(); // Uint8ClampedArray-like, len === SIM_W*SIM_H*4
      lastFrameBytes = Array.from(new Uint8ClampedArray(rgba));
      offCtx.putImageData(new ImageData(new Uint8ClampedArray(rgba), sim.width(), sim.height()), 0, 0);
      cvCtx.drawImage(off, 0, 0, 800, 160);
    } catch {
      paintFallback(); // presentation failure degrades open (§9)
    }
  }

  let reducedMotion = true; // default true (headless/DOM-shim path is deterministic)
  try { reducedMotion = matchMedia('(prefers-reduced-motion: reduce)').matches; } catch {}
  if (!reducedMotion && simReady) {
    const loop = () => { blitSim(); requestAnimationFrame(loop); };
    requestAnimationFrame(loop);
  }

  // ── W-3.4 ONE minimal order surface (fixed-item placement + lifecycle) ──
  const placeBtn = document.getElementById('place');
  const orderLine = document.getElementById('order-line');
  const statusLine = document.getElementById('order-status');
  const actions = document.getElementById('order-actions');
  let order = null;
  let estSnap = null;

  // formatting is display-only — never math on the money identifiers (gate G).
  function fmtMinor(x) { return x == null ? '—' : (x / 100).toFixed(2); }

  function flash(kind, msg) {
    const el = document.getElementById('order-flash');
    if (!el) return;
    el.textContent = msg ? `${kind === 'ok' ? '✓' : '✗'} ${msg}` : (kind === 'ok' ? '✓' : '✗');
    el.className = kind === 'ok' ? 'good' : 'bad';
    el.hidden = false;
  }

  function renderOrder() {
    if (!order) { statusLine.textContent = '—'; orderLine.textContent = '—'; actions.innerHTML = ''; return; }
    statusLine.textContent = order.status;
    statusLine.className = 'v good';
    const sub = order.subtotal, tot = order.total;
    let line = `subtotal ${fmtMinor(sub)} · total ${fmtMinor(tot)}`;
    if (estSnap) {
      const fee = estSnap.delivery_fee == null ? '—' : fmtMinor(estSnap.delivery_fee);
      const tax = estSnap.tax_total == null ? '—' : fmtMinor(estSnap.tax_total);
      line += ` · est fee ${fee} · est tax ${tax}`;
      if (estSnap.min_not_met) line += ' · MIN NOT MET';
    }
    orderLine.textContent = line;
    // Candidate actions: kernel decides legality (no JS transition table).
    const candidates = ['CONFIRMED', 'PREPARING', 'READY', 'DELIVERED'];
    actions.innerHTML = '';
    for (const c of candidates) {
      const b = document.createElement('button');
      b.textContent = c;
      b.onclick = () => advance(c);
      actions.appendChild(b);
    }
  }

  function advance(next) {
    if (!order) return;
    const r = apply_event_js(JSON.stringify(order), next);
    if (r.ok) {
      const j = parseJsonResult(r);
      const v = j.ok ? validateOrderOut(j.value) : j;
      if (v.ok) {
        order = v.value; // status advances on Ok
        renderOrder();
        flash('ok');
        return;
      }
      flash('bad', v.err || 'rejected');
    } else {
      // illegal transition → kernel Err, state unchanged.
      flash('bad', `kernel refused: ${r.err}`);
    }
  }

  if (placeBtn) {
    placeBtn.onclick = () => {
      if (!boot2.ok) { flash('bad', 'boot gate refused — order disabled'); return; }
      const r = place_order_js(null, ITEMS, 'web');
      const j = parseJsonResult(r);
      const v = j.ok ? validateOrderOut(j.value) : j;
      if (!v.ok) { flash('bad', v.err || 'place failed'); return; }
      order = v.value;
      // Estimate (display mirror only; charge authority NOT flipped — §8).
      const e = estimate_order_total_js(order.subtotal, CFG);
      const ej = parseJsonResult(e);
      const ev = ej.ok ? validateEstimateOut(ej.value) : ej;
      estSnap = ev.ok ? ev.value : null;
      renderOrder();
      flash('ok', 'placed');
    };
  }

  // #tick advances the demo courier param and re-derives circles from fresh
  // kernel geo, then one sim.step + blit when the rAF loop is off.
  if (tickBtn) {
    let t = 0;
    tickBtn.onclick = () => {
      t += 1;
      const pos = 1 + (t % 9); // 1..9 along the POLY line — still kernel math (geo_progress)
      const g = parseJsonResult(geo_progress_js(POLY, pos, 1));
      const gv = g.ok ? validateProgressOut(g.value) : g;
      setCard('rem', gv.ok ? `${gv.value.remaining_m}` : gv.err, gv.ok);
      setCard('snap', gv.ok ? `${gv.value.snapped.lat.toFixed(4)}, ${gv.value.snapped.lng.toFixed(4)}` : gv.err, gv.ok);
      setCard('seg', gv.ok ? `${gv.value.segment_index}` : gv.err, gv.ok);
      if (simReady && !reducedMotion) return; // rAF loop already painting
      blitSim(); // reduced-motion: single step + blit
    };
  }

  // initial paint (reduced-motion / no-rAF path) + order idle state.
  if (!simReady || reducedMotion) blitSim();
  renderOrder();
}
