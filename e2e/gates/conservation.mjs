// F8 — THE CONSERVATION AUDIT. The platform's steady-state hypothesis.
//
// WHY THIS IS THE FIRST HOLISTIC GATE. Every chaos, soak and concurrency test
// in the hardening plan is defined as "F8 still holds while X is happening", so
// until this exists those tests have nothing to assert. It is also the one
// question a restaurant actually asks: does the register add up.
//
// Borrowed from logistics rather than from web practice. A WMS is not asked
// "did the API return 200"; it is asked whether the ledger still balances. This
// codebase already has the machinery in two places and ran it in neither:
// `money.rs` nets refunds to exactly zero through double-entry, and
// `StockLedger::stranded()` is a conservation report. What was missing was
// anything that ran them as a gate.
//
// EIGHT LAWS, each over the live platform:
//   1. every order's folded status equals the status it is served with
//   2. no order is stranded: nothing is held by an order that has ended
//   3. the money on an order is its lines plus fees minus its discount
//   4. every delivered order's cash is accounted to a courier
//   5. the image gauges read against their CEILING and are under it
//   6. nothing is quarantined, and the log's claim equals served + withheld
//   7. the nightly witness is not contradicted, and it is still being taken
//   8. every projection rebuilds from the log to what is being served, and the
//      stock ledger holds nothing for an order the log says has ended
//
// It takes an owner token per venue and reads only. Exit 1 on any breach, with
// the order named -- a gate whose failure cannot be chased is a dashboard.
import fs from 'node:fs';

const creds = Object.fromEntries(
  fs.readFileSync('/root/.dowiz_owner', 'utf8')
    .split('\n').filter(l => l.startsWith('export '))
    .map(l => l.slice(7).split('=')));

const HOSTS = (process.env.HOSTS || 'dubin-sushi,sushi-durres').split(',');
const breaches = [];
const note = (venue, law, detail) => breaches.push({ venue, law, detail });

const j = async (host, path, opts = {}) => {
  const r = await fetch(`https://${host}.dowiz.org${path}`, opts);
  const t = await r.text();
  try { return { status: r.status, body: JSON.parse(t) }; } catch { return { status: r.status, body: t }; }
};

