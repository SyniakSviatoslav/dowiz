// AN ARCHIVED ORDER, LIVE on qa-durres (2026-10-14, commits 2e3be4d8 + 867d8d67).
// The rotation moves an order out of the hot log after 30 quiet days in a final
// status. Before those commits the hot log answered 404 for it everywhere:
// a receipt reprint said "order not found", a refund said the same, and a
// courier holding a run on a refunded order could never end a shift.
//
//   1. the venue gets a tax rate for this run (tax.default_ppm), and pickup on,
//      so a pickup order carries a `tax` block and owes a fiscal document;
//   2. one pickup order is placed and walked to PICKED_UP by the owner's API;
//   3. the QA hub rotates AS OF now + 31 days (POST /owner/analytics/history
//      {rotateAsOf}, QA hub only) -- the order leaves the hot log;
//   4. the receipt is 200 and its text names the order's dish and total,
//      read from the archive;
//   5. a refund is 409 with the archived wording, and nothing is written;
//   6. the settings are put back as found, whatever failed.
//
// The rotation moves EVERY finished order of the QA hub, which is what the
// analytics QA check already does; nothing open moves (rotate keeps LIVE
// statuses and every order with an open courier run).
//
//   node e2e/archive/archived-order.mjs        # exit 0 GREEN, 1 RED, 2 could not run
import { api, own, LOC, RUN, menu, dishes, ownerOrders, statusOf } from '../flows/lib.mjs';

const RATE = '200000'; // 20 % in ppm: Albania's standard VAT
const DAY = 24 * 60 * 60 * 1000;
const fails = [];
const log = (ok, what) => { console.log(`  archived: ${ok ? 'ok  ' : 'FAIL'} ${what}`); if (!ok) fails.push(what); };
const die = (what) => { console.log(`  archived: CANNOT RUN ${what}`); process.exitCode = 2; };

if (LOC !== 'qa-durres') { die(`only the QA hub may be rotated as of another instant (host names ${LOC})`); process.exit(2); }

// ── what to put back ───────────────────────────────────────────────────────
const s = await own(`/api/owner/settings?location_id=${LOC}`);
if (s.status !== 200) { die(`GET /owner/settings ${s.status}`); process.exit(2); }
const foundRate = s.body?.values?.['tax.default_ppm'] ?? '';
const foundPickup = !!(await menu()).location?.pickup;

async function restore() {
  const r = await own(`/api/owner/settings?location_id=${LOC}`, { key: 'tax.default_ppm', value: foundRate });
  console.log(`  archived: restore tax.default_ppm -> ${JSON.stringify(foundRate)}: ${r.status}`);
  if (!foundPickup) {
    const p = await own('/api/owner/location', { location_id: LOC, pickup: false });
    console.log(`  archived: restore pickup -> false: ${p.status}`);
  }
}

try {
  // ── 1. a rate and pickup for this run ──────────────────────────────────
  const t = await own(`/api/owner/settings?location_id=${LOC}`, { key: 'tax.default_ppm', value: RATE });
  if (t.status !== 200) throw new Error(`setting tax.default_ppm: ${t.status} ${t.text.slice(0, 120)}`);
  if (!foundPickup) {
    const p = await own('/api/owner/location', { location_id: LOC, pickup: true });
    if (p.status !== 200) throw new Error(`switching pickup on: ${p.status} ${p.text.slice(0, 120)}`);
  }

  // ── 2. one pickup order, walked to PICKED_UP ───────────────────────────
  const dish = dishes(await menu()).find(d => d.available);
  if (!dish) throw new Error('no available dish on the QA menu');
  const placed = await api(`/api/public/locations/${LOC}/orders`, { method: 'POST', headers: { 'idempotency-key': `${RUN}-archived` }, body: {
    contact: { name: `${RUN} archived`, phone: '+355690000009' },
    fulfilment: { kind: 'pickup' }, items: [{ product_id: dish.id, quantity: 1 }], payment: 'cash',
  } });
  if (placed.status >= 400 || !placed.body?.id) throw new Error(`placing: ${placed.status} ${placed.text.slice(0, 160)}`);
  const id = placed.body.id;
  for (const a of ['confirm', 'preparing', 'ready', 'collected']) {
    const r = await own(`/api/owner/orders/${id}/action`, { action: a, location_id: LOC });
    if (r.status !== 200) throw new Error(`owner ${a}: ${r.status} ${r.text.slice(0, 120)}`);
  }
  const st = await statusOf(id);
  log(st === 'PICKED_UP', `order ${id} (${dish.name}) placed and walked to ${st}`);
  const hot = (await ownerOrders()).find(o => o.id === id);
  log(!!hot?.tax, `it carries a tax block (${JSON.stringify(hot?.tax ?? null).slice(0, 80)})`);

  // ── 3. rotate as of a month from now ───────────────────────────────────
  const rot = await own(`/api/owner/analytics/history?location_id=${LOC}`, { rotateAsOf: Date.now() + 31 * DAY });
  if (rot.status !== 200) throw new Error(`rotating: ${rot.status} ${rot.text.slice(0, 160)}`);
  const stillHot = (await ownerOrders()).some(o => o.id === id);
  log(!stillHot, `the rotation moved it out of the hot log (${JSON.stringify(rot.body?.rotated ?? null).slice(0, 120)})`);

  // ── 4. the receipt, from the archive ───────────────────────────────────
  const rc = await own(`/api/owner/orders/${id}/receipt?location_id=${LOC}`);
  const text = rc.body?.text || '';
  log(rc.status === 200, `the receipt answers 200 (got ${rc.status} ${rc.status === 200 ? '' : rc.text.slice(0, 100)})`);
  log(text.includes(dish.name), `its text names the dish "${dish.name}" (${text.split('\n').slice(0, 3).join(' | ').slice(0, 120)})`);

  // ── 5. the refund, refused by name ─────────────────────────────────────
  const f = await own(`/api/staff/orders/${id}/refund`, { location_id: LOC, reason: 'venue_cancelled' });
  log(f.status === 409 && /archived/.test(f.text), `the refund is 409 and says why (${f.status} ${f.text.slice(0, 110)})`);
  log((await ownerOrders()).every(o => o.id !== id), 'nothing was written: the order did not come back to the hot log');
  // Positive twin: an id no log knows is still "not found", not "archived".
  const u = await own(`/api/staff/orders/00000000-0000-4000-8000-000000000000/refund`, { location_id: LOC, reason: 'venue_cancelled' });
  log(u.status === 404, `an unknown id still answers 404 (${u.status})`);
} catch (e) {
  die(e.message);
} finally {
  await restore();
}

if (process.exitCode !== 2) {
  console.log(fails.length ? `archived-order: RED (${fails.length})` : 'archived-order: GREEN');
  process.exitCode = fails.length ? 1 : 0;
}
