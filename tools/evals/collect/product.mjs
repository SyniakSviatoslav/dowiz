// SUITE nightly · PRODUCT: per venue, per window, never per person (§B.1.1, OD-8).
//
// The orders the console already reads (GET /api/owner/orders) are folded into DISTRIBUTIONS and
// dropped: the report holds p50/p90 seconds and counts, never an id, a name, a phone or a courier.
// Booking conversion is counts by status over the last seven days (GET /api/owner/reservations).
import { getJson, ownerToken, slugOf, key } from './http.mjs';
import { ind, pct, unverified } from '../rules.mjs';

export const TERMINAL = ['DELIVERED', 'PICKED_UP', 'REJECTED', 'CANCELLED', 'COMPENSATED_REFUND'];
export const STAGES = [
  ['created', 'CONFIRMED'], ['CONFIRMED', 'PREPARING'], ['PREPARING', 'READY'],
  ['READY', 'IN_DELIVERY'], ['IN_DELIVERY', 'DELIVERED'], ['created', 'DELIVERED'],
];
export const WEEK_MS = 7 * 86_400_000;
export const STUCK_MS = 86_400_000;

const stamp = (o, s) => (s === 'created' ? o.created_at_ms : o.at?.[s]);

/** Seconds between two stamps for every order that has both. */
export function stageSeconds(orders, from, to) {
  const out = [];
  for (const o of orders) {
    const a = stamp(o, from), b = stamp(o, to);
    if (Number.isFinite(a) && Number.isFinite(b) && b >= a) out.push(Math.round((b - a) / 1000));
  }
  return out;
}

/** Orders that left no end state: not terminal, not scheduled, older than a day. */
export function abandoned(orders, now) {
  return orders.filter(o => !TERMINAL.includes(o.status) && o.status !== 'SCHEDULED' && now - o.created_at_ms > STUCK_MS).length;
}

export function countBy(rows, f) {
  const out = {};
  for (const r of rows) { const k = key(String(f(r))); out[k] = (out[k] || 0) + 1; }
  return out;
}

export function orderIndicators(v, orders, now) {
  const p = `product.${v}`;
  const src = `GET /api/owner/orders (${v}), folded to distributions`;
  const week = orders.filter(o => now - o.created_at_ms <= WEEK_MS);
  const out = [ind(`${p}.orders_7d`, week.length, 'orders', 'trend', src, { note: JSON.stringify(countBy(week, o => o.channel)) })];
  for (const [a, b] of STAGES) {
    const s = stageSeconds(orders, a, b);
    const id = `${p}.${key(a.toLowerCase())}_to_${key(b.toLowerCase())}`;
    out.push(ind(`${id}.p50_s`, pct(s, 0.5), 's', 'trend', src, { note: `${s.length} orders` }));
    out.push(ind(`${id}.p90_s`, pct(s, 0.9), 's', a === 'created' && b === 'CONFIRMED' ? 'plus25' : 'trend', src));
  }
  out.push(ind(`${p}.abandoned`, abandoned(orders, now), 'orders', 'ratchet', `${src}: not terminal, not scheduled, > 24 h old`));
  out.push(ind(`${p}.rejected_7d`, week.filter(o => o.status === 'REJECTED').length, 'orders', 'trend', src));
  const delivered = orders.filter(o => o.status === 'DELIVERED');
  const withEta = delivered.filter(o => o.eta !== undefined && o.eta !== null).length;
  out.push(ind(`${p}.eta_coverage_permille`, delivered.length ? Math.round((1000 * withEta) / delivered.length) : null, 'permille', 'trend',
    `${src}: delivered orders that carry an eta`));
  out.push(unverified(`${p}.eta_bias_s`, 's', 'trend', src,
    'the ETA backtest needs the promised time beside DELIVERED; the order record carries `eta` on too few orders and no promise timestamp'));
  return out;
}

export function bookingIndicators(v, rows) {
  const by = countBy(rows, r => r.status);
  const n = rows.length;
  const arrived = (by.SEATED || 0) + (by.COMPLETED || 0) + (by.ARRIVED || 0);
  return [
    ind(`product.${v}.bookings_7d`, n, 'bookings', 'trend', `GET /api/owner/reservations (${v}), last 7 days`, { note: JSON.stringify(by) }),
    ind(`product.${v}.booking_arrived_permille`, n ? Math.round((1000 * arrived) / n) : null, 'permille', 'trend', 'seated or completed / created'),
  ];
}

export async function collect(ctx) {
  const f = ctx.fetch ?? globalThis.fetch;
  const now = ctx.now ? ctx.now() : Date.now();
  const out = [];
  for (const host of ctx.hosts) {
    const v = key(slugOf(host));
    const { token, why } = await ownerToken(f, host, ctx.creds);
    if (!token) { out.push(unverified(`product.${v}.orders_7d`, 'orders', 'trend', 'GET /api/owner/orders', why)); continue; }
    const o = await getJson(f, `${host}/api/owner/orders?since=0`, token);
    const orders = o.json?.orders || [];
    out.push(...orderIndicators(v, orders, now));
    const m = Math.floor(now / 60000);
    const r = await getJson(f, `${host}/api/owner/reservations?from=${m - WEEK_MS / 60000}&to=${m}`, token);
    out.push(...bookingIndicators(v, r.json?.reservations || []));
  }
  return out;
}
