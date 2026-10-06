// LIVE PROBE zc-catalogue (W-ZC: the catalogue read in place) on qa-durres -- main runs it AFTER
// the deploy; a lane never runs it against production.
//
//   node tools/live-proof/probes/feature-zc-catalogue.mjs      (FLOWS_HOST may name another qa- hub)
//
// The change has no new route: the object answers the same folds from the bytes it holds. So the
// probe proves the reads still answer, and that a WRITE is seen by the next read (the crc memo is
// per generation and must never serve a stale index):
// 1. /manifest.webmanifest names the venue (/fold/venue, read in place);
// 2. a QA dish `LIVE-<run> zc` is created at 1000 lek, declared, put on sale;
// 3. a TEST order of it is placed (/fold/basket, read in place): 200 with an integer total;
// 4. its price becomes 1030 and a second TEST order costs exactly 30 lek more -- the edit is read;
// 5. a basket naming a dish that does not exist is refused 4xx, never a 5xx;
// 6. clean-up: the TEST orders end, the dish is deleted.
// Every step that cannot run is a FAIL, never a skip.
import * as lib from '../../../e2e/flows/lib.mjs';
import { contract, reporter } from './stock-lib.mjs';
import { payload, close, sweep } from './_order.mjs';

const C = contract('zc-catalogue');
const { step, schema, verdict } = reporter('feature-zc-catalogue');
const run = `LIVE-${lib.RUN}`;
const must = (ok, why) => { if (!ok) throw new Error(why); };

let dish = null;
const orders = [];
const order = async (items) => {
  const r = await lib.api(`/api/public/locations/${lib.LOC}/orders`, { method: 'POST', body: payload(run, { items }) });
  if (r.status === 200 && r.body?.id) orders.push(r.body.id);
  return r;
};
try {
  await sweep(lib);
  // 1. the venue record, read in place
  const mf = await lib.api('/manifest.webmanifest');
  step('GET /manifest.webmanifest answers 200', mf.status === 200, `${mf.status} ${mf.text.slice(0, 120)}`);
  schema('the manifest matches the contract', mf.body, C.routes.manifest.response_schema);

  // 2. a QA dish on sale at 1000
  const m = await lib.menu();
  const cat = (m.categories || [])[0]?.id;
  must(cat, 'the qa menu has no category');
  const mk = await lib.own('/api/owner/products', { location_id: lib.LOC, category_id: cat, name: `${run} zc`, price: 1000, available: false });
  step('a QA dish is created at 1000 lek', mk.status === 200 && mk.body?.id, `${mk.status} ${mk.text}`);
  dish = mk.body?.id;
  must(dish, 'no dish id');
  const on = await lib.own(`/api/owner/products/${encodeURIComponent(dish)}`, { location_id: lib.LOC, allergens: ['fish'], available: true });
  step('declared and put on sale', on.status === 200, `${on.status} ${on.text}`);

  // 3. a placement reads it
  const line = [{ product_id: dish, modifier_ids: [], quantity: 1 }];
  const a = await order(line);
  step('a TEST order of the dish is placed', a.status === 200, `${a.status} ${a.text.slice(0, 200)}`);
  schema('the placement matches the contract', a.body, C.routes.place.response_schema);

  // 4. an edit is seen by the next read (new generation, new check)
  const up = await lib.own(`/api/owner/products/${encodeURIComponent(dish)}`, { location_id: lib.LOC, price: 1030 });
  step('the price becomes 1030', up.status === 200, `${up.status} ${up.text}`);
  const b = await order(line);
  step('a second TEST order is placed', b.status === 200, `${b.status} ${b.text.slice(0, 200)}`);
  step('it costs exactly 30 lek more (the edit was read, not a stale index)',
    Number.isInteger(a.body?.total) && b.body?.total === a.body.total + 30, `${a.body?.total} -> ${b.body?.total}`);

  // 5. a missing key is a refusal, not a crash
  const miss = await order([{ product_id: `${run}-no-such-dish`, modifier_ids: [], quantity: 1 }]);
  step('a dish that does not exist is refused 4xx', miss.status >= 400 && miss.status < 500, `${miss.status} ${miss.text.slice(0, 160)}`);
} catch (e) {
  step('the probe ran to the end', false, e.message);
} finally {
  for (const id of orders) {
    try { await close({ lib, run, must }, id); step(`TEST order ${id} ended`, true); }
    catch (e) { step(`TEST order ${id} ended`, false, e.message); }
  }
  if (dish) {
    const d = await lib.own(`/api/owner/products/${encodeURIComponent(dish)}/delete`, { location_id: lib.LOC });
    step('the QA dish is deleted', d.status === 200, `${d.status} ${d.text}`);
  }
  verdict();
}
