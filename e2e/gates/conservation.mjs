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
// TEN LAWS, each over the live platform:
//   1. every order's folded status equals the status it is served with
//   2. no order is stranded: nothing is held by an order that has ended
//   3. the money on an order is its lines plus fees minus its discount
//   4. every delivered order's cash is accounted to a courier
//   5. the image gauges read against their CEILING and are under it
//   6. nothing is quarantined, and the log's claim equals served + withheld
//   7. the nightly witness is not contradicted, and it is still being taken
//   8. every projection rebuilds from the log to what is being served, and the
//      stock ledger holds nothing for an order the log says has ended
//   9. the tax block conserves money: groups sum, discounts allocate exactly,
//      bases and totals add up, and stamps are re-derivable from their parts
//  10. the till adds up: per currency, every closed drawer's recorded
//      over/short is its count minus float + cash paid + pay-in - pay-out;
//      the cash the ORDERS hold equals the cash the till folded; and no cash
//      was taken with no till open
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

  // ── 9. the tax block conserves money ─────────────────────────────────────
  //
  // Every order placed at a venue with tax configured carries a tax block.
  // BLUEPRINT-TAX-PRICE-CHANNEL §3.4 defines the block and its constraints:
  // (a) Σ groups.tax == tax.total
  // (b) Σ discount_allocated == the order's discount
  // (c) for inclusive: Σ groups.base + fee.base + tip == total
  //     for exclusive: Σ groups.base + fee.base + tax.total + tip == total
  // (d) tax_of(base, rate_ppm, inclusive) recomputed from the stored values
  //     equals the stored tax (the stamp is re-derivable per §2.6)
  //
  // The rounding is the kernel's own (`crates/dowiz-core/src/eqc_gen.rs`,
  // apply_tax_*_int): half-up with an INTEGER b/2. Math.round(b/2) differs on
  // an odd denominator (base 480001 at 200001 ppm: 80001 vs 80000), which is
  // why the prove script carries that case.
  // ABSENT IS NOT A BREACH.
  for (const o of list) {
    if (!o.tax) continue; // no tax block to check
    const tax_block = o.tax;

    // (a) sum of group taxes must equal tax.total
    const group_taxes = (tax_block.groups || []).reduce((s, g) => s + (g.tax ?? 0), 0);
    const fee_tax = tax_block.fee?.tax ?? 0;
    const total_tax = group_taxes + fee_tax;
    if (total_tax !== tax_block.total) {
      note(venue, 'tax', `${o.id}: group and fee taxes sum to ${total_tax}, but tax.total is ${tax_block.total}`);
    }

    // (b) sum of allocated discounts must equal the order's discount
    const discount_allocated = (tax_block.discount_allocated || []).reduce((s, d) => s + d, 0);
    const order_discount = o.discount ?? o.promo_discount ?? 0;
    if (discount_allocated !== order_discount) {
      note(venue, 'tax', `${o.id}: allocated discounts sum to ${discount_allocated}, but order discount is ${order_discount}`);
    }

    // (c) base amounts and totals must add up correctly
    const group_bases = (tax_block.groups || []).reduce((s, g) => s + (g.base ?? 0), 0);
    const fee_base = tax_block.fee?.base ?? 0;
    const tip = o.tip ?? 0;
    const expected_total = tax_block.inclusive
      ? group_bases + fee_base + tip
      : group_bases + fee_base + total_tax + tip;
    if (o.total != null && o.total !== expected_total) {
      note(venue, 'tax', `${o.id}: bases and tax expected total ${expected_total}, but order total is ${o.total}`);
    }

    // (d) tax_of(base, rate_ppm, inclusive) for each group
    // Use integer rounding: for exclusive: floor((base * rate_ppm + 5e5) / 1e6)
    //                      for inclusive: floor((base * 1e6 + (1e6 + rate_ppm)/2) / (1e6 + rate_ppm))
    for (const g of tax_block.groups || []) {
      const base = g.base ?? 0;
      const rate = g.rate_ppm ?? 0;
      let recomputed;
      if (tax_block.inclusive) {
        // Inclusive: net = floor((gross * 1e6 + (1e6 + rate)/2) / (1e6 + rate))
        //            tax = gross - net
        const numerator = BigInt(base) * BigInt(1e6) + (BigInt(1e6) + BigInt(rate)) / 2n;
        const denominator = BigInt(1e6) + BigInt(rate);
        const net = numerator / denominator;
        recomputed = Number(BigInt(base) - net);
      } else {
        // Exclusive: tax = floor((base * rate + 5e5) / 1e6)
        const numerator = BigInt(base) * BigInt(rate) + BigInt(5e5);
        recomputed = Number(numerator / BigInt(1e6));
      }
      if (recomputed !== (g.tax ?? 0)) {
        note(venue, 'tax', `${o.id}: group at ${rate} ppm, base ${base}: recomputed tax ${recomputed}, but stored tax is ${g.tax}`);
      }
    }

    // Fee tax check (same as group tax)
    if (tax_block.fee) {
      const fee_base = tax_block.fee.base ?? 0;
      const fee_rate = tax_block.fee.rate_ppm ?? 0;
      let recomputed;
      if (tax_block.inclusive) {
        const numerator = BigInt(fee_base) * BigInt(1e6) + (BigInt(1e6) + BigInt(fee_rate)) / 2n;
        const denominator = BigInt(1e6) + BigInt(fee_rate);
        const net = numerator / denominator;
        recomputed = Number(BigInt(fee_base) - net);
      } else {
        const numerator = BigInt(fee_base) * BigInt(fee_rate) + BigInt(5e5);
        recomputed = Number(numerator / BigInt(1e6));
      }
      if (recomputed !== (tax_block.fee.tax ?? 0)) {
        note(venue, 'tax', `${o.id}: fee at ${fee_rate} ppm, base ${fee_base}: recomputed tax ${recomputed}, but stored tax is ${tax_block.fee.tax}`);
      }
    }
  }

  // ── 10. the till adds up, per currency ──────────────────────────────────
  //
  // BLUEPRINT-POS-THE-ROOM §2.5 / G2, with P3-2's currencies. The drawer is
  // one pile per currency -- lek and euro are two piles, never one number --
  // and for every CLOSED period, in every currency it names:
  //
  //   counted - (float + cash_paid + pay_in - pay_out) == over_short (recorded)
  //
  // `over_short` is what the close RECORDED; the parts are what the object
  // folds NOW. So a cash payment that appears or vanishes after the close is a
  // breach, not a quietly different number.
  //
  // AND THE ORDERS ARE ASKED THEMSELVES. `cash_paid` is the object's fold; the
  // gate re-sums every cash payment in the order list whose time falls in the
  // period, in the currency it was PAID in, and the two must agree. A cash
  // payment the till did not fold, or one outside every period, is named.
  //
  // An OPEN period is not balanced -- the drawer is still taking money -- and
  // ABSENT IS NOT ZERO: a Worker without `till` has not been measured, and
  // with no periods at all every cash payment would read as "outside".
  const till = health?.till;
  // A till that could not fold is said out loud, as law 8's rebuild is: an
  // error answered as "no periods" would read as a balanced venue.
  if (till?.error) note(venue, 'till', `the till could not be folded: ${till.error}`);
  if (till && Array.isArray(till.periods)) {
    const cashIn = [];
    for (const o of list) {
      for (const p of o.payments || []) {
        if (p.method === 'cash') cashIn.push({ id: o.id, at: p.at ?? 0, cur: p.currency || o.currency || 'ALL', n: p.amount ?? 0 });
      }
    }
    const within = (per, at) => at >= per.opened_at && (per.closed_at == null || at <= per.closed_at);
    const val = (m, c) => (m && m[c]) || 0;
    for (const per of till.periods) {
      if (per.closed_at == null) continue;
      const mine = {};
      for (const c of cashIn) if (within(per, c.at)) mine[c.cur] = (mine[c.cur] || 0) + c.n;
      const curs = new Set([per.float, per.cash_paid, per.pay_in, per.pay_out, per.counted, per.over_short, mine]
        .flatMap((m) => Object.keys(m || {})));
      for (const cur of curs) {
        const exp = val(per.float, cur) + val(per.cash_paid, cur) + val(per.pay_in, cur) - val(per.pay_out, cur);
        const said = val(per.over_short, cur);
        if (val(per.counted, cur) - exp !== said) {
          note(venue, 'till', `${per.till_id} opened ${per.opened_at} ${cur}: counted ${val(per.counted, cur)} - (float ${val(per.float, cur)} + cash ${val(per.cash_paid, cur)} + in ${val(per.pay_in, cur)} - out ${val(per.pay_out, cur)} = ${exp}) is ${val(per.counted, cur) - exp}, recorded over/short ${said}`);
        }
        if (val(mine, cur) !== val(per.cash_paid, cur)) {
          note(venue, 'till', `${per.till_id} opened ${per.opened_at} ${cur}: the orders hold ${val(mine, cur)} cash, the till folded ${val(per.cash_paid, cur)}`);
        }
      }
    }
    for (const c of cashIn) {
      if (!till.periods.some((per) => within(per, c.at))) {
        note(venue, 'till', `${c.id}: ${c.n} ${c.cur} cash at ${c.at} with no till open`);
      }
    }
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
