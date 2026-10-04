// LIVE PROBE menu-flags (W-MR0 rows MR0 + MR4) on qa-durres -- main runs it AFTER the deploy;
// a lane never runs it against production.
//
//   node tools/live-proof/probes/feature-menu-flags.mjs      (FLOWS_HOST may name another qa- hub)
//
// 0. feature.allergen_filter is read and switched ON for the run (restored at the end);
// 1. a QA dish `LIVE-<run> flags` is created OFF sale with no allergens: the owner's product list
//    holds it with no `allergens` array (UNDECLARED, what the menu screen counts);
// 2. asking it on sale undeclared is a 409; declaring ['fish'] and putting it on sale is a 200, and
//    the storefront menu (?fresh=1) carries allergens ['fish'] on it;
// 3. N TEST orders of it are placed (contact LIVE-...) and confirmed: the analytics pane's weekTop
//    counts them as `test` (>= N) and NOT in `n`; menu/week never badges it;
// 4. every dish menu/week badges has the same n on the pane, and the pane's badge flag says so;
// 5. clean-up: the TEST orders end, the dish is deleted, the feature is restored.
// Every step that cannot run is a FAIL, never a skip.
import * as lib from '../../../e2e/flows/lib.mjs';
import { contract, reporter } from './stock-lib.mjs';
import { place, close, sweep } from './_order.mjs';

const C = contract('menu-flags');
const { step, schema, verdict } = reporter('feature-menu-flags');
const N = 3;
const run = `LIVE-${lib.RUN}`;
const must = (ok, why) => { if (!ok) throw new Error(why); };
const FEATURE = 'feature.allergen_filter';

let dish = null, wasOn = null;
const orders = [];
try {
  await sweep(lib);
  // 0. the feature
  const f = await lib.own(`/api/owner/features?location_id=${lib.LOC}`);
  step('GET /api/owner/features answers 200', f.status === 200, `${f.status} ${f.text}`);
  const list = Array.isArray(f.body) ? f.body : (f.body?.features || []);
  const cur = list.find(x => x.key === FEATURE);
  wasOn = cur ? !!cur.on : false;
  const on = await lib.own(`/api/owner/features?location_id=${lib.LOC}`, { key: FEATURE, on: true });
  step(`${FEATURE} switched on for the run`, on.status === 200 && on.body?.on === true, on.text);

  // 1. an undeclared QA dish, off sale
  const m = await lib.menu();
  const cat = (m.categories || [])[0]?.id;
  must(cat, 'the qa menu has no category');
  const mk = await lib.own('/api/owner/products', { location_id: lib.LOC, category_id: cat, name: `${run} flags`, price: 100, available: false });
  step('a QA dish is created off sale', mk.status === 200 && mk.body?.id, `${mk.status} ${mk.text}`);
  dish = mk.body?.id;
  const ps = await lib.own(`/api/owner/products?location_id=${lib.LOC}&id=${encodeURIComponent(dish)}`);
  const p = (Array.isArray(ps.body) ? ps.body : []).find(x => x.id === dish);
  step('the owner list holds it UNDECLARED (no allergens array)', !!p && !Array.isArray(p.allergens), JSON.stringify(p?.allergens));

  // 2. the gate, then a declaration
  const bad = await lib.own(`/api/owner/products/${encodeURIComponent(dish)}`, { location_id: lib.LOC, available: true });
  step('on sale while undeclared is refused 409', bad.status === 409, `${bad.status} ${bad.text}`);
  const ok = await lib.own(`/api/owner/products/${encodeURIComponent(dish)}`, { location_id: lib.LOC, allergens: ['fish'], available: true });
  step('declared [fish] and put on sale', ok.status === 200, `${ok.status} ${ok.text}`);
  const pub = lib.dishes(await lib.menu()).find(x => x.id === dish);
  step('the storefront card carries allergens [fish]', JSON.stringify(pub?.allergens) === '["fish"]', JSON.stringify(pub?.allergens));

  // 3. TEST orders are counted apart and never badge
  for (let i = 0; i < N; i++) {
    const o = await place({ lib, run, must }, { items: [{ product_id: dish, modifier_ids: [], quantity: 1 }] });
    orders.push(o.id);
    const c = await lib.own(`/api/owner/orders/${encodeURIComponent(o.id)}/action`, { action: 'confirm', location_id: lib.LOC });
    step(`TEST order ${i + 1} placed and confirmed`, c.status === 200, `${c.status} ${c.text.slice(0, 100)}`);
  }
  const a = await lib.own('/api/owner/analytics?v=2');
  step('GET /api/owner/analytics?v=2 answers 200', a.status === 200, `${a.status} ${a.text}`);
  const wt = a.body?.weekTop;
  schema('the pane weekTop matches its schema', wt, C.owner_week_schema);
  const row = (wt?.dishes || []).find(d => d.id === dish);
  step(`the pane counts the ${N} TEST portions as test, not n`, !!row && row.test >= N && row.n === 0 && row.badge === false, JSON.stringify(row));
  const w = await lib.api(`/api/public/locations/${lib.LOC}/menu/week`);
  step('GET menu/week answers 200', w.status === 200, `${w.status} ${w.text}`);
  schema('menu/week matches its schema', w.body, C.week_response_schema);
  step('menu/week never badges the TEST dish', !(w.body?.dishes || []).some(d => d.id === dish), JSON.stringify(w.body?.dishes));

  // 4. the badge and the pane are one number (menu/week may be up to 300 s old: compare by id)
  const badged = w.body?.dishes || [];
  const pane = new Map((wt?.dishes || []).map(d => [d.id, d]));
  const differ = badged.filter(d => pane.get(d.id)?.n !== d.n || pane.get(d.id)?.badge !== true);
  step('every badge n equals the pane n (same window)', differ.length === 0, JSON.stringify(differ));
  step('every badge is at or over the threshold', badged.every(d => d.n >= w.body.threshold), JSON.stringify(badged));
} catch (e) {
  step('the probe ran to the end', false, e.stack || String(e));
} finally {
  for (const id of orders) {
    try { await close({ lib, run, must }, id); } catch (e) { step(`ending TEST order ${id}`, false, String(e.message || e)); }
  }
  if (dish) {
    const d = await lib.own(`/api/owner/products/${encodeURIComponent(dish)}/delete`, { location_id: lib.LOC });
    step('the QA dish is deleted', d.status === 200, `${d.status} ${d.text}`);
  }
  if (wasOn !== null) {
    const r = await lib.own(`/api/owner/features?location_id=${lib.LOC}`, { key: FEATURE, on: wasOn });
    step(`${FEATURE} restored to ${wasOn}`, r.status === 200, r.text);
  }
}
verdict();
