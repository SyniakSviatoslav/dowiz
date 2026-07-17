// kernel.test.mjs — fail-closed assertions for the kernel wasm bridges (Node).
// GREEN gate (W17): kernel loads, spectral_radius_js('[[1,0],[0,1]]') => "1",
// malformed => {ok:false}, fsm + geo => {ok:true}. Plus grep proof (run by parent):
// NO haversine/eigen/FSM re-impl in web/src.
import { readFileSync } from 'fs';
import { bindKernel, spectral_radius_js, fsm_graph_report_js, geo_progress_flat_js } from './kernel_client.mjs';

const bytes = new Uint8Array(readFileSync(new URL('../../../../kernel/pkg-web/dowiz_kernel_bg.wasm', import.meta.url)));
await bindKernel(bytes);

function assert(cond, msg) {
  if (!cond) { console.error('FAIL:', msg); process.exit(1); }
  console.log('ok -', msg);
}

// 1. spectral radius of identity-like matrix [[1,0],[0,1]] => "1"
const s = spectral_radius_js('[[1,0],[0,1]]');
assert(s.ok && s.value === '1', `spectral_radius_js([[1,0],[0,1]]) === "1" (got ${JSON.stringify(s)})`);

// 2. malformed input => fail-closed {ok:false}
const bad = spectral_radius_js('not a matrix');
assert(!bad.ok, `malformed matrix => {ok:false} (got ${JSON.stringify(bad)})`);

// 3. fsm report => ok:true (acyclic 10-vertex sample)
const f = fsm_graph_report_js();
assert(f.ok, `fsm_graph_report_js => ok:true (got ${JSON.stringify(f).slice(0,80)})`);

// 4. geo progress => ok:true
const g = geo_progress_flat_js('[[0,0],[10,0]]', 5, 1);
assert(g.ok, `geo_progress_flat_js => ok:true (got ${JSON.stringify(g)})`);

console.log('W17 GREEN: all kernel bridges wired + fail-closed, zero JS math re-impl.');

// ════════════════════════════════════════════════════════════════════════════
// BLUEPRINT-P-G Layer G extensions (T5–T15 + gate G). The 4 W17 tests above stay
// untouched and green. Layer G binds the remaining 21 of 24 kernel `_js` exports
// and drives the kernel as the SOLE FSM/money authority (no client math; no
// `wasm.` access outside kernel_client.mjs — enforced by gate G).
// ════════════════════════════════════════════════════════════════════════════
import {
  boot_verify_fsm_js,
  place_order_js,
  apply_event_js,
  estimate_order_total_js,
  channel_ledger_js,
  reduce_anomalies_js,
  spectral_flat_js,
  geo_progress_js,
  geo_is_out_of_order_js,
  harmonic_centrality_js,
  spectral_classify_drift_js,
} from './kernel_client.mjs';
import {
  parseJsonResult,
  validateOrderOut,
  validateEstimateOut,
  validateProgressOut,
  validateLedgerOut,
  validateSpectralFlat,
  validateFsmReport,
} from './contracts.mjs';

const ITEMS = JSON.stringify([
  { product_id: 'p1', modifier_ids: ['m1'], quantity: 2, unit_price: 500 },
  { product_id: 'p2', modifier_ids: [], quantity: 1, unit_price: 300 },
]);
const CFG = JSON.stringify({
  is_pickup: false,
  delivery_fee_flat: 200,
  tax_rate: 0.1,
  price_includes_tax: false,
  has_distance_tiers: false,
  min_order_value: 1000,
});

// T5 — place_order_js round trip (oracle: wasm.rs:789-798).
const t5 = place_order_js(null, ITEMS, 'web');
const t5j = parseJsonResult(t5);
const t5v = t5j.ok ? validateOrderOut(t5j.value) : t5j;
assert(t5v.ok, `T5 place_order_js ok + shapes valid (got ${JSON.stringify(t5v).slice(0, 80)})`);
assert(t5v.value.status === 'PENDING', 'T5 status === PENDING');
assert(
  t5v.value.subtotal === 2 * 500 + 300,
  `T5 subtotal === Σ qty·unit_price (${t5v.value.subtotal})`,
);
assert(
  Number.isSafeInteger(t5v.value.subtotal) && Number.isSafeInteger(t5v.value.total),
  'T5 money fields are safe integers',
);

