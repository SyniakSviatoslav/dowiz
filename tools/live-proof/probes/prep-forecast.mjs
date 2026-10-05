// LIVE PROBE kitchen.prep_forecast.v1 (W-PREP, P6) on qa-durres -- main runs
// it after the deploy; a lane never runs it against production.
//
//   set -a; . /root/.dowiz_owner; set +a; node tools/live-proof/probes/prep-forecast.mjs
//   PREP_BACKFILL=40 node tools/live-proof/probes/prep-forecast.mjs     (adds today's orders)
//
// WHAT IT PROVES, through the real Worker, venue object, cube and stock log:
//   1. GET /api/staff/kitchen/prep validates against the contract; today's day.
//   2. The strict query: an unknown key is 400; a day 7 days ahead is 400.
//   3. LEARNING IS HONEST: with fewer than 3 same-weekday samples the value is
//      null and nothing is listed to make; with 3+ it is an integer.
//   4. THE FORECAST IS THE MEDIAN OF WHAT WAS PLACED: every sample day's
//      `placed` equals that day's `orders` in /api/owner/analytics?v=2 (the
//      cube + hot log, read independently), and when the method is the median,
//      `orders.value` is the median of the samples' `orders`, computed HERE.
//   5. The error fields exist (`offBy`, `naiveOffBy`, `masePm`, `checkedDays`).
//   6. BACK-FILL (opt-in, PREP_BACKFILL=N, N <= 300): N TEST pickup orders of
//      QA Water placed through the real storefront route and driven to
//      PICKED_UP. They are TODAY's: today is never a sample, so the samples
//      must not move -- the history grows by one weekday per weekly run; a
//      6-week series cannot be written in one run because the order route
//      stamps its own clock (no back-dating exists, by design).
//      BUDGET (Free plan, GUESS from the DO request cap of 100 000/day): an
//      order costs ~6 requests (place, 4 actions, a read) -> 300 orders ~ 1 800,
//      under 2 % of the day. The probe refuses N > 300.
// Exit 0 = every step held; 1 = a step failed; 3 = NEEDS-KEY.
import { LOC, RUN, api, own, owner, reporter, contract, read } from './stock-lib.mjs';

const C = contract('prep-forecast');
const { step, schema, verdict } = reporter('prep-forecast');
const MAX_BACKFILL = 300;
const median = v => { const s = [...v].sort((a, b) => a - b), n = s.length; return n === 0 ? 0 : n % 2 ? s[(n - 1) / 2] : Math.floor((s[n / 2 - 1] + s[n / 2] + 1) / 2); };
const prep = q => read(own, `/api/staff/kitchen/prep?location_id=${LOC}${q || ''}`);

async function backfill(n) {
  const made = [];
  for (let i = 0; i < n; i++) {
    const body = { items: [{ product_id: 'qa-water', modifier_ids: [], quantity: 1 }], contact: { name: `${RUN} prep`, phone: '+355690000019' },
      fulfilment: { kind: 'pickup' }, payment: 'cash', locale: 'en' };
    const r = await api(`/api/public/locations/${LOC}/orders`, { method: 'POST', body });
    if (r.status !== 200 || !r.body?.id) { step(`back-fill order ${i + 1} placed`, false, `${r.status} ${r.text?.slice(0, 120)}`); break; }
    let ok = true;
    for (const a of ['confirm', 'preparing', 'ready', 'collected']) {
      const x = await own(`/api/owner/orders/${r.body.id}/action`, { action: a, location_id: LOC });
      if (x.status !== 200) { ok = step(`back-fill ${r.body.id} ${a}`, false, `${x.status} ${x.text?.slice(0, 120)}`); break; }
    }
    if (ok) made.push(r.body.id);
  }
  return made;
}

try {
  const tok = await owner();
  step('the QA owner signs in', !!tok);
  const a = await prep();
  step('GET /api/staff/kitchen/prep answers 200', a.status === 200, `${a.status} ${a.text?.slice(0, 160)}`);
  schema('the answer validates against kitchen.prep_forecast.v1', a.body, C.response_schema);
  const p = a.body || {};
  step('the day is today, and the contract is named', p.day === p.today && p.contract === 'kitchen.prep_forecast.v1', `${p.day} / ${p.today}`);

  const bad = await own(`/api/staff/kitchen/prep?location_id=${LOC}&dya=2026-01-01`);
  step('an unknown query key is refused (400)', bad.status === 400, `${bad.status} ${bad.text?.slice(0, 100)}`);
  const ahead = new Date(Date.parse(`${p.today}T12:00:00Z`) + 7 * 86400000).toISOString().slice(0, 10);
  const far = await own(`/api/staff/kitchen/prep?location_id=${LOC}&day=${ahead}`);
  step('a day 7 days ahead is refused (400)', far.status === 400, `${far.status}`);

  const learning = (p.portions?.weeks ?? 0) < 3;
  if (learning) {
    step('under 3 weeks: no number, nothing to make', p.portions?.value === null && p.portions?.learning === true && (p.preps || []).every(x => !x.make), JSON.stringify(p.portions));
  } else {
    step('3+ weeks: an integer forecast', Number.isInteger(p.portions?.value) && p.portions?.learning === false, JSON.stringify(p.portions));
  }

  const hist = await read(own, `/api/owner/analytics?location_id=${LOC}&v=2&days=90`);
  step('the analytics v2 history answers', hist.status === 200, `${hist.status}`);
  const byDay = Object.fromEntries((hist.body?.byDay || []).map(d => [d.day, d.orders]));
  const samples = p.samples || [];
  step('every sample day placed what the analytics counted', samples.every(s => (byDay[s.day] ?? 0) === s.placed), JSON.stringify(samples.map(s => [s.day, s.placed, byDay[s.day] ?? 0])));
  if (!learning && p.orders?.method === 'median') {
    const m = median(samples.map(s => s.orders));
    step('the orders forecast is the median of the samples, computed here', p.orders.value === m, `hub ${p.orders.value}, probe ${m} over ${samples.length} samples`);
  } else {
    step('the median cross-check (needs 3+ weeks with a regular series)', 'NEEDS-KEY', `NEEDS-DATA: weeks ${p.portions?.weeks}, method ${p.orders?.method}`);
  }
  for (const k of ['offBy', 'naiveOffBy', 'masePm', 'checkedDays']) step(`the error field ${k} exists`, k in (p.portions || {}));

  const n = Number(process.env.PREP_BACKFILL || 0);
  if (n > 0) {
    if (n > MAX_BACKFILL) step(`back-fill of ${n} refused: at most ${MAX_BACKFILL} orders`, false);
    else {
      const made = await backfill(n);
      step(`back-fill: ${made.length} of ${n} TEST orders picked up`, made.length === n);
      const b = await prep();
      step('today is not a sample: the samples did not move', JSON.stringify(b.body?.samples) === JSON.stringify(samples), JSON.stringify(b.body?.samples?.slice(-2)));
    }
  }
} catch (e) {
  step('the probe ran to the end', false, e.stack?.slice(0, 300));
} finally {
  verdict();
}
