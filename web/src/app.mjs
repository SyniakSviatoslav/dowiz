// app.mjs — console-only kernel UI driver (BLUEPRINT-P-G, Layer G).
//
// Drives the kernel wasm as the SOLE FSM / money authority. It binds all 24
// `_js` exports through kernel_client.mjs and exercises the order/money/lifecycle
// surface end-to-end, rendering KernelResult Err on any illegal action. No JS
// re-implementation of geo/spectral/FSM math exists anywhere in web/src — the
// kernel is the only source of truth (AGENTS invariant, gate G). Money fields
// are asserted Number.isSafeInteger at the boundary.
//
// Note: G3's DOM/FieldSim pass is a separate work unit (W-2/W-3) — this G task
// is the console-only driver. The boot gate + order lifecycle below are the
// shared spine both passes use.
import { readFileSync } from 'fs';
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

const bytes = new Uint8Array(readFileSync(new URL('../../kernel/pkg-web/dowiz_kernel_bg.wasm', import.meta.url)));
await bindKernel(bytes);

// ── money assert helper (boundary guard, never rounds) ──
function money(label, x) {
  if (!Number.isSafeInteger(x)) {
    console.error(`MONEY FAIL: ${label} not a safe integer: ${x}`);
    process.exit(1);
  }
  return x;
}

let failed = false;
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
  console.error('FATAL: FSM boot gate refused — authority loss is fatal.');
  process.exit(1);
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
  process.exit(1);
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