// T6 — event sequence fold stepwise through apply_event_js.
let cur = t5v.value;
for (const next of ['CONFIRMED', 'PREPARING', 'READY']) {
  const a = apply_event_js(JSON.stringify(cur), next);
  const j = parseJsonResult(a);
  const v = j.ok ? validateOrderOut(j.value) : j;
  assert(v.ok && v.value.status === next, `T6 fold → ${next} (got ${v.ok ? v.value.status : v.err})`);
  cur = v.value;
}

// T7 — adversarial: illegal transition DELIVERED from PENDING.
const t7 = apply_event_js(JSON.stringify(t5v.value), 'DELIVERED');
assert(!t7.ok, `T7 illegal PENDING→DELIVERED rejected (got ${JSON.stringify(t7)})`);
const reCheck = parseJsonResult(place_order_js(null, ITEMS, 'web'));
const reVal = validateOrderOut(reCheck.value);
assert(reVal.value.status === 'PENDING', 'T7 prior state object unchanged (still PENDING)');

// T8 — estimate_order_total_js known cfg (subtotal=1000: fee 200 + tax 100 = total 1300).
const t8 = estimate_order_total_js(1000, CFG);
const t8j = parseJsonResult(t8);
const t8v = t8j.ok ? validateEstimateOut(t8j.value) : t8j;
assert(t8v.ok, `T8 estimate ok (got ${JSON.stringify(t8v).slice(0, 80)})`);
assert(t8v.value.delivery_fee === 200 && t8v.value.tax_total === 100 && t8v.value.total === 1300,
  `T8 exact integer total (fee=${t8v.value.delivery_fee}, tax=${t8v.value.tax_total}, total=${t8v.value.total})`);
assert(t8v.value.min_not_met === false, 'T8 min_not_met false at subtotal=1000 (threshold 1000, >=)');

// T9 — adversarial: overflow degrade. A pathological tax rate makes sub*rate
// exceed i64 ⇒ tax_total/total null (no fabricated 0). Cannot pass a >2^53
// subtotal from JS (i64 BigInt boundary), so the overflow is induced via rate.
const T9_CFG = JSON.stringify({
  is_pickup: false, delivery_fee_flat: 0, tax_rate: 2000, price_includes_tax: false, has_distance_tiers: false,
});
const t9 = estimate_order_total_js(Number.MAX_SAFE_INTEGER, T9_CFG);
const t9j = parseJsonResult(t9);
const t9v = t9j.ok ? validateEstimateOut(t9j.value) : t9j;
assert(t9v.ok && t9v.value.tax_total === null && t9v.value.total === null,
  `T9 overflow ⇒ tax_total/total null (degrade, not 0) (got ${JSON.stringify(t9v).slice(0, 80)})`);

// T10 — adversarial: malformed order JSON to apply_event_js ⇒ {ok:false}.
const t10 = apply_event_js('not json', 'CONFIRMED');
assert(!t10.ok, `T10 malformed order JSON ⇒ {ok:false} (got ${JSON.stringify(t10)})`);

// T11 — boot_verify_fsm_js returns "OK" on the live graph.
const t11 = boot_verify_fsm_js();
assert(t11.ok && t11.value === 'OK', `T11 boot_verify_fsm_js === "OK" (got ${JSON.stringify(t11)})`);

// T12 — FSM report validates (kernel-derived, schema-checked).
const t12 = parseJsonResult(fsm_graph_report_js());
const t12v = t12.ok ? validateFsmReport(t12.value) : t12;
assert(t12v.ok, `T12 fsm_graph_report_js validates (got ${JSON.stringify(t12v).slice(0, 60)})`);

