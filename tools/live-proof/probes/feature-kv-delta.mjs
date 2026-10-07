// LIVE PROBE kv-delta (W-DELTA: a catalogue write appends a delta record instead of rewriting
// the image) on qa-durres -- main runs it AFTER the deploy; a lane never runs it against production.
//
//   node tools/live-proof/probes/feature-kv-delta.mjs      (FLOWS_HOST may name another qa- hub)
//
// No route changes: what changes is the bytes the venue object stores (v3 = v2 base + chain;
// compaction past 32 records or a quarter dead). So the probe proves every read sees each edit:
// 1. a QA dish `LIVE-<run> delta` is created at 1000 lek, declared, put on sale;
// 2. its price becomes 1010 (a delta), and the storefront menu (?fresh=1), the owner's product
//    list and a TEST order all say 1010;
// 3. its price becomes 1020 (a SECOND delta on the same key: the newest must win), and all three
//    say 1020 -- never 1000 (the base) and never 1010 (the older record);
// 4. a second QA dish is created and deleted (a remove on the chain): it is gone from the menu
//    while the first dish still reads 1020 (the remove dropped nothing else);
// 5. clean-up: the TEST orders end, the dish is deleted.
// Every step that cannot run is a FAIL, never a skip.
import * as lib from '../../../e2e/flows/lib.mjs';
import { contract, reporter } from './stock-lib.mjs';
import { payload, close, sweep } from './_order.mjs';

const C = contract('kv-delta');
const { step, schema, verdict } = reporter('feature-kv-delta');
const run = `LIVE-${lib.RUN}`;
const must = (ok, why) => { if (!ok) throw new Error(why); };

let dish = null, gone = null;
const orders = [];
const order = async (items) => {
  const r = await lib.api(`/api/public/locations/${lib.LOC}/orders`, { method: 'POST', body: payload(run, { items }) });
  if (r.status === 200 && r.body?.id) orders.push(r.body.id);
  return r;
};
const priceEverywhere = async (want, label) => {
  const m = await lib.menu();
  const pub = lib.dishes(m).find(x => x.id === dish);
  step(`${label}: the storefront menu says ${want}`, pub?.price === want, JSON.stringify(pub?.price));
  const ps = await lib.own(`/api/owner/products?location_id=${lib.LOC}&id=${encodeURIComponent(dish)}`);
  const p = (Array.isArray(ps.body) ? ps.body : []).find(x => x.id === dish);
  step(`${label}: the owner product list says ${want}`, p?.price === want, `${ps.status} ${JSON.stringify(p?.price)}`);
  const o = await order([{ product_id: dish, modifier_ids: [], quantity: 1 }]);
  step(`${label}: a TEST order is placed`, o.status === 200, `${o.status} ${o.text.slice(0, 160)}`);
  schema(`${label}: the placement matches the contract`, o.body, C.routes.place.response_schema);
  return o.body?.total;
};
try {
  await sweep(lib);
  const m = await lib.menu();
  const cat = (m.categories || [])[0]?.id;
  must(cat, 'the qa menu has no category');
  // 1.
  const mk = await lib.own('/api/owner/products', { location_id: lib.LOC, category_id: cat, name: `${run} delta`, price: 1000, available: false });
  step('a QA dish is created at 1000 lek', mk.status === 200 && mk.body?.id, `${mk.status} ${mk.text}`);
  dish = mk.body?.id;
  must(dish, 'no dish id');
  const on = await lib.own(`/api/owner/products/${encodeURIComponent(dish)}`, { location_id: lib.LOC, allergens: ['fish'], available: true });
  step('declared and put on sale', on.status === 200, `${on.status} ${on.text}`);
  const t0 = await priceEverywhere(1000, 'before');
  // 2. first edit
  const e1 = await lib.own(`/api/owner/products/${encodeURIComponent(dish)}`, { location_id: lib.LOC, price: 1010 });
  step('the price becomes 1010', e1.status === 200, `${e1.status} ${e1.text}`);
  const t1 = await priceEverywhere(1010, 'edit 1');
  step('edit 1: the order costs exactly 10 lek more', Number.isInteger(t0) && t1 === t0 + 10, `${t0} -> ${t1}`);
  // 3. second edit, same key
  const e2 = await lib.own(`/api/owner/products/${encodeURIComponent(dish)}`, { location_id: lib.LOC, price: 1020 });
  step('the price becomes 1020', e2.status === 200, `${e2.status} ${e2.text}`);
  const t2 = await priceEverywhere(1020, 'edit 2 (newest wins)');
  step('edit 2: the order costs exactly 20 lek more than the first', Number.isInteger(t0) && t2 === t0 + 20, `${t0} -> ${t2}`);
  // 4. a remove on the chain
  const mk2 = await lib.own('/api/owner/products', { location_id: lib.LOC, category_id: cat, name: `${run} delta-gone`, price: 500, available: false });
  step('a second QA dish is created', mk2.status === 200 && mk2.body?.id, `${mk2.status} ${mk2.text}`);
  gone = mk2.body?.id;
  const d2 = await lib.own(`/api/owner/products/${encodeURIComponent(gone)}/delete`, { location_id: lib.LOC });
  step('the second QA dish is deleted', d2.status === 200, `${d2.status} ${d2.text}`);
  const ps = await lib.own(`/api/owner/products?location_id=${lib.LOC}`);
  const ids = (Array.isArray(ps.body) ? ps.body : []).map(x => x.id);
  step('the deleted dish is gone from the owner list', !ids.includes(gone), `${ids.length} products`);
  gone = null;
  await priceEverywhere(1020, 'after the remove');
} catch (e) {
  step('the probe ran to the end', false, e.message);
} finally {
  for (const id of orders) {
    try { await close({ lib, run, must }, id); step(`TEST order ${id} ended`, true); }
    catch (e) { step(`TEST order ${id} ended`, false, e.message); }
  }
  for (const id of [dish, gone].filter(Boolean)) {
    const d = await lib.own(`/api/owner/products/${encodeURIComponent(id)}/delete`, { location_id: lib.LOC });
    step(`QA dish ${id} is deleted`, d.status === 200, `${d.status} ${d.text}`);
  }
  verdict();
}
