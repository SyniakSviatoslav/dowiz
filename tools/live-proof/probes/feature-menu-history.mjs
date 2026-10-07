// LIVE PROBE menu-history (W-PITR: every catalogue write journaled; the owner's history and
// "put back as before this change") on qa-durres -- main runs it AFTER the deploy; a lane never
// runs it against production.
//
//   node tools/live-proof/probes/feature-menu-history.mjs      (FLOWS_HOST may name another qa- hub)
//
// 1. a QA dish `LIVE-<run> history` is created at 1000 lek;
// 2. its price becomes 1050; GET /api/owner/menu/history matches the contract, and its newest
//    record is that dish at 1050, by a non-empty editor, restorable;
// 3. POST /api/owner/menu/history/restore with that record's seq and key matches the contract;
//    the owner's product list says 1000 again, and the history's newest record is the restore;
// 4. a restore naming a key that is not the record's is refused (409), never applied;
// 5. clean-up: the dish is deleted.
// Every step that cannot run is a FAIL, never a skip.
import * as lib from '../../../e2e/flows/lib.mjs';
import { contract, reporter } from './stock-lib.mjs';

const C = contract('menu-history');
const { step, schema, verdict } = reporter('feature-menu-history');
const run = `LIVE-${lib.RUN}`;
const must = (ok, why) => { if (!ok) throw new Error(why); };
const history = async () => {
  const h = await lib.own(`/api/owner/menu/history?location_id=${lib.LOC}&limit=20`);
  step('the history answers 200', h.status === 200, `${h.status} ${h.text.slice(0, 200)}`);
  schema('the history matches the contract', h.body, C.routes.list.response_schema);
  return h.body?.edits || [];
};
const ownerPrice = async (dish) => {
  const ps = await lib.own(`/api/owner/products?location_id=${lib.LOC}&id=${encodeURIComponent(dish)}`);
  return (Array.isArray(ps.body) ? ps.body : []).find(x => x.id === dish)?.price;
};

let dish = null;
try {
  const m = await lib.menu();
  const cat = (m.categories || [])[0]?.id;
  must(cat, 'the qa menu has no category');
  // 1.
  const mk = await lib.own('/api/owner/products', { location_id: lib.LOC, category_id: cat, name: `${run} history`, price: 1000, available: false });
  step('a QA dish is created at 1000 lek', mk.status === 200 && mk.body?.id, `${mk.status} ${mk.text}`);
  dish = mk.body?.id;
  must(dish, 'no dish id');
  // 2.
  const e = await lib.own(`/api/owner/products/${encodeURIComponent(dish)}`, { location_id: lib.LOC, price: 1050 });
  step('the price becomes 1050', e.status === 200, `${e.status} ${e.text}`);
  const h1 = await history();
  const top = h1[0];
  step('the newest record is this dish at 1050', top?.key === `product:${dish}` && top?.price === 1050, JSON.stringify(top));
  step('it names its editor and is restorable', !!top?.by && top.by !== '?' && top?.restorable === true, JSON.stringify(top));
  must(top?.key === `product:${dish}`, 'the edit is not the newest record');
  // 4. (before the restore, while the seq is fresh) a wrong key is refused
  const wrong = await lib.own('/api/owner/menu/history/restore', { location_id: lib.LOC, seq: top.seq, key: 'product:not-this-dish' });
  step('a restore naming another key is refused 409', wrong.status === 409, `${wrong.status} ${wrong.text}`);
  // 3.
  const body = { location_id: lib.LOC, seq: top.seq, key: top.key };
  schema('the restore request matches the contract', body, C.routes.restore.request_schema);
  const r = await lib.own('/api/owner/menu/history/restore', body);
  step('the restore answers 200', r.status === 200, `${r.status} ${r.text}`);
  schema('the restore matches the contract', r.body, C.routes.restore.response_schema);
  const p = await ownerPrice(dish);
  step('the owner product list says 1000 again', p === 1000, JSON.stringify(p));
  const h2 = await history();
  step('the restore is itself the newest record', h2[0]?.key === `product:${dish}` && h2[0]?.price === 1000, JSON.stringify(h2[0]));
} catch (e) {
  step('the probe ran to the end', false, e.message);
} finally {
  if (dish) {
    const d = await lib.own(`/api/owner/products/${encodeURIComponent(dish)}/delete`, { location_id: lib.LOC });
    step(`QA dish ${dish} is deleted`, d.status === 200, `${d.status} ${d.text}`);
  }
  verdict();
}
