// LIVE PROBE guest-taste (W-MR0 rows MR7 + MR8; operator ruling 2026-10-04: automatic, no box,
// one-tap objection) on qa-durres -- main runs it AFTER the deploy; a lane never runs it against
// production.
//
//   node tools/live-proof/probes/feature-guest-taste.mjs      (FLOWS_HOST may name another qa- hub)
//
// 1. a TEST order with a phone and a taste_sync, no box -> the guest's view: objected false, a profile
//    with the device vector; the owner's card (by the masked row's key) shows the same; segments validate;
// 2. a malformed taste_sync is refused 400;
// 3. the guest taps "turn off" (withdraw) -> objected true, deleted true; the view reads taste null;
//    the next order keeps nothing and its taste_sync is refused 400 by name;
// 4. a second guest's phone sends taste_off with an order -> their first order's profile is gone;
// 5. a third guest is FORGOTTEN by the owner -> their profile is gone.
// Every TEST order is ended. A step that cannot run is a FAIL, never a skip.
import * as lib from '../../../e2e/flows/lib.mjs';
import { contract, reporter } from './stock-lib.mjs';
import { place, close, sweep } from './_order.mjs';

const C = contract('guest-taste');
const { step, schema, verdict } = reporter('feature-guest-taste');
const run = `LIVE-${lib.RUN}`;
const must = (ok, why) => { if (!ok) throw new Error(why); };
// Three phones this run, distinct in their last two digits (the owner list shows only those).
const tail = Number.parseInt(lib.RUN.replace(/\D/g, '').slice(-2) || '0', 10) % 30;
const phone = i => `+3556900007${String(tail + i * 30).padStart(2, '0').slice(-2)}`;
const SYNC = { v: 1, tags: { spicy: 1000, salmon: 400 }, cats: { rolls: 1000 } };

const orders = [];
const view = async o => lib.api(`/api/order/${encodeURIComponent(o.id)}/taste`, { token: o.token });
const card = async k => lib.own(`/api/owner/customers/${encodeURIComponent(k)}/taste?location_id=${lib.LOC}`);
async function keyOf(ph, since) {
  const c = await lib.own(`/api/owner/customers?location_id=${lib.LOC}&sort=recent`);
  must(c.status === 200, `customers ${c.status}`);
  const row = (c.body.customers || []).find(x => x.phone === `+355•••••${ph.slice(-2)}` && (x.lastAt || 0) >= since);
  must(row, `the TEST customer ${ph.slice(-2)} is not in the owner list`);
  return row.key;
}
const placed = async (extra, ph) => { const o = await place({ lib, run, must }, extra, ph); orders.push(o.id); return o; };

try {
  await sweep(lib);

  // 1. automatic: a profile with no box
  const o1 = await placed({ taste_sync: SYNC }, phone(0));
  const v1 = await view(o1);
  schema('guest view matches its schema', v1.body, C.guest_view_schema);
  step('no box: a profile with the device vector', v1.status === 200 && v1.body?.objected === false && v1.body?.taste?.orders >= 1 && !!v1.body?.taste?.device, `${v1.status} ${v1.text}`);
  const k1 = await keyOf(phone(0), o1.body.created_at_ms || 0);
  const c1 = await card(k1);
  step('the owner card shows the same profile', c1.status === 200 && c1.body?.taste?.segment === v1.body?.taste?.segment && c1.body?.taste?.orders === v1.body?.taste?.orders, `${c1.status} ${c1.text}`);
  const sg = await lib.own(`/api/owner/customers/taste/segments?location_id=${lib.LOC}`);
  schema('segment counts match their schema', sg.body, C.segments_response_schema);
  step('segments count at least this guest', Object.values(sg.body?.segments || {}).reduce((a, b) => a + b, 0) >= 1, sg.text);

  // 2. the closed shape
  const bad = await lib.api(`/api/public/locations/${lib.LOC}/orders`, { method: 'POST', body: {
    items: [{ product_id: 'qa-water', modifier_ids: [], quantity: 1 }], contact: { name: `${run} guest`, phone: phone(0) },
    fulfilment: { kind: 'pickup', note: null }, payment: 'cash', locale: 'en', taste_sync: { v: 1, tags: { x: 5000 } } } });
  if (bad.body?.id) orders.push(bad.body.id);
  step('a malformed taste_sync is refused 400', bad.status === 400, `${bad.status} ${bad.text}`);

  // 3. one tap: objected, deleted, stopped
  const w = await lib.api(`/api/order/${encodeURIComponent(o1.id)}/taste/withdraw`, { method: 'POST', token: o1.token });
  schema('withdraw matches its schema', w.body, C.withdraw_response_schema);
  step('turn off: objected and deleted', w.status === 200 && w.body?.objected === true && w.body?.deleted === true, `${w.status} ${w.text}`);
  const v3 = await view(o1);
  step('after turn off: objected true, taste null', v3.body?.objected === true && v3.body?.taste === null, v3.text);
  await placed({}, phone(0));
  step('the next order keeps nothing', (await card(k1)).body?.taste === null);
  const again = await lib.api(`/api/public/locations/${lib.LOC}/orders`, { method: 'POST', body: {
    items: [{ product_id: 'qa-water', modifier_ids: [], quantity: 1 }], contact: { name: `${run} guest`, phone: phone(0) },
    fulfilment: { kind: 'pickup', note: null }, payment: 'cash', locale: 'en', taste_sync: SYNC } });
  if (again.body?.id) orders.push(again.body.id);
  step('a taste_sync after the objection is refused 400 by name', again.status === 400 && /turned personalisation off/.test(again.text), `${again.status} ${again.text}`);

  // 4. taste_off from a phone with no link
  const o4 = await placed({ taste_sync: SYNC }, phone(1));
  const k4 = await keyOf(phone(1), o4.body.created_at_ms || 0);
  step('a second guest has a profile', (await card(k4)).body?.taste?.orders >= 1);
  await placed({ taste_off: true }, phone(1));
  step('taste_off with an order deletes it', (await card(k4)).body?.taste === null);

  // 5. the owner's forget takes the profile with the card
  const o5 = await placed({}, phone(2));
  const k5 = await keyOf(phone(2), o5.body.created_at_ms || 0);
  step('a third guest has a profile', (await card(k5)).body?.taste?.orders >= 1);
  await close({ lib, run, must }, o5.id);
  const fg = await lib.own(`/api/owner/customers/${encodeURIComponent(k5)}/forget?location_id=${lib.LOC}`, { reason: `${run} live-proof`, lang: 'en' });
  step('the owner forgets that guest', fg.status === 200, `${fg.status} ${fg.text}`);
  const after = await card(k5);
  step('forget: the profile is gone', after.status === 200 && after.body?.taste === null, `${after.status} ${after.text}`);
} catch (e) {
  step('the probe ran to the end', false, e.stack || String(e));
} finally {
  for (const id of orders) {
    try { await close({ lib, run, must }, id); } catch (e) { step(`ending TEST order ${id}`, false, String(e.message || e)); }
  }
}
verdict();