for (const host of HOSTS) {
  const login = await j(host, '/api/auth/login', {
    method: 'POST', headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ email: creds.OWNER_EMAIL, password: creds.OWNER_PASSWORD }),
  });
  const token = login.body?.access_token;
  const venue = login.body?.user?.locationId;
  if (!token || !venue) { note(host, 'login', `cannot sign in: ${login.status}`); continue; }
  const auth = { authorization: `Bearer ${token}` };

  const orders = (await j(host, `/api/owner/orders?location_id=${venue}`, { headers: auth })).body;
  const list = Array.isArray(orders) ? orders : (orders?.orders || []);

  // ── 3. the money on an order adds up ────────────────────────────────────
  //
  // INTEGER MINOR UNITS THROUGHOUT. A float here would make the gate itself a
  // source of false alarms, which is how a conservation check gets switched off.
  for (const o of list) {
    const lines = (o.items || []).reduce(
      (n, i) => n + (i.total ?? ((i.price ?? 0) * (i.quantity ?? 1))), 0);
    const fee = o.delivery_fee ?? 0;
    const discount = o.discount ?? o.promo_discount ?? 0;
    // THE TIP WAS MISSING FROM THIS LAW AND IT IS PART OF THE TOTAL.
    //
    // `storefront.rs:1016` is `subtotal + fee + tip`, and this expected
    // `lines + fee - discount`. So the first order anyone leaves a tip on
    // breaches the platform's own conservation audit — a false alarm on a
    // correct order, which is exactly how a conservation check gets switched
    // off. Measured 2026-09-22 before the fix: 0 of 52 live orders on
    // sushi-durres carry a tip, so it had never fired. A law that is correct
    // only because a feature is unused is a law waiting to be wrong.
    //
    // The tip is ADDED, not subtracted, and it is not the venue's money — see
    // `services/orders/status.rs`, where the courier's tip is excluded from
    // takings. This law is about the total a customer was charged, which does
    // include it.
    const tip = o.tip ?? 0;
    const expect = lines + fee + tip - discount;
    if (o.total != null && lines > 0 && o.total !== expect) {
      note(venue, 'money', `${o.id}: total ${o.total}, lines ${lines} + fee ${fee} + tip ${tip} - discount ${discount} = ${expect}`);
    }
  }

  // ── 2. nothing is stranded ──────────────────────────────────────────────
  //
  // An order past its ending must hold no stock and no courier. The FSM has no
  // exit from several states (a known defect), so this measures the size of
  // that hole rather than assuming it is empty.
  const ENDED = new Set(['DELIVERED', 'CANCELLED', 'REJECTED', 'COLLECTED']);
  const LIVE = new Set(['PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY']);
  const stale = list.filter(o =>
    LIVE.has(o.status) && Date.now() - (o.created_at_ms ?? Date.now()) > 7 * 24 * 3600e3);
  if (stale.length) {
    note(venue, 'stranded', `${stale.length} order(s) live for over a week: ${stale.slice(0, 5).map(o => `${o.id}=${o.status}`).join(', ')}`);
  }

  // ── 4. delivered cash is accounted ──────────────────────────────────────
  for (const o of list.filter(x => x.status === 'DELIVERED' && x.payment === 'cash')) {
    if (o.cash_collected == null) {
      note(venue, 'cash', `${o.id}: delivered for cash and no cash_collected recorded`);
    }
  }

  // ── 5. the gauges, and THE TWO KINDS OF IMAGE ARE NOT THE SAME QUESTION ──
  //
  // This gate's first run failed on `log at 855` and `stock at 967` and both
  // were wrong alarms. Those images carry `grows: true`: a full arena DOUBLES
  // and the chain is copied across, so approaching 1000 per mille is the
  // sawtooth working, not a refusal coming. `bebop-ceiling-not-capacity`
  // records the same confusion in the other direction -- a compacted image
  // measured against its capacity read 829 on a venue under 10% full.
  //
  // So: a COMPACTED image is measured against its ceiling, which is the real
  // doubling limit and the point it refuses. A GROWING image is asked a
  // different question entirely -- is anything bounding it? The nightly
  // rotation is what should, and an image far past the hot-log ceiling means
  // it is not running.
  // CELLS PER ORDER, not cells. The first version of this check said "over
  // twice the hot-log ceiling: rotation is not bounding it" and that was a
  // wrong diagnosis: rotation IS bounding it, at thirty days, and thirty days
  // of one venue's history is simply larger than another's. What is comparable
  // across venues -- and what the hub-cost blueprint actually budgets -- is
  // cells per order.
  //
  // MEASURED 2026-09-21: sushi-durres 13,137 cells over 248 orders = 53 each;
  // dubin-durres 133,085 over 213 = 625 each, TWELVE TIMES more. Both run the
  // same code. The difference is history: dubin is the LEGACY_VENUE and its
  // older events were written before events carried deltas, so each one holds
  // a whole order envelope. The budget is set above the delta-era number and
  // well below the pre-delta one, so it passes a venue writing deltas today
  // and names one that is still carrying the old shape.
  const CELLS_PER_ORDER_BUDGET = 120;
  const health = (await j(host, `/api/owner/health?location_id=${venue}`, { headers: auth })).body;
  for (const [name, g] of Object.entries(health?.images || {})) {
    const ceiling = g.ceilingCells ?? g.ceiling_cells;
    const used = g.usedCells ?? g.used_cells;
    if (!ceiling || used == null) continue;
    if (g.grows) {
      const orders = health?.orders ?? 0;
      if (name === 'log' && orders > 20) {
        const per = Math.round(used / orders);
        if (per > CELLS_PER_ORDER_BUDGET) {
          note(venue, 'per-order', `log costs ${per} cells per order over ${orders} orders (budget ${CELLS_PER_ORDER_BUDGET})`);
        }
      }
      continue;
    }
    const perMille = Math.round((used * 1000) / ceiling);
    if (perMille > 800) note(venue, 'image', `${name} at ${perMille} per mille of its ceiling`);
  }

  // ── 6. nothing is quarantined ───────────────────────────────────────────
  //
  // A record this build cannot read is skipped by `events()` and named by
  // `/api/owner/health`. It is a FAILING LAW rather than a warning for the
  // reason the blueprint gives: a quarantine nobody notices is a data-loss
  // feature with better manners. The arithmetic beside it is the other half --
  // the root's claim must equal what the log delivers plus what it withheld,
  // or a record went somewhere neither list admits to.
  const q = health?.quarantined || [];
  for (const r of q) note(venue, 'quarantine', `${r.image || 'log'} record ${r.id.slice(0, 16)}… at ${r.at} unreadable (${r.reason})`);
  // The arithmetic below is the ORDER log's, so it counts only that log's
  // withheld records. An entry from another image is still a breach -- it was
  // noted above -- but adding it here would make the sum disagree for a
  // reason that has nothing to do with the orders.
  const qLog = q.filter((r) => (r.image || 'log') === 'log');
  // `events` is absent on a deployment older than this field, and a gate that
  // goes red because the Worker has not been deployed yet is a gate that gets
  // switched off. Absent means "not measured", never "zero".
  const claimed = health?.orders ?? 0;
  const served = health?.events;
  if (claimed && served != null && served + qLog.length !== claimed) {
    note(venue, 'quarantine', `log claims ${claimed} records, serves ${served}, withholds ${qLog.length}`);
  }

  // ── 7. the witness is not contradicted, and it is not stale ─────────────
  //
  // The nightly writes a census of the venue's history — record count, tip,
  // and a seal per archive — to the PLATFORM object and into the off-site
  // copy. `found` is what tonight's census said about last night's: an empty
  // list is the good answer, and anything in it means the log no longer
  // agrees with an account kept somewhere its writer does not own.
  //
  // ABSENT IS NOT ZERO. A venue whose nightly has not run since this shipped
  // has no census, and a gate that goes red for that is a gate switched off
  // in a week. A census that EXISTS and is older than two nights is a
  // different thing: the instrument has stopped, and an instrument that has
  // stopped is the failure this repo keeps a catalogue of.
  const backup = (await j(host, `/api/owner/backup/cloud?location_id=${venue}`, { headers: auth })).body;
  const w = backup?.witness;
  if (w) {
    for (const what of w.found || []) note(venue, 'witness', what);
    const ageH = Math.round((Date.now() - w.atMs) / 3600e3);
    if (ageH > 48) note(venue, 'witness', `the last census is ${ageH}h old: the nightly has stopped`);
  }

  // ── 8. the projections rebuild to what is being served ──────────────────
  //
  // THE ONE THAT MAKES EVENT-SOURCING A PROPERTY RATHER THAN A STORAGE
  // CHOICE. The log is append-only and chained, `fold` is state, and every
  // projection is memoised per generation — and none of that was ever
  // CHECKED, because every reader in the platform asks the SAME memo and so
  // nothing is in a position to disagree with it. `/api/owner/health` now
  // refolds from the stored bytes and compares.
  //
  // AND IT CROSSES THE TWO IMAGES. The class `253e1ece` closed for bookings —
  // an aggregate whose status disagreed with its own history — is still open
  // between the ORDER log and the STOCK ledger: the log can say an order is
  // over while the ledger holds its ingredients, and no screen shows it.
  // `StockLedger::stranded()` could report that since it was written and
  // nothing in production ever called it.
  //
  // ABSENT IS NOT INTACT, for the same reason as laws 6 and 7: a deployment
  // older than this field has not been measured, and silence must not read as
  // health. A rebuild that FAILED reports `intact:false` with its error.
  const rb = health?.rebuild;
  if (rb) {
    for (const id of rb.stale || []) {
      note(venue, 'rebuild', `${id}: the served projection differs from a fresh fold of the log`);
    }
    for (const id of rb.stranded || []) {
      note(venue, 'rebuild', `${id}: the ledger still holds ingredients for an order the log says has ended`);
    }
    if (rb.error) note(venue, 'rebuild', `the rebuild could not run: ${rb.error}`);
    // `unheld` is NOT a breach. A venue that models no recipes reserves
    // nothing, which is both live venues today; reporting it would make the
    // gate red for the normal state of the product.
  }

  console.log(`${venue}: ${list.length} orders, ${ENDED.size} terminal states known, ${Object.keys(health?.images || {}).length} images gauged, ${q.length} quarantined, witness ${w ? (w.found?.length ? 'CONTRADICTED' : `${w.total} records`) : 'not taken yet'}`);
}

if (breaches.length === 0) {
  console.log('\nCONSERVATION HOLDS');
  process.exit(0);
}
console.log(`\nCONSERVATION BREACHED — ${breaches.length}:`);
for (const b of breaches) console.log(`  [${b.venue}] ${b.law}: ${b.detail}`);
process.exit(1);