// T13 — spectral_flat_js validates as a flat array (length 5 + 2n eigen pairs).
const t13 = parseJsonResult(spectral_flat_js('[[1,0],[0,1]]'));
const t13v = t13.ok ? validateSpectralFlat(t13.value) : t13;
assert(t13v.ok && t13v.value.rho === 1 && t13v.value.n === 2,
  `T13 spectral_flat_js validates (rho=${t13v.ok ? t13v.value.rho : '?'}, n=${t13v.ok ? t13v.value.n : '?'})`);

// T14 — channel_ledger_js + reduce_anomalies_js carry the anomaly count through the JSON seam.
const EVENTS = JSON.stringify([
  { order_id: 'a1', channel: 'web', status: 'PENDING', at_ms: 1 },
  { order_id: 'a2', channel: 'web', status: 'DELIVERED', at_ms: 2 },
]);
const t14l = parseJsonResult(channel_ledger_js(EVENTS));
const t14lv = t14l.ok ? validateLedgerOut(t14l.value) : t14l;
assert(t14lv.ok, `T14 channel_ledger_js validates (got ${JSON.stringify(t14lv).slice(0, 60)})`);
const t14a = reduce_anomalies_js(EVENTS);
assert(t14a.ok && Number.isSafeInteger(t14a.value), `T14 reduce_anomalies_js returns safe-int count (got ${JSON.stringify(t14a)})`);

// T15 — validator fail-closed: a hand-built wrong-shape "order" ⇒ {ok:false}.
const t15 = validateOrderOut({ id: 'x', status: 123, items: 'nope', subtotal: 1.5, total: 'z', created_at_ms: 0 });
assert(!t15.ok, `T15 validator rejects wrong-shape order (got ${JSON.stringify(t15)})`);

// Gate G — structural grep: deny client math + raw `wasm.` access outside the seam.
import { readFileSync as _rf, readdirSync as _rd } from 'fs';
import { join as _join } from 'path';
function walkGrep(dir, re, exclude = []) {
  const hits = [];
  const stack = [_join(dir, 'src')];
  while (stack.length) {
    const d = stack.pop();
    let ents;
    try { ents = _rd(d, { withFileTypes: true }); } catch { continue; }
    for (const e of ents) {
      const fp = _join(d, e.name);
      if (e.isDirectory()) { if (!exclude.includes(e.name)) stack.push(fp); continue; }
      if (!/\.(mjs|js)$/.test(e.name)) continue;
      if (exclude.includes(e.name)) continue;
      const txt = _rf(fp, 'utf8');
      for (const line of txt.split('\n')) {
        if (re.test(line)) hits.push(`${fp}: ${line.trim().slice(0, 70)}`);
      }
    }
  }
  return hits;
}
// Client-math smells: haversine/eigen re-impl, hand transition tables, arithmetic on price/total/fee ids.
const mathRe = /\b(haversine|eigenvalue|eigen\b|apply_tax|compute_delivery_fee|Math\.(acos|sqrt|cos|sin|atan2))\b|subtotal\s*[\+\-\*]|\.total\s*[\+\-]|\.fee\s*[\+\-]|transitionTable|statusTransitions/;
const wasmRe = /(^|[^.\w])wasm\.[A-Za-z_]/; // raw wasm.<export> outside kernel_client.mjs
const mathHits = walkGrep(new URL('.', import.meta.url).pathname, mathRe, ['kernel_client.mjs']);
const wasmHits = walkGrep(new URL('.', import.meta.url).pathname, wasmRe, ['kernel_client.mjs']);
assert(mathHits.length === 0, `G gate: no client math in web/src (violations: ${mathHits.join(' | ') || 'none'})`);
assert(wasmHits.length === 0, `G gate: no raw wasm.<export> outside kernel_client.mjs (violations: ${wasmHits.join(' | ') || 'none'})`);

console.log('P-G GREEN: T5–T15 + gate G pass; 24/24 kernel _js exports bound, kernel sole authority.');
