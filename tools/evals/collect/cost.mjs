// SUITE nightly · COST: $/venue/month, MODELLED over the counts this run collected.
//
// The model is dowiz-hub-unit-costs (2026-09-20): requests, not duration, are what bind. It runs
// LAST in the nightly suite and reads the other collectors' numbers (`ctx.results`): the venue
// count from /api/platform/health (else the hosts probed), orders per day from the product
// suite, requests per order from the order suite (else the traffic budget, and says so).
//
// Money is integer MICRO-dollars (µ$) — no float near money. When the Cloudflare analytics
// token is present (collect/cf.mjs) the MEASURED Worker and object requests of the last 24 h
// price the month, the modelled ones are kept beside them, and the gap is judged (measured
// <= 1.2 x modelled, §B.2). Without the token the modelled number prices it, and says so.
import { ind, unverified } from '../rules.mjs';
import { measure } from './cf.mjs';

export const USD = 1_000_000; // µ$ per dollar
/** µ$ per million requests at overage (developers.cloudflare.com, 2026-09-20). */
export const PRICE = { worker: 300_000, durableObject: 150_000 };
export const FIXED_MONTH = 5 * USD + Math.round(11_200_000 / 12); // Workers Paid + the .org
export const MINUTES = 1_440;
export const DAYS = 30;
export const REQUESTS_PER_ORDER_BUDGET = 120; // traffic.mjs BUDGET.apiRequests
export const LOG_SAMPLING_PER_MILLE = 100; // head_sampling_rate = 0.1
export const LOGS_FREE_MONTH = 20_000_000;
export const MARGINAL_MAX = USD; // ≤ $1.00 per venue-month (§B.2 Cost)

const val = (r, id) => (r[id] && Number.isFinite(r[id].value) ? r[id].value : null);

/** The model's inputs, each with where it came from. */
export function inputs(results, hosts) {
  const venuesMeasured = val(results, 'platform.venues');
  const venues = venuesMeasured ?? hosts.length;
  const weekly = Object.keys(results).filter(k => /^product\.[^.]+\.orders_7d$/.test(k)).map(k => val(results, k)).filter(v => v !== null);
  const ordersPerDay = weekly.length ? Math.round(weekly.reduce((a, b) => a + b, 0) / 7) : 0;
  const perOrderMeasured = val(results, 'order.requests_total');
  return {
    venues: Math.max(1, venues),
    venuesFrom: venuesMeasured === null ? `the ${hosts.length} hosts probed` : 'GET /api/platform/health',
    ordersPerDay,
    ordersFrom: `${weekly.length} venues' orders_7d / 7`,
    perOrder: perOrderMeasured ?? REQUESTS_PER_ORDER_BUDGET,
    perOrderFrom: perOrderMeasured === null ? 'MODELLED: traffic.mjs budget (the order suite did not run)' : 'order.requests_total',
  };
}

/** µ$ per venue-month for a day's Worker and object requests, integers throughout. */
export function price(worker, durable, venues) {
  const monthMicro = Math.round((DAYS * (worker * PRICE.worker + durable * PRICE.durableObject)) / 1_000_000);
  const marginal = Math.round(monthMicro / venues);
  return { marginal, allIn: marginal + Math.round(FIXED_MONTH / venues) };
}

/** Modelled requests per day and their price. */
export function model(i) {
  const cronDo = 2 * MINUTES * i.venues; // outbox + ebills sweep, one object request each per venue
  const cronWorker = MINUTES + 1;
  const user = i.ordersPerDay * i.perOrder; // each: one Worker request and one object request
  const worker = cronWorker + user;
  const durable = cronDo + user;
  return {
    cronShare: Math.round((1000 * cronDo) / (cronDo + user)),
    worker, durable, ...price(worker, durable, i.venues),
  };
}

export async function collect(ctx) {
  const i = inputs(ctx.results || {}, ctx.hosts || []);
  const m = model(i);
  const cf = await measure(ctx);
  const src = `modelled: ${i.venues} venues (${i.venuesFrom}); ${i.ordersPerDay} orders/day (${i.ordersFrom}); ${i.perOrder} requests/order (${i.perOrderFrom})`;
  const out = [
    ind('cost.modelled_worker_requests_day', m.worker, 'requests', 'trend', src),
    ind('cost.modelled_do_requests_day', m.durable, 'requests', 'trend', src),
    ind('cost.cron_share_permille', m.cronShare, 'permille', 'max', `${src}; F18 closes it`, { limit: 500 }),
    ...cf.out,
    unverified('cost.plan', 'plan', 'trend', 'Cloudflare dashboard', 'Free vs Paid is not in the analytics dataset the token reads'),
  ];
  const p = cf.m ? price(cf.m.workerRequests, cf.m.doRequests, i.venues) : m;
  const psrc = cf.m ? `MEASURED: the last 24 h of Worker and object requests (cf.*) x 30, over ${i.venues} venues` : src;
  const worker = cf.m ? cf.m.workerRequests : m.worker;
  out.push(
    ind('cost.marginal_venue_month_micro_usd', p.marginal, 'µ$', 'max', psrc, { limit: MARGINAL_MAX }),
    ind('cost.all_in_venue_month_micro_usd', p.allIn, 'µ$', 'trend', `${psrc}; + $5 plan + $11.20/yr domain, shared`),
    ind('cost.logs_month', Math.round((worker * DAYS * LOG_SAMPLING_PER_MILLE) / 1000), 'lines', 'max',
      `${cf.m ? 'measured' : 'modelled'} worker requests/day x 30 x head_sampling_rate 0.1`, { limit: LOGS_FREE_MONTH }),
  );
  if (cf.m) {
    out.push(ind('cost.do_measured_over_modelled_permille', Math.round((1000 * cf.m.doRequests) / m.durable), 'permille', 'max',
      'cf.do_requests_day / cost.modelled_do_requests_day', { limit: 1200 }));
  }
  return out;
}
