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
//
// WHAT THE GAP JUDGES (W-EVALFIX 2026-10-08). The model predicts the requests the PLATFORM makes
// on its own -- each order's alarm run, each venue's nightly, the platform's fan-out -- and the
// requests an order makes. It has no driver for the ones PEOPLE make without ordering (an owner's
// console socket, a storefront browsed, our own probes): on 2026-10-08, 0 orders in 7 days, those
// were 1,565 of 3,028 object requests. So the judged gap is the ALARMS' (cf.do_alarms_day over the
// modelled timer requests: a stuck or minute-cadence alarm is exactly what it must catch -- it
// caught qa-durres' 1,449), and the people's requests are reported beside it, never folded into a
// ratio over a model that cannot see them. The timer share is the MEASURED one when it exists.
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
/** Object requests one alarm run costs when an order's message is sent (EST from the code,
 * 2026-09-28: the alarm, the runner, the drain's outbox read + lease + re-read + settings +
 * verdict write, the till link's tick). The minute cron that cost 2 x 1,440 per venue is gone. */
export const TIMER_DO_PER_ORDER = 10;

const val = (r, id) => (r[id] && Number.isFinite(r[id].value) ? r[id].value : null);

/** The model's inputs, each with where it came from. */
export function inputs(results, hosts, cf = null) {
  const venuesMeasured = val(results, 'platform.venues');
  const objects = cf && cf.venueObjects > 0 ? cf.venueObjects : null;
  const venues = venuesMeasured ?? objects ?? hosts.length;
  const weekly = Object.keys(results).filter(k => /^product\.[^.]+\.orders_7d$/.test(k)).map(k => val(results, k)).filter(v => v !== null);
  const ordersPerDay = weekly.length ? Math.round(weekly.reduce((a, b) => a + b, 0) / 7) : 0;
  const perOrderMeasured = val(results, 'order.requests_total');
  return {
    venues: Math.max(1, venues),
    venuesFrom: venuesMeasured !== null ? 'GET /api/platform/health' : objects !== null ? 'cf.venue_objects_day (venue objects invoked)' : `the ${hosts.length} hosts probed`,
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

/** Venues the platform object pings per alarm turn of the night (`cron::timer::FAN_BATCH`). */
export const FAN_BATCH = 25;

/** Modelled requests per day and their price. TIMERS, NOT A MINUTE CRON (DAG Phase 2): each
 * order's alarm run, each venue's night (one alarm), and the platform object's fan-out (one
 * alarm per FAN_BATCH venues); one scheduled firing a day. `cronShare` is null when the model
 * holds no user request at all (no orders): a share of nothing but timers is 1000 by definition. */
export function model(i) {
  const cronDo = i.ordersPerDay * TIMER_DO_PER_ORDER + i.venues + Math.ceil(i.venues / FAN_BATCH);
  const cronWorker = 1;
  const user = i.ordersPerDay * i.perOrder; // each: one Worker request and one object request
  const worker = cronWorker + user;
  const durable = cronDo + user;
  return {
    cronDo, user,
    cronShare: user > 0 ? Math.round((1000 * cronDo) / (cronDo + user)) : null,
    worker, durable, ...price(worker, durable, i.venues),
  };
}

export async function collect(ctx) {
  const cf = await measure(ctx);
  const i = inputs(ctx.results || {}, ctx.hosts || [], cf.m);
  const m = model(i);
  const src = `modelled: ${i.venues} venues (${i.venuesFrom}); ${i.ordersPerDay} orders/day (${i.ordersFrom}); ${i.perOrder} requests/order (${i.perOrderFrom})`;
  const out = [
    ind('cost.modelled_worker_requests_day', m.worker, 'requests', 'trend', src),
    ind('cost.modelled_do_requests_day', m.durable, 'requests', 'trend', src),
    cf.m && cf.m.doRequests > 0
      ? ind('cost.cron_share_permille', Math.round((1000 * cf.m.doAlarms) / cf.m.doRequests), 'permille', 'max',
        'MEASURED: cf.do_alarms_day / cf.do_requests_day (object requests the platform\'s own timers made)', { limit: 500, note: `${cf.m.doAlarms}/${cf.m.doRequests}; modelled ${m.cronShare ?? 'n/a'}` })
      : m.cronShare === null
        ? unverified('cost.cron_share_permille', 'permille', 'max', src, 'no measured object requests and no order in the model: a share of timers alone is 1000 by definition')
        : ind('cost.cron_share_permille', m.cronShare, 'permille', 'max', `${src}; F18 closes it`, { limit: 500 }),
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
    out.push(
      ind('cost.do_measured_over_modelled_permille', Math.round((1000 * cf.m.doAlarms) / m.cronDo), 'permille', 'max',
        `cf.do_alarms_day / the modelled timer requests (${i.ordersPerDay} orders x ${TIMER_DO_PER_ORDER} + ${i.venues} nights + ${m.cronDo - i.ordersPerDay * TIMER_DO_PER_ORDER - i.venues} fan-out)`,
        { limit: 1200, note: `${cf.m.doAlarms}/${m.cronDo}; ${src}` }),
      ind('cost.do_people_requests_day', cf.m.doRequests - cf.m.doAlarms, 'requests', 'trend',
        'cf.do_requests_day - cf.do_alarms_day: object requests people and probes made (consoles, sockets, browsing, orders)',
        { note: `the model's order traffic: ${m.user} (${i.ordersPerDay} orders x ${i.perOrder}); the rest has no driver in the model` }),
    );
  }
  return out;
}
