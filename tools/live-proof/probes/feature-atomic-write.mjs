// LIVE PROBE atomic-write (W-ATOMIC: one image write = one atomic storage write; the rollback
// compaction route) on qa-durres -- main runs it AFTER the deploy; a lane never runs it against production.
//
//   node tools/live-proof/probes/feature-atomic-write.mjs      (FLOWS_HOST may name another qa- hub)
//
// A cut write cannot be caused from outside, so the probe proves the write path still lands whole:
// 1. a QA dish `LIVE-<run> atomic` is created at 1000 lek and put on sale (catalogue saves);
// 2. its price becomes 1070 (a catalogue save that rewrites the image in place);
// 3. a TEST order of it is placed (a log append) and costs exactly 1070 -- the second save landed whole;
// 4. POST /api/platform/compact with NO token is refused 401 (the route exists and is closed);
//    POST /api/platform/admins without / with a wrong x-dowiz-bootstrap is 404 (row 4; never sent the real secret).
//    It is NEVER called with a token: as a platform administrator it compacts AND PINS every venue.
// 5. clean-up: the TEST order ends, the dish is deleted.
// Every step that cannot run is a FAIL, never a skip.
import * as lib from '../../../e2e/flows/lib.mjs';
import { contract, reporter } from './stock-lib.mjs';
import { payload, close, sweep } from './_order.mjs';

const C = contract('atomic-write');
const { step, schema, verdict } = reporter('feature-atomic-write');
const run = `LIVE-${lib.RUN}`;
const must = (ok, why) => { if (!ok) throw new Error(why); };

let dish = null;
let placed = null;
try {
  await sweep(lib);
  const m = await lib.menu();
  const cat = (m.categories || [])[0]?.id;
  must(cat, 'the qa menu has no category');
  const mk = await lib.own('/api/owner/products', { location_id: lib.LOC, category_id: cat, name: `${run} atomic`, price: 1000, available: false });
  step('a QA dish is created at 1000 lek', mk.status === 200 && mk.body?.id, `${mk.status} ${mk.text}`);
  dish = mk.body?.id;
  must(dish, 'no dish id');
  const on = await lib.own(`/api/owner/products/${encodeURIComponent(dish)}`, { location_id: lib.LOC, allergens: ['fish'], available: true, price: 1070 });
  step('the price becomes 1070 and it is on sale', on.status === 200, `${on.status} ${on.text}`);

  const r = await lib.api(`/api/public/locations/${lib.LOC}/orders`, {
    method: 'POST', body: payload(run, { items: [{ product_id: dish, modifier_ids: [], quantity: 1 }] }) });
  if (r.status === 200 && r.body?.id) placed = r.body.id;
  step('a TEST order of the dish is placed (a log append)', r.status === 200, `${r.status} ${r.text.slice(0, 200)}`);
  schema('the placement matches the contract', r.body, C.routes.place.response_schema);
  step('it costs exactly 1070: the second catalogue save landed whole', r.body?.total === 1070, `${r.body?.total}`);

  const anon = await lib.api('/api/platform/compact', { method: 'POST', body: {} });
  step('POST /api/platform/compact without a token is refused 401', anon.status === 401, `${anon.status} ${anon.text.slice(0, 120)}`);

  // Row 4: the first-administrator route answers 404 to a caller without the secret -- never called WITH it here.
  const noSecret = await lib.api('/api/platform/admins', { method: 'POST', body: { email: `${run}@probe.invalid`, password: 'probe-password-1' } });
  step('POST /api/platform/admins without x-dowiz-bootstrap is 404', noSecret.status === 404, `${noSecret.status} ${noSecret.text.slice(0, 120)}`);
  const wrong = await lib.api('/api/platform/admins', { method: 'POST', headers: { 'x-dowiz-bootstrap': 'x'.repeat(40) }, body: { email: `${run}@probe.invalid`, password: 'probe-password-1' } });
  step('POST /api/platform/admins with a wrong secret is 404', wrong.status === 404, `${wrong.status} ${wrong.text.slice(0, 120)}`);
} catch (e) {
  step('the probe ran to the end', false, e.message);
} finally {
  if (placed) {
    try { await close({ lib, run, must }, placed); step(`TEST order ${placed} ended`, true); }
    catch (e) { step(`TEST order ${placed} ended`, false, e.message); }
  }
  if (dish) {
    const d = await lib.own(`/api/owner/products/${encodeURIComponent(dish)}/delete`, { location_id: lib.LOC });
    step('the QA dish is deleted', d.status === 200, `${d.status} ${d.text}`);
  }
  verdict();
}
